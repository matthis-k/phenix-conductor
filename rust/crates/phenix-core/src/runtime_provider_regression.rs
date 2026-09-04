use crate::{
    runtime_provider_service, ArtifactRevision, Authority, CapabilityId, GraphReconciler, Kernel,
    KernelConfig, KernelError, LiveReconciliationError, PluginArtifact, PluginExecution,
    PluginHost, PluginId, PluginInstance, PluginManifest, PluginRuntimeProvider, ResolvedHarness,
    ResolvedHarnessActivation, RuntimeId, RuntimePluginCandidate, ServiceContribution, ServiceRole,
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

fn plugin(value: &str) -> PluginId {
    PluginId::parse(value).unwrap()
}

fn runtime(value: &str) -> RuntimeId {
    RuntimeId::parse(value).unwrap()
}

fn artifact(revision: &str) -> PluginArtifact {
    PluginArtifact {
        locator: "fixture.plugin".into(),
        revision: ArtifactRevision::from_content(revision.as_bytes()),
        configuration: BTreeMap::new(),
    }
}

fn bridge_manifest(
    id: &str,
    provided_runtime: &RuntimeId,
    execution: PluginExecution,
) -> PluginManifest {
    PluginManifest {
        id: plugin(id),
        version: 1,
        execution,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            service: runtime_provider_service(provided_runtime),
            role: ServiceRole::Terminal,
            priority: 0,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::default(),
    }
}

fn guest_manifest(
    id: &str,
    runtime: RuntimeId,
    revision: &str,
    authority: Authority,
) -> PluginManifest {
    PluginManifest {
        id: plugin(id),
        version: 1,
        execution: PluginExecution::Runtime {
            runtime,
            artifact: artifact(revision),
        },
        dependencies: Vec::new(),
        services: Vec::new(),
        resource_namespaces: Vec::new(),
        maximum_authority: authority,
    }
}

struct PreparedGuest {
    started_authority: Arc<Mutex<Option<Authority>>>,
}

impl PluginInstance for PreparedGuest {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        *self.started_authority.lock().unwrap() = Some(host.authority().clone());
        Ok(())
    }
}

struct Bridge {
    fail: Arc<AtomicBool>,
    prepared_authority: Arc<Mutex<Option<Authority>>>,
    started_authority: Arc<Mutex<Option<Authority>>>,
}

impl PluginRuntimeProvider for Bridge {
    fn prepare(
        &mut self,
        candidate: RuntimePluginCandidate<'_>,
    ) -> Result<Box<dyn PluginInstance>, String> {
        *self.prepared_authority.lock().unwrap() = Some(candidate.guest_authority.clone());
        if self.fail.load(Ordering::Acquire) {
            return Err("candidate rejected".into());
        }
        Ok(Box::new(PreparedGuest {
            started_authority: Arc::clone(&self.started_authority),
        }))
    }
}

impl PluginInstance for Bridge {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn runtime_provider(&mut self) -> Option<&mut dyn PluginRuntimeProvider> {
        Some(self)
    }
}

#[test]
fn arbitrary_runtime_identity_resolves_through_an_embedded_provider() {
    let runtime = runtime("vendor.runtime");
    let bridge = bridge_manifest("fixture.bridge", &runtime, PluginExecution::Embedded);
    let guest = guest_manifest(
        "fixture.guest",
        runtime.clone(),
        "sha256:guest-v1",
        Authority::default(),
    );

    let config = KernelConfig::new([guest.clone(), bridge.clone()]).unwrap();
    let binding = config.runtime_binding(&guest.id).unwrap();

    assert_eq!(binding.runtime, runtime);
    assert_eq!(binding.provider, bridge.id);
    assert_eq!(
        binding.artifact_revision,
        ArtifactRevision::from_content(b"sha256:guest-v1")
    );
    assert_eq!(config.activation_order(), &[bridge.id, guest.id]);
}

#[test]
fn unknown_runtime_is_rejected_during_graph_resolution() {
    let runtime = runtime("missing.runtime");
    let guest = guest_manifest(
        "fixture.guest",
        runtime.clone(),
        "sha256:guest-v1",
        Authority::default(),
    );

    assert_eq!(
        KernelConfig::new([guest]).unwrap_err(),
        KernelError::RuntimeProviderUnavailable(runtime)
    );
}

#[test]
fn runtime_provider_cycles_are_rejected_by_the_normal_dependency_graph() {
    let runtime_a = runtime("runtime.a");
    let runtime_b = runtime("runtime.b");
    let bridge_a = bridge_manifest(
        "bridge.a",
        &runtime_a,
        PluginExecution::Runtime {
            runtime: runtime_b.clone(),
            artifact: artifact("sha256:a"),
        },
    );
    let bridge_b = bridge_manifest(
        "bridge.b",
        &runtime_b,
        PluginExecution::Runtime {
            runtime: runtime_a,
            artifact: artifact("sha256:b"),
        },
    );

    assert!(matches!(
        KernelConfig::new([bridge_a, bridge_b]),
        Err(KernelError::DependencyCycle(_))
    ));
}

#[test]
fn guest_authority_is_independent_from_bridge_authority() {
    let runtime = runtime("vendor.runtime");
    let guest_capability = CapabilityId::parse("guest.read").unwrap();
    let bridge_capability = CapabilityId::parse("bridge.exec").unwrap();
    let guest_authority = Authority::new([guest_capability.clone()]);
    let mut bridge = bridge_manifest("fixture.bridge", &runtime, PluginExecution::Embedded);
    bridge.maximum_authority = Authority::new([bridge_capability.clone()]);
    let guest = guest_manifest(
        "fixture.guest",
        runtime,
        "sha256:guest-v1",
        guest_authority.clone(),
    );
    let prepared_authority = Arc::new(Mutex::new(None));
    let started_authority = Arc::new(Mutex::new(None));
    let fail = Arc::new(AtomicBool::new(false));
    let mut kernel = Kernel::new(KernelConfig::new([bridge.clone(), guest]).unwrap());
    let prepared_for_factory = Arc::clone(&prepared_authority);
    let started_for_factory = Arc::clone(&started_authority);
    let fail_for_factory = Arc::clone(&fail);
    kernel
        .register_embedded_factory(bridge.id, move || {
            Box::new(Bridge {
                fail: Arc::clone(&fail_for_factory),
                prepared_authority: Arc::clone(&prepared_for_factory),
                started_authority: Arc::clone(&started_for_factory),
            })
        })
        .unwrap();

    kernel.activate_all().unwrap();

    let prepared = prepared_authority.lock().unwrap().clone().unwrap();
    let started = started_authority.lock().unwrap().clone().unwrap();
    assert!(prepared.permits(&guest_capability));
    assert!(!prepared.permits(&bridge_capability));
    assert_eq!(prepared, guest_authority);
    assert_eq!(started, guest_authority);
}

#[test]
fn artifact_revision_and_resolved_provider_are_pinned_by_generation() {
    let runtime = runtime("vendor.runtime");
    let bridge = bridge_manifest("fixture.bridge", &runtime, PluginExecution::Embedded);
    let first_guest = guest_manifest(
        "fixture.guest",
        runtime.clone(),
        "sha256:guest-v1",
        Authority::default(),
    );
    let second_guest = guest_manifest(
        "fixture.guest",
        runtime,
        "sha256:guest-v2",
        Authority::default(),
    );
    let first =
        ResolvedHarness::resolve([bridge.clone(), first_guest], [], [], &Authority::default())
            .unwrap();
    let second =
        ResolvedHarness::resolve([bridge, second_guest], [], [], &Authority::default()).unwrap();

    assert_ne!(first.generation(), second.generation());
    assert_eq!(
        first
            .kernel_config()
            .runtime_binding(&plugin("fixture.guest"))
            .unwrap()
            .provider,
        plugin("fixture.bridge")
    );
    assert_eq!(
        first
            .kernel_config()
            .runtime_binding(&plugin("fixture.guest"))
            .unwrap()
            .artifact_revision,
        ArtifactRevision::from_content(b"sha256:guest-v1")
    );
}

#[test]
fn failed_runtime_prepare_keeps_the_active_generation() {
    let runtime = runtime("vendor.runtime");
    let bridge = bridge_manifest("fixture.bridge", &runtime, PluginExecution::Embedded);
    let initial_guest = guest_manifest(
        "fixture.guest",
        runtime.clone(),
        "sha256:guest-v1",
        Authority::default(),
    );
    let candidate_guest = guest_manifest(
        "fixture.guest",
        runtime,
        "sha256:guest-v2",
        Authority::default(),
    );
    let initial = ResolvedHarness::resolve(
        [bridge.clone(), initial_guest],
        [],
        [],
        &Authority::default(),
    )
    .unwrap();
    let candidate = ResolvedHarness::resolve(
        [bridge.clone(), candidate_guest],
        [],
        [],
        &Authority::default(),
    )
    .unwrap();
    let active_generation = initial.generation().clone();
    let fail = Arc::new(AtomicBool::new(false));
    let prepared_authority = Arc::new(Mutex::new(None));
    let started_authority = Arc::new(Mutex::new(None));
    let mut kernel = Kernel::new(initial.kernel_config().clone());
    kernel.activate_resolved_harness(&initial).unwrap();
    let fail_for_factory = Arc::clone(&fail);
    let prepared_for_factory = Arc::clone(&prepared_authority);
    let started_for_factory = Arc::clone(&started_authority);
    kernel
        .register_embedded_factory(bridge.id, move || {
            Box::new(Bridge {
                fail: Arc::clone(&fail_for_factory),
                prepared_authority: Arc::clone(&prepared_for_factory),
                started_authority: Arc::clone(&started_for_factory),
            })
        })
        .unwrap();
    kernel.activate_all().unwrap();
    fail.store(true, Ordering::Release);
    let mut reconciler = GraphReconciler::new(initial);

    let error = reconciler
        .activate_candidate_on_kernel(&mut kernel, candidate)
        .unwrap_err();

    assert!(matches!(
        error,
        LiveReconciliationError::Runtime(KernelError::RuntimePrepare { .. })
    ));
    assert_eq!(kernel.graph_generation(), Some(&active_generation));
    assert_eq!(reconciler.active().generation(), &active_generation);
}

struct StartFailGuest {
    fail: Arc<AtomicBool>,
}

impl PluginInstance for StartFailGuest {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        if self.fail.load(Ordering::Acquire) {
            return Err("candidate start rejected".into());
        }
        Ok(())
    }
}

struct StartFailBridge {
    fail: Arc<AtomicBool>,
}

impl PluginRuntimeProvider for StartFailBridge {
    fn prepare(
        &mut self,
        _candidate: RuntimePluginCandidate<'_>,
    ) -> Result<Box<dyn PluginInstance>, String> {
        Ok(Box::new(StartFailGuest {
            fail: Arc::clone(&self.fail),
        }))
    }
}

impl PluginInstance for StartFailBridge {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn runtime_provider(&mut self) -> Option<&mut dyn PluginRuntimeProvider> {
        Some(self)
    }
}

#[test]
fn unknown_runtime_candidate_keeps_the_active_generation() {
    let active_runtime = runtime("vendor.runtime");
    let bridge = bridge_manifest("fixture.bridge", &active_runtime, PluginExecution::Embedded);
    let guest = guest_manifest(
        "fixture.guest",
        active_runtime,
        "sha256:guest-v1",
        Authority::default(),
    );
    let active =
        ResolvedHarness::resolve([bridge.clone(), guest], [], [], &Authority::default()).unwrap();
    let active_generation = active.generation().clone();
    let fail = Arc::new(AtomicBool::new(false));
    let prepared_authority = Arc::new(Mutex::new(None));
    let started_authority = Arc::new(Mutex::new(None));
    let mut kernel = Kernel::new(active.kernel_config().clone());
    kernel.activate_resolved_harness(&active).unwrap();
    kernel
        .register_embedded_factory(bridge.id, move || {
            Box::new(Bridge {
                fail: Arc::clone(&fail),
                prepared_authority: Arc::clone(&prepared_authority),
                started_authority: Arc::clone(&started_authority),
            })
        })
        .unwrap();
    kernel.activate_all().unwrap();

    let missing = runtime("missing.runtime");
    let invalid_guest = guest_manifest(
        "fixture.guest",
        missing.clone(),
        "sha256:guest-v2",
        Authority::default(),
    );
    let error =
        ResolvedHarness::resolve([invalid_guest], [], [], &Authority::default()).unwrap_err();

    assert!(matches!(
        error,
        crate::ResolvedHarnessError::Kernel(KernelError::RuntimeProviderUnavailable(runtime))
            if runtime == missing
    ));
    assert_eq!(kernel.graph_generation(), Some(&active_generation));
}

#[test]
fn failed_runtime_start_keeps_the_active_generation() {
    let runtime = runtime("vendor.runtime");
    let bridge = bridge_manifest("fixture.bridge", &runtime, PluginExecution::Embedded);
    let initial_guest = guest_manifest(
        "fixture.guest",
        runtime.clone(),
        "sha256:guest-v1",
        Authority::default(),
    );
    let candidate_guest = guest_manifest(
        "fixture.guest",
        runtime,
        "sha256:guest-v2",
        Authority::default(),
    );
    let initial = ResolvedHarness::resolve(
        [bridge.clone(), initial_guest],
        [],
        [],
        &Authority::default(),
    )
    .unwrap();
    let candidate = ResolvedHarness::resolve(
        [bridge.clone(), candidate_guest],
        [],
        [],
        &Authority::default(),
    )
    .unwrap();
    let active_generation = initial.generation().clone();
    let fail = Arc::new(AtomicBool::new(false));
    let fail_for_factory = Arc::clone(&fail);
    let mut kernel = Kernel::new(initial.kernel_config().clone());
    kernel.activate_resolved_harness(&initial).unwrap();
    kernel
        .register_embedded_factory(bridge.id, move || {
            Box::new(StartFailBridge {
                fail: Arc::clone(&fail_for_factory),
            })
        })
        .unwrap();
    kernel.activate_all().unwrap();
    fail.store(true, Ordering::Release);
    let mut reconciler = GraphReconciler::new(initial);

    let error = reconciler
        .activate_candidate_on_kernel(&mut kernel, candidate)
        .unwrap_err();

    assert!(matches!(
        error,
        LiveReconciliationError::Runtime(KernelError::PluginStart { .. })
    ));
    assert_eq!(kernel.graph_generation(), Some(&active_generation));
    assert_eq!(reconciler.active().generation(), &active_generation);
}
