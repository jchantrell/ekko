import json
import logging
import sqlite3
import asyncio
from datetime import datetime, timezone
from pathlib import Path

logger = logging.getLogger("ekko-shim")

DB_PATH = Path(__file__).resolve().parent.parent / "queue.db"


class QueueService:
    def __init__(self):
        self._conn = sqlite3.connect(str(DB_PATH))
        self._conn.row_factory = sqlite3.Row
        self._migrate()
        self._processing: str | None = None
        self._processing_origin: str | None = None
        self._worker_task: asyncio.Task | None = None

    def _migrate(self):
        self._conn.executescript(
            "CREATE TABLE IF NOT EXISTS queue ("
            "  id INTEGER PRIMARY KEY AUTOINCREMENT,"
            "  origin TEXT NOT NULL,"
            "  name TEXT NOT NULL,"
            "  params TEXT NOT NULL,"
            "  created_at TEXT NOT NULL,"
            "  status TEXT NOT NULL DEFAULT 'pending'"
            ");"
            "UPDATE queue SET status = 'pending' WHERE status = 'processing';"
        )
        self._conn.commit()

    def enqueue(self, origin: str, name: str, params: dict) -> int:
        now = datetime.now(timezone.utc).isoformat()
        self._conn.execute(
            "INSERT INTO queue (origin, name, params, created_at) VALUES (?, ?, ?, ?)",
            (origin, name, json.dumps(params), now),
        )
        self._conn.commit()
        self._ensure_worker()
        count = self._conn.execute(
            "SELECT COUNT(*) FROM queue WHERE status = 'pending'"
        ).fetchone()[0]
        return count

    def _ensure_worker(self):
        if self._worker_task is None or self._worker_task.done():
            self._worker_task = asyncio.create_task(self._worker())

    def start_if_pending(self):
        count = self._conn.execute(
            "SELECT COUNT(*) FROM queue WHERE status = 'pending'"
        ).fetchone()[0]
        if count > 0:
            logger.info("resuming %d pending episodes from previous session", count)
            self._ensure_worker()

    async def _worker(self):
        while True:
            row = self._conn.execute(
                "SELECT * FROM queue WHERE status = 'pending' ORDER BY id LIMIT 1"
            ).fetchone()
            if not row:
                break

            self._conn.execute(
                "UPDATE queue SET status = 'processing' WHERE id = ?", (row["id"],)
            )
            self._conn.commit()
            self._processing = row["name"]
            self._processing_origin = row["origin"]

            try:
                params = json.loads(row["params"])
                await self._process(params)
                self._conn.execute("DELETE FROM queue WHERE id = ?", (row["id"],))
                self._conn.commit()
                logger.info("episode processed: %s (%s)", row["name"], row["origin"])
            except Exception:
                logger.exception("episode failed: %s (%s)", row["name"], row["origin"])
                self._conn.execute(
                    "UPDATE queue SET status = 'failed' WHERE id = ?", (row["id"],)
                )
                self._conn.commit()
            finally:
                self._processing = None
                self._processing_origin = None

    async def _process(self, params: dict):
        import driver as drv
        from graphiti_core.nodes import EpisodeType

        try:
            episode_type = EpisodeType[params["source"].lower()]
        except (KeyError, AttributeError):
            episode_type = EpisodeType.text

        ref_time = datetime.fromisoformat(params["reference_time"])

        await drv.client.add_episode(
            name=params["name"],
            episode_body=params["content"],
            source_description=params.get("source_description", ""),
            source=episode_type,
            group_id=params["group_id"],
            reference_time=ref_time,
            uuid=params.get("uuid"),
        )

    def status(self) -> list[dict]:
        rows = self._conn.execute(
            "SELECT origin, "
            "  SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending "
            "FROM queue WHERE status IN ('pending', 'processing') "
            "GROUP BY origin"
        ).fetchall()

        result = []
        for row in rows:
            result.append({
                "group_id": row["origin"],
                "processing": self._processing if row["origin"] == self._processing_origin else None,
                "pending": row["pending"],
            })

        if self._processing_origin and not any(
            r["group_id"] == self._processing_origin for r in result
        ):
            result.append({
                "group_id": self._processing_origin,
                "processing": self._processing,
                "pending": 0,
            })

        return sorted(result, key=lambda r: r["group_id"])
