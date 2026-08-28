from pathlib import Path

runtime = Path("rust/crates/phenix-harness/src/runtime_config.rs")
text = runtime.read_text()
old = '''    ModelTarget, OptionAssignment, OptionCommand, OptionKey, OptionResponse, OptionScope,
    OptionSubjectId, OptionValue, OrchestrationDefinition, RoutingProfile,
'''
new = '''    ModelTarget, OptionAssignment, OptionCommand, OptionKey, OptionResponse, OptionScope,
    OptionStartupPrecedence, OptionSubjectId, OptionValue, OrchestrationDefinition, RoutingProfile,
'''
if old not in text:
    raise SystemExit("runtime config import anchor missing")
runtime.write_text(text.replace(old, new, 1))

nix = Path("modules/plugin-packaging.nix")
text = nix.read_text()
old = '''                  ${pkgs.lib.optionalString (configDirectory != null || resources != [ ]) ''
                    wrapProgram "$out/bin/$program" \\
                      --set PHENIX_CONFIG_DIR ${
                        pkgs.lib.escapeShellArg (
                          if configDirectory != null then toString configDirectory else "$out/share/phenix"
                        )
                      }
                  ''}
'''
new = '''                  ${pkgs.lib.optionalString (configDirectory != null) ''
                    wrapProgram "$out/bin/$program" \\
                      --set PHENIX_CONFIG_DIR ${pkgs.lib.escapeShellArg (toString configDirectory)}
                  ''}
                  ${pkgs.lib.optionalString (configDirectory == null && resources != [ ]) ''
                    wrapProgram "$out/bin/$program" \\
                      --set PHENIX_CONFIG_DIR "$out/share/phenix"
                  ''}
'''
if old not in text:
    raise SystemExit("config directory wrapper anchor missing")
nix.write_text(text.replace(old, new, 1))
