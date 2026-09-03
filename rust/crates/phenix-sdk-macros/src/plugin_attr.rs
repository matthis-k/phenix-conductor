#[path = "plugin_attr_core.rs"]
mod core;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::Parser, punctuated::Punctuated, Expr, ExprLit, Fields, ImplItem, ItemImpl, ItemStruct,
    Lit, Meta, Token, Type,
};

pub(crate) fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let stateful = syn::parse2::<ItemStruct>(input.clone()).ok();
    let lifecycle = syn::parse2::<ItemImpl>(input.clone()).ok();
    if let Some(lifecycle) = lifecycle.as_ref() {
        validate_lifecycle_runtime_abi(lifecycle)?;
    }

    let expanded = core::expand(args, input)?;
    if let Some(lifecycle) = lifecycle.as_ref() {
        return append_lifecycle_runtime(expanded, lifecycle);
    }
    let Some(item) = stateful else {
        return Ok(expanded);
    };

    let name = &item.ident;
    let components = component_fields(&item)?;
    let dispatch_arms = components.iter().map(|component| {
        let field = &component.field;
        let id = component_id(component);

        quote! {
            if component == &#id {
                return ::phenix_sdk::StaticComponentRuntimeDispatch::dispatch_runtime(
                    &mut self.#field,
                    service,
                    input,
                    host,
                );
            }
        }
    });
    let layer_arms = components.iter().map(|component| {
        let field = &component.field;
        quote! {
            if let Some(result) = ::phenix_sdk::StaticComponentRuntimeDispatch::dispatch_layer_runtime(
                &mut self.#field,
                service,
                input,
                host,
            ) {
                return result;
            }
        }
    });
    let listener_collectors = components.iter().map(|component| {
        let field = &component.field;
        let ty = &component.ty;
        let id = component_id(component);
        quote! {
            {
                let component = #id;
                for listener in <#ty as ::phenix_sdk::StaticComponentBehavior>::listeners() {
                    let state = ::std::sync::Weak::clone(&state);
                    let owner = owner.clone();
                    let graph_generation = graph_generation.clone();
                    let method = listener.method;
                    subscriptions.push(
                        ::phenix_sdk::StaticPluginInstance::<Self>::listener_subscription(
                            owner,
                            &component,
                            &listener,
                            maximum_authority.clone(),
                            move |envelope, authority| {
                                let Some(state) = state.upgrade() else {
                                    return Err(
                                        Box::new(::std::io::Error::other(format!(
                                            "stateful listener {method} plugin state is unavailable"
                                        )))
                                            as Box<dyn ::std::error::Error + Send + Sync>,
                                    );
                                };
                                let mut plugin = match state.try_lock() {
                                    Ok(plugin) => plugin,
                                    Err(::std::sync::TryLockError::WouldBlock) => {
                                        return Err(
                                            Box::new(::std::io::Error::other(format!(
                                                "stateful listener {method} skipped because plugin state is busy"
                                            )))
                                                as Box<dyn ::std::error::Error + Send + Sync>,
                                        );
                                    }
                                    Err(::std::sync::TryLockError::Poisoned(_)) => {
                                        return Err(
                                            Box::new(::std::io::Error::other(format!(
                                                "stateful listener {method} state lock poisoned"
                                            )))
                                                as Box<dyn ::std::error::Error + Send + Sync>,
                                        );
                                    }
                                };
                                let context = ::phenix_sdk::EventContext::from_event(
                                    authority,
                                    graph_generation.as_ref(),
                                );
                                ::phenix_sdk::StaticComponentRuntimeDispatch::dispatch_listener_runtime(
                                    &mut plugin.#field,
                                    method,
                                    &context,
                                    &envelope.payload,
                                )
                                .unwrap_or_else(|| {
                                    Err(
                                        Box::new(::std::io::Error::other(format!(
                                            "unsupported component listener: {method}"
                                        )))
                                            as Box<dyn ::std::error::Error + Send + Sync>,
                                    )
                                })
                            },
                        ),
                    );
                }
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

            fn dispatch_layer(
                &mut self,
                service: &::phenix_sdk::__phenix_plugin::ServiceId,
                input: &[u8],
                host: &::phenix_sdk::__phenix_plugin::PluginHost<'_>,
            ) -> Result<::phenix_sdk::LayerResult, String> {
                #(#layer_arms)*
                Err(format!("unsupported static plugin layer: {service}"))
            }

            fn listener_subscriptions(
                state: ::std::sync::Weak<::std::sync::Mutex<Self>>,
                host: &::phenix_sdk::__phenix_plugin::PluginHost<'_>,
            ) -> Vec<::phenix_sdk::__phenix_plugin::EventSubscription>
            where
                Self: Send + 'static,
            {
                let owner = host.plugin().clone();
                let maximum_authority = host.authority().clone();
                let graph_generation = host.graph_generation().cloned();
                let mut subscriptions = Vec::new();
                #(#listener_collectors)*
                subscriptions
            }
        }
    })
}

fn component_id(component: &ComponentField) -> TokenStream {
    let field = &component.field;
    let ty = &component.ty;
    match &component.id {
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
    }
}

fn append_lifecycle_runtime(expanded: TokenStream, item: &ItemImpl) -> syn::Result<TokenStream> {
    let (start, stop) = lifecycle_methods(item)?;
    let self_ty = &item.self_ty;

    let start_adapter = start.as_ref().map(|method| {
        quote! {
            #[doc(hidden)]
            fn __phenix_runtime_start(
                &mut self,
                host: &::phenix_sdk::__phenix_plugin::PluginHost<'_>,
            ) -> Result<(), String> {
                let context = ::phenix_sdk::PluginContext::new(host, (), (), ());
                self.#method(&context).map_err(|error| error.to_string())
            }
        }
    });
    let stop_adapter = stop.as_ref().map(|method| {
        quote! {
            #[doc(hidden)]
            fn __phenix_runtime_stop(
                &mut self,
                host: &::phenix_sdk::__phenix_plugin::PluginHost<'_>,
            ) -> Result<(), String> {
                let context = ::phenix_sdk::PluginContext::new(host, (), (), ());
                self.#method(&context).map_err(|error| error.to_string())
            }
        }
    });
    let with_start = start.as_ref().map(|_| {
        quote! {
            let instance = instance.with_start(Self::__phenix_runtime_start);
        }
    });
    let with_stop = stop.as_ref().map(|_| {
        quote! {
            let instance = instance.with_stop(Self::__phenix_runtime_stop);
        }
    });

    Ok(quote! {
        #expanded

        impl #self_ty {
            #start_adapter
            #stop_adapter

            #[doc(hidden)]
            pub fn __phenix_into_plugin_instance(
                self,
            ) -> Box<dyn ::phenix_sdk::__phenix_plugin::PluginInstance>
            where
                Self: ::phenix_sdk::StaticPluginComponentDispatch
                    + ::phenix_sdk::StaticPluginResources
                    + Send
                    + 'static,
            {
                let instance = ::phenix_sdk::StaticPluginInstance::from_component_dispatch(self);
                #with_start
                #with_stop
                Box::new(instance)
            }
        }
    })
}

fn validate_lifecycle_runtime_abi(item: &ItemImpl) -> syn::Result<()> {
    for member in &item.items {
        let ImplItem::Fn(method) = member else {
            continue;
        };
        if !has_lifecycle_role(method)? {
            continue;
        }
        if method.sig.unsafety.is_some()
            || method.sig.abi.is_some()
            || method.sig.variadic.is_some()
        {
            return Err(syn::Error::new_spanned(
                &method.sig,
                "plugin lifecycle methods must use the ordinary safe Rust ABI",
            ));
        }
    }
    Ok(())
}

fn has_lifecycle_role(method: &syn::ImplItemFn) -> syn::Result<bool> {
    for attribute in &method.attrs {
        if !attribute.path().is_ident("phenix") {
            continue;
        }
        let Meta::List(meta) = &attribute.meta else {
            continue;
        };
        let roles = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
        if roles.iter().any(|role| {
            matches!(role, Meta::Path(path) if path.is_ident("start") || path.is_ident("stop"))
        }) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn lifecycle_methods(item: &ItemImpl) -> syn::Result<(Option<syn::Ident>, Option<syn::Ident>)> {
    let mut start = None;
    let mut stop = None;
    for member in &item.items {
        let ImplItem::Fn(method) = member else {
            continue;
        };
        for attribute in &method.attrs {
            if !attribute.path().is_ident("phenix") {
                continue;
            }
            let Meta::List(meta) = &attribute.meta else {
                continue;
            };
            let roles =
                Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
            for role in roles {
                let Meta::Path(role) = role else {
                    continue;
                };
                if role.is_ident("start") {
                    start = Some(method.sig.ident.clone());
                } else if role.is_ident("stop") {
                    stop = Some(method.sig.ident.clone());
                }
            }
        }
    }
    Ok((start, stop))
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

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn lifecycle_impl_wires_runtime_callbacks_into_stateful_adapter() {
        let output = expand(
            TokenStream::new(),
            quote! {
                impl Plugin {
                    #[phenix(start)]
                    fn activate(
                        &mut self,
                        _context: &phenix_sdk::PluginContext<'_, '_, ()>,
                    ) -> Result<(), String> {
                        Ok(())
                    }

                    #[phenix(stop)]
                    fn deactivate(
                        &mut self,
                        _context: &phenix_sdk::PluginContext<'_, '_, ()>,
                    ) -> Result<(), String> {
                        Ok(())
                    }
                }
            },
        )
        .unwrap()
        .to_string();

        assert!(output.contains("__phenix_runtime_start"));
        assert!(output.contains("__phenix_runtime_stop"));
        assert!(output.contains("with_start"));
        assert!(output.contains("with_stop"));
        assert!(output.contains("__phenix_into_plugin_instance"));
    }

    #[test]
    fn lifecycle_impl_rejects_unsafe_runtime_abi() {
        let error = expand(
            TokenStream::new(),
            quote! {
                impl Plugin {
                    #[phenix(start)]
                    unsafe fn activate(
                        &mut self,
                        _context: &phenix_sdk::PluginContext<'_, '_, ()>,
                    ) -> Result<(), String> {
                        Ok(())
                    }
                }
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("ordinary safe Rust ABI"));
    }
}
