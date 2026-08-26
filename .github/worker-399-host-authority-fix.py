from pathlib import Path

external = Path("rust/crates/phenix-kernel/src/external.rs")
text = external.read_text()
old = '''    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        authority: &Authority,
    ) -> Result<Vec<u8>, String> {
        self.invoke_service(service, input, authority)
            .map_err(|error| error.to_string())
    }
'''
new = '''    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        self.invoke_service(service, input, host.authority())
            .map_err(|error| error.to_string())
    }
'''
if text.count(old) != 1:
    raise SystemExit("expected one external PluginInstance::invoke implementation")
external.write_text(text.replace(old, new, 1))

runtime = Path("rust/crates/phenix-kernel/src/runtime.rs")
text = runtime.read_text()
old = '''        let host = PluginHost {
            config: kernel.config(),
            plugin: &plugin("owner"),
            authority: &authority,
            persistence: &kernel.persistence,
        };
'''
new = '''        let owner_plugin = plugin("owner");
        let host = PluginHost {
            config: kernel.config(),
            states: &kernel.states,
            instances: &kernel.instances,
            plugin: &owner_plugin,
            authority: &authority,
            call_stack: BTreeSet::from([owner_plugin.clone()]),
            events: &kernel.events,
            tasks: &kernel.tasks,
            persistence: &kernel.persistence,
        };
'''
if text.count(old) != 1:
    raise SystemExit("expected one direct PluginHost test initializer")
runtime.write_text(text.replace(old, new, 1))
