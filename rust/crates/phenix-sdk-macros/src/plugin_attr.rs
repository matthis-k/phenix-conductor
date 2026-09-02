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
        "#[phenix_sdk::plugin] applies to a plugin struct, stateless inline module, or lifecycle impl",
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
            "stateless plugin modules reserve the name Plugin for the generated zero-sized plugin type",
        ));
    }

    let identity: Item = parse_quote! {
        #[doc(hidden)]
        pub struct Plugin;
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
    let components_impl: Item = parse_quote! {
        impl ::phenix_sdk::StaticPluginComponents for Plugin {
            fn components() -> Vec<::phenix_sdk::StaticComponentDescriptor> {
                Vec::new()
            }
        }
    };
    items.extend([identity, identity_impl, definition_impl, components_impl]);

    Ok(quote!(#item))
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
    match item {
        Item::Struct(item) => item.ident == "Plugin",
        Item::Enum(item) => item.ident == "Plugin",
        Item::Union(item) => item.ident == "Plugin",
        Item::Type(item) => item.ident == "Plugin",
        _ => false,
    }
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
}

struct ComponentContribution {
    field: Ident,
    ty: Type,
    id: Option<LitStr>,
}

enum FieldRole {
    Dependency,
    Component { id: Option<LitStr> },
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
    if !role.is_ident("component") {
        return Err(syn::Error::new_spanned(
            role,
            "unsupported plugin field attribute",
        ));
    }

    let mut id = None;
    for argument in arguments {
        let Meta::NameValue(argument) = argument else {
            return Err(syn::Error::new_spanned(
                argument,
                "component metadata must use key = value syntax",
            ));
        };
        if !argument.path.is_ident("id") {
            return Err(syn::Error::new_spanned(
                argument.path,
                "unsupported component metadata",
            ));
        }
        if id.is_some() {
            return Err(syn::Error::new_spanned(argument, "duplicate component id"));
        }
        let value = string_literal(argument.value, "component id must be a string literal")?;
        validate_static_id(&value.value(), "component")
            .map_err(|error| syn::Error::new_spanned(&value, error))?;
        id = Some(value);
    }
    Ok(FieldRole::Component { id })
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
    fn stateless_module_lowers_to_zero_sized_plugin_definition() {
        let output = expand(
            quote!("phenix.stateless"),
            quote! {
                pub mod plugin {}
            },
        )
        .unwrap()
        .to_string();

        assert!(output.contains("pub struct Plugin"));
        assert!(output.contains("StaticPluginDefinition for Plugin"));
        assert!(output.contains("phenix.stateless"));
    }

    #[test]
    fn stateless_module_reserves_generated_plugin_type_name() {
        let error = expand(
            quote!("phenix.stateless"),
            quote! {
                mod plugin {
                    struct Plugin;
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
}
