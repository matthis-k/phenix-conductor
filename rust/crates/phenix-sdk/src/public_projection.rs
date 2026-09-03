use crate::{Authority, ComponentId, StaticPluginComponents, StaticPluginDefinition};
use phenix_core::{InterfaceId, InterfaceSchema, PhenixSchema};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticPublicCallable {
    pub component: ComponentId,
    pub interface: InterfaceId,
    pub path: Vec<String>,
    pub method: &'static str,
    pub schema: InterfaceSchema,
    pub required_authority: Authority,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticPublicValue {
    pub component: ComponentId,
    pub id: InterfaceId,
    pub method: &'static str,
    pub value_type: &'static str,
    pub schema: PhenixSchema,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct StaticPublicProjection {
    pub callables: Vec<StaticPublicCallable>,
    pub values: Vec<StaticPublicValue>,
}

pub trait StaticPluginPublicProjection: StaticPluginDefinition + StaticPluginComponents {
    fn public_projection() -> StaticPublicProjection {
        let mut projection = StaticPublicProjection::default();

        for component in Self::components() {
            projection.callables.extend(
                component
                    .exports()
                    .iter()
                    .filter(|export| export.public)
                    .map(|export| StaticPublicCallable {
                        component: component.id.clone(),
                        interface: export.interface.clone(),
                        path: callable_path(&export.interface, export.method),
                        method: export.method,
                        schema: export.schema.clone(),
                        required_authority: export.required_authority.clone(),
                    }),
            );
            projection
                .values
                .extend(
                    component
                        .values()
                        .iter()
                        .filter(|value| value.public)
                        .map(|value| StaticPublicValue {
                            component: component.id.clone(),
                            id: value.id.clone(),
                            method: value.method,
                            value_type: value.value_type,
                            schema: value.schema.clone(),
                        }),
                );
        }

        projection
    }
}

fn callable_path(interface: &InterfaceId, method: &str) -> Vec<String> {
    let Some((_, encoded)) = interface.as_str().rsplit_once("/public/") else {
        return vec![method.to_owned()];
    };
    let path = encoded
        .rsplit_once('@')
        .map_or(encoded, |(path, _version)| path);
    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if segments.is_empty() {
        vec![method.to_owned()]
    } else {
        segments
    }
}

impl<T> StaticPluginPublicProjection for T where T: StaticPluginDefinition + StaticPluginComponents {}
