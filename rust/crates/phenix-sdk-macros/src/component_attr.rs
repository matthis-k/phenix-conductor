use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

pub(crate) fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    if !args.is_empty() {
        return Err(syn::Error::new_spanned(
            args,
            "component attributes do not accept root arguments yet",
        ));
    }

    let item = syn::parse2::<ItemStruct>(input)?;
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "Phenix components must have one concrete static definition",
        ));
    }
    let name = &item.ident;

    Ok(quote! {
        #item

        impl ::phenix_sdk::StaticComponentDefinition for #name {}
    })
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
}
