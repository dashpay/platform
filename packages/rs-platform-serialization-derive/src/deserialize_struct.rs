use crate::VersionAttributes;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DeriveInput, LitInt, LitStr, Meta, Path, Type};

pub(super) fn derive_platform_deserialize_struct(
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
        derive_bincode: nested,
        ..
    } = version_attributes;

    // Extract the generics.
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let deserialize_into = match platform_serialize_into {
        Some(inner) => quote! {
            let inner: #inner = bincode::decode_from_slice(&bytes, config).map(|(a, _)| a)?;
            Ok(inner.into())
        },
        None => quote! {
            bincode::decode_from_slice(&bytes, config).map(|(a, _)| a)
        },
    };

    // if we have passthrough or untagged we can't decode directly
    let bincode_decode_body = if nested {
        let bincode_decode_body: proc_macro2::TokenStream =
            crate::derive_bincode::derive_decode_inner(token_stream_input.clone())
                .unwrap_or_else(|e| e.into_token_stream())
                .into();

        quote! {
            #bincode_decode_body
        }
    } else {
        quote! {}
    };

    let expanded = if let Some(limit) = platform_serialize_limit {
        quote! {
            impl #impl_generics #crate_name::serialization::PlatformDeserializable for #name #ty_generics #where_clause
            {
                fn deserialize(bytes: &[u8]) -> Result<Self, #error_type> {
                    let config = bincode::config::standard().with_big_endian().with_limit::<{ #limit }>();
                    #deserialize_into.map_err(|e| {
                        match e {
                            bincode::error::DecodeError::Io{inner} => #error_type::MaxEncodedBytesReachedError{max_size_kbytes: #limit},
                            _ => #error_type::PlatformDeserializationError(format!("unable to deserialize {}: {}", stringify!(#name), e)),
                        }
                    })
                }

                fn deserialize_no_limit(bytes: &[u8]) -> Result<Self, #error_type> {
                    let config = bincode::config::standard().with_big_endian().with_no_limit();
                    #deserialize_into.map_err(|e| {
                        match e {
                            bincode::error::DecodeError::Io{inner} => #error_type::MaxEncodedBytesReachedError{max_size_kbytes: #limit},
                            _ => #error_type::PlatformDeserializationError(format!("unable to deserialize {}: {}", stringify!(#name), e)),
                        }
                    })
                }
            }
            #bincode_decode_body
        }
    } else {
        // we only need deserialize_no_limit, as deserialize will use deserialize_no_limit
        quote! {
            impl #impl_generics #crate_name::serialization::PlatformDeserializable for #name #ty_generics #where_clause
            {
                fn deserialize_no_limit(bytes: &[u8]) -> Result<Self, #error_type> {
                    let config = bincode::config::standard().with_big_endian().with_no_limit();
                    #deserialize_into.map_err(|e| {
                        #error_type::PlatformDeserializationError(format!("unable to deserialize {}: {}", stringify!(#name), e))
                    })
                }
            }

            #bincode_decode_body
        }
    };

    TokenStream::from(expanded)
}
