fn sql_u64(value: u64, field: &str) -> Result<i64, PersistenceError> {
    i64::try_from(value).map_err(|_| invalid(format!("{field} exceeds SQLite range")))
}

fn sql_usize(value: usize, field: &str) -> Result<i64, PersistenceError> {
    i64::try_from(value).map_err(|_| invalid(format!("{field} exceeds SQLite range")))
}

fn invalid(message: impl Into<String>) -> PersistenceError {
    PersistenceError::InvalidJournal(message.into())
}
