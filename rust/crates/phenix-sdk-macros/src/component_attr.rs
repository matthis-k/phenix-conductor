use crate::interface_attr::validate_interface_id;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::Parser, punctuated::Punctuated, Attribute, Fields, Ident, ImplItem, ItemImpl,
    ItemStruct, LitStr, Meta, Token, Type,
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
    let imports = component_imports(&mut item)?;
    let name = &item.ident;
    let descriptors = imports.iter().map(|import| {
        let field = &import.field;
        let ty = &import.ty;
        quote! {
            ::phenix_sdk::StaticComponentImport::of::<#ty>(stringify!(#field))
        }
    });

    Ok(quote! {
        #item

        impl ::phenix_sdk::StaticComponentDefinition for #name {}

        impl ::phenix_sdk::StaticComponentImports for #name {
            fn imports() -> Vec<::phenix_sdk::StaticComponentImport> {
                vec![#(#descriptors),*]
            }
        }
    })
}

struct ImportContribution {
    field: Ident,
    ty: Type,
}

fn component_imports(item: &mut ItemStruct) -> syn::Result<Vec<ImportContribution>> {
    let Fields::Named(fields) = &mut item.fields else {
        return Ok(Vec::new());
    };
    let mut imports = Vec::new();

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
            if arguments.len() != 1
                || !matches!(arguments.first(), Some(Meta::Path(path)) if path.is_ident("import"))
            {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "unsupported component field contribution",
                ));
            }
            contribution = Some(ImportContribution {
                field: field.ident.clone().expect("named field has an identifier"),
                ty: field.ty.clone(),
            });
        }
        field.attrs = retained;
        if let Some(contribution) = contribution {
            imports.push(contribution);
        }
    }

    Ok(imports)
}

fn expand_impl(mut item: ItemImpl) -> syn::Result<TokenStream> {
    if item.trait_.is_some() || !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item,
            "component behavior must be a non-generic inherent impl",
        ));
    }

    let mut exports = Vec::new();
    for member in &mut item.items {
        let ImplItem::Fn(method) = member else {
            continue;
        };
        let mut retained = Vec::new();
        let mut export = None;
        for attribute in std::mem::take(&mut method.attrs) {
            if !attribute.path().is_ident("phenix") {
                retained.push(attribute);
                continue;
            }
            if export.is_some() {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "a component method may declare only one Phenix contribution",
                ));
            }
            export = Some(parse_export(&attribute)?);
        }
        method.attrs = retained;
        if let Some(export) = export {
            exports.push((method.sig.ident.clone(), export));
        }
    }

    let self_ty = &item.self_ty;
    let descriptors = exports
        .iter()
        .map(|(method, export)| export_descriptor(method, export));

    Ok(quote! {
        #item

        impl ::phenix_sdk::StaticComponentBehavior for #self_ty {
            fn exports() -> Vec<::phenix_sdk::StaticComponentExport> {
                vec![#(#descriptors),*]
            }
        }
    })
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

    let mut public = false;
    for argument in arguments {
        let Meta::Path(path) = argument else {
            return Err(syn::Error::new_spanned(
                argument,
                "export modifiers must be bare flags",
            ));
        };
        if !path.is_ident("public") {
            return Err(syn::Error::new_spanned(path, "unsupported export modifier"));
        }
        if public {
            return Err(syn::Error::new_spanned(path, "duplicate public modifier"));
        }
        public = true;
    }

    Ok(ExportContribution { interface, public })
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
    fn component_impl_lowers_export_metadata_and_strips_helper_attribute() {
        let output = expand(
            TokenStream::new(),
            quote! {
                impl Api {
                    #[phenix(export("fixture.api.run@1"), public)]
                    fn run(&mut self, request: Request) -> Result<Response, Error> {
                        todo!()
                    }
                }
            },
        )
        .unwrap();
        let output = output.to_string();

        assert!(output.contains("StaticComponentBehavior"));
        assert!(output.contains("fixture.api.run@1"));
        assert!(output.contains("public : true"));
        assert!(!output.contains("phenix (export"));
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
}
