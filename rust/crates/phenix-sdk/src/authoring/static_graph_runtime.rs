use super::{StaticPluginDefinition, StaticPluginGraph};

impl StaticPluginGraph {
    /// Preload every reusable zero-input embedded factory carried by this static graph.
    ///
    /// An embedded descriptor without a factory is not invalid. It represents state that must be
    /// constructed explicitly and prepared with `preload_embedded_instance`.
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
                (phenix_core::PluginExecution::Embedded, None) | (_, None) => {}
                (_, Some(_)) => {
                    return Err(phenix_core::KernelError::WrongExecutionKind(id.clone()));
                }
            }
        }
        Ok(())
    }

    /// Prepare an already-constructed stateful plugin for normal kernel activation.
    ///
    /// Construction remains ordinary Rust. This method only performs the generic type-to-runtime
    /// boundary and derives the plugin identity from the authored type.
    #[doc(hidden)]
    pub fn preload_embedded_instance<T: StaticPluginDefinition>(
        &self,
        kernel: &mut phenix_core::Kernel,
        instance: Box<dyn phenix_core::PluginInstance>,
    ) -> Result<(), phenix_core::KernelError> {
        let authored = T::descriptor();
        let id = authored.id.clone();
        let descriptor = self
            .descriptor(&id)
            .ok_or_else(|| phenix_core::KernelError::UnknownPlugin(id.clone()))?;
        if descriptor.definition != authored.definition {
            return Err(phenix_core::KernelError::UnknownPlugin(id));
        }
        if !matches!(descriptor.execution, phenix_core::PluginExecution::Embedded) {
            return Err(phenix_core::KernelError::WrongExecutionKind(id));
        }
        kernel.preload_embedded_instance(id, instance);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Authority, PluginExecution, PluginId, StaticPluginComponents, StaticPluginDependency,
        StaticPluginDescriptor, StaticPluginResources,
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
    struct ExplicitState;

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

    impl StaticPluginDefinition for ExplicitState {
        fn descriptor() -> StaticPluginDescriptor {
            let mut descriptor = descriptor("fixture.graph.explicit-state", Vec::new());
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

    impl StaticPluginComponents for ExplicitState {
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

    impl StaticPluginResources for ExplicitState {
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
    fn embedded_graph_accepts_explicitly_constructed_state() {
        let graph = StaticPluginGraph::compose::<ExplicitState>().unwrap();
        let id = PluginId::parse("fixture.graph.explicit-state").unwrap();
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

        graph.preload_embedded_factories(&mut kernel).unwrap();
        assert_eq!(
            kernel.activate_all(),
            Err(phenix_core::KernelError::EmbeddedFactoryMissing(id.clone()))
        );

        graph
            .preload_embedded_instance::<ExplicitState>(&mut kernel, Box::new(Instance))
            .unwrap();
        kernel.activate_all().unwrap();
    }
}
