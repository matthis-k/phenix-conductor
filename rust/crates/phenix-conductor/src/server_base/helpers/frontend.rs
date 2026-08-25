fn normal_frontend_disconnect(result: Result<(), ServerError>) -> Result<(), ServerError> {
    match result {
        Err(ServerError::Io(error)) if is_disconnect_kind(error.kind()) => Ok(()),
        Err(ServerError::Json(error)) if error.io_error_kind().is_some_and(is_disconnect_kind) => {
            Ok(())
        }
        Err(ServerError::OutputClosed) => Ok(()),
        result => result,
    }
}

fn is_disconnect_kind(kind: io::ErrorKind) -> bool {
    matches!(
        kind,
        io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::NotConnected
            | io::ErrorKind::UnexpectedEof
    )
}
