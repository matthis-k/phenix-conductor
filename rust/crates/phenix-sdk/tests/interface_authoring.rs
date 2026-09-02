#[phenix_sdk::interface("fixture.models@1")]
struct Models;

#[test]
fn interface_attribute_owns_canonical_runtime_identity() {
    let id = <Models as phenix_sdk::InterfaceMarker>::interface_id();

    assert_eq!(id.as_str(), "fixture.models@1");
}
