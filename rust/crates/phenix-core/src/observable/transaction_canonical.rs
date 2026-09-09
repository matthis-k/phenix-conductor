impl<'roots> ObservableTransaction<'_, 'roots> {
    fn canonicalize_root(
        &self,
        value: &ValueId,
    ) -> Result<SmallVec<[ValueChange; 8]>, ObservableError> {
        let registered = self
            .state
            .values
            .get(value)
            .expect("touched observable remains registered");
        let mut selected = SmallVec::<[TouchedMutation<'roots>; 8]>::new();
        for touched in self.touched.iter().filter(|touched| touched.value == value) {
            match &touched.kind {
                TouchedKind::Replace | TouchedKind::Remove => {
                    if selected.iter().any(|existing| {
                        touched.path.starts_with(&existing.path)
                            && matches!(existing.kind, TouchedKind::Replace | TouchedKind::Remove)
                    }) {
                        continue;
                    }
                    selected.retain(|existing| !existing.path.starts_with(&touched.path));
                    selected.push(touched.clone());
                }
                TouchedKind::Splice { .. } => {
                    if !selected.iter().any(|existing| {
                        touched.path.starts_with(&existing.path)
                            && matches!(existing.kind, TouchedKind::Replace | TouchedKind::Remove)
                    }) {
                        selected.push(touched.clone());
                    }
                }
            }
        }

        selected.sort_by(|left, right| left.path.cmp(&right.path));
        let mut changes = SmallVec::<[ValueChange; 8]>::new();
        for touched in selected {
            match touched.kind {
                TouchedKind::Replace | TouchedKind::Remove => {
                    let current = value_at_optional(&registered.current, touched.path.segments());
                    let original = self.original_at(value, &touched.path)?;
                    if current == original.as_ref() {
                        continue;
                    }
                    match current {
                        Some(current) => changes.push(ValueChange::Replace {
                            path: touched.path,
                            value: current.clone(),
                        }),
                        None => changes.push(ValueChange::Remove { path: touched.path }),
                    }
                }
                TouchedKind::Splice {
                    start,
                    delete_count,
                    inserted,
                } => {
                    let original = self.original_at(value, &touched.path)?;
                    let current = value_at_optional(&registered.current, touched.path.segments());
                    if current == original.as_ref() {
                        continue;
                    }
                    changes.push(ValueChange::Splice {
                        path: touched.path,
                        start,
                        delete_count,
                        inserted,
                    });
                }
            }
        }
        Ok(changes)
    }

    fn original_at(
        &self,
        value: &ValueId,
        path: &ValuePath,
    ) -> Result<Option<PhenixValue>, ObservableError> {
        let registered = self
            .state
            .values
            .get(value)
            .expect("touched observable remains registered");
        let mut probe = value_at_optional(&registered.current, path.segments()).cloned();

        for undo in self.undo.iter().rev() {
            match undo {
                UndoRecord::Replace {
                    value: undo_value,
                    path: undo_path,
                    previous,
                } if *undo_value == value => {
                    if undo_path == path {
                        probe = previous.clone();
                    } else if undo_path.starts_with(path) {
                        if let Some(probe_value) = probe.as_mut() {
                            let relative = undo_path.relative_to(path).expect("prefix checked");
                            restore_optional(probe_value, relative.segments(), previous.clone())
                                .map_err(|message| ObservableError::TransactionConflict {
                                    value: value.clone(),
                                    message,
                                })?;
                        }
                    } else if path.starts_with(undo_path) {
                        if let Some(previous) = previous.as_ref() {
                            let relative = path.relative_to(undo_path).expect("prefix checked");
                            probe = value_at_optional(previous, relative.segments()).cloned();
                        } else {
                            probe = None;
                        }
                    }
                }
                UndoRecord::Splice {
                    value: undo_value,
                    path: undo_path,
                    start,
                    inserted_count,
                    removed,
                } if *undo_value == value => {
                    if undo_path == path {
                        if let Some(PhenixValue::List(items)) = probe.as_mut() {
                            let end = start.saturating_add(*inserted_count).min(items.len());
                            items.splice(*start..end, removed.iter().cloned());
                        }
                    } else if undo_path.starts_with(path) {
                        if let Some(probe_value) = probe.as_mut() {
                            let relative = undo_path.relative_to(path).expect("prefix checked");
                            if let Ok(PhenixValue::List(items)) =
                                value_at_mut(probe_value, relative.segments())
                            {
                                let end = start.saturating_add(*inserted_count).min(items.len());
                                items.splice(*start..end, removed.iter().cloned());
                            }
                        }
                    } else if path.starts_with(undo_path) {
                        // Index-relative paths can change identity under a splice. Reconstruct the
                        // containing list, then re-read the addressed descendant.
                        let container = value_at_optional(&registered.current, undo_path.segments())
                            .cloned();
                        if let Some(PhenixValue::List(mut items)) = container {
                            let end = start.saturating_add(*inserted_count).min(items.len());
                            items.splice(*start..end, removed.iter().cloned());
                            let relative = path.relative_to(undo_path).expect("prefix checked");
                            probe = value_at_optional(
                                &PhenixValue::List(items),
                                relative.segments(),
                            )
                            .cloned();
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(probe)
    }
}
