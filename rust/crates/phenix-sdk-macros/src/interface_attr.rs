use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, ItemStruct, LitStr};

pub(crate) fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let id = syn::parse2::<LitStr>(args)?;
    let item = syn::parse2::<ItemStruct>(input)?;
    if !item.generics.params.is_empty() || !matches!(item.fields, Fields::Unit) {
        return Err(syn::Error::new_spanned(
            &item,
            "Phenix interface markers must be non-generic unit structs",
        ));
    }
    if id.value().is_empty() {
        return Err(syn::Error::new_spanned(&id, "interface id must not be empty"));
    }
    let name = &item.ident;

    Ok(quote! {
        #item

        impl ::phenix_sdk::InterfaceMarker for #name {
            fn interface_id() -> ::phenix_sdk::InterfaceId {
                ::phenix_sdk::InterfaceId::parse(#id)
                    .expect("interface attribute contains a valid static interface id")
            }
        }
    })
}
