import logging

from graphiti_core.driver.driver import GraphProvider

logger = logging.getLogger("ekko-shim")

_default_database = "default_db"
client = None


def set_client(c):
    global client
    client = c


def set_default_database(db: str):
    global _default_database
    _default_database = db


def parse_uri(uri: str) -> tuple[str, int]:
    """Extract host and port from a URI like bolt://localhost:6379."""
    uri = uri.split("://", 1)[-1]
    if ":" in uri:
        host, port_str = uri.rsplit(":", 1)
        return host, int(port_str)
    return uri, 6379


def ensure_driver_for_groups(group_ids: list[str]) -> None:
    """For FalkorDB, clone the driver to the target group's database.

    FalkorDB stores each group_id in a separate graph. The
    handle_multiple_group_ids decorator only clones for len > 1.
    For a single group_id we must clone explicitly.
    """
    if (
        client is not None
        and client.driver.provider == GraphProvider.FALKORDB
        and len(group_ids) == 1
        and group_ids[0] != client.driver._database
    ):
        client.driver = client.driver.clone(database=group_ids[0])
        client.clients.driver = client.driver
