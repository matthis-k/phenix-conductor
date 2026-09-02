use crate::component_attr::{export_descriptor, parse_export, ExportContribution};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::Parser, parse_quote, punctuated::Punctuated, Attribute, Expr, ExprLit, Fields, Ident,
    Item, ItemMod, ItemStruct, Lit, LitStr, Meta, Token, Type,
};

pub(crate) fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(item) = syn::parse2::<ItemStruct>(input.clone()) {
        return expand_struct(args, item);
    }
    if let Ok(item) = syn::parse2::<ItemMod>(input.clone()) {
        return expand_module(args, item);
    }

    Err(syn::Error::new_spanned(
        input,
        "#[phenix_sdk::plugin] applies to a plugin struct or stateless inline module",
    ))
}

fn expand_struct(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "Phenix plugins must have one concrete runtime identity",
        ));
    }

    let contributions = field_contributions(&mut item)?;
    let dependencies = contributions.dependencies;
    let component_descriptors = contributions.components.iter().map(|component| {
        let field = &component.field;
        let ty = &component.ty;
        if let Some(id) = &component.id {
            quote! {
                ::phenix_sdk::StaticComponentDescriptor::explicit::<#ty>(
                    #id,
                    stringify!(#field),
                )
            }
        } else {
            quote! {
                ::phenix_sdk::StaticComponentDescriptor::derived::<#ty>(
                    &Self::plugin_id(),
                    stringify!(#field),
                )
            }
        }
    });
    let resource_descriptors = contributions.resources.iter().map(|resource| {
        let field = &resource.field;
        let ty = &resource.ty;
        let features = &resource.features;
        if let Some(id) = &resource.id {
            quote! {
                ::phenix_sdk::StaticResourceDescriptor::explicit::<#ty>(
                    #id,
                    stringify!(#field),
                    [#(::phenix_sdk::BackendFeature::#features),*],
                )
            }
        } else {
            quote! {
                ::phenix_sdk::StaticResourceDescriptor::derived::<#ty>(
                    &Self::plugin_id(),
                    stringify!(#field),
                    [#(::phenix_sdk::BackendFeature::#features),*],
                )
            }
        }
    });
    let id = resolve_plugin_id(args, &item.ident)?;
    let name = &item.ident;
    let identity_impl = plugin_identity_impl(name, &id);

    Ok(quote! {
        #item

        #identity_impl

        impl ::phenix_sdk::StaticPluginDefinition for #name {
            fn descriptor() -> ::phenix_sdk::StaticPluginDescriptor {
                ::phenix_sdk::StaticPluginDescriptor {
                    id: Self::plugin_id(),
                    definition: concat!(module_path!(), "::", stringify!(#name)),
                    dependencies: vec![
                        #(::phenix_sdk::StaticPluginDependency::of::<#dependencies>()),*
                    ],
                }
            }
        }

        impl ::phenix_sdk::StaticPluginComponents for #name {
            fn components() -> Vec<::phenix_sdk::StaticComponentDescriptor> {
                vec![#(#component_descriptors),*]
            }
        }

        impl ::phenix_sdk::StaticPluginResources for #name {
            fn resources() -> Vec<::phenix_sdk::StaticResourceDescriptor> {
                vec![#(#resource_descriptors),*]
            }
        }
    })
}

fn expand_module(args: TokenStream, mut item: ItemMod) -> syn::Result<TokenStream> {
    let id = resolve_plugin_id(args, &item.ident)?;
    let Some((_, items)) = item.content.as_mut() else {
        return Err(syn::Error::new_spanned(
            &item,
            "stateless Phenix plugins must use an inline module so authoring contributions stay visible to the macro",
        ));
    };
    if items.iter().any(defines_generated_plugin_type) {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "stateless plugin modules reserve the names Plugin and Component for generated zero-sized types",
        ));
    }

    let exports = module_exports(items)?;
    let descriptors = exports
        .iter()
        .map(|(function, export)| export_descriptor(function, export))
        .collect::<Vec<_>>();

    let identity: Item = parse_quote! {
        #[doc(hidden)]
        pub struct Plugin;
    };
    let component: Item = parse_quote! {
        #[doc(hidden)]
        pub struct Component;
    };
    let identity_impl: Item = parse_quote! {
        impl Plugin {
            #[must_use]
            pub fn plugin_id() -> ::phenix_sdk::PluginId {
                ::phenix_sdk::PluginId::parse(#id)
                    .expect("plugin attribute validated the static plugin id")
            }

            #[must_use]
            pub fn component_id() -> ::phenix_sdk::ComponentId {
                ::phenix_sdk::ComponentId::parse(#id)
                    .expect("plugin attribute validated the default component id")
            }
        }
    };
    let definition_impl: Item = parse_quote! {
        impl ::phenix_sdk::StaticPluginDefinition for Plugin {
            fn descriptor() -> ::phenix_sdk::StaticPluginDescriptor {
                ::phenix_sdk::StaticPluginDescriptor {
                    id: Self::plugin_id(),
                    definition: concat!(module_path!(), "::Plugin"),
                    dependencies: Vec::new(),
                }
            }
        }
    };
    let component_definition: Item = parse_quote! {
        impl ::phenix_sdk::StaticComponentDefinition for Component {}
    };
    let component_behavior: Item = syn::parse2(quote! {
        impl ::phenix_sdk::StaticComponentBehavior for Component {
            fn exports() -> Vec<::phenix_sdk::StaticComponentExport> {
                vec![#(#descriptors),*]
            }
        }
    })?;
    let components_impl: Item = parse_quote! {
        impl ::phenix_sdk::StaticPluginComponents for Plugin {
            fn components() -> Vec<::phenix_sdk::StaticComponentDescriptor> {
                vec![::phenix_sdk::StaticComponentDescriptor::explicit::<Component>(
                    #id,
                    "default",
                )]
            }
        }
    };
    items.extend([
        identity,
        component,
        identity_impl,
        definition_impl,
        component_definition,
        component_behavior,
        components_impl,
    ]);

    Ok(quote!(#item))
}

fn module_exports(items: &mut [Item]) -> syn::Result<Vec<(Ident, ExportContribution)>> {
    let mut exports = Vec::new();
    for item in items {
        let Item::Fn(function) = item else {
            continue;
        };
        let mut retained = Vec::new();
        let mut export = None;
        for attribute in std::mem::take(&mut function.attrs) {
            if !attribute.path().is_ident("phenix") {
                retained.push(attribute);
                continue;
            }
            if export.is_some() {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "a stateless plugin function may declare only one Phenix contribution",
                ));
            }
            export = Some(parse_export(&attribute)?);
        }
        function.attrs = retained;
        if let Some(export) = export {
            exports.push((function.sig.ident.clone(), export));
        }
    }
    Ok(exports)
}

fn plugin_identity_impl(name: &Ident, id: &LitStr) -> TokenStream {
    quote! {
        impl #name {
            #[must_use]
            pub fn plugin_id() -> ::phenix_sdk::PluginId {
                ::phenix_sdk::PluginId::parse(#id)
                    .expect("plugin attribute validated the static plugin id")
            }

            #[must_use]
            pub fn component_id() -> ::phenix_sdk::ComponentId {
                ::phenix_sdk::ComponentId::parse(#id)
                    .expect("plugin attribute validated the default component id")
            }
        }
    }
}

fn defines_generated_plugin_type(item: &Item) -> bool {
    let ident = match item {
        Item::Struct(item) => Some(&item.ident),
        Item::Enum(item) => Some(&item.ident),
        Item::Union(item) => Some(&item.ident),
        Item::Type(item) => Some(&item.ident),
        _ => None,
    };
    ident.is_some_and(|ident| ident == "Plugin" || ident == "Component")
}

fn resolve_plugin_id(args: TokenStream, item: &Ident) -> syn::Result<LitStr> {
    if let Some(id) = explicit_plugin_id(args)? {
        return Ok(id);
    }

    let package = std::env::var("CARGO_PKG_NAME")
        .map_err(|_| syn::Error::new_spanned(item, "plugin package identity is unavailable"))?;
    let id = default_plugin_id(&package).ok_or_else(|| {
        syn::Error::new_spanned(
            item,
            "plugin without an explicit id requires a phenix-plugin-* package name",
        )
    })?;
    Ok(LitStr::new(&id, item.span()))
}

fn explicit_plugin_id(args: TokenStream) -> syn::Result<Option<LitStr>> {
    if args.is_empty() {
        return Ok(None);
    }

    if let Ok(value) = syn::parse2::<LitStr>(args.clone()) {
        validate_static_id(&value.value(), "plugin")
            .map_err(|error| syn::Error::new_spanned(&value, error))?;
        return Ok(Some(value));
    }

    let args = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;
    let mut id = None;
    for argument in args {
        let Meta::NameValue(argument) = argument else {
            return Err(syn::Error::new_spanned(
                argument,
                "plugin attributes must use a string id or key = value syntax",
            ));
        };
        if !argument.path.is_ident("id") {
            return Err(syn::Error::new_spanned(
                argument.path,
                "unsupported plugin attribute",
            ));
        }
        if id.is_some() {
            return Err(syn::Error::new_spanned(argument, "duplicate plugin id"));
        }
        let value = string_literal(argument.value, "plugin id must be a string literal")?;
        validate_static_id(&value.value(), "plugin")
            .map_err(|error| syn::Error::new_spanned(&value, error))?;
        id = Some(value);
    }
    Ok(id)
}

#[derive(Default)]
struct FieldContributions {
    dependencies: Vec<Type>,
    components: Vec<ComponentContribution>,
    resources: Vec<ResourceContribution>,
}

struct ComponentContribution {
    field: Ident,
    ty: Type,
    id: Option<LitStr>,
}

struct ResourceContribution {
    field: Ident,
    ty: Type,
    id: Option<LitStr>,
    features: Vec<Ident>,
}

enum FieldRole {
    Dependency,
    Component {
        id: Option<LitStr>,
    },
    Resource {
        id: Option<LitStr>,
        features: Vec<Ident>,
    },
}

fn field_contributions(item: &mut ItemStruct) -> syn::Result<FieldContributions> {
    let Fields::Named(fields) = &mut item.fields else {
        return Ok(FieldContributions::default());
    };
    let mut contributions = FieldContributions::default();

    for field in &mut fields.named {
        let mut retained = Vec::new();
        let mut role = None;
        for attribute in std::mem::take(&mut field.attrs) {
            if !attribute.path().is_ident("phenix") {
                retained.push(attribute);
                continue;
            }
            if role.is_some() {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "a plugin field may have only one Phenix role",
                ));
            }
            role = Some(field_role(&attribute)?);
        }
        field.attrs = retained;

        match role {
            Some(FieldRole::Dependency) => contributions.dependencies.push(field.ty.clone()),
            Some(FieldRole::Component { id }) => {
                let name = field.ident.clone().expect("named field has an identifier");
                contributions.components.push(ComponentContribution {
                    field: name,
                    ty: field.ty.clone(),
                    id,
                });
            }
            Some(FieldRole::Resource { id, features }) => {
                let name = field.ident.clone().expect("named field has an identifier");
                contributions.resources.push(ResourceContribution {
                    field: name,
                    ty: field.ty.clone(),
                    id,
                    features,
                });
            }
            None => {}
        }
    }

    Ok(contributions)
}

fn field_role(attribute: &Attribute) -> syn::Result<FieldRole> {
    let Meta::List(meta) = &attribute.meta else {
        return Err(syn::Error::new_spanned(
            attribute,
            "plugin field attributes must use #[phenix(...)] syntax",
        ));
    };
    let arguments = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
    let mut arguments = arguments.into_iter();
    let Some(Meta::Path(role)) = arguments.next() else {
        return Err(syn::Error::new_spanned(
            attribute,
            "plugin field attribute must begin with a field role",
        ));
    };

    if role.is_ident("dep") {
        if let Some(argument) = arguments.next() {
            return Err(syn::Error::new_spanned(
                argument,
                "dependency fields do not accept metadata",
            ));
        }
        return Ok(FieldRole::Dependency);
    }
    let kind = if role.is_ident("component") {
        "component"
    } else if role.is_ident("resource") {
        "resource"
    } else {
        return Err(syn::Error::new_spanned(
            role,
            "unsupported plugin field attribute",
        ));
    };

    let mut id = None;
    let mut features = Vec::new();
    for argument in arguments {
        if kind == "resource" {
            if let Meta::List(feature_list) = &argument {
                if feature_list.path.is_ident("features") {
                    if !features.is_empty() {
                        return Err(syn::Error::new_spanned(
                            argument,
                            "duplicate resource features",
                        ));
                    }
                    let values = Punctuated::<Ident, Token![,]>::parse_terminated
                        .parse2(feature_list.tokens.clone())?;
                    for feature in values {
                        if !matches!(
                            feature.to_string().as_str(),
                            "Transactions"
                                | "UniqueKeys"
                                | "ForeignKeys"
                                | "OrderedAppend"
                                | "IndexedRange"
                                | "Migrations"
                        ) {
                            return Err(syn::Error::new_spanned(
                                feature,
                                "unsupported resource backend feature",
                            ));
                        }
                        features.push(feature);
                    }
                    continue;
                }
            }
        }

        let Meta::NameValue(argument) = argument else {
            return Err(syn::Error::new_spanned(
                argument,
                format!("{kind} metadata must use key = value syntax"),
            ));
        };
        if !argument.path.is_ident("id") {
            return Err(syn::Error::new_spanned(
                argument.path,
                format!("unsupported {kind} metadata"),
            ));
        }
        if id.is_some() {
            return Err(syn::Error::new_spanned(
                argument,
                format!("duplicate {kind} id"),
            ));
        }
        let value = string_literal(argument.value, "field id must be a string literal")?;
        validate_static_id(&value.value(), kind)
            .map_err(|error| syn::Error::new_spanned(&value, error))?;
        id = Some(value);
    }
    Ok(if kind == "component" {
        FieldRole::Component { id }
    } else {
        FieldRole::Resource { id, features }
    })
}

fn string_literal(value: Expr, message: &'static str) -> syn::Result<LitStr> {
    let Expr::Lit(ExprLit {
        lit: Lit::Str(value),
        ..
    }) = value
    else {
        return Err(syn::Error::new_spanned(value, message));
    };
    Ok(value)
}

fn validate_static_id(value: &str, kind: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{kind} id must not be empty"));
    }
    if value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/' | b':')
    }) {
        Ok(())
    } else {
        Err(format!("{kind} id contains unsupported characters"))
    }
}

fn default_plugin_id(package: &str) -> Option<String> {
    package
        .strip_prefix("phenix-plugin-")
        .filter(|name| !name.is_empty())
        .map(|name| format!("phenix.{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn package_name_derives_stable_plugin_id() {
        assert_eq!(
            default_plugin_id("phenix-plugin-session-tree").as_deref(),
            Some("phenix.session-tree")
        );
        assert_eq!(default_plugin_id("phenix-sdk"), None);
    }

    #[test]
    fn canonical_positional_plugin_id_is_accepted() {
        let id = explicit_plugin_id(quote!("phenix.example"))
            .unwrap()
            .unwrap();
        assert_eq!(id.value(), "phenix.example");
    }

    #[test]
    fn keyed_plugin_id_remains_supported() {
        let id = explicit_plugin_id(quote!(id = "phenix.example"))
            .unwrap()
            .unwrap();
        assert_eq!(id.value(), "phenix.example");
    }

    #[test]
    fn stateless_module_lowers_default_component_and_exports() {
        let output = expand(
            quote!("phenix.stateless"),
            quote! {
                pub mod plugin {
                    #[phenix(export("phenix.stateless.run@1"), public)]
                    pub fn run() {}
                }
            },
        )
        .unwrap()
        .to_string();

        assert!(output.contains("pub struct Plugin"));
        assert!(output.contains("pub struct Component"));
        assert!(output.contains("StaticComponentBehavior for Component"));
        assert!(output.contains("phenix.stateless.run@1"));
        assert!(!output.contains("phenix (export"));
    }

    #[test]
    fn stateless_module_reserves_generated_type_names() {
        let error = expand(
            quote!("phenix.stateless"),
            quote! {
                mod plugin {
                    struct Component;
                }
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("reserve"));
    }

    #[test]
    fn component_field_is_lowered_without_leaking_helper_attribute() {
        let mut item: ItemStruct = parse_quote! {
            struct Plugin {
                #[phenix(component)]
                api: Api,
            }
        };

        let contributions = field_contributions(&mut item).unwrap();
        assert_eq!(contributions.components.len(), 1);
        assert_eq!(contributions.components[0].field, "api");
        assert!(contributions.components[0].id.is_none());
        assert!(item.fields.iter().all(|field| field.attrs.is_empty()));
    }

    #[test]
    fn component_field_preserves_explicit_stable_id() {
        let mut item: ItemStruct = parse_quote! {
            struct Plugin {
                #[phenix(component, id = "legacy.component")]
                api: Api,
            }
        };

        let contributions = field_contributions(&mut item).unwrap();
        assert_eq!(
            contributions.components[0].id.as_ref().unwrap().value(),
            "legacy.component"
        );
    }

    #[test]
    fn resource_field_is_lowered_without_leaking_helper_attribute() {
        let mut item: ItemStruct = parse_quote! {
            struct Plugin {
                #[phenix(resource)]
                state: phenix_sdk::Durable<State>,
            }
        };

        let contributions = field_contributions(&mut item).unwrap();
        assert_eq!(contributions.resources.len(), 1);
        assert_eq!(contributions.resources[0].field, "state");
        assert!(contributions.resources[0].id.is_none());
        assert!(contributions.resources[0].features.is_empty());
        assert!(item.fields.iter().all(|field| field.attrs.is_empty()));
    }

    #[test]
    fn resource_field_preserves_required_backend_features() {
        let mut item: ItemStruct = parse_quote! {
            struct Plugin {
                #[phenix(resource, features(Transactions, Migrations))]
                state: phenix_sdk::Durable<State>,
            }
        };

        let contributions = field_contributions(&mut item).unwrap();
        let features = contributions.resources[0]
            .features
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert_eq!(features, ["Transactions", "Migrations"]);
    }

    #[test]
    fn resource_field_rejects_unknown_backend_feature() {
        let mut item: ItemStruct = parse_quote! {
            struct Plugin {
                #[phenix(resource, features(Transactions, Telepathy))]
                state: phenix_sdk::Durable<State>,
            }
        };

        let error = field_contributions(&mut item).unwrap_err();
        assert!(error
            .to_string()
            .contains("unsupported resource backend feature"));
    }

    #[test]
    fn plugin_expansion_emits_resource_descriptor_from_resource_field() {
        let output = expand(
            quote!("phenix.resource-owner"),
            quote! {
                struct Plugin {
                    #[phenix(resource, id = "legacy.state")]
                    state: phenix_sdk::Durable<State>,
                }
            },
        )
        .unwrap()
        .to_string();

        assert!(output.contains("StaticPluginResources for Plugin"));
        assert!(output.contains("StaticResourceDescriptor :: explicit"));
        assert!(output.contains("legacy.state"));
        assert!(!output.contains("phenix (resource"));
    }
}
