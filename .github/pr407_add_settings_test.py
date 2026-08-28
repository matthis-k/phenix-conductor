from pathlib import Path

path = Path("modules/plugin-packaging.nix")
text = path.read_text()

composition_anchor = '''      defaultComposition = mkPhenix {
        inherit pkgs;
        plugins = defaultPlugins;
        resources = [ harnessResources ];
      };
'''
composition = composition_anchor + '''      settingsComposition = mkPhenix {
        inherit pkgs;
        plugins = defaultPlugins;
        resources = [ harnessResources ];
        settings = {
          global = {
            "session.auto_create" = false;
          };
          agents = {
            "agent.scout" = {
              "agent.max_parallel_tasks" = 4;
            };
          };
        };
      };
'''
if "settingsComposition = mkPhenix" not in text:
    if composition_anchor not in text:
        raise SystemExit("default composition anchor missing")
    text = text.replace(composition_anchor, composition, 1)

test_anchor = '''            jq -e '(.plugins | length == 17) and ([.plugins[] | select(startswith("phenix.basic-"))] | length == 0) and (.services | index("phenix.sessions@1") != null)' "$TMPDIR/default-services.json" >/dev/null

'''
test = test_anchor + '''            jq -e '.global["session.auto_create"] == false and .agents["agent.scout"]["agent.max_parallel_tasks"] == 4' \\
              "${settingsComposition}/share/phenix/settings.json" >/dev/null
            export PHENIX_STATE_DB="$TMPDIR/settings.sqlite"
            printf '%s\\n' '{"id":1,"service":"phenix.options@1","input":{"operation":"resolve","key":"session.auto_create","context":{}}}' \\
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-option.json"
            jq -e '.status == "ok" and .output.result == "value" and .output.option.value.type == "bool" and .output.option.value.value == false and .output.option.source == "global"' \\
              "$TMPDIR/settings-option.json" >/dev/null
            printf '%s\\n' '{"id":2,"service":"phenix.sdk.config@1","input":{"operation":"read","path":"settings.json"}}' \\
              | "${settingsComposition}/bin/phenix" > "$TMPDIR/settings-config.json"
            jq -e '.status == "ok" and .output.result == "file" and ((.output.content | implode | fromjson).global["session.auto_create"] == false)' \\
              "$TMPDIR/settings-config.json" >/dev/null

'''
if "settings-option.json" not in text:
    if test_anchor not in text:
        raise SystemExit("packaging test anchor missing")
    text = text.replace(test_anchor, test, 1)

path.write_text(text)
