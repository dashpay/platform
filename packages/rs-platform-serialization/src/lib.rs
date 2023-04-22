extern crate proc_macro;

use bincode;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(PlatformSerialize)]
pub fn derive_serialize_data_contract(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl #name {
            pub fn serialize(&self) -> Result<Vec<u8>, Error> {
                let config = config::standard().with_big_endian().with_no_limit();
                bincode::encode_to_vec(self, config).map_err(|e| {
                    Error::SerializationError(format!("unable to serialize {}: {}", stringify!(#name), e))
                })
            }
        }
    };

    TokenStream::from(expanded)
}
