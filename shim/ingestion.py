import asyncio
import logging
from collections.abc import Awaitable, Callable

logger = logging.getLogger("ekko-shim")

MAX_CONCURRENT = 1


class QueueService:
    def __init__(self):
        self._queues: dict[str, asyncio.Queue] = {}
        self._workers: dict[str, bool] = {}
        self._processing: dict[str, str | None] = {}
        self._semaphore = asyncio.Semaphore(MAX_CONCURRENT)

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
                    async with self._semaphore:
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
