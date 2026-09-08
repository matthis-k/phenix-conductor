impl<'roots> ObservableTransaction<'_, 'roots> {
    fn ensure_declared(&self, value: &ValueId) -> Result<(), ObservableError> {
        if self.declared.contains(&value) {
            Ok(())
        } else {
            Err(ObservableError::TransactionConflict {
                value: value.clone(),
                message: "root was not declared before the transaction".to_owned(),
            })
        }
    }

    fn rollback(&mut self) {
        while let Some(undo) = self.undo.pop() {
            let _ = apply_undo(self.state, undo);
        }
        self.touched.clear();
    }

    fn finish(&mut self) -> Result<SmallVec<[PreparedRoot<'roots>; 4]>, ObservableError> {
        let mut roots = SmallVec::<[&'roots ValueId; 4]>::new();
        for touched in &self.touched {
            if !roots.contains(&touched.value) {
                roots.push(touched.value);
            }
        }
        roots.sort_unstable();

        let mut canonical = SmallVec::<[(&'roots ValueId, SmallVec<[ValueChange; 8]>); 4]>::new();
        for root in roots {
            let changes = self.canonicalize_root(root)?;
            if !changes.is_empty() {
                canonical.push((root, changes));
            }
        }
        if canonical.is_empty() {
            return Ok(SmallVec::new());
        }

        let commit = CommitId(self.state.next_commit);
        self.state.next_commit = self.state.next_commit.saturating_add(1);
        let mut prepared = SmallVec::<[PreparedRoot<'roots>; 4]>::new();

        for (value, changes) in canonical {
            let (from_version, version, snapshot_policy, snapshot) = {
                let registered = self
                    .state
                    .values
                    .get_mut(value)
                    .expect("touched observable remains registered");
                let from_version = registered.version;
                registered.version = ValueVersion(registered.version.get().saturating_add(1));
                let snapshot = (registered.snapshot_policy == SnapshotPolicy::CopyOnChange)
                    .then(|| Arc::new(registered.current.clone()));
                (
                    from_version,
                    registered.version,
                    registered.snapshot_policy,
                    snapshot,
                )
            };
            #[cfg(test)]
            if snapshot.is_some() {
                self.state.snapshot_copies += 1;
            }

            let mut matched = SmallVec::<[ObservationId; 8]>::new();
            #[cfg(test)]
            let mut match_visits = 0usize;
            if let Some(index) = self.state.indexes.get(value) {
                for change in &changes {
                    #[cfg(test)]
                    {
                        match_visits += change.path().len() + 1;
                    }
                    match change {
                        ValueChange::Splice {
                            path,
                            start,
                            delete_count,
                            inserted,
                        } => index.collect_for_splice(
                            path.segments(),
                            *start,
                            *delete_count,
                            inserted.len() as u32,
                            &mut matched,
                        ),
                        _ => index.collect_for_path(change.path().segments(), &mut matched),
                    }
                }
            }
            #[cfg(test)]
            {
                self.state.match_visits += match_visits;
            }

            let current_root = &self
                .state
                .values
                .get(value)
                .expect("touched observable remains registered")
                .current;
            let mut deliveries = SmallVec::<[PreparedDelivery; 8]>::new();
            for id in matched {
                let Some(entry) = self.state.subscriptions.get(&id) else {
                    continue;
                };
                let relative = changes_for_subscription(&entry.spec, &changes, current_root);
                if relative.is_empty() {
                    continue;
                }
                if entry.spec.mode == ObservationMode::Full
                    && snapshot_policy != SnapshotPolicy::CopyOnChange
                {
                    return Err(ObservableError::UnsupportedSnapshotPolicy {
                        value: (*value).clone(),
                        policy: snapshot_policy,
                        mode: entry.spec.mode,
                        initial: entry.spec.initial,
                    });
                }
                deliveries.push(PreparedDelivery {
                    entry: Arc::clone(entry),
                    changes: relative,
                });
            }

            prepared.push(PreparedRoot {
                commit,
                value,
                from_version,
                version,
                snapshot,
                deliveries,
            });
        }
        Ok(prepared)
    }

}
