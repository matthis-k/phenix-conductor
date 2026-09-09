impl<'roots> ObservableTransaction<'_, 'roots> {
    pub fn replace(
        &mut self,
        value: &'roots ValueId,
        path: ValuePath,
        replacement: PhenixValue,
    ) -> Result<(), ObservableError> {
        self.ensure_declared(value)?;
        let registered = self
            .state
            .values
            .get_mut(value)
            .expect("declared observable exists");
        let target_schema = validate_path(value, &registered.schema, &path)?;
        target_schema
            .parse(&replacement)
            .map_err(|error| ObservableError::SchemaMismatch {
                value: Box::new(value.clone()),
                path: Box::new(path.clone()),
                message: error.to_string().into(),
            })?;
        let previous = replace_at(&mut registered.current, path.segments(), replacement)
            .map_err(|message| ObservableError::TransactionConflict {
                value: value.clone(),
                message,
            })?;
        self.undo.push(UndoRecord::Replace {
            value,
            path: path.clone(),
            previous,
        });
        self.touched.push(TouchedMutation {
            value,
            path,
            kind: TouchedKind::Replace,
        });
        Ok(())
    }

    pub fn remove(&mut self, value: &'roots ValueId, path: ValuePath) -> Result<(), ObservableError> {
        self.ensure_declared(value)?;
        if path.is_root() {
            return Err(ObservableError::InvalidPath {
                value: Box::new(value.clone()),
                path: Box::new(path),
                message: "registered roots cannot be removed".into(),
            });
        }
        if let Some(ValuePathSegment::Index(index)) = path.segments().last() {
            let index = *index;
            let mut list_path = path.clone();
            list_path.segments.pop();
            return self.splice(value, list_path, index, 1, std::iter::empty());
        }

        let registered = self
            .state
            .values
            .get_mut(value)
            .expect("declared observable exists");
        validate_path(value, &registered.schema, &path)?;
        match path.segments().last() {
            Some(ValuePathSegment::MapKey(_)) | Some(ValuePathSegment::OptionPayload) => {
                let previous = remove_at(&mut registered.current, path.segments()).map_err(
                    |message| ObservableError::TransactionConflict {
                        value: value.clone(),
                        message,
                    },
                )?;
                self.undo.push(UndoRecord::Replace {
                    value,
                    path: path.clone(),
                    previous,
                });
                self.touched.push(TouchedMutation {
                    value,
                    path,
                    kind: TouchedKind::Remove,
                });
                Ok(())
            }
            _ => Err(ObservableError::InvalidPath {
                value: Box::new(value.clone()),
                path: Box::new(path),
                message: "fixed structural members cannot be removed".into(),
            }),
        }
    }

    pub fn splice(
        &mut self,
        value: &'roots ValueId,
        path: ValuePath,
        start: u32,
        delete_count: u32,
        inserted: impl IntoIterator<Item = PhenixValue>,
    ) -> Result<(), ObservableError> {
        self.ensure_declared(value)?;
        let registered = self
            .state
            .values
            .get_mut(value)
            .expect("declared observable exists");
        let target_schema = validate_path(value, &registered.schema, &path)?;
        let (item_schema, fixed_len) = match target_schema {
            Type::List(item) => (item.as_ref(), None),
            Type::Array { item, len } => (item.as_ref(), Some(*len)),
            _ => {
                return Err(ObservableError::InvalidPath {
                    value: Box::new(value.clone()),
                    path: Box::new(path),
                    message: "splice target must be a list or array".into(),
                });
            }
        };
        let inserted = inserted.into_iter().collect::<SmallVec<[PhenixValue; 4]>>();
        for item in &inserted {
            item_schema
                .parse(item)
                .map_err(|error| ObservableError::SchemaMismatch {
                    value: Box::new(value.clone()),
                    path: Box::new(path.clone()),
                    message: error.to_string().into(),
                })?;
        }

        let list = value_at_mut(&mut registered.current, path.segments()).map_err(|message| {
            ObservableError::TransactionConflict {
                value: value.clone(),
                message,
            }
        })?;
        let PhenixValue::List(items) = list else {
            return Err(ObservableError::TransactionConflict {
                value: value.clone(),
                message: "splice target is not a list value".to_owned(),
            });
        };
        let start = usize::try_from(start).map_err(|_| ObservableError::TransactionConflict {
            value: value.clone(),
            message: "splice start does not fit usize".to_owned(),
        })?;
        let delete_count =
            usize::try_from(delete_count).map_err(|_| ObservableError::TransactionConflict {
                value: value.clone(),
                message: "splice delete count does not fit usize".to_owned(),
            })?;
        let end = start.checked_add(delete_count).ok_or_else(|| {
            ObservableError::TransactionConflict {
                value: value.clone(),
                message: "splice range overflows".to_owned(),
            }
        })?;
        if start > items.len() || end > items.len() {
            return Err(ObservableError::TransactionConflict {
                value: value.clone(),
                message: format!(
                    "splice range {start}..{end} exceeds list length {}",
                    items.len()
                ),
            });
        }
        if let Some(len) = fixed_len {
            let final_len = items.len() - delete_count + inserted.len();
            if final_len != len {
                return Err(ObservableError::SchemaMismatch {
                    value: Box::new(value.clone()),
                    path: Box::new(path),
                    message: format!("fixed array length must remain {len}, got {final_len}").into(),
                });
            }
        }

        let inserted_count = inserted.len();
        let inserted_for_change = inserted.clone();
        let removed = items
            .splice(start..end, inserted)
            .collect::<SmallVec<[PhenixValue; 4]>>();
        self.undo.push(UndoRecord::Splice {
            value,
            path: path.clone(),
            start,
            inserted_count,
            removed,
        });
        self.touched.push(TouchedMutation {
            value,
            path,
            kind: TouchedKind::Splice {
                start: start as u32,
                delete_count: delete_count as u32,
                inserted: inserted_for_change,
            },
        });
        Ok(())
    }

}
