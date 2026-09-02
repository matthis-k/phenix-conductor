use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::Parser, punctuated::Punctuated, Attribute, Expr, ExprLit, ImplItem, ItemImpl, Lit, Meta,
    Token,
};

pub(crate) fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let schema = parse_schema(args)?;
    let mut item = syn::parse2::<ItemImpl>(input)?;
    if item.trait_.is_some() {
        return Err(syn::Error::new_spanned(
            &item,
            "#[phenix_sdk::resource] applies to an inherent resource impl",
        ));
    }

    let mut migrations = Vec::new();
    for member in &mut item.items {
        let ImplItem::Fn(method) = member else {
            continue;
        };
        let mut retained = Vec::new();
        let mut migration = None;
        for attribute in std::mem::take(&mut method.attrs) {
            if !attribute.path().is_ident("phenix") {
                retained.push(attribute);
                continue;
            }
            if migration.is_some() {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "a resource method may declare only one Phenix contribution",
                ));
            }
            let from = parse_migration(&attribute)?;
            if from >= schema {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "resource migration source must be older than the declared schema",
                ));
            }
            migration = Some((from, method.sig.ident.clone()));
        }
        method.attrs = retained;
        if let Some(migration) = migration {
            migrations.push(migration);
        }
    }

    migrations.sort_by_key(|(from, _)| *from);
    for pair in migrations.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(syn::Error::new_spanned(
                &pair[1].1,
                "duplicate migration source version",
            ));
        }
    }

    let self_ty = &item.self_ty;
    let migration_descriptors = migrations.iter().map(|(from, method)| {
        quote! {
            ::phenix_sdk::StaticResourceMigration {
                from_version: #from,
                to_version: #schema,
                method: stringify!(#method),
            }
        }
    });

    Ok(quote! {
        #item

        impl ::phenix_sdk::StaticResourceDefinition for #self_ty {
            fn schema_version() -> u32 {
                #schema
            }

            fn migrations() -> Vec<::phenix_sdk::StaticResourceMigration> {
                vec![#(#migration_descriptors),*]
            }
        }
    })
}

fn parse_schema(args: TokenStream) -> syn::Result<u32> {
    let arguments = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;
    let mut schema = None;
    for argument in arguments {
        let Meta::NameValue(argument) = argument else {
            return Err(syn::Error::new_spanned(
                argument,
                "resource attributes must use key = value syntax",
            ));
        };
        if !argument.path.is_ident("schema") {
            return Err(syn::Error::new_spanned(
                argument.path,
                "unsupported resource attribute",
            ));
        }
        if schema.is_some() {
            return Err(syn::Error::new_spanned(
                argument,
                "duplicate resource schema",
            ));
        }
        schema = Some(integer_literal(
            argument.value,
            "resource schema must be an integer",
        )?);
    }
    let schema = schema.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "resource requires schema = <version>",
        )
    })?;
    if schema == 0 {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "resource schema version must be positive",
        ));
    }
    Ok(schema)
}

fn parse_migration(attribute: &Attribute) -> syn::Result<u32> {
    let Meta::List(meta) = &attribute.meta else {
        return Err(syn::Error::new_spanned(
            attribute,
            "resource method attributes must use #[phenix(...)] syntax",
        ));
    };
    let arguments = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(meta.tokens.clone())?;
    let mut arguments = arguments.into_iter();
    let Some(Meta::List(migrate)) = arguments.next() else {
        return Err(syn::Error::new_spanned(
            attribute,
            "resource method contribution must begin with migrate(...)",
        ));
    };
    if !migrate.path.is_ident("migrate") || arguments.next().is_some() {
        return Err(syn::Error::new_spanned(
            attribute,
            "resource methods only support #[phenix(migrate(from = <version>))]",
        ));
    }
    let metadata = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(migrate.tokens)?;
    let mut metadata = metadata.into_iter();
    let Some(Meta::NameValue(from)) = metadata.next() else {
        return Err(syn::Error::new_spanned(
            migrate.path,
            "migration requires from = <version>",
        ));
    };
    if metadata.next().is_some() || !from.path.is_ident("from") {
        return Err(syn::Error::new_spanned(
            from.path,
            "migration requires exactly one from = <version>",
        ));
    }
    integer_literal(from.value, "migration source must be an integer")
}

fn integer_literal(value: Expr, message: &'static str) -> syn::Result<u32> {
    let Expr::Lit(ExprLit {
        lit: Lit::Int(value),
        ..
    }) = value
    else {
        return Err(syn::Error::new_spanned(value, message));
    };
    value.base10_parse::<u32>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn resource_impl_lowers_schema_and_migration_metadata() {
        let output = expand(
            quote!(schema = 3),
            quote! {
                impl Store {
                    #[phenix(migrate(from = 2))]
                    fn v2_to_v3() {}
                }
            },
        )
        .unwrap()
        .to_string();

        assert!(output.contains("StaticResourceDefinition for Store"));
        assert!(output.contains("from_version : 2"));
        assert!(output.contains("to_version : 3"));
        assert!(!output.contains("phenix (migrate"));
    }

    #[test]
    fn resource_impl_rejects_multiple_contributions_on_one_method() {
        let error = expand(
            quote!(schema = 3),
            quote! {
                impl Store {
                    #[phenix(migrate(from = 1))]
                    #[phenix(migrate(from = 2))]
                    fn migrate() {}
                }
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("only one Phenix contribution"));
    }
}
