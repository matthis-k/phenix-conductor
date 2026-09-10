use phenix_acp_stdio::{
    serve_sdk_application, serve_stdio_with_events_and_callbacks, ChannelTransport,
    ClientCapabilityCallbacks, ClientCapabilityIdentity, SdkApplicationService,
};
use phenix_application_interface::{GetSdk, InvokeCapability, Operation};
use phenix_core::{
    CapabilityGenerationId, ClientConnectionId, ContractId, ObservableRegistration,
    ObservableStore, PhenixValue, PluginId, PluginManifest, ResolvedSdkContributions, RuntimeId,
    SdkContribution, SdkNamespace, SdkObservableResource, SdkResourceId, SharedCapabilityRegistry,
    SnapshotPolicy, Type, ValueId, ValuePath,
};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let provider = PluginId::parse("fixture").expect("fixture provider id is valid");
    let value_id = ValueId::parse("fixture.state@1").expect("fixture value id is valid");
    let store = ObservableStore::default();
    store
        .register(ObservableRegistration {
            id: value_id.clone(),
            owner: provider.clone(),
            schema: Type::U64,
            snapshot_policy: SnapshotPolicy::CopyOnChange,
            initial: PhenixValue::U64(41),
        })
        .expect("fixture observable registers");

    let mut contribution = SdkContribution::new(
        provider.clone(),
        SdkNamespace::parse("fixture").expect("fixture namespace is valid"),
    );
    contribution.insert_observable(SdkObservableResource::new(
        SdkResourceId::parse("sdk/fixture/state").expect("fixture resource id is valid"),
        ["state"],
        value_id,
        ValuePath::root(),
        Type::U64,
    ));
    let sdk = ResolvedSdkContributions::resolve(
        &[PluginManifest::resource_only(provider)],
        &[],
        [contribution],
    )
    .expect("fixture SDK resolves");

    let capabilities = SharedCapabilityRegistry::default();
    let (client_callbacks, callback_receiver) = ClientCapabilityCallbacks::bounded(8);
    let service = SdkApplicationService::new(
        &sdk,
        &store,
        capabilities,
        RuntimeId::parse("fixture.runtime").expect("fixture runtime id is valid"),
        CapabilityGenerationId::parse("fixture-generation")
            .expect("fixture generation id is valid"),
        client_callbacks,
        ClientCapabilityIdentity::new(
            ClientConnectionId::parse("fixture-client").expect("fixture client id is valid"),
            CapabilityGenerationId::parse("fixture-client-generation")
                .expect("fixture client generation is valid"),
        ),
    )
    .expect("fixture SDK service builds");
    let (transport, application_receiver) = ChannelTransport::new(8);
    let application = tokio::spawn(serve_sdk_application(service, application_receiver));
    let (_event_sender, event_receiver) = mpsc::channel(1);

    serve_stdio_with_events_and_callbacks(
        transport,
        [
            ContractId::parse(GetSdk::ID).expect("SDK get operation id is valid"),
            ContractId::parse(InvokeCapability::ID)
                .expect("capability invoke operation id is valid"),
        ],
        event_receiver,
        callback_receiver,
    )
    .await
    .expect("fixture ACP server exits cleanly");

    application.abort();
}
