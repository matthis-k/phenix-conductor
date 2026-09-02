use crate::interface_attr::validate_interface_id;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::Parser, punctuated::Punctuated, Attribute, Expr, Fields, Ident, ImplItem, ItemImpl,
    ItemStruct, LitStr, Meta, Path, Token, Type,
};

pub(crate) fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    if !args.is_empty() {
        return Err(syn::Error::new_spanned(
            args,
            "component attributes do not accept root arguments yet",
        ));
    }

    if let Ok(item) = syn::parse2::<ItemStruct>(input.clone()) {
        return expand_struct(item);
    }
    if let Ok(item) = syn::parse2::<ItemImpl>(input.clone()) {
        return expand_impl(item);
    }

    Err(syn::Error::new_spanned(
        input,
        "#[phenix_sdk::component] applies to a component struct or inherent impl",
    ))
}

fn expand_struct(item: ItemStruct) -> syn::Result<TokenStream> {
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "Phenix components must have one concrete static definition",
        ));
    }
    let mut item = item;
    let contributions = component_fields(&mut item)?;
    let name = &item.ident;
    let imports = contributions.imports.iter().map(|import| {
        let field = &import.field;
        let ty = &import.ty;
        quote! {
            ::phenix_sdk::StaticComponentImport::of::<#ty>(stringify!(#field))
        }
    });
    let hosts = contributions.hosts.iter().map(|host| {
        let field = &host.field;
        let ty = &host.ty;
        quote! {
            ::phenix_sdk::StaticComponentHost::of::<#ty>(stringify!(#field))
        }
    });
    let events = contributions.events.iter().map(|event| {
        let field = &event.field;
        let ty = &event.ty;
        let id = &event.event;
        quote! {
            ::phenix_sdk::StaticComponentEvent::of::<#ty>(#id, stringify!(#field))
        }
    });

    Ok(quote! {
        #item

        impl ::phenix_sdk::StaticComponentDefinition for #name {}

        impl ::phenix_sdk::StaticComponentImports for #name {
            fn imports() -> Vec<::phenix_sdk::StaticComponentImport> {
                vec![#(#imports),*]
            }

            fn hosts() -> Vec<::phenix_sdk::StaticComponentHost> {
                vec![#(#hosts),*]
            }

            fn events() -> Vec<::phenix_sdk::StaticComponentEvent> {
                vec![#(#events),*]
            }
        }
    })
}

#[derive(Default)]
struct ComponentFieldContributions {
    imports: Vec<ImportContribution>,
    hosts: Vec<ImportContribution>,
    events: Vec<EventFieldContribution>,
}

struct ImportContribution {
    field: Ident,
    ty: Type,
}

struct EventFieldContribution {
    field: Ident,
    ty: Type,
    event: LitStr,
}

enum FieldRole {
    Import(ImportContribution),
    Host(ImportContribution),
    Event(EventFieldContribution),
}

fn component_fields(item: &mut ItemStruct) -> syn::Result<ComponentFieldContributions> {
    let Fields::Named(fields) = &mut item.fields else {
        return Ok(ComponentFieldContributions::default());
    };
    let mut contributions = ComponentFieldContributions::default();

    for field in &mut fields.named {
        let mut retained = Vec::new();
        let mut contribution = None;
        for attribute in std::mem::take(&mut field.attrs) {
            if !attribute.path().is_ident("phenix") {
                retained.push(attribute);
                continue;
            }
            if contribution.is_some() {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "a component field may have only one Phenix role",
                ));
            }
            let Meta::List(meta) = &attribute.meta else {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "component field attributes must use #[phenix(...)] syntax",
                ));
            };
            let arguments =
                Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
            let name = field.ident.clone().expect("named field has an identifier");
            contribution = Some(match arguments.first() {
                Some(Meta::Path(path)) if arguments.len() == 1 && path.is_ident("import") => {
                    FieldRole::Import(ImportContribution {
                        field: name,
                        ty: field.ty.clone(),
                    })
                }
                Some(Meta::Path(path)) if arguments.len() == 1 && path.is_ident("host") => {
                    FieldRole::Host(ImportContribution {
                        field: name,
                        ty: field.ty.clone(),
                    })
                }
                Some(Meta::List(event)) if arguments.len() == 1 && event.path.is_ident("event") => {
                    let id = syn::parse2::<LitStr>(event.tokens.clone())?;
                    validate_event_id(&id.value())
                        .map_err(|error| syn::Error::new_spanned(&id, error))?;
                    FieldRole::Event(EventFieldContribution {
                        field: name,
                        ty: field.ty.clone(),
                        event: id,
                    })
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        attribute,
                        "unsupported component field contribution",
                    ));
                }
            });
        }
        field.attrs = retained;
        if let Some(contribution) = contribution {
            match contribution {
                FieldRole::Import(value) => contributions.imports.push(value),
                FieldRole::Host(value) => contributions.hosts.push(value),
                FieldRole::Event(value) => contributions.events.push(value),
            }
        }
    }

    Ok(contributions)
}

fn expand_impl(mut item: ItemImpl) -> syn::Result<TokenStream> {
    if item.trait_.is_some() || !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item,
            "component behavior must be a non-generic inherent impl",
        ));
    }

    let mut exports = Vec::new();
    let mut layers = Vec::new();
    let mut listeners = Vec::new();
    let mut values = Vec::new();
    for member in &mut item.items {
        let ImplItem::Fn(method) = member else {
            continue;
        };
        let mut retained = Vec::new();
        let mut contribution = None;
        for attribute in std::mem::take(&mut method.attrs) {
            if !attribute.path().is_ident("phenix") {
                retained.push(attribute);
                continue;
            }
            if contribution.is_some() {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "a component method may declare only one Phenix contribution",
                ));
            }
            contribution = Some(parse_method_contribution(&attribute)?);
        }
        method.attrs = retained;
        let Some(contribution) = contribution else {
            continue;
        };
        match contribution {
            MethodContribution::Export(export) => {
                exports.push((method.sig.ident.clone(), export));
            }
            MethodContribution::Layer(layer) => {
                layers.push((method.sig.ident.clone(), layer));
            }
            MethodContribution::Listen(listener) => {
                listeners.push((method.sig.ident.clone(), listener));
            }
            MethodContribution::Value(value) => {
                values.push((method.sig.ident.clone(), value));
            }
        }
    }

    let self_ty = &item.self_ty;
    let export_descriptors = exports
        .iter()
        .map(|(method, export)| export_descriptor(method, export));
    let layer_descriptors = layers.iter().map(|(method, layer)| {
        let interface = &layer.interface;
        let priority = &layer.priority;
        quote! {
            ::phenix_sdk::StaticComponentLayer::of::<#interface>(
                stringify!(#method),
                #priority,
            )
        }
    });
    let listener_descriptors = listeners.iter().map(|(method, listener)| {
        let event = &listener.event;
        quote! {
            ::phenix_sdk::StaticComponentListener::new(#event, stringify!(#method))
        }
    });
    let value_descriptors = values.iter().map(|(method, value)| {
        let id = &value.id;
        let public = value.public;
        quote! {
            ::phenix_sdk::StaticComponentValue::new(#id, stringify!(#method), #public)
        }
    });

    Ok(quote! {
        #item

        impl ::phenix_sdk::StaticComponentBehavior for #self_ty {
            fn exports() -> Vec<::phenix_sdk::StaticComponentExport> {
                vec![#(#export_descriptors),*]
            }

            fn layers() -> Vec<::phenix_sdk::StaticComponentLayer> {
                vec![#(#layer_descriptors),*]
            }

            fn listeners() -> Vec<::phenix_sdk::StaticComponentListener> {
                vec![#(#listener_descriptors),*]
            }

            fn values() -> Vec<::phenix_sdk::StaticComponentValue> {
                vec![#(#value_descriptors),*]
            }
        }
    })
}

enum MethodContribution {
    Export(ExportContribution),
    Layer(LayerContribution),
    Listen(ListenContribution),
    Value(ValueContribution),
}

struct LayerContribution {
    interface: Path,
    priority: Expr,
}

struct ListenContribution {
    event: LitStr,
}

struct ValueContribution {
    id: LitStr,
    public: bool,
}

fn parse_method_contribution(attribute: &Attribute) -> syn::Result<MethodContribution> {
    let Meta::List(meta) = &attribute.meta else {
        return Err(syn::Error::new_spanned(
            attribute,
            "component method attributes must use #[phenix(...)] syntax",
        ));
    };
    let outer = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
    let Some(Meta::List(kind)) = outer.first() else {
        return Err(syn::Error::new_spanned(
            attribute,
            "component method contribution must begin with a contribution",
        ));
    };

    if kind.path.is_ident("export") {
        return parse_export(attribute).map(MethodContribution::Export);
    }
    if kind.path.is_ident("layer") {
        return parse_layer(attribute).map(MethodContribution::Layer);
    }
    if kind.path.is_ident("listen") {
        return parse_listener(attribute).map(MethodContribution::Listen);
    }
    if kind.path.is_ident("value") {
        return parse_value(attribute).map(MethodContribution::Value);
    }

    Err(syn::Error::new_spanned(
        &kind.path,
        "unsupported component method contribution",
    ))
}

pub(crate) struct ExportContribution {
    interface: ExportInterface,
    public: bool,
}

enum ExportInterface {
    Literal(LitStr),
    Marker(Box<Type>),
}

impl ExportInterface {
    fn expression(&self) -> TokenStream {
        match self {
            Self::Literal(id) => quote! {
                ::phenix_sdk::__phenix_plugin::InterfaceId::parse(#id)
                    .expect("component export contains a valid static interface id")
            },
            Self::Marker(marker) => quote! {
                <#marker as ::phenix_sdk::InterfaceMarker>::interface_id()
            },
        }
    }
}

pub(crate) fn export_descriptor(method: &Ident, export: &ExportContribution) -> TokenStream {
    let interface = export.interface.expression();
    let public = export.public;
    quote! {
        ::phenix_sdk::StaticComponentExport {
            interface: #interface,
            method: stringify!(#method),
            public: #public,
        }
    }
}

pub(crate) fn parse_export(attribute: &Attribute) -> syn::Result<ExportContribution> {
    let Meta::List(meta) = &attribute.meta else {
        return Err(syn::Error::new_spanned(
            attribute,
            "component method attributes must use #[phenix(...)] syntax",
        ));
    };
    let arguments = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
    let mut arguments = arguments.into_iter();
    let Some(Meta::List(export)) = arguments.next() else {
        return Err(syn::Error::new_spanned(
            attribute,
            "component method contribution must begin with export(...)",
        ));
    };
    if !export.path.is_ident("export") {
        return Err(syn::Error::new_spanned(
            export.path,
            "unsupported component method contribution",
        ));
    }

    let interface = if let Ok(id) = syn::parse2::<LitStr>(export.tokens.clone()) {
        validate_interface_id(&id.value()).map_err(|error| syn::Error::new_spanned(&id, error))?;
        ExportInterface::Literal(id)
    } else {
        ExportInterface::Marker(Box::new(syn::parse2::<Type>(export.tokens)?))
    };

    let public = parse_public_modifier(arguments, "export")?;
    Ok(ExportContribution { interface, public })
}

fn parse_layer(attribute: &Attribute) -> syn::Result<LayerContribution> {
    let Meta::List(meta) = &attribute.meta else {
        unreachable!("method contribution parser already validated list syntax")
    };
    let outer = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
    if outer.len() != 1 {
        return Err(syn::Error::new_spanned(
            attribute,
            "layer modifiers belong inside layer(...) metadata",
        ));
    }
    let Some(Meta::List(layer)) = outer.first() else {
        unreachable!("method contribution parser already validated nested list syntax")
    };
    let arguments = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(layer.tokens.clone())?;
    let mut arguments = arguments.into_iter();
    let Some(Meta::Path(interface)) = arguments.next() else {
        return Err(syn::Error::new_spanned(
            layer,
            "layer requires an interface marker type",
        ));
    };
    let Some(Meta::NameValue(priority)) = arguments.next() else {
        return Err(syn::Error::new_spanned(
            layer,
            "layer requires priority = <expression>",
        ));
    };
    if !priority.path.is_ident("priority") || arguments.next().is_some() {
        return Err(syn::Error::new_spanned(
            priority.path,
            "layer requires exactly one priority = <expression>",
        ));
    }
    Ok(LayerContribution {
        interface,
        priority: priority.value,
    })
}

fn parse_listener(attribute: &Attribute) -> syn::Result<ListenContribution> {
    let Meta::List(meta) = &attribute.meta else {
        unreachable!("method contribution parser already validated list syntax")
    };
    let outer = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
    if outer.len() != 1 {
        return Err(syn::Error::new_spanned(
            attribute,
            "listener attributes do not accept modifiers",
        ));
    }
    let Some(Meta::List(listener)) = outer.first() else {
        unreachable!("method contribution parser already validated nested list syntax")
    };
    let event = syn::parse2::<LitStr>(listener.tokens.clone())?;
    validate_event_id(&event.value()).map_err(|error| syn::Error::new_spanned(&event, error))?;
    Ok(ListenContribution { event })
}

fn parse_value(attribute: &Attribute) -> syn::Result<ValueContribution> {
    let Meta::List(meta) = &attribute.meta else {
        unreachable!("method contribution parser already validated list syntax")
    };
    let arguments = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
    let mut arguments = arguments.into_iter();
    let Some(Meta::List(value)) = arguments.next() else {
        unreachable!("method contribution parser already validated nested list syntax")
    };
    let id = syn::parse2::<LitStr>(value.tokens)?;
    validate_interface_id(&id.value()).map_err(|error| syn::Error::new_spanned(&id, error))?;
    let public = parse_public_modifier(arguments, "value")?;
    Ok(ValueContribution { id, public })
}

fn parse_public_modifier(
    arguments: impl IntoIterator<Item = Meta>,
    kind: &'static str,
) -> syn::Result<bool> {
    let mut public = false;
    for argument in arguments {
        let Meta::Path(path) = argument else {
            return Err(syn::Error::new_spanned(
                argument,
                format!("{kind} modifiers must be bare flags"),
            ));
        };
        if !path.is_ident("public") {
            return Err(syn::Error::new_spanned(
                path,
                format!("unsupported {kind} modifier"),
            ));
        }
        if public {
            return Err(syn::Error::new_spanned(
                path,
                format!("duplicate {kind} public modifier"),
            ));
        }
        public = true;
    }
    Ok(public)
}

fn validate_event_id(value: &str) -> Result<(), &'static str> {
    if value.is_empty() {
        return Err("event id must not be empty");
    }
    if value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/' | b':')
    }) {
        Ok(())
    } else {
        Err("event id contains unsupported characters")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn component_struct_lowers_to_static_definition() {
        let output = expand(
            TokenStream::new(),
            quote!(
                struct Api;
            ),
        )
        .unwrap();
        let output = output.to_string();

        assert!(output.contains("StaticComponentDefinition"));
        assert!(output.contains("Api"));
    }

    #[test]
    fn component_struct_lowers_typed_import_fields() {
        let output = expand(
            TokenStream::new(),
            quote! {
                struct Api {
                    #[phenix(import)]
                    models: Required<Call<Models, Request, Response>>,
                }
            },
        )
        .unwrap()
        .to_string();

        assert!(output.contains("StaticComponentImports for Api"));
        assert!(output.contains("StaticComponentImport :: of"));
        assert!(output.contains("Required < Call < Models , Request , Response > >"));
        assert!(output.contains("stringify ! (models)"));
        assert!(!output.contains("phenix (import"));
    }

    #[test]
    fn component_struct_rejects_duplicate_field_roles() {
        let error = expand(
            TokenStream::new(),
            quote! {
                struct Api {
                    #[phenix(import)]
                    #[phenix(import)]
                    models: Required<Call<Models, Request, Response>>,
                }
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("only one Phenix role"));
    }

    #[test]
    fn component_struct_rejects_unknown_field_contributions() {
        let error = expand(
            TokenStream::new(),
            quote!(
                struct Api {
                    #[phenix(unknown)]
                    value: String,
                }
            ),
        )
        .unwrap_err();

        assert!(error.to_string().contains("unsupported component field"));
    }

    #[test]
    fn component_impl_lowers_all_documented_behavior_metadata() {
        let output = expand(
            TokenStream::new(),
            quote! {
                impl Api {
                    #[phenix(export("fixture.api.run@1"), public)]
                    fn run(&mut self, request: Request) -> Result<Response, Error> {
                        todo!()
                    }

                    #[phenix(layer(Models, priority = 17))]
                    fn policy(&mut self) {}

                    #[phenix(listen("fixture.completed"))]
                    fn completed(&mut self) {}

                    #[phenix(value("fixture.status@1"), public)]
                    fn status(&self) -> Status {
                        todo!()
                    }
                }
            },
        )
        .unwrap()
        .to_string();

        assert!(output.contains("StaticComponentBehavior"));
        assert!(output.contains("fixture.api.run@1"));
        assert!(output.contains("StaticComponentLayer :: of :: < Models >"));
        assert!(output.contains("fixture.completed"));
        assert!(output.contains("fixture.status@1"));
        assert!(output.contains("public : true"));
        assert!(!output.contains("phenix (export"));
        assert!(!output.contains("phenix (layer"));
        assert!(!output.contains("phenix (listen"));
        assert!(!output.contains("phenix (value"));
    }

    #[test]
    fn component_impl_accepts_interface_marker_type() {
        let output = expand(
            TokenStream::new(),
            quote! {
                impl Api {
                    #[phenix(export(Planning))]
                    fn plan(&mut self) {}
                }
            },
        )
        .unwrap()
        .to_string();

        assert!(output.contains("Planning as :: phenix_sdk :: InterfaceMarker"));
    }

    #[test]
    fn component_impl_rejects_unversioned_literal_interface() {
        let error = expand(
            TokenStream::new(),
            quote! {
                impl Api {
                    #[phenix(export("fixture.api.run"))]
                    fn run(&mut self) {}
                }
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("@version"));
    }

    #[test]
    fn component_impl_rejects_invalid_event_id() {
        let error = expand(
            TokenStream::new(),
            quote! {
                impl Api {
                    #[phenix(listen("fixture completed"))]
                    fn completed(&mut self) {}
                }
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("event id contains unsupported"));
    }
}
