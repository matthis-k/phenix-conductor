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

mod diamond {
    #[phenix_sdk::plugin("fixture.diamond.base")]
    pub struct Base;

    #[allow(dead_code)]
    #[phenix_sdk::plugin("fixture.diamond.left")]
    pub struct Left {
        #[phenix(dep)]
        base: Base,
    }

    #[allow(dead_code)]
    #[phenix_sdk::plugin("fixture.diamond.right")]
    pub struct Right {
        #[phenix(dep)]
        base: Base,
    }

    #[allow(dead_code)]
    #[phenix_sdk::plugin("fixture.diamond.root")]
    pub struct Root {
        #[phenix(dep)]
        left: Left,
        #[phenix(dep)]
        right: Right,
    }
}

struct DuplicateFirst;
struct DuplicateSecond;
struct DuplicateRoot;
struct CycleA;
struct CycleB;

fn descriptor(id: &str, definition: &'static str) -> phenix_sdk::StaticPluginDescriptor {
    phenix_sdk::StaticPluginDescriptor {
        id: phenix_sdk::PluginId::parse(id).unwrap(),
        definition,
        dependencies: Vec::new(),
    }
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

#[test]
fn diamond_dependencies_are_deduplicated_by_plugin_id() {
    let graph = phenix_sdk::StaticPluginGraph::compose::<diamond::Root>().unwrap();
    let ids = graph
        .ids()
        .map(phenix_sdk::PluginId::as_str)
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        [
            "fixture.diamond.base",
            "fixture.diamond.left",
            "fixture.diamond.right",
            "fixture.diamond.root",
        ]
    );
}

impl phenix_sdk::StaticPluginDefinition for DuplicateFirst {
    fn descriptor() -> phenix_sdk::StaticPluginDescriptor {
        descriptor("fixture.duplicate", "fixture::DuplicateFirst")
    }
}

impl phenix_sdk::StaticPluginDefinition for DuplicateSecond {
    fn descriptor() -> phenix_sdk::StaticPluginDescriptor {
        descriptor("fixture.duplicate", "fixture::DuplicateSecond")
    }
}

impl phenix_sdk::StaticPluginDefinition for DuplicateRoot {
    fn descriptor() -> phenix_sdk::StaticPluginDescriptor {
        let mut descriptor = descriptor("fixture.duplicate-root", "fixture::DuplicateRoot");
        descriptor.dependencies = vec![
            phenix_sdk::StaticPluginDependency::of::<DuplicateFirst>(),
            phenix_sdk::StaticPluginDependency::of::<DuplicateSecond>(),
        ];
        descriptor
    }
}

#[test]
fn incompatible_duplicate_plugin_ids_are_rejected() {
    let error = match phenix_sdk::StaticPluginGraph::compose::<DuplicateRoot>() {
        Ok(_) => panic!("duplicate plugin identities must be rejected"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        phenix_sdk::StaticPluginGraphError::DuplicateId { ref id, first, second }
            if id.as_str() == "fixture.duplicate"
                && first == "fixture::DuplicateFirst"
                && second == "fixture::DuplicateSecond"
    ));
}

impl phenix_sdk::StaticPluginDefinition for CycleA {
    fn descriptor() -> phenix_sdk::StaticPluginDescriptor {
        let mut descriptor = descriptor("fixture.cycle-a", "fixture::CycleA");
        descriptor.dependencies = vec![phenix_sdk::StaticPluginDependency::of::<CycleB>()];
        descriptor
    }
}

impl phenix_sdk::StaticPluginDefinition for CycleB {
    fn descriptor() -> phenix_sdk::StaticPluginDescriptor {
        let mut descriptor = descriptor("fixture.cycle-b", "fixture::CycleB");
        descriptor.dependencies = vec![phenix_sdk::StaticPluginDependency::of::<CycleA>()];
        descriptor
    }
}

#[test]
fn dependency_cycles_are_rejected() {
    let error = match phenix_sdk::StaticPluginGraph::compose::<CycleA>() {
        Ok(_) => panic!("dependency cycles must be rejected"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        phenix_sdk::StaticPluginGraphError::Cycle(ref id) if id.as_str() == "fixture.cycle-a"
    ));
}
