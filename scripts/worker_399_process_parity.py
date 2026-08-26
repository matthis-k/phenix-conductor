from pathlib import Path
import re

root = Path('.')

process_test = r'''use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

fn run_harness(state: &Path, requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_phenix-harness"))
        .env("PHENIX_STATE_DB", state)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("supported Harness binary must start");

    {
        let mut stdin = child.stdin.take().expect("Harness stdin must be piped");
        for request in requests {
            serde_json::to_writer(&mut stdin, request).unwrap();
            stdin.write_all(b"\n").unwrap();
        }
    }

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "Harness process failed: {output:?}");
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn process_roundtrip_routes_and_restores_plugin_owned_state() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let state = std::env::temp_dir().join(format!(
        "phenix-harness-process-roundtrip-{}-{nonce}.sqlite",
        std::process::id()
    ));
    let _ = fs::remove_file(&state);

    let first = run_harness(
        &state,
        &[
            serde_json::json!({
                "id": 1,
                "service": "phenix.sessions@1",
                "input": {"operation": "create", "id": "process-session", "parent": null}
            }),
            serde_json::json!({
                "id": 2,
                "service": "phenix.context@1",
                "input": {
                    "operation": "register",
                    "resource_id": "process:context",
                    "kind": "external",
                    "source": "process-roundtrip",
                    "scope": "workspace",
                    "content": [112, 114, 111, 99, 101, 115, 115]
                }
            }),
            serde_json::json!({
                "id": 3,
                "service": "phenix.planning@1",
                "input": {
                    "operation": "create_objective",
                    "id": "process-objective",
                    "title": "Process parity",
                    "parent": null
                }
            }),
        ],
    );
    assert_eq!(first.len(), 3);
    assert_eq!(first[0]["status"], "ok");
    assert_eq!(first[0]["output"]["session"]["id"], "process-session");
    assert_eq!(first[1]["status"], "ok");
    assert_eq!(
        first[1]["output"]["resource"]["descriptor"]["resource_id"],
        "process:context"
    );
    assert_eq!(first[2]["status"], "ok");
    assert_eq!(
        first[2]["output"]["objective"]["id"],
        "process-objective"
    );

    let second = run_harness(
        &state,
        &[
            serde_json::json!({
                "id": 4,
                "service": "phenix.sessions@1",
                "input": {"operation": "get", "id": "process-session"}
            }),
            serde_json::json!({
                "id": 5,
                "service": "phenix.context@1",
                "input": {"operation": "list"}
            }),
            serde_json::json!({
                "id": 6,
                "service": "phenix.planning@1",
                "input": {"operation": "get_objective", "id": "process-objective"}
            }),
        ],
    );
    assert_eq!(second.len(), 3);
    assert_eq!(second[0]["status"], "ok");
    assert_eq!(second[0]["output"]["session"]["id"], "process-session");
    assert_eq!(second[1]["status"], "ok");
    assert!(second[1]["output"]["descriptors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|descriptor| descriptor["resource_id"] == "process:context"));
    assert_eq!(second[2]["status"], "ok");
    assert_eq!(
        second[2]["output"]["objective"]["id"],
        "process-objective"
    );

    let _ = fs::remove_file(&state);
}
'''
(root / 'rust/crates/phenix-harness/tests/process_roundtrip.rs').write_text(process_test)

path = root / 'modules/development.nix'
text = path.read_text()
marker = '''        {
          id = "harness-supported-product";
          package = "phenix-harness";
          test = "supported_product_journeys";
          label = "harness / supported_product_journeys";
        }
'''
addition = marker + '''        {
          id = "harness-process-roundtrip";
          package = "phenix-harness";
          test = "process_roundtrip";
          label = "harness / process_roundtrip";
        }
'''
if 'id = "harness-process-roundtrip";' not in text:
    if text.count(marker) != 1:
        raise SystemExit(f'expected one Harness system target marker, found {text.count(marker)}')
    text = text.replace(marker, addition, 1)
path.write_text(text)

path = root / 'rust/crates/phenix-acp/src/lib.rs'
text = path.read_text()
old = '''//! This crate deliberately contains no Phenix application/runtime semantics.
//! Session trees, executions, routing, workflows, callables, tools, policy and
//! persistence belong to `phenix-conductor` and its domain crates. ACP is only
//! one backend wire protocol.
'''
new = '''//! This crate deliberately contains no Phenix application/runtime semantics.
//! The Phenix Plugin Suite owns first-party agent-domain semantics. `phenix-harness`
//! composes those plugins over `phenix-kernel`. ACP remains one wire/adaptation
//! boundary and does not own session, execution, routing, tool, or durable state.
'''
if old not in text:
    raise SystemExit('ACP ownership text did not match expected source')
path.write_text(text.replace(old, new, 1))

path = root / 'AGENTS.md'
text = path.read_text()
start = text.index('## Architecture discipline\n')
end = text.index('\n## Change discipline\n')
architecture = '''## Architecture discipline

- `phenix-kernel` owns generic mechanisms, plugin hosting, persistence enforcement, authority attenuation, events, and tasks. It has no first-party agent-domain fallback.
- `phenix-plugin-suite` owns Phenix-specific session, context, execution, planning, workspace, routing, language, frontend, hook, job, debug, and repository-worker semantics.
- `phenix-harness` is the supported product assembly. It selects and configures plugins through ordinary kernel contracts.
- `phenix-acp` is a wire/adaptation boundary. ACP types do not own application semantics or durable state.
- `phenix-conductor` is migration source and compatibility code while #399 removes duplicated product ownership. Do not add new agent-domain ownership there.
- Frontends remain clients. Rendering, input handling, editor integration, and frontend packaging belong in frontend repositories.

The supported product path is kernel plus selected plugins through `phenix-harness`. Alternate or omitted services use the same plugin resolver; no kernel or compatibility registry may restore a missing first-party service.
'''
text = text[:start] + architecture + text[end:]
text = text.replace('`maintenance test system`: black-box conductor/process/protocol tests;', '`maintenance test system`: black-box Harness/process/protocol tests plus migration-only conductor coverage;')
path.write_text(text)

path = root / 'README.md'
text = path.read_text()
text = text.replace('# Phenix ACP\n\n`phenix-acp` is the headless ACP protocol, conductor, and backend-orchestration repository for Phenix.\n', '# Phenix AI\n\nThis repository contains the Phenix kernel, replaceable first-party Plugin Suite, Harness product assembly, and protocol/backend adapters. The current GitHub repository name is temporary.\n')
arch_start = text.index('## Architecture\n')
arch_end = text.index('\n## Configuration\n')
arch = '''## Architecture

```text
frontends / protocol adapters
            |
            v
      phenix-harness
      product policy
            |
            v
      phenix-kernel
 generic mechanisms only
            |
            v
  selected plugin providers
  Phenix Plugin Suite or alternatives
```

`phenix-kernel` owns plugin lifecycle, provider resolution, authority attenuation, generic persistence, events, and tasks. It does not own session, context, execution, planning, tool, model, frontend, or other agent-domain semantics.

`phenix-plugin-suite` implements the first-party Phenix services through the same contracts available to alternate plugins. `phenix-harness` selects the plugin set and product policy. Omitting a provider removes the service. Replacing a provider does not require a kernel change.

`phenix-conductor` remains in the workspace as migration source and compatibility coverage while the plugin migration is completed. It is not the supported product package and must not gain new domain ownership.

ACP is one protocol boundary. `phenix-acp` contains wire interoperability types; backend adapters translate ACP agents without becoming a second semantic runtime.

### Rust boundaries

| Crate | Responsibility |
| --- | --- |
| `phenix-kernel` | Generic plugin host, trust boundaries, persistence enforcement, events, tasks |
| `phenix-plugin-suite` | Replaceable first-party Phenix services |
| `phenix-harness` | Supported kernel + selected-plugin product assembly |
| `phenix-acp` | ACP wire interoperability boundary |
| `phenix-backend-acp` | ACP backend adapter |
| `phenix-conductor` | Migration source and compatibility coverage until duplicate ownership is removed |

There is no UI crate or Neovim plugin in this repository.
'''
text = text[:arch_start] + arch + text[arch_end:]
ctx_start = text.index('### Project context and skills\n')
ctx_end = text.index('\n## Packages\n')
ctx = '''### Project context and skills

Project context and skills are first-party Plugin Suite services. The context plugin owns discovery, exact content identity, injection history, and projection. The supported Harness reaches them through the ordinary kernel service contract. Kernel-only mode has no context or skill behavior.

Project instructions remain ambient input. Discoverable project documents and skills are revisioned resources rather than configuration identity. Skill metadata never expands execution authority; script execution still uses ordinary workspace/tool authority.
'''
text = text[:ctx_start] + ctx + text[ctx_end:]
pkg_start = text.index('## Packages\n')
pkg_end = text.index('\n## Built-in runtime authentication\n')
packages = '''## Packages

The flake exposes the supported compositions directly:

- `packages.<system>.phenix-kernel`: kernel-only runtime;
- `packages.<system>.phenix-harness`: default Harness composition;
- `packages.<system>.phenix`: supported product alias for the Harness;
- `lib.mkPhenixPlugin`: external/resource plugin packaging;
- `lib.mkPhenix`: declarative kernel + plugin composition.

The legacy conductor crate remains a migration source inside the Rust workspace. Product composition goes through `phenix-harness`.
'''
text = text[:pkg_start] + packages + text[pkg_end:]
text = text.replace('The conductor is mechanism, not policy. It validates and executes supplied backends, routing tables, workflows, and tool policy; it does not silently install preferred models, roles, or workflows.\n', 'The kernel is mechanism, not Phenix policy. Harness composition and plugins own the selected product behavior.\n')
text = text.replace('Validation is separated into source, Rust, integration/system, and realized ACP product boundaries. The product layer exercises the installed ACP/conductor artifacts; frontend behavior is tested in `phenix-nvim`.\n', 'Validation is separated into source, Rust, integration/system, and realized product boundaries. The product layer exercises the installed Harness and plugin compositions; frontend behavior is tested in frontend repositories.\n')
path.write_text(text)

path = root / 'modules/flake-module.nix'
text = path.read_text()
old = '''      phenixWrapped = {
        phenix = inputs.self.packages.${system}.phenix;
        conductor = inputs.self.packages.${system}.phenix-conductor;
        runtime = inputs.self.packages.${system}.phenix-conductor;
        stitch = inputs.self.packages.${system}.stitch;
        stitchMcp = inputs.self.packages.${system}.stitch-mcp;
      };
'''
new = '''      phenixWrapped = {
        phenix = inputs.self.packages.${system}.phenix;
        conductor = inputs.self.packages.${system}.phenix-kernel;
        harness = inputs.self.packages.${system}.phenix-harness;
        runtime = inputs.self.packages.${system}.phenix-harness;
        stitch = inputs.self.packages.${system}.stitch;
        stitchMcp = inputs.self.packages.${system}.stitch-mcp;
      };
'''
if old not in text:
    raise SystemExit('flake module ownership mapping did not match expected source')
path.write_text(text.replace(old, new, 1))

path = root / 'spec/plugin-implementation.md'
text = path.read_text()
for item in [
    'sessions and session-tree behavior',
    'artifacts, readers, read reuse, and invalidation',
    'context and skills',
    'tools, callables, execution, orchestration, and workers',
    'planning/objectives/decisions/history behavior that exists in the current product',
    'workspace and repository services',
    'default CLI suite, including Git/GitHub/search/read/write/shell integration where applicable',
    'model/provider/auth/routing services',
    'language intelligence',
    'frontend-facing services and projections',
    'hooks and persistent jobs',
    'debug/diagnostic services',
    'Expose the normal Harness package and keep `phenix` as the supported plugin-composed product package/alias.',
    'Add packaging checks for embedded, external, resource-only, replacement, and kernel-only compositions.',
    'Nix kernel-only, default Harness, and alternate-plugin compositions build',
]:
    text = text.replace(f'- [ ] {item}', f'- [x] {item}')
old_boundary = '''## Current boundary

The first-party Harness assembly now registers repository worker, sessions, artifacts, CLI probes, context, execution, language, planning, workspace, model routing, jobs, frontend services, hooks, and debug services through ordinary kernel manifests and factories. Focused Plugin Suite and Harness unit runs are green after repairing frontend test authority and the durable hook immutability assertion.

The supported `phenix`/`phenix-harness` Nix package still maps to the legacy conductor binary, so these userspace services are not yet the canonical product path. The remaining migration must switch the product/runtime boundary before the domain checklist can be promoted.
'''
new_boundary = '''## Current boundary

The supported `phenix` product is `phenix-harness`, built from `phenix-kernel` plus selected plugins. The complete first-party Plugin Suite is available through ordinary kernel service contracts. Nix composition exercises default, selected-suite, external replacement, resource-only, omission, and kernel-only runtime behavior.

The remaining migration is compatibility removal and parity closure. The legacy conductor crate still owns duplicate domain registries/state and several canonical tests. Move or replace those journeys with Harness-owned coverage before removing the corresponding conductor paths and tables.
'''
if old_boundary in text:
    text = text.replace(old_boundary, new_boundary, 1)
path.write_text(text)
