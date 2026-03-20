import logging

logger = logging.getLogger("ekko-shim")

client = None


def set_client(c):
    global client
    client = c


def parse_uri(uri: str) -> tuple[str, int]:
    """Extract host and port from a URI like bolt://localhost:7687."""
    uri = uri.split("://", 1)[-1]
    if ":" in uri:
        host, port_str = uri.rsplit(":", 1)
        return host, int(port_str)
    return uri, 7687
