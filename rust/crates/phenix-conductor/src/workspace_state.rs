use phenix_core::WorkspaceDescriptor;
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum WorkspaceStateError {
    MissingStateHome,
}

impl Display for WorkspaceStateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingStateHome => {
                f.write_str("cannot determine Phenix state directory: set XDG_STATE_HOME or HOME")
            }
        }
    }
}

impl Error for WorkspaceStateError {}

pub fn default_database_path(
    workspace: &WorkspaceDescriptor,
) -> Result<PathBuf, WorkspaceStateError> {
    let state_home = match env::var_os("XDG_STATE_HOME") {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => env::var_os("HOME")
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .map(|home| home.join(".local/state"))
            .ok_or(WorkspaceStateError::MissingStateHome)?,
    };
    Ok(database_path_under(&state_home, workspace))
}

fn database_path_under(state_home: &Path, workspace: &WorkspaceDescriptor) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(workspace.id.as_str().as_bytes());
    let workspace_key = format!("{:x}", hasher.finalize());
    state_home
        .join("phenix")
        .join("workspaces")
        .join(workspace_key)
        .join("workspace.db")
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{WorkspaceDescriptor, WorkspaceId};
    use std::collections::BTreeSet;

    fn workspace(id: &str, root: &str) -> WorkspaceDescriptor {
        WorkspaceDescriptor {
            id: WorkspaceId::parse(id).unwrap(),
            root: PathBuf::from(root),
            scratch_paths: BTreeSet::new(),
        }
    }

    #[test]
    fn workspace_state_path_is_stable_and_outside_the_repository() {
        let workspace = workspace("workspace:/repo", "/repo");
        let path = database_path_under(Path::new("/state"), &workspace);
        assert_eq!(path.file_name(), Some("workspace.db".as_ref()));
        assert!(path.starts_with("/state/phenix/workspaces"));
        assert!(!path.starts_with(&workspace.root));
        assert_eq!(path, database_path_under(Path::new("/state"), &workspace));
    }

    #[test]
    fn distinct_workspaces_have_distinct_database_paths() {
        let first = workspace("workspace:/repo", "/repo");
        let second = workspace("workspace:/repo-worktree", "/repo-worktree");
        assert_ne!(
            database_path_under(Path::new("/state"), &first),
            database_path_under(Path::new("/state"), &second)
        );
    }
}
