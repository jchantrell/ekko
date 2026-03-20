"""ekko <-> graphiti-core bridge. Speaks JSONL on stdin/stdout."""

import asyncio
import json
import logging
import sys

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(name)s: %(message)s",
    stream=sys.stderr,
)
logging.getLogger("neo4j").setLevel(logging.WARNING)

logger = logging.getLogger("ekko-shim")

from handlers import (
    do_init,
    do_add_memory,
    do_search_facts,
    do_search_nodes,
    do_get_episodes,
    do_get_entity_edge,
    do_delete_entity_edge,
    do_delete_episode,
    do_clear_graph,
    do_health,
    do_status,
    do_queue_status,
    do_shutdown,
)

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
