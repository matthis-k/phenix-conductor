use super::StaticPluginGraph;

impl StaticPluginGraph {
    pub fn preload_embedded_factories(
        &self,
        kernel: &mut phenix_core::Kernel,
    ) -> Result<(), phenix_core::KernelError> {
        for id in self.ids() {
            let descriptor = self
                .descriptor(id)
                .expect("static graph ids always resolve to descriptors");
            match (&descriptor.execution, descriptor.embedded_factory) {
                (phenix_core::PluginExecution::Embedded, Some(factory)) => {
                    kernel.preload_embedded_factory(id.clone(), factory);
                }
                (phenix_core::PluginExecution::Embedded, None) => {
                    return Err(phenix_core::KernelError::EmbeddedFactoryMissing(id.clone()));
                }
                (_, None) => {}
                (_, Some(_)) => {
                    return Err(phenix_core::KernelError::WrongExecutionKind(id.clone()));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Authority, PluginExecution, PluginId, StaticPluginComponents, StaticPluginDefinition,
        StaticPluginDependency, StaticPluginDescriptor, StaticPluginResources,
    };

    struct Instance;

    impl phenix_core::PluginInstance for Instance {
        fn start(&mut self, _host: &phenix_core::PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }
    }

    fn factory() -> Box<dyn phenix_core::PluginInstance> {
        Box::new(Instance)
    }

    struct Leaf;
    struct Root;
    struct MissingFactory;

    impl StaticPluginDefinition for Leaf {
        fn descriptor() -> StaticPluginDescriptor {
            descriptor("fixture.graph.leaf", Vec::new())
        }
    }

    impl StaticPluginDefinition for Root {
        fn descriptor() -> StaticPluginDescriptor {
            descriptor(
                "fixture.graph.root",
                vec![StaticPluginDependency::of::<Leaf>()],
            )
        }
    }

    impl StaticPluginDefinition for MissingFactory {
        fn descriptor() -> StaticPluginDescriptor {
            let mut descriptor = descriptor("fixture.graph.missing-factory", Vec::new());
            descriptor.embedded_factory = None;
            descriptor
        }
    }

    impl StaticPluginComponents for Leaf {
        fn components() -> Vec<crate::StaticComponentDescriptor> {
            Vec::new()
        }
    }

    impl StaticPluginComponents for Root {
        fn components() -> Vec<crate::StaticComponentDescriptor> {
            Vec::new()
        }
    }

    impl StaticPluginResources for Leaf {
        fn resources() -> Vec<crate::StaticResourceDescriptor> {
            Vec::new()
        }
    }

    impl StaticPluginResources for Root {
        fn resources() -> Vec<crate::StaticResourceDescriptor> {
            Vec::new()
        }
    }

    fn descriptor(
        id: &'static str,
        dependencies: Vec<StaticPluginDependency>,
    ) -> StaticPluginDescriptor {
        StaticPluginDescriptor {
            id: PluginId::parse(id).unwrap(),
            definition: id,
            version: 1,
            execution: PluginExecution::Embedded,
            maximum_authority: Authority::default(),
            dependencies,
            embedded_factory: Some(factory),
        }
    }

    #[test]
    fn graph_preloads_transitive_embedded_factories() {
        let graph = StaticPluginGraph::compose::<Root>().unwrap();
        let manifests = [Leaf::manifest(), Root::manifest()];
        let mut kernel =
            phenix_core::Kernel::new(phenix_core::KernelConfig::new(manifests).unwrap());

        graph.preload_embedded_factories(&mut kernel).unwrap();
        kernel.activate_all().unwrap();
    }

    #[test]
    fn embedded_graph_rejects_missing_generated_factory() {
        let graph = StaticPluginGraph::compose::<MissingFactory>().unwrap();
        let id = PluginId::parse("fixture.graph.missing-factory").unwrap();
        let mut kernel = phenix_core::Kernel::new(
            phenix_core::KernelConfig::new([phenix_core::PluginManifest {
                id: id.clone(),
                version: 1,
                execution: PluginExecution::Embedded,
                dependencies: Vec::new(),
                services: Vec::new(),
                resource_namespaces: Vec::new(),
                maximum_authority: Authority::default(),
            }])
            .unwrap(),
        );

        assert_eq!(
            graph.preload_embedded_factories(&mut kernel),
            Err(phenix_core::KernelError::EmbeddedFactoryMissing(id))
        );
    }
}
