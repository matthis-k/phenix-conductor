#[path = "plugin_attr_core.rs"]
mod core;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::Parser, punctuated::Punctuated, Expr, ExprLit, Fields, ItemStruct, Lit, Meta, Token,
    Type,
};

pub(crate) fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let stateful = syn::parse2::<ItemStruct>(input.clone()).ok();
    let expanded = core::expand(args, input)?;
    let Some(item) = stateful else {
        return Ok(expanded);
    };

    let name = &item.ident;
    let components = component_fields(&item)?;
    let dispatch_arms = components.iter().map(|component| {
        let field = &component.field;
        let ty = &component.ty;
        let id = match &component.id {
            Some(id) => quote! {
                ::phenix_sdk::StaticComponentDescriptor::explicit::<#ty>(
                    #id,
                    stringify!(#field),
                )
                .id
            },
            None => quote! {
                ::phenix_sdk::StaticComponentDescriptor::derived::<#ty>(
                    &Self::plugin_id(),
                    stringify!(#field),
                )
                .id
            },
        };

        quote! {
            if component == &#id {
                return ::phenix_sdk::StaticComponentDispatch::dispatch(
                    &mut self.#field,
                    service,
                    input,
                    host,
                );
            }
        }
    });

    Ok(quote! {
        #expanded

        impl ::phenix_sdk::StaticPluginComponentDispatch for #name {
            fn dispatch_component(
                &mut self,
                component: &::phenix_sdk::__phenix_plugin::ComponentId,
                service: &::phenix_sdk::__phenix_plugin::ServiceId,
                input: &[u8],
                host: &::phenix_sdk::__phenix_plugin::PluginHost<'_>,
            ) -> Result<Vec<u8>, String> {
                #(#dispatch_arms)*
                Err(format!("unsupported static plugin component: {component}"))
            }
        }
    })
}

struct ComponentField {
    field: syn::Ident,
    ty: Type,
    id: Option<syn::LitStr>,
}

fn component_fields(item: &ItemStruct) -> syn::Result<Vec<ComponentField>> {
    let Fields::Named(fields) = &item.fields else {
        return Ok(Vec::new());
    };

    let mut components = Vec::new();
    for field in &fields.named {
        for attribute in &field.attrs {
            if !attribute.path().is_ident("phenix") {
                continue;
            }
            let Meta::List(meta) = &attribute.meta else {
                continue;
            };
            let arguments =
                Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
            let Some(Meta::Path(role)) = arguments.first() else {
                continue;
            };
            if !role.is_ident("component") {
                continue;
            }

            let id = arguments.iter().skip(1).find_map(|argument| {
                let Meta::NameValue(value) = argument else {
                    return None;
                };
                if !value.path.is_ident("id") {
                    return None;
                }
                let Expr::Lit(ExprLit {
                    lit: Lit::Str(value),
                    ..
                }) = &value.value
                else {
                    return None;
                };
                Some(value.clone())
            });
            components.push(ComponentField {
                field: field.ident.clone().expect("named field has an identifier"),
                ty: field.ty.clone(),
                id,
            });
        }
    }
    Ok(components)
}
