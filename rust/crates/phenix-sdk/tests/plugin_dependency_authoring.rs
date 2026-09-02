mod sessions {
    #[phenix_sdk::plugin("fixture.sessions")]
    pub struct Plugin;
}

#[allow(dead_code)]
#[phenix_sdk::plugin("fixture.parent")]
struct Plugin {
    #[phenix(dep)]
    sessions: sessions::Plugin,
}

#[test]
fn direct_dependency_module_reexports_only_the_declared_plugin_type() {
    fn accepts_dependency(_: plugin::dependencies::sessions::Plugin) {}

    accepts_dependency(sessions::Plugin);
}

#[test]
fn concrete_dependency_still_participates_in_recursive_graph_composition() {
    let graph = phenix_sdk::StaticPluginGraph::compose::<Plugin>().unwrap();
    let ids = graph
        .ids()
        .map(phenix_sdk::PluginId::as_str)
        .collect::<Vec<_>>();

    assert_eq!(ids, ["fixture.sessions", "fixture.parent"]);
}
