extern crate proc_macro;

use bincode;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(
    PlatformSerialize,
    attributes(platform_serialize_error_type, platform_serialize_limit)
)]
pub fn derive_platform_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Extract the error type from the attribute.
    let error_type = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("platform_serialize_error_type") {
                Some(attr.parse_args::<syn::Path>().unwrap())
            } else {
                None
            }
        })
        .expect("Missing platform_serialize_error_type attribute");

    // Extract the serialization limit from the attribute, if it exists.
    let limit = input.attrs.iter().find_map(|attr| {
        if attr.path().is_ident("platform_serialize_limit") {
            Some(attr.parse_args::<syn::LitInt>().unwrap())
        } else {
            None
        }
    });

    // Extract the generics.
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = if let Some(limit) = limit {
        quote! {
            impl #impl_generics #name #ty_generics #where_clause
            where
                Self: bincode::Encode
            {
                pub fn serialize(&self) -> Result<Vec<u8>, #error_type> {
                    let config = config::standard().with_big_endian().with_limit(#limit);
                    bincode::encode_to_vec(self, config).map_err(|e| {
                        #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e))
                    })
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics #name #ty_generics #where_clause
            where
                Self: bincode::Encode
            {
                pub fn serialize(&self) -> Result<Vec<u8>, #error_type> {
                    let config = config::standard().with_big_endian().with_no_limit();
                    bincode::encode_to_vec(self, config).map_err(|e| {
                        #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e))
                    })
                }
            }
        }
    };

    TokenStream::from(expanded)
}
