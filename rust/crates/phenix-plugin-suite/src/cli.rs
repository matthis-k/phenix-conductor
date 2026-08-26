use crate::{workspace_service, WorkspaceCommand, WorkspaceResponse};
use phenix_kernel::{
    Authority, CapabilityId, PluginExecution, PluginHost, PluginId, PluginInstance, PluginManifest,
    ServiceContribution, ServiceId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const CLI_DISCOVER_SERVICE: &str = "phenix.cli.discover@1";
pub const CLI_VERSION_SERVICE: &str = "phenix.cli.version@1";
pub const CLI_AUTH_STATE_SERVICE: &str = "phenix.cli.auth-state@1";
const CLI_PLUGIN: &str = "phenix.cli";
const WORKSPACE_PLUGIN: &str = "phenix.workspace";
const WORKSPACE_SHELL: &str = "workspace.shell";
const SUPPORTED: &[&str] = &["git", "gh", "jj", "rg", "fd", "jq", "nix", "cargo"];

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliProbeRequest {
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliAvailability {
    Available,
    Unavailable,
    Limited,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliAuthState {
    Unsupported,
    Unknown,
    Authenticated,
    Unauthenticated,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliDescriptor {
    pub name: String,
    pub availability: CliAvailability,
    pub executable_identity: Option<String>,
    pub version: Option<String>,
    pub auth_state: Option<CliAuthState>,
    pub supported_probe_capabilities: BTreeSet<String>,
    pub observation_provenance: String,
}

#[must_use]
pub fn cli_manifest(maximum_authority: Authority) -> PluginManifest {
    let shell = Authority::new([capability(WORKSPACE_SHELL)]);
    PluginManifest {
        id: plugin(CLI_PLUGIN),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: vec![plugin(WORKSPACE_PLUGIN)],
        services: vec![
            contribution(cli_discover_service()),
            contribution(cli_version_service()),
            contribution(cli_auth_state_service()),
        ],
        resource_namespaces: Vec::new(),
        maximum_authority: maximum_authority.attenuate(&shell),
    }
}

#[must_use]
pub fn cli_factory() -> Box<dyn PluginInstance> {
    Box::new(CliPlugin)
}

#[must_use]
pub fn cli_discover_service() -> ServiceId {
    service(CLI_DISCOVER_SERVICE)
}

#[must_use]
pub fn cli_version_service() -> ServiceId {
    service(CLI_VERSION_SERVICE)
}

#[must_use]
pub fn cli_auth_state_service() -> ServiceId {
    service(CLI_AUTH_STATE_SERVICE)
}

fn contribution(service: ServiceId) -> ServiceContribution {
    ServiceContribution {
        service,
        priority: 100,
        required_authority: Authority::default(),
    }
}

fn plugin(value: &str) -> PluginId {
    PluginId::parse(value).expect("static plugin id is valid")
}

fn service(value: &str) -> ServiceId {
    ServiceId::parse(value).expect("static service id is valid")
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::parse(value).expect("static capability is valid")
}

fn validate_target(name: &str) -> Result<(), String> {
    if SUPPORTED.contains(&name) {
        Ok(())
    } else {
        Err(format!("unsupported CLI probe target: {name}"))
    }
}

fn probe_capabilities(name: &str) -> BTreeSet<String> {
    let mut capabilities = BTreeSet::from(["discover".to_owned(), "version".to_owned()]);
    if name == "gh" {
        capabilities.insert("auth_state".to_owned());
    }
    capabilities
}

fn descriptor(name: &str, availability: CliAvailability) -> CliDescriptor {
    CliDescriptor {
        name: name.to_owned(),
        availability,
        executable_identity: None,
        version: None,
        auth_state: None,
        supported_probe_capabilities: probe_capabilities(name),
        observation_provenance: "workspace-shell-probe".to_owned(),
    }
}

struct CliPlugin;

impl CliPlugin {
    fn shell(host: &PluginHost<'_>, command: String) -> Result<WorkspaceResponse, String> {
        let request = serde_json::to_vec(&WorkspaceCommand::Shell { command })
            .map_err(|error| error.to_string())?;
        let authority = Authority::new([capability(WORKSPACE_SHELL)]);
        let output = host
            .invoke_service(&workspace_service(), &request, &authority, None)
            .map_err(|error| error.to_string())?;
        serde_json::from_slice(&output).map_err(|error| error.to_string())
    }

    fn discover(host: &PluginHost<'_>, name: &str) -> Result<CliDescriptor, String> {
        validate_target(name)?;
        let response = match Self::shell(host, format!("command -v -- {name}")) {
            Ok(response) => response,
            Err(error) if error.contains(WORKSPACE_SHELL) || error.contains("authority") => {
                return Ok(descriptor(name, CliAvailability::Limited));
            }
            Err(error) => return Err(error),
        };
        let WorkspaceResponse::Process {
            exit_code, stdout, ..
        } = response
        else {
            return Err("workspace shell returned a non-process response".into());
        };
        if exit_code != 0 {
            return Ok(descriptor(name, CliAvailability::Unavailable));
        }
        let mut result = descriptor(name, CliAvailability::Available);
        result.executable_identity = stdout
            .lines()
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        Ok(result)
    }

    fn version(host: &PluginHost<'_>, name: &str) -> Result<CliDescriptor, String> {
        let mut result = Self::discover(host, name)?;
        if result.availability != CliAvailability::Available {
            return Ok(result);
        }
        let response = Self::shell(host, format!("{name} --version"))?;
        let WorkspaceResponse::Process {
            exit_code,
            stdout,
            stderr,
        } = response
        else {
            return Err("workspace shell returned a non-process response".into());
        };
        if exit_code == 0 {
            result.version = stdout
                .lines()
                .chain(stderr.lines())
                .find(|line| !line.trim().is_empty())
                .map(|line| line.trim().to_owned());
        }
        Ok(result)
    }

    fn auth_state(host: &PluginHost<'_>, name: &str) -> Result<CliDescriptor, String> {
        let mut result = Self::discover(host, name)?;
        if name != "gh" {
            result.auth_state = Some(CliAuthState::Unsupported);
            return Ok(result);
        }
        if result.availability != CliAvailability::Available {
            result.auth_state = Some(match result.availability {
                CliAvailability::Limited => CliAuthState::Unknown,
                CliAvailability::Unavailable => CliAuthState::Unauthenticated,
                CliAvailability::Available => unreachable!(),
            });
            return Ok(result);
        }
        let response = Self::shell(host, "gh auth status >/dev/null 2>&1".to_owned())?;
        let WorkspaceResponse::Process { exit_code, .. } = response else {
            return Err("workspace shell returned a non-process response".into());
        };
        result.auth_state = Some(if exit_code == 0 {
            CliAuthState::Authenticated
        } else {
            CliAuthState::Unauthenticated
        });
        Ok(result)
    }
}

impl PluginInstance for CliPlugin {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        let request: CliProbeRequest =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let result = if service == &cli_discover_service() {
            Self::discover(host, &request.name)?
        } else if service == &cli_version_service() {
            Self::version(host, &request.name)?
        } else if service == &cli_auth_state_service() {
            Self::auth_state(host, &request.name)?
        } else {
            return Err(format!("unsupported CLI service: {service}"));
        };
        serde_json::to_vec(&result).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{workspace_factory_for, workspace_manifest};
    use phenix_kernel::{Kernel, KernelConfig};
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_workspace() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("phenix-cli-plugin-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn kernel(authority: Authority) -> Kernel {
        let root = temp_workspace();
        let workspace = workspace_manifest();
        let workspace_id = workspace.id.clone();
        let cli = cli_manifest(authority);
        let cli_id = cli.id.clone();
        let mut kernel = Kernel::new(KernelConfig::new([workspace, cli]).unwrap());
        kernel
            .register_embedded_factory(workspace_id, move || workspace_factory_for(root.clone()))
            .unwrap();
        kernel
            .register_embedded_factory(cli_id, cli_factory)
            .unwrap();
        kernel.activate_all().unwrap();
        kernel
    }

    fn invoke(
        kernel: &mut Kernel,
        service: ServiceId,
        name: &str,
        authority: Authority,
    ) -> Result<CliDescriptor, String> {
        let input = serde_json::to_vec(&CliProbeRequest {
            name: name.to_owned(),
        })
        .unwrap();
        let output = kernel
            .invoke(&service, &input, &authority, None)
            .map_err(|error| error.to_string())?;
        serde_json::from_slice(&output).map_err(|error| error.to_string())
    }

    #[test]
    fn manifest_depends_on_workspace_and_cannot_self_grant_shell() {
        let denied = cli_manifest(Authority::default());
        assert_eq!(denied.dependencies, vec![plugin(WORKSPACE_PLUGIN)]);
        assert!(!denied
            .maximum_authority
            .permits(&capability(WORKSPACE_SHELL)));

        let granted = cli_manifest(Authority::new([capability(WORKSPACE_SHELL)]));
        assert!(granted
            .maximum_authority
            .permits(&capability(WORKSPACE_SHELL)));
    }

    #[test]
    fn unsupported_target_is_rejected_before_shell_execution() {
        let shell = Authority::new([capability(WORKSPACE_SHELL)]);
        let mut kernel = kernel(shell.clone());
        let error = invoke(
            &mut kernel,
            cli_discover_service(),
            "git;touch-pwned",
            shell,
        )
        .unwrap_err();
        assert!(error.contains("unsupported CLI probe target"));
    }

    #[test]
    fn missing_shell_authority_returns_a_typed_limited_result() {
        let mut kernel = kernel(Authority::default());
        let result = invoke(
            &mut kernel,
            cli_discover_service(),
            "git",
            Authority::default(),
        )
        .unwrap();
        assert_eq!(result.availability, CliAvailability::Limited);
        assert_eq!(result.executable_identity, None);
    }

    #[test]
    fn non_gh_auth_probe_is_typed_unsupported() {
        let shell = Authority::new([capability(WORKSPACE_SHELL)]);
        let mut kernel = kernel(shell.clone());
        let result = invoke(&mut kernel, cli_auth_state_service(), "git", shell).unwrap();
        assert_eq!(result.auth_state, Some(CliAuthState::Unsupported));
    }
}
