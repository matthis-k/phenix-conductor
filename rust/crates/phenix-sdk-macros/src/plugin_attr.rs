#[path = "plugin_attr_core.rs"]
mod core;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::Parser, parse_quote, punctuated::Punctuated, Attribute, Expr, ExprLit, Fields, Ident,
    ImplItem, Item, ItemImpl, ItemStruct, Lit, LitStr, Meta, Token, Type,
};

const ROOT_FIELD: &str = "__phenix_root";

pub(crate) fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(item) = syn::parse2::<ItemImpl>(input.clone()) {
        validate_lifecycle_runtime_abi(&item)?;
        if has_root_behavior(&item)? {
            return expand_root_impl(args, item);
        }
        let expanded = core::expand(args, input)?;
        return append_lifecycle_runtime(expanded, &item);
    }

    let Ok(item) = syn::parse2::<ItemStruct>(input.clone()) else {
        return core::expand(args, input);
    };

    let (args, root) = split_root_argument(args)?;
    expand_struct(args, item, root)
}

fn expand_struct(
    args: TokenStream,
    item: ItemStruct,
    root_requested: bool,
) -> syn::Result<TokenStream> {
    let root = should_inject_root(&args, &item, root_requested)?;
    let name = &item.ident;
    let root_ident = root_component_ident(name);

    let expanded = if root {
        let id = plugin_id_literal(args.clone(), name)?;
        let mut core_item = item.clone();
        let Fields::Named(fields) = &mut core_item.fields else {
            unreachable!("root injection is restricted to named plugin structs")
        };
        let root_ty: Type = parse_quote!(#root_ident);
        let field: syn::Field = parse_quote! {
            #[phenix(component, id = #id)]
            __phenix_root: #root_ty
        };
        fields.named.push(field);
        let expanded = core::expand(args.clone(), quote!(#core_item))?;
        strip_synthetic_root(expanded, name)?
    } else {
        core::expand(args.clone(), quote!(#item))?
    };

    let components = component_fields(&item)?;
    let root_dispatch = root.then(|| {
        quote! {
            if component == &Self::component_id() {
                return ::phenix_sdk::StaticComponentRuntimeDispatch::dispatch_runtime(
                    self,
                    service,
                    input,
                    host,
                );
            }
        }
    });
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
    let root_layer = root.then(|| {
        quote! {
            if let Some(result) = ::phenix_sdk::StaticComponentRuntimeDispatch::dispatch_layer_runtime(
                self,
                service,
                input,
                host,
            ) {
                return result;
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
    let root_listener_collector = root.then(|| {
        quote! {
            {
                let component = Self::component_id();
                for listener in <#root_ident as ::phenix_sdk::StaticComponentBehavior>::listeners() {
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
                                    &mut *plugin,
                                    method,
                                    &context,
                                    &envelope.payload,
                                )
                                .unwrap_or_else(|| {
                                    Err(
                                        Box::new(::std::io::Error::other(format!(
                                            "unsupported plugin listener: {method}"
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
                #root_dispatch
                #(#dispatch_arms)*
                Err(format!("unsupported static plugin component: {component}"))
            }

            fn dispatch_layer(
                &mut self,
                service: &::phenix_sdk::__phenix_plugin::ServiceId,
                input: &[u8],
                host: &::phenix_sdk::__phenix_plugin::PluginHost<'_>,
            ) -> Result<::phenix_sdk::LayerResult, String> {
                #root_layer
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
                #root_listener_collector
                #(#listener_collectors)*
                subscriptions
            }
        }
    })
}

fn expand_root_impl(args: TokenStream, item: ItemImpl) -> syn::Result<TokenStream> {
    if !args.is_empty() {
        return Err(syn::Error::new_spanned(
            args,
            "plugin behavior impls inherit plugin identity and do not accept root arguments",
        ));
    }
    if item.trait_.is_some() || !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item,
            "plugin behavior must be a non-generic inherent impl",
        ));
    }

    let self_ident = self_type_ident(&item)?;
    let root_ident = root_component_ident(&self_ident);
    let lifecycle = lifecycle_trait_impl(&item)?;
    let (behavior, markers) = normalize_root_behavior(item.clone())?;
    let expanded = crate::component_runtime_attr::expand(TokenStream::new(), quote!(#behavior))?;
    let expanded = retarget_behavior(expanded, &root_ident)?;
    let expanded = quote! {
        #expanded
        #lifecycle
        #(#markers)*

        #[doc(hidden)]
        struct #root_ident;

        impl ::phenix_sdk::StaticComponentDefinition for #root_ident {}
        impl ::phenix_sdk::StaticComponentImports for #root_ident {}
    };
    append_lifecycle_runtime(expanded, &item)
}

fn lifecycle_trait_impl(item: &ItemImpl) -> syn::Result<TokenStream> {
    let mut has_lifecycle = false;
    let mut lifecycle = item.clone();
    for member in &mut lifecycle.items {
        let ImplItem::Fn(method) = member else {
            continue;
        };
        let mut retained = Vec::new();
        for attribute in std::mem::take(&mut method.attrs) {
            if !attribute.path().is_ident("phenix") {
                retained.push(attribute);
                continue;
            }
            if lifecycle_role(&attribute)?.is_some() {
                has_lifecycle = true;
                retained.push(attribute);
            }
        }
        method.attrs = retained;
    }
    if !has_lifecycle {
        return Ok(TokenStream::new());
    }
    let expanded = core::expand(TokenStream::new(), quote!(#lifecycle))?;
    let file = syn::parse2::<syn::File>(expanded)?;
    file.items
        .into_iter()
        .find_map(|item| match item {
            Item::Impl(item) if trait_is(&item, "StaticPluginLifecycle") => Some(quote!(#item)),
            _ => None,
        })
        .ok_or_else(|| syn::Error::new_spanned(item, "plugin lifecycle metadata was not generated"))
}

fn normalize_root_behavior(mut item: ItemImpl) -> syn::Result<(ItemImpl, Vec<TokenStream>)> {
    let self_ty = (*item.self_ty).clone();
    let self_ident = self_type_ident(&item)?;
    let mut markers = Vec::new();

    for member in &mut item.items {
        let ImplItem::Fn(method) = member else {
            continue;
        };
        let phenix = method
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident("phenix"))
            .count();
        if phenix > 1 {
            return Err(syn::Error::new_spanned(
                &method.sig.ident,
                "a plugin method may declare only one Phenix role",
            ));
        }

        let mut retained = Vec::new();
        for mut attribute in std::mem::take(&mut method.attrs) {
            if !attribute.path().is_ident("phenix") {
                retained.push(attribute);
                continue;
            }
            if lifecycle_role(&attribute)?.is_some() {
                continue;
            }
            if let Some(expose) = expose_declaration(&attribute, &method.sig.ident)? {
                let marker = expose_marker_ident(&self_ident, &method.sig.ident);
                let public_name = expose.name;
                let authority = expose.authority;
                let export: Attribute = match authority {
                    Some(authority) => parse_quote! {
                        #[phenix(export(#marker), public, authority = #authority)]
                    },
                    None => parse_quote! {
                        #[phenix(export(#marker), public)]
                    },
                };
                retained.push(export);
                markers.push(quote! {
                    #[doc(hidden)]
                    struct #marker;

                    impl ::phenix_sdk::InterfaceMarker for #marker {
                        fn interface_id() -> ::phenix_sdk::__phenix_plugin::InterfaceId {
                            ::phenix_sdk::__phenix_plugin::InterfaceId::parse(format!(
                                "{}/public/{}@1",
                                <#self_ty>::plugin_id().as_str(),
                                #public_name,
                            ))
                            .expect("exposed plugin method derives a valid private interface id")
                        }
                    }
                });
                continue;
            }
            normalize_on_event(&mut attribute)?;
            retained.push(attribute);
        }
        method.attrs = retained;
    }

    Ok((item, markers))
}

struct ExposeDeclaration {
    name: LitStr,
    authority: Option<Expr>,
}

fn expose_declaration(
    attribute: &Attribute,
    method: &Ident,
) -> syn::Result<Option<ExposeDeclaration>> {
    let Meta::List(meta) = &attribute.meta else {
        return Ok(None);
    };
    let outer = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
    let Some(first) = outer.first() else {
        return Ok(None);
    };

    let (is_expose, modifiers) = match first {
        Meta::Path(path) if path.is_ident("expose") => {
            (true, outer.iter().skip(1).cloned().collect::<Vec<_>>())
        }
        Meta::List(list) if list.path.is_ident("expose") => {
            if outer.len() != 1 {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "expose metadata belongs inside expose(...) when that form is used",
                ));
            }
            let inner = Punctuated::<Meta, Token![,]>::parse_terminated
                .parse2(list.tokens.clone())?
                .into_iter()
                .collect::<Vec<_>>();
            (true, inner)
        }
        _ => (false, Vec::new()),
    };
    if !is_expose {
        return Ok(None);
    }

    let mut name = None;
    let mut authority = None;
    for modifier in modifiers {
        let Meta::NameValue(value) = modifier else {
            return Err(syn::Error::new_spanned(
                modifier,
                "expose metadata must use name = \"...\" or authority = <expression>",
            ));
        };
        if value.path.is_ident("name") {
            if name.is_some() {
                return Err(syn::Error::new_spanned(value, "duplicate expose name"));
            }
            let name_value = match value.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(value),
                    ..
                }) => value,
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "expose name must be a string literal",
                    ));
                }
            };
            validate_public_segment(&name_value)?;
            name = Some(name_value);
        } else if value.path.is_ident("authority") {
            if authority.is_some() {
                return Err(syn::Error::new_spanned(value, "duplicate expose authority"));
            }
            authority = Some(value.value);
        } else {
            return Err(syn::Error::new_spanned(
                value.path,
                "unsupported expose metadata",
            ));
        }
    }

    let name = name.unwrap_or_else(|| LitStr::new(&method.to_string(), method.span()));
    validate_public_segment(&name)?;
    Ok(Some(ExposeDeclaration { name, authority }))
}

fn validate_public_segment(value: &LitStr) -> syn::Result<()> {
    let name = value.value();
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(syn::Error::new_spanned(
            value,
            "public member name must be one non-empty alphanumeric, '_' or '-' path segment",
        ));
    }
    Ok(())
}

fn normalize_on_event(attribute: &mut Attribute) -> syn::Result<()> {
    let Meta::List(meta) = &attribute.meta else {
        return Ok(());
    };
    let mut outer = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
    let Some(first) = outer.first_mut() else {
        return Ok(());
    };
    let Meta::List(kind) = first else {
        return Ok(());
    };
    if !kind.path.is_ident("on_event") {
        return Ok(());
    }
    kind.path = parse_quote!(listen);
    attribute.meta = syn::parse2(quote!(phenix(#outer)))?;
    Ok(())
}

fn retarget_behavior(expanded: TokenStream, root: &Ident) -> syn::Result<TokenStream> {
    let mut file = syn::parse2::<syn::File>(expanded)?;
    let target: Type = parse_quote!(#root);
    let mut found = false;
    for item in &mut file.items {
        let Item::Impl(item) = item else {
            continue;
        };
        if trait_is(item, "StaticComponentBehavior") {
            item.self_ty = Box::new(target.clone());
            found = true;
        }
    }
    if !found {
        return Err(syn::Error::new_spanned(
            root,
            "plugin root behavior metadata was not generated",
        ));
    }
    Ok(quote!(#file))
}

fn trait_is(item: &ItemImpl, expected: &str) -> bool {
    item.trait_
        .as_ref()
        .and_then(|(_, path, _)| path.segments.last())
        .is_some_and(|segment| segment.ident == expected)
}

fn has_root_behavior(item: &ItemImpl) -> syn::Result<bool> {
    for member in &item.items {
        let ImplItem::Fn(method) = member else {
            continue;
        };
        for attribute in &method.attrs {
            if !attribute.path().is_ident("phenix") {
                continue;
            }
            if lifecycle_role(attribute)?.is_none() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn lifecycle_role(attribute: &Attribute) -> syn::Result<Option<&'static str>> {
    if !attribute.path().is_ident("phenix") {
        return Ok(None);
    }
    let Meta::List(meta) = &attribute.meta else {
        return Ok(None);
    };
    let roles = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
    if roles.len() != 1 {
        return Ok(None);
    }
    let Some(Meta::Path(role)) = roles.first() else {
        return Ok(None);
    };
    if role.is_ident("start") {
        Ok(Some("start"))
    } else if role.is_ident("stop") {
        Ok(Some("stop"))
    } else {
        Ok(None)
    }
}

fn self_type_ident(item: &ItemImpl) -> syn::Result<Ident> {
    let Type::Path(path) = item.self_ty.as_ref() else {
        return Err(syn::Error::new_spanned(
            &item.self_ty,
            "plugin behavior requires a concrete path self type",
        ));
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.clone())
        .ok_or_else(|| syn::Error::new_spanned(&item.self_ty, "plugin self type is empty"))
}

fn root_component_ident(plugin: &Ident) -> Ident {
    format_ident!("__PhenixPluginRootComponentFor{}", plugin)
}

fn expose_marker_ident(plugin: &Ident, method: &Ident) -> Ident {
    format_ident!("__PhenixExposed{}_{}", plugin, method)
}

fn should_inject_root(
    args: &TokenStream,
    item: &ItemStruct,
    root_requested: bool,
) -> syn::Result<bool> {
    if !root_requested {
        return Ok(false);
    }
    if !matches!(item.fields, Fields::Named(_)) {
        return Err(syn::Error::new_spanned(
            item,
            "plugin root behavior requires named struct fields",
        ));
    }
    if !component_fields(item)?.is_empty() || has_root_fields(item)? {
        return Err(syn::Error::new_spanned(
            item,
            "plugin root behavior cannot be combined with explicit or structural root components",
        ));
    }
    if !embedded_execution(args.clone())? {
        return Err(syn::Error::new_spanned(
            item,
            "plugin root behavior requires embedded execution",
        ));
    }
    Ok(true)
}

fn split_root_argument(args: TokenStream) -> syn::Result<(TokenStream, bool)> {
    if args.is_empty() || syn::parse2::<LitStr>(args.clone()).is_ok() {
        return Ok((args, false));
    }

    let arguments = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;
    let mut root = false;
    let mut retained = Punctuated::<Meta, Token![,]>::new();
    for argument in arguments {
        if matches!(&argument, Meta::Path(path) if path.is_ident("root")) {
            if root {
                return Err(syn::Error::new_spanned(argument, "duplicate plugin root flag"));
            }
            root = true;
            continue;
        }
        retained.push(argument);
    }
    Ok((quote!(#retained), root))
}

fn embedded_execution(args: TokenStream) -> syn::Result<bool> {
    if args.is_empty() || syn::parse2::<LitStr>(args.clone()).is_ok() {
        return Ok(true);
    }
    let args = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;
    for argument in args {
        let Meta::NameValue(argument) = argument else {
            continue;
        };
        if !argument.path.is_ident("execution") {
            continue;
        }
        return Ok(match argument.value {
            Expr::Path(path) => path
                .path
                .segments
                .last()
                .is_none_or(|segment| segment.ident != "ResourceOnly"),
            Expr::Struct(_) => false,
            _ => true,
        });
    }
    Ok(true)
}

fn has_root_fields(item: &ItemStruct) -> syn::Result<bool> {
    let Fields::Named(fields) = &item.fields else {
        return Ok(false);
    };
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
            let Some(first) = arguments.first() else {
                continue;
            };
            match first {
                Meta::Path(path) if path.is_ident("import") || path.is_ident("host") => {
                    return Ok(true)
                }
                Meta::List(list) if list.path.is_ident("event") => return Ok(true),
                _ => {}
            }
        }
    }
    Ok(false)
}

fn plugin_id_literal(args: TokenStream, item: &Ident) -> syn::Result<LitStr> {
    if let Ok(value) = syn::parse2::<LitStr>(args.clone()) {
        return Ok(value);
    }
    if !args.is_empty() {
        let args = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;
        for argument in args {
            let Meta::NameValue(argument) = argument else {
                continue;
            };
            if !argument.path.is_ident("id") {
                continue;
            }
            let value = match argument.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(value),
                    ..
                }) => value,
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "plugin id must be a string literal",
                    ));
                }
            };
            return Ok(value);
        }
    }

    let package = std::env::var("CARGO_PKG_NAME")
        .map_err(|_| syn::Error::new_spanned(item, "plugin package identity is unavailable"))?;
    let name = package
        .strip_prefix("phenix-plugin-")
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            syn::Error::new_spanned(
                item,
                "plugin without an explicit id requires a phenix-plugin-* package name",
            )
        })?;
    Ok(LitStr::new(&format!("phenix.{name}"), item.span()))
}

fn strip_synthetic_root(expanded: TokenStream, plugin: &Ident) -> syn::Result<TokenStream> {
    let mut file = syn::parse2::<syn::File>(expanded)?;
    for item in &mut file.items {
        let Item::Struct(item) = item else {
            continue;
        };
        if &item.ident != plugin {
            continue;
        }
        let Fields::Named(fields) = &mut item.fields else {
            continue;
        };
        fields.named = fields
            .named
            .clone()
            .into_iter()
            .filter(|field| {
                field
                    .ident
                    .as_ref()
                    .is_none_or(|ident| ident.to_string() != ROOT_FIELD)
            })
            .collect();
        break;
    }

    Ok(quote!(#file))
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
        if lifecycle_role(attribute)?.is_some() {
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
            match lifecycle_role(attribute)? {
                Some("start") => start = Some(method.sig.ident.clone()),
                Some("stop") => stop = Some(method.sig.ident.clone()),
                _ => {}
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
    fn root_impl_lowers_expose_and_on_event_without_api_redirect() {
        let output = expand(
            TokenStream::new(),
            quote! {
                impl Plugin {
                    #[phenix(expose(name = "run"))]
                    async fn execute(&mut self, request: Request) -> Response {
                        todo!()
                    }

                    #[phenix(on_event("fixture.changed"))]
                    async fn changed(
                        &mut self,
                        _context: &phenix_sdk::EventContext,
                        _event: Event,
                    ) {
                    }
                }
            },
        )
        .unwrap()
        .to_string();

        assert!(output.contains("StaticComponentBehavior for __PhenixPluginRootComponentForPlugin"));
        assert!(output.contains("StaticComponentRuntimeDispatch for Plugin"));
        assert!(output.contains("/public/"));
        assert!(output.contains("fixture.changed"));
        assert!(!output.contains("on_event"));
    }

    #[test]
    fn root_flag_generates_hidden_default_component_without_struct_field() {
        let output = expand(
            quote!(root, id = "fixture.root"),
            quote! {
                struct Plugin {
                    state: usize,
                }
            },
        )
        .unwrap()
        .to_string();

        assert!(output.contains("__PhenixPluginRootComponentForPlugin"));
        assert!(output.contains("fixture.root"));
        assert!(!output.contains("__phenix_root :"));
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
