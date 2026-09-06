use phenix_core::{
    Authority, InvocationOutcome, Kernel, KernelConfig, PhenixValue, Project, ResolvedHarness,
    ResolvedHarnessActivation, ServiceId,
};
use phenix_sdk::{
    HasPhenixSchema, StaticComponentBehavior, StaticPluginDefinition, StaticPluginFactory,
};
use std::fmt::{self, Display, Formatter};

const SERVICE: &str = "fixture.fallible-provider.run@1";

#[derive(Clone, Debug, Eq, PartialEq, phenix_sdk::PhenixValue)]
struct Request {
    outcome: String,
}

#[derive(Clone, Debug, Eq, PartialEq, phenix_sdk::PhenixValue)]
struct Response {
    value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, phenix_sdk::PhenixValue)]
enum DomainFailure {
    Conflict { resource: String },
    Disconnected { provider: String },
}

impl Display for DomainFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("provider rejected request")
    }
}

#[phenix_sdk::plugin("fixture.fallible-provider")]
mod provider {
    use super::*;

    #[phenix(export(SERVICE), public, terminal)]
    pub fn run(request: Request) -> Result<Response, DomainFailure> {
        match request.outcome.as_str() {
            "conflict" => Err(DomainFailure::Conflict {
                resource: "workspace".into(),
            }),
            "disconnected" => Err(DomainFailure::Disconnected {
                provider: "remote".into(),
            }),
            value => Ok(Response {
                value: value.to_owned(),
            }),
        }
    }
}

fn active_kernel() -> (Kernel, Authority) {
    let authority = Authority::default();
    let manifest = <provider::Plugin as StaticPluginDefinition>::manifest();
    let components = <provider::Plugin as StaticPluginDefinition>::component_manifests();
    let resolved = ResolvedHarness::resolve(
        [manifest.clone()],
        components,
        std::iter::empty(),
        &authority,
    )
    .unwrap();
    let mut kernel = Kernel::new(KernelConfig::new([manifest.clone()]).unwrap());
    kernel
        .register_embedded_factory(manifest.id, || {
            <provider::Plugin as StaticPluginFactory>::factory()
        })
        .unwrap();
    kernel.activate_resolved_harness(&resolved).unwrap();
    kernel.activate_all().unwrap();
    (kernel, authority)
}

fn invoke(kernel: &mut Kernel, authority: &Authority, outcome: &str) -> InvocationOutcome {
    let input = serde_json::to_vec(&PhenixValue::from(&Request {
        outcome: outcome.into(),
    }))
    .unwrap();
    let output = kernel
        .invoke(&ServiceId::parse(SERVICE).unwrap(), &input, authority, None)
        .unwrap();
    let value: PhenixValue = serde_json::from_slice(&output).unwrap();
    InvocationOutcome::from_transport_value(value)
}

#[test]
fn generated_fallible_export_declares_domain_error_schema() {
    let exports = <provider::Component as StaticComponentBehavior>::exports();

    assert_eq!(exports.len(), 1);
    assert_eq!(exports[0].schema.error(), &DomainFailure::phenix_schema());
}

#[test]
fn generated_provider_dispatch_preserves_structural_domain_errors() {
    let (mut kernel, authority) = active_kernel();

    let conflict = invoke(&mut kernel, &authority, "conflict");
    let disconnected = invoke(&mut kernel, &authority, "disconnected");

    let InvocationOutcome::DomainError(conflict) = conflict else {
        panic!("conflict was not lowered as a domain error");
    };
    let InvocationOutcome::DomainError(disconnected) = disconnected else {
        panic!("disconnect was not lowered as a domain error");
    };

    let conflict = DomainFailure::try_from(Project(&conflict)).unwrap();
    let disconnected = DomainFailure::try_from(Project(&disconnected)).unwrap();

    assert_eq!(
        conflict,
        DomainFailure::Conflict {
            resource: "workspace".into(),
        }
    );
    assert_eq!(
        disconnected,
        DomainFailure::Disconnected {
            provider: "remote".into(),
        }
    );
    assert_eq!(conflict.to_string(), disconnected.to_string());
    assert_ne!(conflict, disconnected);
}

#[test]
fn generated_provider_dispatch_keeps_success_as_a_bare_value() {
    let (mut kernel, authority) = active_kernel();

    let outcome = invoke(&mut kernel, &authority, "ok");

    let InvocationOutcome::Success(value) = outcome else {
        panic!("success was not lowered as a success outcome");
    };
    assert_eq!(
        Response::try_from(Project(&value)).unwrap(),
        Response { value: "ok".into() }
    );
}
