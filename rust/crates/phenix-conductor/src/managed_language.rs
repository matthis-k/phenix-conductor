use phenix_core::{
    FileKind, FileVersion, LanguageDocumentIdentity, LanguageDocumentProvenance, LanguageOperation,
    LanguageOperationResult, LanguagePosition, LanguageProviderId, LanguageServiceKind,
    ManagedLanguageProviderDefinition,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

#[derive(Clone, Default)]
pub(super) struct ManagedLanguageProviders {
    state: Arc<Mutex<ManagedLanguageState>>,
}

#[derive(Default)]
struct ManagedLanguageState {
    next_generation: u64,
    processes: BTreeMap<(LanguageServiceKind, LanguageProviderId), ManagedLanguageProcess>,
}

struct ManagedLanguageProcess {
    service: LanguageServiceKind,
    provider: LanguageProviderId,
    generation: u64,
    child: Child,
    input: ChildStdin,
    responses: mpsc::Receiver<Result<Value, ManagedLanguageError>>,
    reader: Option<thread::JoinHandle<()>>,
    next_request: u64,
    opened: BTreeMap<PathBuf, OpenDocument>,
    diagnostics: Arc<Mutex<BTreeMap<PathBuf, Value>>>,
    workspace_root: PathBuf,
}

#[derive(Clone)]
struct OpenDocument {
    version: i64,
    file_version: FileVersion,
}

#[derive(Debug)]
pub(super) enum ManagedLanguageError {
    StatePoisoned,
    Io(io::Error),
    Protocol(String),
    Remote(Value),
    ProviderUnavailable(LanguageProviderId),
    ProviderChanged(LanguageProviderId),
    WorkspacePath(PathBuf),
}

impl Display for ManagedLanguageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::StatePoisoned => f.write_str("managed language state lock poisoned"),
            Self::Io(error) => write!(f, "managed language I/O error: {error}"),
            Self::Protocol(message) => write!(f, "managed language protocol error: {message}"),
            Self::Remote(error) => write!(f, "managed language provider error: {error}"),
            Self::ProviderUnavailable(provider) => {
                write!(f, "managed language provider {provider} is unavailable")
            }
            Self::ProviderChanged(provider) => {
                write!(
                    f,
                    "managed language provider {provider} changed during the request"
                )
            }
            Self::WorkspacePath(path) => write!(
                f,
                "language document {} is outside the workspace or unavailable",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ManagedLanguageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for ManagedLanguageError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl ManagedLanguageProviders {
    pub(super) fn ensure_definitions(
        &self,
        workspace_root: &Path,
        definitions: impl IntoIterator<Item = ManagedLanguageProviderDefinition>,
    ) -> Result<BTreeMap<LanguageProviderId, u64>, ManagedLanguageError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ManagedLanguageError::StatePoisoned)?;
        let mut live = BTreeMap::new();
        for definition in definitions {
            let key = (definition.service.clone(), definition.provider.clone());
            let dead = state
                .processes
                .get_mut(&key)
                .is_some_and(|process| process.child.try_wait().ok().flatten().is_some());
            if dead {
                state.processes.remove(&key);
            }
            if !state.processes.contains_key(&key) {
                state.next_generation = state.next_generation.saturating_add(1);
                let generation = state.next_generation;
                let process =
                    ManagedLanguageProcess::spawn(workspace_root, definition, generation)?;
                state.processes.insert(key.clone(), process);
            }
            let process = state
                .processes
                .get(&key)
                .expect("managed process was inserted or already present");
            live.insert(process.provider.clone(), process.generation);
        }
        Ok(live)
    }

    pub(super) fn request(
        &self,
        service: &LanguageServiceKind,
        provider: &LanguageProviderId,
        operation: &LanguageOperation,
    ) -> Result<(u64, LanguageOperationResult), ManagedLanguageError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ManagedLanguageError::StatePoisoned)?;
        let key = (service.clone(), provider.clone());
        let result = {
            let process = state
                .processes
                .get_mut(&key)
                .ok_or_else(|| ManagedLanguageError::ProviderUnavailable(provider.clone()))?;
            let generation = process.generation;
            process
                .execute(operation)
                .map(|result| (generation, result))
        };
        match result {
            Err(ManagedLanguageError::Io(_)) => {
                state.processes.remove(&key);
                Err(ManagedLanguageError::ProviderChanged(provider.clone()))
            }
            Err(ManagedLanguageError::Protocol(message)) => {
                state.processes.remove(&key);
                Err(ManagedLanguageError::Protocol(message))
            }
            result => result,
        }
    }
}

impl ManagedLanguageProcess {
    fn spawn(
        workspace_root: &Path,
        definition: ManagedLanguageProviderDefinition,
        generation: u64,
    ) -> Result<Self, ManagedLanguageError> {
        let workspace_root = workspace_root
            .canonicalize()
            .map_err(ManagedLanguageError::Io)?;
        let mut child = Command::new(&definition.command)
            .args(&definition.args)
            .current_dir(&workspace_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let input = child.stdin.take().ok_or_else(|| {
            ManagedLanguageError::Protocol("managed provider has no stdin".to_owned())
        })?;
        let output = child.stdout.take().ok_or_else(|| {
            ManagedLanguageError::Protocol("managed provider has no stdout".to_owned())
        })?;
        let diagnostics = Arc::new(Mutex::new(BTreeMap::new()));
        let (response_sender, responses) = mpsc::channel();
        let reader_root = workspace_root.clone();
        let reader_diagnostics = Arc::clone(&diagnostics);
        let reader = thread::spawn(move || {
            let mut output = BufReader::new(output);
            loop {
                match read_lsp_message(&mut output) {
                    Ok(message) => {
                        if message.get("method").and_then(Value::as_str)
                            == Some("textDocument/publishDiagnostics")
                        {
                            if let Err(error) = capture_diagnostics_notification(
                                &reader_root,
                                &reader_diagnostics,
                                &message,
                            ) {
                                let _ = response_sender.send(Err(error));
                                break;
                            }
                        } else if response_sender.send(Ok(message)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = response_sender.send(Err(error));
                        break;
                    }
                }
            }
        });
        let mut process = Self {
            service: definition.service,
            provider: definition.provider,
            generation,
            child,
            input,
            responses,
            reader: Some(reader),
            next_request: 0,
            opened: BTreeMap::new(),
            diagnostics,
            workspace_root,
        };
        let initialize = json!({
            "processId": std::process::id(),
            "rootUri": path_to_file_uri(&process.workspace_root),
            "capabilities": {
                "workspace": {"workspaceFolders": true},
                "textDocument": {
                    "definition": {},
                    "references": {},
                    "implementation": {},
                    "hover": {},
                    "documentSymbol": {},
                    "callHierarchy": {}
                }
            },
            "workspaceFolders": [{
                "uri": path_to_file_uri(&process.workspace_root),
                "name": process
                    .workspace_root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("workspace")
            }]
        });
        process.request_raw("initialize", initialize)?;
        process.notify("initialized", json!({}))?;
        Ok(process)
    }

    fn execute(
        &mut self,
        operation: &LanguageOperation,
    ) -> Result<LanguageOperationResult, ManagedLanguageError> {
        let mut documents = Vec::new();
        let value = match operation {
            LanguageOperation::Definition { document, position } => {
                let (uri, identity) = self.acquire(document)?;
                documents.push(identity);
                self.request_raw(
                    "textDocument/definition",
                    text_document_position(&uri, *position),
                )?
            }
            LanguageOperation::References { document, position } => {
                let (uri, identity) = self.acquire(document)?;
                documents.push(identity);
                let mut params = text_document_position(&uri, *position);
                params["context"] = json!({"includeDeclaration": true});
                self.request_raw("textDocument/references", params)?
            }
            LanguageOperation::Implementations { document, position } => {
                let (uri, identity) = self.acquire(document)?;
                documents.push(identity);
                self.request_raw(
                    "textDocument/implementation",
                    text_document_position(&uri, *position),
                )?
            }
            LanguageOperation::Hover { document, position } => {
                let (uri, identity) = self.acquire(document)?;
                documents.push(identity);
                self.request_raw(
                    "textDocument/hover",
                    text_document_position(&uri, *position),
                )?
            }
            LanguageOperation::DocumentSymbols { document } => {
                let (uri, identity) = self.acquire(document)?;
                documents.push(identity);
                self.request_raw(
                    "textDocument/documentSymbol",
                    json!({"textDocument": {"uri": uri}}),
                )?
            }
            LanguageOperation::WorkspaceSymbols { query } => {
                self.request_raw("workspace/symbol", json!({"query": query}))?
            }
            LanguageOperation::Diagnostics { document } => {
                if let Some(document) = document {
                    let (_, identity) = self.acquire(document)?;
                    documents.push(identity);
                }
                self.diagnostics_value(document.as_deref())?
            }
            LanguageOperation::CallHierarchy { document, position } => {
                let (uri, identity) = self.acquire(document)?;
                documents.push(identity);
                self.request_raw(
                    "textDocument/prepareCallHierarchy",
                    text_document_position(&uri, *position),
                )?
            }
        };
        self.collect_result_documents(&value, &mut documents);
        documents.sort_by(|left, right| left.path.cmp(&right.path));
        documents.dedup_by(|left, right| left.path == right.path);
        Ok(LanguageOperationResult { value, documents })
    }

    fn acquire(
        &mut self,
        document: &Path,
    ) -> Result<(String, LanguageDocumentIdentity), ManagedLanguageError> {
        let absolute = workspace_path(&self.workspace_root, document)?;
        let bytes = fs::read(&absolute)?;
        let text = String::from_utf8(bytes).map_err(|error| {
            ManagedLanguageError::Protocol(format!(
                "language document {} is not UTF-8: {error}",
                document.display()
            ))
        })?;
        let file_version = FileVersion::Present {
            content_hash: format!("sha256:{:x}", Sha256::digest(text.as_bytes())),
            kind: FileKind::Regular,
        };
        let relative = absolute
            .strip_prefix(&self.workspace_root)
            .expect("workspace path was checked")
            .to_path_buf();
        let uri = path_to_file_uri(&absolute);
        match self.opened.get_mut(&relative) {
            None => {
                self.notify(
                    "textDocument/didOpen",
                    json!({
                        "textDocument": {
                            "uri": uri,
                            "languageId": self.service.as_str(),
                            "version": 1,
                            "text": text
                        }
                    }),
                )?;
                self.opened.insert(
                    relative.clone(),
                    OpenDocument {
                        version: 1,
                        file_version: file_version.clone(),
                    },
                );
            }
            Some(open) if open.file_version != file_version => {
                open.version = open.version.saturating_add(1);
                open.file_version = file_version.clone();
                let version = open.version;
                self.notify(
                    "textDocument/didChange",
                    json!({
                        "textDocument": {"uri": uri, "version": version},
                        "contentChanges": [{"text": text}]
                    }),
                )?;
            }
            Some(_) => {}
        }
        Ok((
            uri,
            LanguageDocumentIdentity {
                path: relative,
                workspace_version: Some(file_version),
                provenance: LanguageDocumentProvenance::WorkspaceBacked,
            },
        ))
    }

    fn request_raw(&mut self, method: &str, params: Value) -> Result<Value, ManagedLanguageError> {
        self.next_request = self.next_request.saturating_add(1);
        let id = self.next_request;
        write_lsp_message(
            &mut self.input,
            &json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}),
        )?;
        loop {
            let message = self.responses.recv().map_err(|_| {
                ManagedLanguageError::Io(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "managed provider response stream closed",
                ))
            })??;
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(ManagedLanguageError::Remote(error.clone()));
            }
            return message.get("result").cloned().ok_or_else(|| {
                ManagedLanguageError::Protocol("response has no result".to_owned())
            });
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), ManagedLanguageError> {
        write_lsp_message(
            &mut self.input,
            &json!({"jsonrpc": "2.0", "method": method, "params": params}),
        )?;
        Ok(())
    }

    fn diagnostics_value(&self, document: Option<&Path>) -> Result<Value, ManagedLanguageError> {
        let diagnostics = self
            .diagnostics
            .lock()
            .map_err(|_| ManagedLanguageError::StatePoisoned)?;
        Ok(match document {
            Some(path) => diagnostics.get(path).cloned().unwrap_or_else(|| json!([])),
            None => Value::Object(
                diagnostics
                    .iter()
                    .map(|(path, value)| (path.to_string_lossy().into_owned(), value.clone()))
                    .collect(),
            ),
        })
    }

    fn collect_result_documents(
        &self,
        value: &Value,
        documents: &mut Vec<LanguageDocumentIdentity>,
    ) {
        let mut uris = BTreeSet::new();
        collect_uris(value, &mut uris);
        for uri in uris {
            let Some(relative) = file_uri_to_workspace_path(&self.workspace_root, &uri) else {
                continue;
            };
            let absolute = self.workspace_root.join(&relative);
            let Ok(bytes) = fs::read(&absolute) else {
                continue;
            };
            documents.push(LanguageDocumentIdentity {
                path: relative,
                workspace_version: Some(FileVersion::Present {
                    content_hash: format!("sha256:{:x}", Sha256::digest(&bytes)),
                    kind: FileKind::Regular,
                }),
                provenance: LanguageDocumentProvenance::WorkspaceBacked,
            });
        }
    }
}

impl Drop for ManagedLanguageProcess {
    fn drop(&mut self) {
        let paths = self.opened.keys().cloned().collect::<Vec<_>>();
        for path in paths {
            let uri = path_to_file_uri(&self.workspace_root.join(path));
            let _ = self.notify(
                "textDocument/didClose",
                json!({"textDocument": {"uri": uri}}),
            );
        }
        let _ = self.request_raw("shutdown", Value::Null);
        let _ = self.notify("exit", Value::Null);
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn capture_diagnostics_notification(
    workspace_root: &Path,
    diagnostics: &Arc<Mutex<BTreeMap<PathBuf, Value>>>,
    message: &Value,
) -> Result<(), ManagedLanguageError> {
    let params = message.get("params").ok_or_else(|| {
        ManagedLanguageError::Protocol("diagnostics notification has no params".to_owned())
    })?;
    let uri = params.get("uri").and_then(Value::as_str).ok_or_else(|| {
        ManagedLanguageError::Protocol("diagnostics notification has no uri".to_owned())
    })?;
    if let Some(path) = file_uri_to_workspace_path(workspace_root, uri) {
        diagnostics
            .lock()
            .map_err(|_| ManagedLanguageError::StatePoisoned)?
            .insert(
                path,
                params
                    .get("diagnostics")
                    .cloned()
                    .unwrap_or_else(|| json!([])),
            );
    }
    Ok(())
}

fn text_document_position(uri: &str, position: LanguagePosition) -> Value {
    json!({
        "textDocument": {"uri": uri},
        "position": {"line": position.line, "character": position.character}
    })
}

fn workspace_path(root: &Path, path: &Path) -> Result<PathBuf, ManagedLanguageError> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(ManagedLanguageError::WorkspacePath(path.to_path_buf()));
    }
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let canonical = candidate
        .canonicalize()
        .map_err(|_| ManagedLanguageError::WorkspacePath(path.to_path_buf()))?;
    canonical
        .starts_with(root)
        .then_some(canonical)
        .ok_or_else(|| ManagedLanguageError::WorkspacePath(path.to_path_buf()))
}

fn path_to_file_uri(path: &Path) -> String {
    let value = path.to_string_lossy();
    let mut encoded = String::with_capacity(value.len() + 7);
    encoded.push_str("file://");
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => {
                use std::fmt::Write as _;
                let _ = write!(&mut encoded, "%{byte:02X}");
            }
        }
    }
    encoded
}

fn file_uri_to_workspace_path(root: &Path, uri: &str) -> Option<PathBuf> {
    let encoded = uri.strip_prefix("file://")?;
    let decoded = percent_decode(encoded)?;
    let absolute = PathBuf::from(decoded);
    let relative = absolute.strip_prefix(root).ok()?.to_path_buf();
    Some(relative)
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            output.push((hex(high)? << 4) | hex(low)?);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).ok()
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn collect_uris(value: &Value, uris: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(uri) = object.get("uri").and_then(Value::as_str) {
                uris.insert(uri.to_owned());
            }
            for child in object.values() {
                collect_uris(child, uris);
            }
        }
        Value::Array(array) => {
            for child in array {
                collect_uris(child, uris);
            }
        }
        _ => {}
    }
}

fn write_lsp_message(writer: &mut impl Write, value: &Value) -> Result<(), ManagedLanguageError> {
    let body = serde_json::to_vec(value)
        .map_err(|error| ManagedLanguageError::Protocol(error.to_string()))?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()?;
    Ok(())
}

fn read_lsp_message(reader: &mut impl BufRead) -> Result<Value, ManagedLanguageError> {
    let mut content_length = None;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            return Err(ManagedLanguageError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "managed provider closed stdout",
            )));
        }
        if header == "\r\n" || header == "\n" {
            break;
        }
        if let Some(value) = header
            .strip_prefix("Content-Length:")
            .map(str::trim)
            .and_then(|value| value.parse::<usize>().ok())
        {
            content_length = Some(value);
        }
    }
    let length = content_length.ok_or_else(|| {
        ManagedLanguageError::Protocol("LSP message has no Content-Length".to_owned())
    })?;
    let mut body = vec![0_u8; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(|error| ManagedLanguageError::Protocol(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn lsp_framing_round_trips_json_rpc_messages() {
        let value = json!({"jsonrpc": "2.0", "id": 7, "result": {"ok": true}});
        let mut bytes = Vec::new();
        write_lsp_message(&mut bytes, &value).unwrap();
        let decoded = read_lsp_message(&mut BufReader::new(Cursor::new(bytes))).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn file_uri_round_trip_preserves_workspace_relative_path() {
        let root = PathBuf::from("/tmp/phenix workspace");
        let path = root.join("src/lib.rs");
        let uri = path_to_file_uri(&path);
        assert_eq!(
            file_uri_to_workspace_path(&root, &uri),
            Some(PathBuf::from("src/lib.rs"))
        );
    }

    #[test]
    fn diagnostics_notifications_update_shared_state_without_request_reader() {
        let root = PathBuf::from("/workspace");
        let diagnostics = Arc::new(Mutex::new(BTreeMap::new()));
        let message = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///workspace/src/lib.rs",
                "diagnostics": [{"message": "broken"}]
            }
        });
        capture_diagnostics_notification(&root, &diagnostics, &message).unwrap();
        assert_eq!(
            diagnostics.lock().unwrap()[Path::new("src/lib.rs")][0]["message"],
            "broken"
        );
    }

    #[test]
    fn result_document_collection_finds_nested_locations() {
        let mut uris = BTreeSet::new();
        collect_uris(
            &json!({"items": [{"uri": "file:///workspace/src/lib.rs"}]}),
            &mut uris,
        );
        assert_eq!(
            uris,
            BTreeSet::from(["file:///workspace/src/lib.rs".to_owned()])
        );
    }
}
