use crate::{
    Authority, ComponentId, StaticComponentBehavior, StaticComponentExport,
    StaticComponentRuntimeDispatch, StaticPluginComponents, StaticPluginDefinition,
};
use phenix_core::{InterfaceId, InterfaceSchema, PhenixSchema, PluginHost, ServiceId};

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

/// Recursive public members declared by fields on one Rust value.
///
/// Implementations are generated from individually annotated fields. The
/// caller supplies the public owner so the same nested Rust type can be
/// mounted at different plugin paths without knowing those paths itself.
pub trait StaticExposeFields {
    fn exposed_field_exports_for(_owner: &str) -> Vec<StaticComponentExport> {
        Vec::new()
    }

    #[doc(hidden)]
    fn dispatch_exposed_field_for(
        &self,
        _owner: &str,
        _service: &ServiceId,
        _input: &[u8],
        _host: &PluginHost<'_>,
    ) -> Option<Result<Vec<u8>, String>> {
        None
    }
}

/// Relative public projection for a reusable Rust value.
///
/// Direct methods use a type-local internal owner. Parents remount the
/// resulting relative paths under their annotated field path.
pub trait StaticExpose:
    StaticExposeFields + StaticComponentBehavior + StaticComponentRuntimeDispatch + Sized
{
    fn exposed_exports() -> Vec<StaticComponentExport> {
        let owner = exposed_owner::<Self>();
        let mut exports = <Self as StaticComponentBehavior>::exports();
        exports.extend(<Self as StaticExposeFields>::exposed_field_exports_for(
            &owner,
        ));
        exports
    }

    #[doc(hidden)]
    fn dispatch_exposed(
        &self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        let owner = exposed_owner::<Self>();
        if let Some(result) = <Self as StaticExposeFields>::dispatch_exposed_field_for(
            self, &owner, service, input, host,
        ) {
            return result;
        }
        <Self as StaticComponentRuntimeDispatch>::dispatch_runtime(self, service, input, host)
    }
}

impl<T> StaticExpose for T where
    T: StaticExposeFields + StaticComponentBehavior + StaticComponentRuntimeDispatch + Sized
{
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticExposeRemap {
    from: &'static str,
    to: &'static str,
}

impl StaticExposeRemap {
    #[must_use]
    pub const fn new(from: &'static str, to: &'static str) -> Self {
        Self { from, to }
    }
}

#[doc(hidden)]
pub trait StaticRootExpose: StaticPluginDefinition + StaticExposeFields + Sized {
    const EXPOSED_REMAPS: &'static [StaticExposeRemap];

    fn root_exposed_interface(path: &str) -> InterfaceId {
        let owner = Self::descriptor().id;
        let path = remap_public_path(path, Self::EXPOSED_REMAPS);
        InterfaceId::parse(format!("{}/public/{path}@1", owner.as_str()))
            .expect("plugin exposure paths are validated by the plugin attribute")
    }

    fn root_exposed_field_exports() -> Vec<StaticComponentExport> {
        let owner = Self::descriptor().id;
        <Self as StaticExposeFields>::exposed_field_exports_for(owner.as_str())
            .into_iter()
            .map(|export| remap_root_exposed_export(&owner, Self::EXPOSED_REMAPS, export))
            .collect()
    }

    fn dispatch_root_exposed_field(
        &self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Option<Result<Vec<u8>, String>> {
        let owner = Self::descriptor().id;
        let service = <Self as StaticExposeFields>::exposed_field_exports_for(owner.as_str())
            .into_iter()
            .find_map(|export| {
                let original = ServiceId::parse(export.interface.as_str()).ok()?;
                let remapped = remap_root_exposed_export(&owner, Self::EXPOSED_REMAPS, export);
                (remapped.interface.as_str() == service.as_str()).then_some(original)
            })?;
        <Self as StaticExposeFields>::dispatch_exposed_field_for(
            self,
            owner.as_str(),
            &service,
            input,
            host,
        )
    }
}

#[doc(hidden)]
#[must_use]
pub fn exposed_owner<T>() -> String {
    let type_name = std::any::type_name::<T>();
    let mut owner = String::from("phenix.expose/");
    for character in type_name.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | ':' | '/') {
            owner.push(character);
        } else {
            owner.push('_');
        }
    }
    owner
}

#[doc(hidden)]
#[must_use]
pub fn exposed_interface<T>(path: &str) -> InterfaceId {
    InterfaceId::parse(format!("{}/public/{path}@1", exposed_owner::<T>()))
        .expect("exposed Rust member path derives a valid internal interface id")
}

#[doc(hidden)]
#[must_use]
pub fn remount_exposed_export(
    owner: &str,
    prefix: &str,
    mut export: StaticComponentExport,
) -> StaticComponentExport {
    let (relative, version) = public_interface_parts(&export.interface)
        .expect("StaticExpose exports use generated public interface identities");
    let path = join_public_path(prefix, relative);
    export.interface = InterfaceId::parse(format!("{owner}/public/{path}@{version}"))
        .expect("annotated public path derives a valid interface id");
    export
}

#[doc(hidden)]
#[must_use]
pub fn remap_exposed_service<T>(
    owner: &str,
    prefix: &str,
    service: &ServiceId,
) -> Option<ServiceId> {
    let public_prefix = format!("{owner}/public/{prefix}/");
    let rest = service.as_str().strip_prefix(&public_prefix)?;
    ServiceId::parse(format!("{}/public/{rest}", exposed_owner::<T>())).ok()
}

fn remap_root_exposed_export(
    owner: &crate::PluginId,
    remaps: &[StaticExposeRemap],
    mut export: StaticComponentExport,
) -> StaticComponentExport {
    let prefix = format!("{}/public/", owner.as_str());
    let Some(encoded) = export.interface.as_str().strip_prefix(&prefix) else {
        return export;
    };
    let Some((path, version)) = encoded.rsplit_once('@') else {
        return export;
    };
    let path = remap_public_path(path, remaps);
    export.interface = InterfaceId::parse(format!("{prefix}{path}@{version}"))
        .expect("plugin exposure paths are validated by the plugin attribute");
    export
}

fn remap_public_path(path: &str, remaps: &[StaticExposeRemap]) -> String {
    let selected = remaps
        .iter()
        .filter_map(|remap| {
            path_suffix(path, remap.from).map(|suffix| (remap.from, remap.to, suffix))
        })
        .max_by_key(|(prefix, _, _)| prefix.split('/').count());

    let Some((_, replacement, suffix)) = selected else {
        return path.to_owned();
    };
    join_public_path(replacement, suffix)
}

fn path_suffix<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    if path == prefix {
        return Some("");
    }
    path.strip_prefix(prefix)?.strip_prefix('/')
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
    let Some((path, _version)) = public_interface_parts(interface) else {
        return vec![method.to_owned()];
    };
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

fn public_interface_parts(interface: &InterfaceId) -> Option<(&str, &str)> {
    let (_, encoded) = interface.as_str().rsplit_once("/public/")?;
    encoded.rsplit_once('@')
}

fn join_public_path(prefix: &str, relative: &str) -> String {
    match (prefix.trim_matches('/'), relative.trim_matches('/')) {
        ("", relative) => relative.to_owned(),
        (prefix, "") => prefix.to_owned(),
        (prefix, relative) => format!("{prefix}/{relative}"),
    }
}

impl<T> StaticPluginPublicProjection for T where T: StaticPluginDefinition + StaticPluginComponents {}
