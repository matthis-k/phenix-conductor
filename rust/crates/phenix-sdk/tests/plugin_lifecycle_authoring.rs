use phenix_sdk::StaticPluginLifecycle;

#[phenix_sdk::plugin("fixture.lifecycle")]
struct Plugin;

#[allow(dead_code)]
#[phenix_sdk::plugin]
impl Plugin {
    #[phenix(start)]
    fn activate(&mut self, _context: &phenix_sdk::PluginContext<'_, '_, ()>) -> Result<(), String> {
        Ok(())
    }

    #[phenix(stop)]
    fn deactivate(
        &mut self,
        _context: &phenix_sdk::PluginContext<'_, '_, ()>,
    ) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn lifecycle_impl_preserves_start_and_stop_semantics() {
    let lifecycle = Plugin::lifecycle();

    assert_eq!(lifecycle.start, Some("activate"));
    assert_eq!(lifecycle.stop, Some("deactivate"));
    assert!(!lifecycle.uses_kernel_defaults());
}

#[test]
fn lifecycle_impl_generates_plugin_instance_adaptation() {
    let instance: Box<dyn phenix_sdk::__phenix_plugin::PluginInstance> =
        Plugin.__phenix_into_plugin_instance();

    drop(instance);
}

#[test]
fn plugin_identity_remains_owned_by_the_plugin_declaration() {
    assert_eq!(Plugin::plugin_id().as_str(), "fixture.lifecycle");
}
