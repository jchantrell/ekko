def format_edge(edge) -> dict:
    result = edge.model_dump(mode="json", exclude={"fact_embedding"})
    result.get("attributes", {}).pop("fact_embedding", None)
    return result


def format_node(node) -> dict:
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
