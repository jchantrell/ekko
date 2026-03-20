import logging
from datetime import datetime, timezone

from graphiti_core import Graphiti
from graphiti_core.cross_encoder.openai_reranker_client import OpenAIRerankerClient
from graphiti_core.edges import EntityEdge
from graphiti_core.embedder.openai import OpenAIEmbedder, OpenAIEmbedderConfig
from graphiti_core.llm_client.config import LLMConfig
from graphiti_core.llm_client.openai_client import OpenAIClient
from graphiti_core.nodes import EpisodeType, EpisodicNode
from graphiti_core.search.search_config_recipes import NODE_HYBRID_SEARCH_RRF
from graphiti_core.search.search_filters import SearchFilters
from graphiti_core.utils.maintenance.graph_data_operations import clear_data

import driver as drv
from formatting import format_edge, format_node
from ingestion import QueueService

logger = logging.getLogger("ekko-shim")

queue = QueueService()


async def do_init(params: dict) -> dict:
    db = params["database"]
    llm = params["llm"]
    emb = params["embedder"]

    model = llm.get("model", "qwen3:8b")
    llm_config = LLMConfig(
        api_key=llm.get("api_key", "ollama"),
        base_url=llm.get("api_url", "http://localhost:11434/v1"),
        model=model,
        small_model=model,
    )
    llm_client = OpenAIClient(config=llm_config)

    emb_config = OpenAIEmbedderConfig(
        api_key=emb.get("api_key", "ollama"),
        base_url=emb.get("api_url", "http://localhost:11434/v1"),
        embedding_model=emb.get("model", "qwen3-embedding:8b"),
        embedding_dim=emb.get("dimensions", 4096),
    )
    embedder = OpenAIEmbedder(config=emb_config)

    cross_encoder = OpenAIRerankerClient(config=llm_config)

    c = Graphiti(
        uri=db.get("uri", "bolt://localhost:7687"),
        user=db.get("user", "neo4j"),
        password=db.get("password", ""),
        llm_client=llm_client,
        embedder=embedder,
        cross_encoder=cross_encoder,
    )

    await c.build_indices_and_constraints()
    drv.set_client(c)
    logger.info("graphiti client initialized (neo4j)")
    return {"message": "ok"}


async def do_add_memory(params: dict) -> dict:
    group_id = params.get("group_id", "default")
    name = params.get("name", "")
    content = params.get("episode_body", "")
    source = params.get("source", "text")
    source_description = params.get("source_description", "")
    uuid = params.get("uuid")
    sync = params.get("sync", False)
    ref_time_str = params.get("reference_time")

    if ref_time_str:
        try:
            ref_time = datetime.fromisoformat(ref_time_str)
        except (ValueError, TypeError):
            ref_time = datetime.now(timezone.utc)
    else:
        ref_time = datetime.now(timezone.utc)

    try:
        episode_type = EpisodeType[source.lower()]
    except (KeyError, AttributeError):
        episode_type = EpisodeType.text

    async def process():
        logger.info("processing episode %s for group %s", uuid, group_id)
        await drv.client.add_episode(
            name=name,
            episode_body=content,
            source_description=source_description,
            source=episode_type,
            group_id=group_id,
            reference_time=ref_time,
            uuid=uuid,
        )
        logger.info("episode processed for group %s", group_id)

    if sync:
        await process()
        return {"message": f"Episode '{name}' processed in group '{group_id}'"}

    await queue.enqueue(group_id, name, process)
    return {"message": f"Episode '{name}' queued for processing in group '{group_id}'"}


async def do_search_facts(params: dict) -> dict:
    group_ids = params.get("group_ids") or []
    query = params["query"]
    max_facts = params.get("max_facts") or 10
    center_node_uuid = params.get("center_node_uuid")

    edges = await drv.client.search(
        group_ids=group_ids,
        query=query,
        num_results=max_facts,
        center_node_uuid=center_node_uuid,
    )

    facts = [format_edge(e) for e in (edges or [])]
    return {
        "message": "Facts retrieved successfully" if facts else "No relevant facts found",
        "facts": facts,
    }


async def do_search_nodes(params: dict) -> dict:
    group_ids = params.get("group_ids") or []
    query = params["query"]
    max_nodes = params.get("max_nodes") or 10
    entity_types = params.get("entity_types")

    search_filters = SearchFilters(node_labels=entity_types)
    results = await drv.client.search_(
        query=query,
        config=NODE_HYBRID_SEARCH_RRF,
        group_ids=group_ids,
        search_filter=search_filters,
    )

    nodes = results.nodes[:max_nodes] if results.nodes else []
    return {
        "message": "Nodes retrieved successfully" if nodes else "No relevant nodes found",
        "nodes": [format_node(n) for n in nodes],
    }


async def do_get_episodes(params: dict) -> dict:
    group_ids = params.get("group_ids") or []
    max_episodes = params.get("max_episodes") or 10

    episodes = []
    if group_ids:
        episodes = await EpisodicNode.get_by_group_ids(
            drv.client.driver, group_ids, limit=max_episodes
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


async def do_get_entity_edge(params: dict) -> dict:
    edge = await EntityEdge.get_by_uuid(drv.client.driver, params["uuid"])
    return format_edge(edge)


async def do_delete_entity_edge(params: dict) -> dict:
    edge = await EntityEdge.get_by_uuid(drv.client.driver, params["uuid"])
    await edge.delete(drv.client.driver)
    return {"message": f"Entity edge {params['uuid']} deleted"}


async def do_delete_episode(params: dict) -> dict:
    node = await EpisodicNode.get_by_uuid(drv.client.driver, params["uuid"])
    await node.delete(drv.client.driver)
    return {"message": f"Episode {params['uuid']} deleted"}


async def do_clear_graph(params: dict) -> dict:
    group_ids = params.get("group_ids") or []
    if not group_ids:
        return {"message": "No group IDs specified"}
    await clear_data(drv.client.driver, group_ids=group_ids)
    return {"message": f"Graph cleared for groups: {', '.join(group_ids)}"}


async def do_health(_params: dict) -> dict:
    return {"status": "ok" if drv.client is not None else "not_initialized"}


async def do_status(_params: dict) -> dict:
    if drv.client is None:
        return {"status": "error", "message": "not initialized"}
    try:
        async with drv.client.driver.session() as session:
            result = await session.run("MATCH (n) RETURN count(n) as count")
            if result:
                _ = [record async for record in result]
        return {"status": "ok", "message": "connected to graph database"}
    except Exception as e:
        return {"status": "error", "message": str(e)}


async def do_queue_status(_params: dict) -> dict:
    return {"groups": queue.status()}


async def do_shutdown(_params: dict) -> dict:
    if drv.client is not None:
        await drv.client.close()
    return {"message": "ok"}
