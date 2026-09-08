struct PreparedDelivery {
    entry: Arc<SubscriptionEntry>,
    changes: SmallVec<[ValueChange; 8]>,
}

struct PreparedRoot<'roots> {
    commit: CommitId,
    value: &'roots ValueId,
    from_version: ValueVersion,
    version: ValueVersion,
    snapshot: Option<Arc<PhenixValue>>,
    deliveries: SmallVec<[PreparedDelivery; 8]>,
}

fn changes_for_subscription(
    spec: &ObservationSpec,
    changes: &[ValueChange],
    current_root: &PhenixValue,
) -> SmallVec<[ValueChange; 8]> {
    let mut output = SmallVec::new();
    let mut ancestor_changed = false;
    for change in changes {
        let path = change.path();
        let matches = match spec.scope {
            ObservationScope::Exact => {
                path == &spec.address.path || spec.address.path.starts_with(path)
            }
            ObservationScope::Recursive => {
                path.starts_with(&spec.address.path) || spec.address.path.starts_with(path)
            }
        };
        if !matches {
            continue;
        }
        if spec.address.path.starts_with(path) && path != &spec.address.path {
            ancestor_changed = true;
            continue;
        }
        if let Some(relative) = path.relative_to(&spec.address.path) {
            output.push(rebase_change(change, relative));
        }
    }
    if ancestor_changed {
        output.clear();
        match value_at_optional(current_root, spec.address.path.segments()) {
            Some(value) => output.push(ValueChange::Replace {
                path: ValuePath::root(),
                value: value.clone(),
            }),
            None => output.push(ValueChange::Remove {
                path: ValuePath::root(),
            }),
        }
    }
    output
}

fn rebase_change(change: &ValueChange, path: ValuePath) -> ValueChange {
    match change {
        ValueChange::Replace { value, .. } => ValueChange::Replace {
            path,
            value: value.clone(),
        },
        ValueChange::Remove { .. } => ValueChange::Remove { path },
        ValueChange::Splice {
            start,
            delete_count,
            inserted,
            ..
        } => ValueChange::Splice {
            path,
            start: *start,
            delete_count: *delete_count,
            inserted: inserted.clone(),
        },
    }
}

fn apply_undo(state: &mut StoreState, undo: UndoRecord<'_>) -> Result<(), String> {
    match undo {
        UndoRecord::Replace {
            value,
            path,
            previous,
        } => {
            let registered = state
                .values
                .get_mut(value)
                .ok_or_else(|| format!("observable value {value} disappeared during rollback"))?;
            restore_optional(&mut registered.current, path.segments(), previous)
        }
        UndoRecord::Splice {
            value,
            path,
            start,
            inserted_count,
            removed,
        } => {
            let registered = state
                .values
                .get_mut(value)
                .ok_or_else(|| format!("observable value {value} disappeared during rollback"))?;
            let list = value_at_mut(&mut registered.current, path.segments())?;
            let PhenixValue::List(items) = list else {
                return Err("rollback splice target is not a list".to_owned());
            };
            let end = start.saturating_add(inserted_count).min(items.len());
            items.splice(start..end, removed);
            Ok(())
        }
    }
}

fn validate_path<'schema>(
    value: &ValueId,
    schema: &'schema PhenixSchema,
    path: &ValuePath,
) -> Result<&'schema PhenixSchema, ObservableError> {
    schema_at_path(schema, path.segments()).map_err(|message| ObservableError::InvalidPath {
        value: Box::new(value.clone()),
        path: Box::new(path.clone()),
        message: message.into(),
    })
}

fn schema_at_path<'schema>(
    schema: &'schema PhenixSchema,
    path: &[ValuePathSegment],
) -> Result<&'schema PhenixSchema, String> {
    let Some((head, tail)) = path.split_first() else {
        return Ok(schema);
    };
    match (schema, head) {
        (Type::Table(fields), ValuePathSegment::Field(key)) => fields
            .get(key)
            .ok_or_else(|| format!("table has no field {key}"))
            .and_then(|schema| schema_at_path(schema, tail)),
        (Type::Map(item), ValuePathSegment::MapKey(_)) => schema_at_path(item, tail),
        (Type::List(item), ValuePathSegment::Index(_)) => schema_at_path(item, tail),
        (Type::Array { item, len }, ValuePathSegment::Index(index)) => {
            if usize::try_from(*index).map_or(true, |index| index >= *len) {
                return Err(format!("array index {index} exceeds fixed length {len}"));
            }
            schema_at_path(item, tail)
        }
        (Type::Option(item), ValuePathSegment::OptionPayload) => schema_at_path(item, tail),
        (Type::Variant(variants), ValuePathSegment::VariantPayload) => {
            let mut resolved = None;
            for variant in variants.values() {
                let candidate = schema_at_path(variant, tail)?;
                match resolved {
                    Some(previous) if previous != candidate => {
                        return Err(
                            "variant payload path is not structurally identical for every variant"
                                .to_owned(),
                        );
                    }
                    Some(_) => {}
                    None => resolved = Some(candidate),
                }
            }
            resolved.ok_or_else(|| "variant has no payload schemas".to_owned())
        }
        _ => Err(format!(
            "path segment {head:?} is invalid for schema {}",
            schema.kind()
        )),
    }
}

fn value_at<'value>(
    value: &'value PhenixValue,
    path: &[ValuePathSegment],
) -> Result<&'value PhenixValue, String> {
    value_at_optional(value, path).ok_or_else(|| "addressed observable value is absent".to_owned())
}

fn value_at_optional<'value>(
    value: &'value PhenixValue,
    path: &[ValuePathSegment],
) -> Option<&'value PhenixValue> {
    let Some((head, tail)) = path.split_first() else {
        return Some(value);
    };
    let next = match (value, head) {
        (PhenixValue::Table(fields), ValuePathSegment::Field(key)) => fields.get(key)?,
        (PhenixValue::Map(values), ValuePathSegment::MapKey(key)) => values.get(key)?,
        (PhenixValue::List(values), ValuePathSegment::Index(index)) => {
            values.get(usize::try_from(*index).ok()?)?
        }
        (PhenixValue::Option(Some(value)), ValuePathSegment::OptionPayload) => value,
        (PhenixValue::Variant { value, .. }, ValuePathSegment::VariantPayload) => value,
        _ => return None,
    };
    value_at_optional(next, tail)
}

fn value_at_mut<'value>(
    value: &'value mut PhenixValue,
    path: &[ValuePathSegment],
) -> Result<&'value mut PhenixValue, String> {
    let Some((head, tail)) = path.split_first() else {
        return Ok(value);
    };
    let next = match (value, head) {
        (PhenixValue::Table(fields), ValuePathSegment::Field(key)) => fields
            .get_mut(key)
            .ok_or_else(|| format!("table field {key} is absent"))?,
        (PhenixValue::Map(values), ValuePathSegment::MapKey(key)) => values
            .get_mut(key)
            .ok_or_else(|| format!("map key {key} is absent"))?,
        (PhenixValue::List(values), ValuePathSegment::Index(index)) => values
            .get_mut(*index as usize)
            .ok_or_else(|| format!("list index {index} is absent"))?,
        (PhenixValue::Option(Some(value)), ValuePathSegment::OptionPayload) => value,
        (PhenixValue::Variant { value, .. }, ValuePathSegment::VariantPayload) => value,
        _ => return Err(format!("path segment {head:?} does not match current value")),
    };
    value_at_mut(next, tail)
}

fn replace_at(
    value: &mut PhenixValue,
    path: &[ValuePathSegment],
    replacement: PhenixValue,
) -> Result<Option<PhenixValue>, String> {
    let Some((head, tail)) = path.split_first() else {
        return Ok(Some(std::mem::replace(value, replacement)));
    };
    if tail.is_empty() {
        return match (value, head) {
            (PhenixValue::Table(fields), ValuePathSegment::Field(key)) => fields
                .get_mut(key)
                .map(|slot| Some(std::mem::replace(slot, replacement)))
                .ok_or_else(|| format!("table field {key} is absent")),
            (PhenixValue::Map(values), ValuePathSegment::MapKey(key)) => {
                Ok(values.insert(key.clone(), replacement))
            }
            (PhenixValue::List(values), ValuePathSegment::Index(index)) => values
                .get_mut(*index as usize)
                .map(|slot| Some(std::mem::replace(slot, replacement)))
                .ok_or_else(|| format!("list index {index} is absent")),
            (PhenixValue::Option(option), ValuePathSegment::OptionPayload) => {
                Ok(option.replace(Box::new(replacement)).map(|value| *value))
            }
            (PhenixValue::Variant { value, .. }, ValuePathSegment::VariantPayload) => {
                Ok(Some(std::mem::replace(value, Box::new(replacement))).map(|value| *value))
            }
            _ => Err(format!("path segment {head:?} does not match current value")),
        };
    }
    let next = value_at_mut(value, std::slice::from_ref(head))?;
    replace_at(next, tail, replacement)
}

fn remove_at(
    value: &mut PhenixValue,
    path: &[ValuePathSegment],
) -> Result<Option<PhenixValue>, String> {
    let Some((head, tail)) = path.split_first() else {
        return Err("registered roots cannot be removed".to_owned());
    };
    if tail.is_empty() {
        return match (value, head) {
            (PhenixValue::Map(values), ValuePathSegment::MapKey(key)) => Ok(values.remove(key)),
            (PhenixValue::Option(option), ValuePathSegment::OptionPayload) => {
                Ok(option.take().map(|value| *value))
            }
            _ => Err(format!("path segment {head:?} is not removable")),
        };
    }
    let next = value_at_mut(value, std::slice::from_ref(head))?;
    remove_at(next, tail)
}

fn restore_optional(
    value: &mut PhenixValue,
    path: &[ValuePathSegment],
    previous: Option<PhenixValue>,
) -> Result<(), String> {
    let Some((head, tail)) = path.split_first() else {
        let previous = previous.ok_or_else(|| "cannot remove registered root during rollback".to_owned())?;
        *value = previous;
        return Ok(());
    };
    if tail.is_empty() {
        return match (value, head, previous) {
            (PhenixValue::Table(fields), ValuePathSegment::Field(key), Some(previous)) => {
                fields.insert(key.clone(), previous);
                Ok(())
            }
            (PhenixValue::Map(values), ValuePathSegment::MapKey(key), previous) => {
                if let Some(previous) = previous {
                    values.insert(key.clone(), previous);
                } else {
                    values.remove(key);
                }
                Ok(())
            }
            (PhenixValue::List(values), ValuePathSegment::Index(index), Some(previous)) => {
                let slot = values
                    .get_mut(*index as usize)
                    .ok_or_else(|| format!("rollback list index {index} is absent"))?;
                *slot = previous;
                Ok(())
            }
            (PhenixValue::Option(option), ValuePathSegment::OptionPayload, previous) => {
                *option = previous.map(Box::new);
                Ok(())
            }
            (PhenixValue::Variant { value, .. }, ValuePathSegment::VariantPayload, Some(previous)) => {
                **value = previous;
                Ok(())
            }
            _ => Err(format!("cannot restore path segment {head:?}")),
        };
    }
    let next = value_at_mut(value, std::slice::from_ref(head))?;
    restore_optional(next, tail, previous)
}
