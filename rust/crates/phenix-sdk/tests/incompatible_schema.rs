use phenix_core::{
    Authority, Kernel, KernelConfig, PhenixValue, PluginHost, PluginId, PluginInstance, Project,
    ResolvedHarness, ResolvedHarnessActivation, ServiceId,
};
use phenix_sdk::{phenix_plugin, PhenixValue as DerivePhenixValue};

mod provider {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct Request {
        prompt: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct Response {
        value: String,
    }

    phenix_plugin! {
        "fixture.incompatible-provider";

        provides {
            models: "fixture.incompatible.models@1" => Request => Response,
        }
    }
}

mod consumer {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct Request {
        prompt: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct Response {
        value: String,
        required_extra: u64,
    }

    phenix_plugin! {
        "fixture.incompatible-consumer";

        uses {
            models: "fixture.incompatible.models@1" => Request => Response,
        }
    }
}

mod replacement_provider_a {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct Request {
        prompt: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct Response {
        value: String,
    }

    phenix_plugin! {
        "fixture.replacement-provider-a";

        provides {
            models: "fixture.replacement.models@1" => Request => Response,
        }
    }

    pub struct Plugin;

    impl PluginInstance for Plugin {
        fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }

        fn invoke(
            &mut self,
            service: &ServiceId,
            input: &[u8],
            host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            if service != &models_service() {
                return Err(format!("unsupported provider-a service: {service}"));
            }
            phenix_plugin::provides::models::dispatch(
                host,
                input,
                |_request: Request| -> Result<Response, String> {
                    Ok(Response {
                        value: "provider.a".into(),
                    })
                },
            )
        }
    }

    pub fn models_service() -> ServiceId {
        ServiceId::parse("fixture.replacement.models@1").unwrap()
    }
}

mod replacement_provider_b {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct Request {
        prompt: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct Response {
        value: String,
        implementation: String,
    }

    phenix_plugin! {
        "fixture.replacement-provider-b";

        provides {
            models: "fixture.replacement.models@1" => Request => Response,
        }
    }

    pub struct Plugin;

    impl PluginInstance for Plugin {
        fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }

        fn invoke(
            &mut self,
            service: &ServiceId,
            input: &[u8],
            host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            if service != &models_service() {
                return Err(format!("unsupported provider-b service: {service}"));
            }
            phenix_plugin::provides::models::dispatch(
                host,
                input,
                |_request: Request| -> Result<Response, String> {
                    Ok(Response {
                        value: "provider.b".into(),
                        implementation: "replacement".into(),
                    })
                },
            )
        }
    }

    pub fn models_service() -> ServiceId {
        ServiceId::parse("fixture.replacement.models@1").unwrap()
    }
}

mod replacement_consumer {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct ModelRequest {
        prompt: String,
        trace_id: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    struct ModelResponse {
        value: PluginId,
    }

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    pub struct RunRequest {
        pub prompt: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, DerivePhenixValue)]
    pub struct RunResponse {
        pub value: String,
    }

    phenix_plugin! {
        "fixture.replacement-consumer";

        uses {
            models: "fixture.replacement.models@1" => ModelRequest => ModelResponse,
        }

        provides {
            run: "fixture.replacement.run@1" => RunRequest => RunResponse,
        }
    }

    pub struct Plugin;

    impl PluginInstance for Plugin {
        fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }

        fn invoke(
            &mut self,
            service: &ServiceId,
            input: &[u8],
            host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            if service != &run_service() {
                return Err(format!(
                    "unsupported replacement consumer service: {service}"
                ));
            }
            phenix_plugin::provides::run::dispatch(
                host,
                input,
                |request: RunRequest| -> Result<RunResponse, String> {
                    let context = phenix_plugin::context(host, (), ());
                    let response = context
                        .sdk
                        .models
                        .invoke(&ModelRequest {
                            prompt: request.prompt,
                            trace_id: "consumer-local".into(),
                        })
                        .map_err(|error| error.to_string())?;
                    Ok(RunResponse {
                        value: response.value.to_string(),
                    })
                },
            )
        }
    }

    pub fn run_service() -> ServiceId {
        ServiceId::parse("fixture.replacement.run@1").unwrap()
    }
}

#[test]
fn incompatible_component_schemas_fail_before_activation() {
    let authority = Authority::default();
    let error = ResolvedHarness::resolve(
        [
            provider::phenix_plugin::plugin_manifest(authority.clone()),
            consumer::phenix_plugin::plugin_manifest(authority.clone()),
        ],
        [
            provider::phenix_plugin::component_manifest(authority.clone()),
            consumer::phenix_plugin::component_manifest(authority.clone()),
        ],
        [],
        &authority,
    )
    .unwrap_err();

    assert!(error.to_string().contains("incompatible"));
}

fn invoke_with_provider(
    provider_manifest: phenix_core::PluginManifest,
    provider_component: phenix_core::ComponentManifest,
    provider_factory: impl Fn() -> Box<dyn PluginInstance> + Send + Sync + 'static,
) -> String {
    let authority = Authority::default();
    let consumer = replacement_consumer::phenix_plugin::plugin_manifest(authority.clone());
    let manifests = [provider_manifest.clone(), consumer.clone()];
    let resolved = ResolvedHarness::resolve(
        manifests.clone(),
        [
            provider_component,
            replacement_consumer::phenix_plugin::component_manifest(authority.clone()),
        ],
        [],
        &authority,
    )
    .unwrap();

    let mut kernel = Kernel::new(KernelConfig::new(manifests).unwrap());
    kernel
        .register_embedded_factory(provider_manifest.id, provider_factory)
        .unwrap();
    kernel
        .register_embedded_factory(consumer.id, || Box::new(replacement_consumer::Plugin))
        .unwrap();
    kernel.activate_resolved_harness(&resolved).unwrap();
    kernel.activate_all().unwrap();

    let input = serde_json::to_vec(&PhenixValue::from(&replacement_consumer::RunRequest {
        prompt: "hello".into(),
    }))
    .unwrap();
    let output = kernel
        .invoke(
            &replacement_consumer::run_service(),
            &input,
            &authority,
            None,
        )
        .unwrap();
    let value: PhenixValue = serde_json::from_slice(&output).unwrap();
    replacement_consumer::RunResponse::try_from(Project(&value))
        .unwrap()
        .value
}

#[test]
fn consumer_runs_unchanged_against_compatible_replacement_providers() {
    let authority = Authority::default();
    let first = invoke_with_provider(
        replacement_provider_a::phenix_plugin::plugin_manifest(authority.clone()),
        replacement_provider_a::phenix_plugin::component_manifest(authority.clone()),
        || Box::new(replacement_provider_a::Plugin),
    );
    let second = invoke_with_provider(
        replacement_provider_b::phenix_plugin::plugin_manifest(authority.clone()),
        replacement_provider_b::phenix_plugin::component_manifest(authority),
        || Box::new(replacement_provider_b::Plugin),
    );

    assert_eq!(first, "provider.a");
    assert_eq!(second, "provider.b");
}
