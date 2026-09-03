use phenix_sdk::{HasPhenixSchema, StaticPluginDefinition, StaticPluginPublicProjection};

#[derive(Clone, Debug, Eq, PartialEq, phenix_sdk::PhenixValue)]
struct Request {
    value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, phenix_sdk::PhenixValue)]
struct Response {
    value: String,
}

#[phenix_sdk::interface("fixture.public-projection.call@1")]
struct PublicCall;

#[phenix_sdk::interface("fixture.public-projection.private@1")]
struct PrivateCall;

#[phenix_sdk::component]
struct Api;

#[phenix_sdk::component]
impl Api {
    #[phenix(export(PublicCall), public, terminal)]
    fn public_call(&mut self, request: Request) -> Response {
        Response {
            value: request.value,
        }
    }

    #[phenix(export(PrivateCall))]
    fn private_call(&mut self, request: Request) -> Response {
        Response {
            value: request.value,
        }
    }

    #[phenix(value("fixture.public-projection.status@1"), public)]
    fn status(&self) -> u64 {
        1
    }

    #[phenix(value("fixture.public-projection.internal@1"))]
    fn internal(&self) -> u64 {
        0
    }
}

#[phenix_sdk::plugin("fixture.public-projection")]
struct Plugin {
    #[phenix(component)]
    api: Api,
}

#[test]
fn public_projection_is_derived_from_ordinary_component_contributions() {
    let projection = Plugin::public_projection();

    assert_eq!(projection.callables.len(), 1);
    assert_eq!(
        projection.callables[0].component.as_str(),
        "fixture.public-projection.api"
    );
    assert_eq!(
        projection.callables[0].interface.as_str(),
        "fixture.public-projection.call@1"
    );
    assert_eq!(projection.callables[0].method, "public_call");

    assert_eq!(projection.values.len(), 1);
    assert_eq!(
        projection.values[0].component.as_str(),
        "fixture.public-projection.api"
    );
    assert_eq!(
        projection.values[0].id.as_str(),
        "fixture.public-projection.status@1"
    );
    assert_eq!(projection.values[0].method, "status");
    assert_eq!(
        projection.values[0].value_type,
        std::any::type_name::<u64>()
    );
    assert_eq!(projection.values[0].schema, u64::phenix_schema());

    let manifest = <Plugin as StaticPluginDefinition>::manifest();
    assert_eq!(manifest.id.as_str(), "fixture.public-projection");

    let mut plugin = Plugin { api: Api };
    let public = plugin.api.public_call(Request {
        value: "public".into(),
    });
    assert_eq!(public.value, "public");
    let private = plugin.api.private_call(Request {
        value: "private".into(),
    });
    assert_eq!(private.value, "private");
    assert_eq!(plugin.api.status(), 1);
    assert_eq!(plugin.api.internal(), 0);
}
