use phenix_core::{
    Authority, CapabilityId, PluginExecution, PluginHost, PluginId, PluginInstance, PluginManifest,
    ServiceContribution, ServiceId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

pub const WORKSPACE_SERVICE: &str = "phenix.workspace@1";
const WORKSPACE_PLUGIN: &str = "phenix.workspace";
const WORKSPACE_READ: &str = "workspace.read";
const WORKSPACE_WRITE: &str = "workspace.write";
const WORKSPACE_SHELL: &str = "workspace.shell";
const WORKSPACE_GIT: &str = "workspace.git";
const MAX_CAPTURE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum WorkspaceFileVersion {
    Absent,
    Present { content_hash: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSearchMatch {
    pub path: String,
    pub line: usize,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum WorkspaceCommand {
    Read {
        path: String,
    },
    Write {
        path: String,
        content: String,
        expected_version: WorkspaceFileVersion,
    },
    Search {
        needle: String,
        path: Option<String>,
        case_sensitive: bool,
    },
    Shell {
        command: String,
    },
    Git {
        arguments: Vec<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "response", rename_all = "snake_case")]
pub enum WorkspaceResponse {
    Read {
        path: String,
        content: String,
        version: WorkspaceFileVersion,
    },
    Written {
        path: String,
        version: WorkspaceFileVersion,
    },
    Search {
        matches: Vec<WorkspaceSearchMatch>,
    },
    Process {
        exit_code: i32,
        stdout: String,
        stderr: String,
    },
}

#[must_use]
pub fn workspace_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(WORKSPACE_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: phenix_core::ServiceRole::Terminal,
            service: workspace_service(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::new([
            capability(WORKSPACE_READ),
            capability(WORKSPACE_WRITE),
            capability(WORKSPACE_SHELL),
            capability(WORKSPACE_GIT),
        ]),
    }
}

#[must_use]
pub fn workspace_factory() -> Box<dyn PluginInstance> {
    Box::new(WorkspacePlugin::new(
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    ))
}

#[must_use]
pub fn workspace_factory_for(root: impl Into<PathBuf>) -> Box<dyn PluginInstance> {
    Box::new(WorkspacePlugin::new(root.into()))
}

#[must_use]
pub fn workspace_service() -> ServiceId {
    ServiceId::parse(WORKSPACE_SERVICE).expect("static service id is valid")
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).expect("static capability is valid")
}

struct WorkspacePlugin {
    root: PathBuf,
}

impl WorkspacePlugin {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn resolve(&self, input: &str) -> Result<PathBuf, String> {
        let input = Path::new(input);
        if input.is_absolute() {
            return Err("workspace paths must be relative".into());
        }
        let mut relative = PathBuf::new();
        for component in input.components() {
            match component {
                Component::Normal(value) => relative.push(value),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err("workspace path escapes the configured root".into());
                }
            }
        }
        Ok(self.root.join(relative))
    }

    fn require(host: &PluginHost<'_>, value: &str) -> Result<(), String> {
        let capability = capability(value);
        if host.authority().permits(&capability) {
            Ok(())
        } else {
            Err(format!("workspace authority denied: {value}"))
        }
    }

    fn read(&self, host: &PluginHost<'_>, path: String) -> Result<WorkspaceResponse, String> {
        Self::require(host, WORKSPACE_READ)?;
        let resolved = self.resolve(&path)?;
        let bytes = fs::read(&resolved).map_err(|error| format!("read {path}: {error}"))?;
        let content = String::from_utf8(bytes.clone())
            .map_err(|_| format!("workspace read requires UTF-8 text: {path}"))?;
        Ok(WorkspaceResponse::Read {
            path,
            content,
            version: version_for_bytes(&bytes),
        })
    }

    fn write(
        &self,
        host: &PluginHost<'_>,
        path: String,
        content: String,
        expected_version: WorkspaceFileVersion,
    ) -> Result<WorkspaceResponse, String> {
        Self::require(host, WORKSPACE_WRITE)?;
        let resolved = self.resolve(&path)?;
        let observed = match fs::read(&resolved) {
            Ok(bytes) => version_for_bytes(&bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                WorkspaceFileVersion::Absent
            }
            Err(error) => return Err(format!("inspect {path}: {error}")),
        };
        if observed != expected_version {
            return Err(format!(
                "workspace version conflict for {path}: expected {expected_version:?}, observed {observed:?}"
            ));
        }
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("create parent for {path}: {error}"))?;
        }
        fs::write(&resolved, content.as_bytes())
            .map_err(|error| format!("write {path}: {error}"))?;
        Ok(WorkspaceResponse::Written {
            path,
            version: version_for_bytes(content.as_bytes()),
        })
    }

    fn search(
        &self,
        host: &PluginHost<'_>,
        needle: String,
        path: Option<String>,
        case_sensitive: bool,
    ) -> Result<WorkspaceResponse, String> {
        Self::require(host, WORKSPACE_READ)?;
        if needle.is_empty() {
            return Err("workspace search needle must not be empty".into());
        }
        let relative = path.unwrap_or_else(|| ".".into());
        let root = self.resolve(&relative)?;
        let mut matches = Vec::new();
        self.search_path(&root, &needle, case_sensitive, &mut matches)?;
        matches.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.line.cmp(&right.line))
        });
        Ok(WorkspaceResponse::Search { matches })
    }

    fn search_path(
        &self,
        path: &Path,
        needle: &str,
        case_sensitive: bool,
        matches: &mut Vec<WorkspaceSearchMatch>,
    ) -> Result<(), String> {
        if path.file_name().is_some_and(|name| name == ".git") {
            return Ok(());
        }
        if path.is_dir() {
            let mut entries = fs::read_dir(path)
                .map_err(|error| format!("search {}: {error}", path.display()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                self.search_path(&entry.path(), needle, case_sensitive, matches)?;
            }
            return Ok(());
        }
        if !path.is_file() {
            return Ok(());
        }
        let Ok(bytes) = fs::read(path) else {
            return Ok(());
        };
        let Ok(content) = String::from_utf8(bytes) else {
            return Ok(());
        };
        let query = if case_sensitive {
            needle.to_owned()
        } else {
            needle.to_lowercase()
        };
        for (index, line) in content.lines().enumerate() {
            let candidate = if case_sensitive {
                line.to_owned()
            } else {
                line.to_lowercase()
            };
            if candidate.contains(&query) {
                let relative = path.strip_prefix(&self.root).unwrap_or(path);
                matches.push(WorkspaceSearchMatch {
                    path: relative.to_string_lossy().into_owned(),
                    line: index + 1,
                    text: line.to_owned(),
                });
            }
        }
        Ok(())
    }

    fn process(
        &self,
        host: &PluginHost<'_>,
        program: &str,
        args: &[String],
        capability: &str,
    ) -> Result<WorkspaceResponse, String> {
        Self::require(host, capability)?;
        let output = Command::new(program)
            .args(args)
            .current_dir(&self.root)
            .output()
            .map_err(|error| format!("spawn {program}: {error}"))?;
        Ok(WorkspaceResponse::Process {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: capture(&output.stdout),
            stderr: capture(&output.stderr),
        })
    }
}

impl PluginInstance for WorkspacePlugin {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        if !self.root.is_dir() {
            return Err(format!(
                "workspace root is not a directory: {}",
                self.root.display()
            ));
        }
        Ok(())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &workspace_service() {
            return Err(format!("unsupported workspace service: {service}"));
        }
        let command: WorkspaceCommand =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = match command {
            WorkspaceCommand::Read { path } => self.read(host, path)?,
            WorkspaceCommand::Write {
                path,
                content,
                expected_version,
            } => self.write(host, path, content, expected_version)?,
            WorkspaceCommand::Search {
                needle,
                path,
                case_sensitive,
            } => self.search(host, needle, path, case_sensitive)?,
            WorkspaceCommand::Shell { command } => {
                if command.trim().is_empty() {
                    return Err("shell command must not be empty".into());
                }
                self.process(host, "bash", &["-c".into(), command], WORKSPACE_SHELL)?
            }
            WorkspaceCommand::Git { arguments } => {
                self.process(host, "git", &arguments, WORKSPACE_GIT)?
            }
        };
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn version_for_bytes(bytes: &[u8]) -> WorkspaceFileVersion {
    WorkspaceFileVersion::Present {
        content_hash: format!("{:x}", Sha256::digest(bytes)),
    }
}

fn capture(bytes: &[u8]) -> String {
    let bytes = if bytes.len() > MAX_CAPTURE_BYTES {
        &bytes[..MAX_CAPTURE_BYTES]
    } else {
        bytes
    };
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{Kernel, KernelConfig};
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_workspace(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("phenix-{name}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn kernel(root: PathBuf) -> Kernel {
        let manifest = workspace_manifest();
        let plugin = manifest.id.clone();
        let mut kernel = Kernel::new(KernelConfig::new([manifest]).unwrap());
        kernel
            .register_embedded_factory(plugin, move || workspace_factory_for(root.clone()))
            .unwrap();
        kernel.activate_all().unwrap();
        kernel
    }

    fn authority(values: &[&str]) -> Authority {
        Authority::new(values.iter().map(|value| capability(value)))
    }

    fn invoke(
        kernel: &mut Kernel,
        command: WorkspaceCommand,
        authority: &Authority,
    ) -> Result<WorkspaceResponse, String> {
        let input = serde_json::to_vec(&command).unwrap();
        let output = kernel
            .invoke(&workspace_service(), &input, authority, None)
            .map_err(|error| error.to_string())?;
        serde_json::from_slice(&output).map_err(|error| error.to_string())
    }

    #[test]
    fn read_write_use_exact_versions_and_cannot_escape_workspace() {
        let root = temp_workspace("workspace-versions");
        fs::write(root.join("input.txt"), "one\n").unwrap();
        let mut kernel = kernel(root.clone());
        let read = authority(&[WORKSPACE_READ]);
        let write = authority(&[WORKSPACE_WRITE]);
        let version = match invoke(
            &mut kernel,
            WorkspaceCommand::Read {
                path: "input.txt".into(),
            },
            &read,
        )
        .unwrap()
        {
            WorkspaceResponse::Read { version, .. } => version,
            other => panic!("unexpected response: {other:?}"),
        };
        invoke(
            &mut kernel,
            WorkspaceCommand::Write {
                path: "input.txt".into(),
                content: "two\n".into(),
                expected_version: version.clone(),
            },
            &write,
        )
        .unwrap();
        let conflict = invoke(
            &mut kernel,
            WorkspaceCommand::Write {
                path: "input.txt".into(),
                content: "three\n".into(),
                expected_version: version,
            },
            &write,
        )
        .unwrap_err();
        assert!(conflict.contains("version conflict"));
        assert!(invoke(
            &mut kernel,
            WorkspaceCommand::Read {
                path: "../outside".into(),
            },
            &read,
        )
        .unwrap_err()
        .contains("escapes"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn search_is_deterministic_and_read_authority_cannot_write() {
        let root = temp_workspace("workspace-search");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/b.rs"), "needle b\n").unwrap();
        fs::write(root.join("src/a.rs"), "needle a\n").unwrap();
        let mut kernel = kernel(root.clone());
        let read = authority(&[WORKSPACE_READ]);
        let response = invoke(
            &mut kernel,
            WorkspaceCommand::Search {
                needle: "needle".into(),
                path: Some("src".into()),
                case_sensitive: true,
            },
            &read,
        )
        .unwrap();
        match response {
            WorkspaceResponse::Search { matches } => {
                assert_eq!(matches.len(), 2);
                assert_eq!(matches[0].path, "src/a.rs");
                assert_eq!(matches[1].path, "src/b.rs");
            }
            other => panic!("unexpected response: {other:?}"),
        }
        let denied = invoke(
            &mut kernel,
            WorkspaceCommand::Write {
                path: "new.txt".into(),
                content: "no".into(),
                expected_version: WorkspaceFileVersion::Absent,
            },
            &read,
        )
        .unwrap_err();
        assert!(denied.contains("authority denied"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn git_runs_through_the_same_replaceable_workspace_service() {
        let root = temp_workspace("workspace-git");
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .status()
            .unwrap();
        let mut kernel = kernel(root.clone());
        let response = invoke(
            &mut kernel,
            WorkspaceCommand::Git {
                arguments: vec!["status".into(), "--porcelain".into()],
            },
            &authority(&[WORKSPACE_GIT]),
        )
        .unwrap();
        assert!(matches!(
            response,
            WorkspaceResponse::Process { exit_code: 0, .. }
        ));
        let _ = fs::remove_dir_all(root);
    }
}
