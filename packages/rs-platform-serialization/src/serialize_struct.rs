use crate::VersionAttributes;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{
    parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Expr, Lit, LitInt, LitStr, Meta,
    Path, Type,
};

pub(super) fn derive_platform_serialize_struct(
    token_stream_input: TokenStream,
    input: &DeriveInput,
    version_attributes: VersionAttributes,
    data_struct: &DataStruct,
    error_type: Path,
    name: &Ident,
) -> TokenStream {
    let VersionAttributes {
        crate_name,
        platform_serialize_limit,
        platform_serialize_into,
        nested,
        ..
    } = version_attributes;

    // Extract the generics.
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let serialize_into = match platform_serialize_into.clone() {
        Some(inner) => quote! {
            let inner: #inner = self.clone().into();
            bincode::encode_to_vec(inner, config)
        },
        None => quote! {
            bincode::encode_to_vec(self, config)
        },
    };

    let serialize_into_consume = match platform_serialize_into {
        Some(inner) => quote! {
            let inner: #inner = self.into();
            bincode::encode_to_vec(inner, config)
        },
        None => quote! {
            bincode::encode_to_vec(self, config)
        },
    };

    let encode_body = if nested {
        quote! { self.encode(encoder) }
    } else {
        quote! {}
    };

    let bincode_encode_body = quote! {
        impl #impl_generics bincode::Encode for #name #ty_generics #where_clause {
            fn encode<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
                #encode_body
            }
        }
    };

    let expanded = if let Some(limit) = platform_serialize_limit {
        quote! {
            impl #impl_generics #crate_name::serialization_traits::PlatformSerializable for #name #ty_generics #where_clause
            {
                fn serialize(&self) -> Result<Vec<u8>, #error_type> {
                    let config = bincode::config::standard().with_big_endian().with_limit::<{ #limit }>();
                    #serialize_into.map_err(|e| {
                    match e {
                        bincode::error::EncodeError::Io{inner, index} => #error_type::MaxEncodedBytesReachedError{max_size_kbytes: #limit, size_hit: index},
                        _ => #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e)),
                    }})
                }

                fn serialize_consume(self) -> Result<Vec<u8>, #error_type> {
                    let config = bincode::config::standard().with_big_endian().with_limit::<{ #limit }>();
                    #serialize_into_consume.map_err(|e| {
                    match e {
                        bincode::error::EncodeError::Io{inner, index} => #error_type::MaxEncodedBytesReachedError{max_size_kbytes: #limit, size_hit: index},
                        _ => #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e)),
                    }})
                }
            }
            #bincode_encode_body
        }
    } else {
        quote! {
            impl #impl_generics #crate_name::serialization_traits::PlatformSerializable for #name #ty_generics #where_clause
            {
                fn serialize(&self) -> Result<Vec<u8>, #error_type> {
                    let config = bincode::config::standard().with_big_endian().with_no_limit();
                    #serialize_into.map_err(|e| {
                        #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e))
                    })
                }

                fn serialize_consume(self) -> Result<Vec<u8>, #error_type> {
                    let config = bincode::config::standard().with_big_endian().with_no_limit();
                    #serialize_into_consume.map_err(|e| {
                        #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e))
                    })
                }
            }

            #bincode_encode_body
        }
    };

    TokenStream::from(expanded)
}
