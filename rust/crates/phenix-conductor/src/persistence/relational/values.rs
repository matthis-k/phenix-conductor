fn materialize_value_node(
    node_id: i64,
    nodes: &[StoredValueNode],
    visited: &mut BTreeSet<i64>,
) -> Result<Value, PersistenceError> {
    if !visited.insert(node_id) {
        return Err(invalid("structured value contains a cycle"));
    }
    let node = nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| invalid("structured value references a missing node"))?;
    let scalar = |name: &str| {
        node.scalar
            .clone()
            .ok_or_else(|| invalid(format!("{name} node has no scalar value")))
    };
    match node.kind.as_str() {
        "null" => Ok(Value::Null),
        "boolean" => match scalar("boolean")?.as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            other => Err(invalid(format!("invalid persisted boolean: {other}"))),
        },
        "number" => Ok(Value::Number(
            scalar("number")?
                .parse::<Number>()
                .map_err(|_| invalid("invalid persisted number"))?,
        )),
        "string" => Ok(Value::String(scalar("string")?)),
        "array" => {
            let mut children = nodes
                .iter()
                .filter(|child| child.parent == Some(node_id))
                .map(|child| {
                    child
                        .array_index
                        .ok_or_else(|| invalid("array child has no index"))
                        .map(|index| (index, child.id))
                })
                .collect::<Result<Vec<_>, _>>()?;
            children.sort_by_key(|(index, _)| *index);
            for (expected, (actual, _)) in children.iter().enumerate() {
                if *actual != sql_usize(expected, "structured array index")? {
                    return Err(invalid("structured array indexes are not contiguous"));
                }
            }
            Ok(Value::Array(
                children
                    .into_iter()
                    .map(|(_, child)| materialize_value_node(child, nodes, visited))
                    .collect::<Result<_, _>>()?,
            ))
        }
        "object" => {
            let mut children = nodes
                .iter()
                .filter(|child| child.parent == Some(node_id))
                .map(|child| {
                    child
                        .object_key
                        .clone()
                        .ok_or_else(|| invalid("object child has no key"))
                        .map(|key| (key, child.id))
                })
                .collect::<Result<Vec<_>, _>>()?;
            children.sort_by(|left, right| left.0.cmp(&right.0));
            let mut object = Map::new();
            for (key, child) in children {
                object.insert(key, materialize_value_node(child, nodes, visited)?);
            }
            Ok(Value::Object(object))
        }
        other => Err(invalid(format!(
            "unknown structured value node kind: {other}"
        ))),
    }
}
