from pathlib import Path

path = Path("modules/plugin-packaging.nix")
text = path.read_text()
old = '''            ''
              mkdir -p "$out/share/phenix"
              rm -f "$out/share/phenix/settings.json"
              cp ${settingsFile} "$out/share/phenix/settings.json"

              for program in phenix phenix-harness; do
                if [ -e "$out/bin/$program" ]; then
                  wrapProgram "$out/bin/$program" \\
                    --set PHENIX_CONFIG_DIR "$out/share/phenix"
'''
new = '''            ''
              ${pkgs.lib.optionalString (resources != [ ] || settings != { }) ''
                mkdir -p "$out/share/phenix"
                rm -f "$out/share/phenix/settings.json"
                cp ${settingsFile} "$out/share/phenix/settings.json"
              ''}

              for program in phenix phenix-harness; do
                if [ -e "$out/bin/$program" ]; then
                  ${pkgs.lib.optionalString (resources != [ ] || settings != { }) ''
                    wrapProgram "$out/bin/$program" \\
                      --set PHENIX_CONFIG_DIR "$out/share/phenix"
                  ''}
'''
if old not in text:
    raise SystemExit("config root wrapper anchor missing")
path.write_text(text.replace(old, new, 1))
