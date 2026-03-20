"""ekko <-> graphiti-core bridge. Speaks JSONL on stdin/stdout."""

import asyncio
import json
import logging
import os
import signal
import sys
from collections.abc import Awaitable, Callable
from datetime import datetime, timezone

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(name)s: %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger("ekko-shim")

# Suppress noisy index-already-exists warnings from FalkorDB driver
logging.getLogger("graphiti_core.driver.falkordb_driver").setLevel(logging.WARNING)

# ---------------------------------------------------------------------------
# Queue service (ported from upstream mcp_server/src/services/queue_service.py)
# ---------------------------------------------------------------------------


class QueueService:
    def __init__(self):
        self._queues: dict[str, asyncio.Queue] = {}
        self._workers: dict[str, bool] = {}
        self._processing: dict[str, str | None] = {}

    async def enqueue(
        self, group_id: str, name: str, func: Callable[[], Awaitable[None]]
    ) -> int:
        if group_id not in self._queues:
            self._queues[group_id] = asyncio.Queue()
        await self._queues[group_id].put((name, func))
        if not self._workers.get(group_id, False):
            asyncio.create_task(self._worker(group_id))
        return self._queues[group_id].qsize()

    async def _worker(self, group_id: str) -> None:
        self._workers[group_id] = True
        try:
            while True:
                name, func = await self._queues[group_id].get()
                self._processing[group_id] = name
                try:
                    await func()
                except Exception:
                    logger.exception("episode processing failed for %s", group_id)
                finally:
                    self._processing[group_id] = None
                    self._queues[group_id].task_done()
        except asyncio.CancelledError:
            pass
        finally:
            self._workers[group_id] = False
            self._processing.pop(group_id, None)

    def status(self) -> list[dict]:
        groups = set(self._queues.keys()) | set(self._processing.keys())
        result = []
        for gid in sorted(groups):
            pending = self._queues[gid].qsize() if gid in self._queues else 0
            processing = self._processing.get(gid)
            if pending > 0 or processing is not None:
                result.append({
                    "group_id": gid,
                    "processing": processing,
                    "pending": pending,
                })
        return result


# ---------------------------------------------------------------------------
# Graphiti wrapper
# ---------------------------------------------------------------------------

client = None  # graphiti_core.Graphiti
queue = QueueService()
_default_database = "default_db"


async def do_init(params: dict) -> dict:
    global client

    from graphiti_core import Graphiti
    from graphiti_core.llm_client.openai_client import OpenAIClient
    from graphiti_core.llm_client.config import LLMConfig
    from graphiti_core.embedder.openai import OpenAIEmbedder, OpenAIEmbedderConfig
    from graphiti_core.cross_encoder.openai_reranker_client import OpenAIRerankerClient

    db = params["database"]
    llm = params["llm"]
    emb = params["embedder"]

    # LLM client (OpenAI-compatible, pointed at Ollama)
    model = llm.get("model", "llama3.2:3b")
    llm_config = LLMConfig(
        api_key=llm.get("api_key", "ollama"),
        base_url=llm.get("api_url", "http://localhost:11434/v1"),
        model=model,
        small_model=model,
    )
    llm_client = OpenAIClient(config=llm_config)

    # Embedder (OpenAI-compatible, pointed at Ollama)
    emb_config = OpenAIEmbedderConfig(
        api_key=emb.get("api_key", "ollama"),
        base_url=emb.get("api_url", "http://localhost:11434/v1"),
        embedding_model=emb.get("model", "nomic-embed-text"),
        embedding_dim=emb.get("dimensions", 768),
    )
    embedder = OpenAIEmbedder(config=emb_config)

    # Cross-encoder reranker (reuse LLM config so it talks to Ollama)
    cross_encoder = OpenAIRerankerClient(config=llm_config)

    provider = db.get("provider", "falkordb")
    if provider == "falkordb":
        from graphiti_core.driver.falkordb_driver import FalkorDriver

        uri = db.get("uri", "bolt://localhost:6379")
        host, port = _parse_uri(uri)
        driver = FalkorDriver(
            host=host,
            port=port,
            password=db.get("password") or "",
            database=db.get("database") or "default_db",
        )
        client = Graphiti(
            graph_driver=driver,
            llm_client=llm_client,
            embedder=embedder,
            cross_encoder=cross_encoder,
        )
    elif provider == "neo4j":
        client = Graphiti(
            uri=db.get("uri", "bolt://localhost:7687"),
            user=db.get("user", "neo4j"),
            password=db.get("password", ""),
            llm_client=llm_client,
            embedder=embedder,
            cross_encoder=cross_encoder,
        )
    else:
        raise ValueError(f"unsupported database provider: {provider}")

    await client.build_indices_and_constraints()
    global _default_database
    _default_database = client.driver._database
    logger.info("graphiti client initialized (provider=%s)", provider)
    return {"message": "ok"}


def _parse_uri(uri: str) -> tuple[str, int]:
    """Extract host and port from a URI like bolt://localhost:6379."""
    uri = uri.split("://", 1)[-1]
    if ":" in uri:
        host, port_str = uri.rsplit(":", 1)
        return host, int(port_str)
    return uri, 6379


def _ensure_driver_for_groups(group_ids: list[str]) -> None:
    """For FalkorDB, clone the driver to the target group's database.

    FalkorDB stores each group_id in a separate graph. The
    handle_multiple_group_ids decorator only clones for len > 1.
    For a single group_id we must clone explicitly.
    """
    from graphiti_core.driver.driver import GraphProvider

    if (
        client is not None
        and client.driver.provider == GraphProvider.FALKORDB
        and len(group_ids) == 1
        and group_ids[0] != client.driver._database
    ):
        client.driver = client.driver.clone(database=group_ids[0])
        client.clients.driver = client.driver


def _reset_driver() -> None:
    """Reset the driver back to the default database after an operation."""
    from graphiti_core.driver.driver import GraphProvider

    if (
        client is not None
        and client.driver.provider == GraphProvider.FALKORDB
        and client.driver._database != _default_database
    ):
        client.driver = client.driver.clone(database=_default_database)
        client.clients.driver = client.driver


# -- Memory operations -----------------------------------------------------


async def do_add_memory(params: dict) -> dict:
    from graphiti_core.nodes import EpisodeType

    group_id = params.get("group_id", "default")
    name = params.get("name", "")
    content = params.get("episode_body", "")
    source = params.get("source", "text")
    source_description = params.get("source_description", "")
    uuid = params.get("uuid")
    sync = params.get("sync", False)

    try:
        episode_type = EpisodeType[source.lower()]
    except (KeyError, AttributeError):
        episode_type = EpisodeType.text

    async def process():
        logger.info("processing episode %s for group %s", uuid, group_id)
        await client.add_episode(
            name=name,
            episode_body=content,
            source_description=source_description,
            source=episode_type,
            group_id=group_id,
            reference_time=datetime.now(timezone.utc),
            uuid=uuid,
        )
        logger.info("episode processed for group %s", group_id)

    if sync:
        await process()
        return {"message": f"Episode '{name}' processed in group '{group_id}'"}

    await queue.enqueue(group_id, name, process)
    return {"message": f"Episode '{name}' queued for processing in group '{group_id}'"}


# -- Search operations -----------------------------------------------------


async def do_search_facts(params: dict) -> dict:
    group_ids = params.get("group_ids") or []
    query = params["query"]
    max_facts = params.get("max_facts") or 10
    center_node_uuid = params.get("center_node_uuid")

    _ensure_driver_for_groups(group_ids)

    edges = await client.search(
        group_ids=group_ids,
        query=query,
        num_results=max_facts,
        center_node_uuid=center_node_uuid,
    )

    facts = [_format_edge(e) for e in (edges or [])]
    return {
        "message": "Facts retrieved successfully" if facts else "No relevant facts found",
        "facts": facts,
    }


async def do_search_nodes(params: dict) -> dict:
    from graphiti_core.search.search_config_recipes import NODE_HYBRID_SEARCH_RRF
    from graphiti_core.search.search_filters import SearchFilters

    group_ids = params.get("group_ids") or []
    query = params["query"]
    max_nodes = params.get("max_nodes") or 10
    entity_types = params.get("entity_types")

    _ensure_driver_for_groups(group_ids)

    search_filters = SearchFilters(node_labels=entity_types)
    results = await client.search_(
        query=query,
        config=NODE_HYBRID_SEARCH_RRF,
        group_ids=group_ids,
        search_filter=search_filters,
    )

    nodes = results.nodes[:max_nodes] if results.nodes else []
    return {
        "message": "Nodes retrieved successfully" if nodes else "No relevant nodes found",
        "nodes": [_format_node(n) for n in nodes],
    }


# -- Episode operations ----------------------------------------------------


async def do_get_episodes(params: dict) -> dict:
    from graphiti_core.nodes import EpisodicNode

    group_ids = params.get("group_ids") or []
    max_episodes = params.get("max_episodes") or 10

    _ensure_driver_for_groups(group_ids)

    episodes = []
    if group_ids:
        episodes = await EpisodicNode.get_by_group_ids(
            client.driver, group_ids, limit=max_episodes
        )

    return {
        "message": "Episodes retrieved successfully"
        if episodes
        else "No episodes found",
        "episodes": [
            {
                "uuid": ep.uuid,
                "name": ep.name,
                "content": ep.content,
                "created_at": ep.created_at.isoformat() if ep.created_at else None,
                "source": ep.source.value
                if hasattr(ep.source, "value")
                else str(ep.source),
                "source_description": ep.source_description,
                "group_id": ep.group_id,
            }
            for ep in episodes
        ],
    }


# -- Entity edge operations ------------------------------------------------


async def do_get_entity_edge(params: dict) -> dict:
    from graphiti_core.edges import EntityEdge

    edge = await EntityEdge.get_by_uuid(client.driver, params["uuid"])
    return _format_edge(edge)


async def do_delete_entity_edge(params: dict) -> dict:
    from graphiti_core.edges import EntityEdge

    edge = await EntityEdge.get_by_uuid(client.driver, params["uuid"])
    await edge.delete(client.driver)
    return {"message": f"Entity edge {params['uuid']} deleted"}


async def do_delete_episode(params: dict) -> dict:
    from graphiti_core.nodes import EpisodicNode

    node = await EpisodicNode.get_by_uuid(client.driver, params["uuid"])
    await node.delete(client.driver)
    return {"message": f"Episode {params['uuid']} deleted"}


# -- Graph operations ------------------------------------------------------


async def do_clear_graph(params: dict) -> dict:
    from graphiti_core.utils.maintenance.graph_data_operations import clear_data

    group_ids = params.get("group_ids") or []
    if not group_ids:
        return {"message": "No group IDs specified"}
    _ensure_driver_for_groups(group_ids)
    await clear_data(client.driver, group_ids=group_ids)
    return {"message": f"Graph cleared for groups: {', '.join(group_ids)}"}


# -- Health / status -------------------------------------------------------


async def do_health(_params: dict) -> dict:
    return {"status": "ok" if client is not None else "not_initialized"}


async def do_status(_params: dict) -> dict:
    if client is None:
        return {"status": "error", "message": "not initialized"}
    try:
        async with client.driver.session() as session:
            result = await session.run("MATCH (n) RETURN count(n) as count")
            if result:
                _ = [record async for record in result]
        return {"status": "ok", "message": "connected to graph database"}
    except Exception as e:
        return {"status": "error", "message": str(e)}


async def do_queue_status(_params: dict) -> dict:
    return {"groups": queue.status()}


async def do_shutdown(_params: dict) -> dict:
    if client is not None:
        await client.close()
    return {"message": "ok"}


# ---------------------------------------------------------------------------
# Formatting helpers
# ---------------------------------------------------------------------------


def _format_edge(edge) -> dict:
    result = edge.model_dump(mode="json", exclude={"fact_embedding"})
    result.get("attributes", {}).pop("fact_embedding", None)
    return result


def _format_node(node) -> dict:
    attrs = node.attributes if hasattr(node, "attributes") else {}
    attrs = {k: v for k, v in attrs.items() if "embedding" not in k.lower()}
    return {
        "uuid": node.uuid,
        "name": node.name,
        "labels": node.labels or [],
        "created_at": node.created_at.isoformat() if node.created_at else None,
        "summary": node.summary,
        "group_id": node.group_id,
        "attributes": attrs,
    }


# ---------------------------------------------------------------------------
# Dispatch table
# ---------------------------------------------------------------------------

METHODS = {
    "init": do_init,
    "add_memory": do_add_memory,
    "search_facts": do_search_facts,
    "search_nodes": do_search_nodes,
    "get_episodes": do_get_episodes,
    "get_entity_edge": do_get_entity_edge,
    "delete_entity_edge": do_delete_entity_edge,
    "delete_episode": do_delete_episode,
    "clear_graph": do_clear_graph,
    "health": do_health,
    "status": do_status,
    "queue_status": do_queue_status,
    "shutdown": do_shutdown,
}


# ---------------------------------------------------------------------------
# Main loop
# ---------------------------------------------------------------------------


def _write(obj: dict) -> None:
    line = json.dumps(obj, default=str)
    sys.stdout.write(line + "\n")
    sys.stdout.flush()


async def main() -> None:
    loop = asyncio.get_event_loop()
    reader = asyncio.StreamReader()
    protocol = asyncio.StreamReaderProtocol(reader)
    await loop.connect_read_pipe(lambda: protocol, sys.stdin)

    while True:
        line = await reader.readline()
        if not line:
            break

        try:
            req = json.loads(line)
        except json.JSONDecodeError:
            _write({"id": None, "error": {"code": -32700, "message": "parse error"}})
            continue

        req_id = req.get("id")
        method = req.get("method", "")
        params = req.get("params", {})

        handler = METHODS.get(method)
        if handler is None:
            _write(
                {
                    "id": req_id,
                    "error": {"code": -32601, "message": f"unknown method: {method}"},
                }
            )
            continue

        try:
            result = await handler(params)
            _write({"id": req_id, "result": result})
        except Exception as e:
            logger.exception("error handling %s", method)
            _write({"id": req_id, "error": {"code": -1, "message": str(e)}})

        if method == "shutdown":
            break


if __name__ == "__main__":
    asyncio.run(main())
