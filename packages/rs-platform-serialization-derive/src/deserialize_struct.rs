use crate::{DecodeTrust, TrustNames, VersionAttributes};
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{DataStruct, DeriveInput, Path};

pub(super) fn derive_platform_deserialize_struct(
    token_stream_input: TokenStream,
    input: &DeriveInput,
    version_attributes: VersionAttributes,
    _data_struct: &DataStruct,
    error_type: Path,
    name: &Ident,
) -> TokenStream {
    let VersionAttributes {
        crate_name,
        unversioned,
        trust,
        platform_serialize_limit,
        platform_serialize_into,
        ..
    } = version_attributes;

    let TrustNames {
        decode_from_slice,
        deserializable,
        deserialize,
        deserialize_no_limit,
        from_versioned_structure,
        versioned_deserialize,
        limit_from_versioned_structure,
        versioned_limit_deserialize,
    } = trust.names();

    if !unversioned && trust == DecodeTrust::Untrusted {
        panic!(
            "PlatformDeserializeUntrusted needs `platform_serialize(unversioned)`: a versioned \
             structure decodes through PlatformVersionedDecode, which has no untrusted form"
        );
    }

    // Extract the generics.
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let (config, limit_err) = if let Some(limit) = platform_serialize_limit {
        (
            quote! {
                let config = bincode::config::standard().with_big_endian().with_limit::<{ #limit }>();
            },
            quote! {
                .map_err(|e| {
                        match e {
                            bincode::error::DecodeError::Io { .. } | bincode::error::DecodeError::LimitExceeded => #error_type::MaxEncodedBytesReachedError{max_size_kbytes: #limit, size_hit: bytes.len()},
                            _ => #error_type::PlatformDeserializationError(format!("unable to deserialize {}: {}", stringify!(#name), e)),
                        }
                    })
            },
        )
    } else {
        (
            quote! {
                let config = bincode::config::standard().with_big_endian().with_no_limit();
            },
            quote! {
                    .map_err(|e| {
                        #error_type::PlatformDeserializationError(format!("unable to deserialize {}: {}", stringify!(#name), e))
                    })
            },
        )
    };

    let deserialize_into = match platform_serialize_into {
        Some(inner) => quote! {
            #config
            let inner: #inner = #decode_from_slice(bytes, config).map(|(a, _)| a)?;
            Ok(inner.into())
        },
        None => {
            if !unversioned {
                quote! {
                    #config
                    platform_serialization::platform_versioned_decode_from_slice(&bytes, config, platform_version).map(|(a, _)| a)#limit_err
                }
            } else {
                quote! {
                    #config
                        #decode_from_slice(bytes, config).map(|(a,_)| a)
                        #limit_err
                }
            }
        }
    };

    // if we have passthrough or untagged we can't decode directly

    // Only the trusted derive emits the ordinary `PlatformVersionedDecode` body,
    // so a type deriving both trust levels gets it exactly once.
    let bincode_decode_body: proc_macro2::TokenStream = if trust == DecodeTrust::Trusted {
        crate::derive_bincode::derive_decode_inner(token_stream_input.clone())
            .unwrap_or_else(|e| e.into_token_stream())
            .into()
    } else {
        quote! {}
    };

    let expanded = if unversioned {
        quote! {
            impl #impl_generics #crate_name::serialization::#deserializable for #name #ty_generics #where_clause
            {
                fn #deserialize(bytes: &[u8]) -> Result<Self, #error_type> {
                    #deserialize_into
                }

                fn #deserialize_no_limit(bytes: &[u8]) -> Result<Self, #error_type> {
                    #deserialize_into
                }
            }

            #bincode_decode_body
        }
    } else {
        quote! {
            impl #impl_generics #crate_name::serialization::#from_versioned_structure for #name #ty_generics #where_clause
            {
                fn #versioned_deserialize(bytes: &[u8], platform_version: &#crate_name::version::PlatformVersion) -> Result<Self, #error_type> {
                    #deserialize_into
                }
            }
            impl #impl_generics #crate_name::serialization::#limit_from_versioned_structure for #name #ty_generics #where_clause
            {

                fn #versioned_limit_deserialize(bytes: &[u8], platform_version: &#crate_name::version::PlatformVersion) -> Result<Self, #error_type> {
                    #deserialize_into
                }
            }

            #bincode_decode_body
        }
    };

    TokenStream::from(expanded)
}
