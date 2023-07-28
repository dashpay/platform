use crate::VersionAttributes;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use std::str::FromStr;
use syn::{parse_macro_input, Data, DataEnum, DeriveInput, LitInt, LitStr, Meta, Path, Type};

pub(super) fn derive_platform_serialize_enum(
    token_stream_input: TokenStream,
    input: &DeriveInput,
    version_attributes: VersionAttributes,
    data_enum: &DataEnum,
    error_type: Path,
    name: &Ident,
) -> TokenStream {
    let VersionAttributes {
        passthrough,
        platform_serialize_limit,
        platform_version_path,
        untagged,
        unversioned,
        allow_prepend_version,
        force_prepend_version,
        crate_name,
        ..
    } = version_attributes;

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
                                bincode::error::EncodeError::Io{inner, index} => #error_type::MaxEncodedBytesReachedError{max_size_kbytes: #limit, size_hit: index},
                                _ => #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e)),
                            }})
            },
        )
    } else {
        (
            quote! {
                let config = bincode::config::standard().with_big_endian().with_no_limit();
            },
            quote! {
                .map_err(|e| {
                        #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e))
                    })
            },
        )
    };

    // if passthrough {
    //     let match_exprs = data_enum
    //         .variants
    //         .iter()
    //         .map(|v| {
    //             let ident = &v.ident;
    //             quote! { #name::#ident(inner) => #crate_name::serialization::PlatformSerializable::serialize_with_platform_version(inner, platform_version) }
    //         })
    //         .collect::<Vec<_>>();
    //
    //     serialize_into = quote! {
    //         match self {
    //             #( #match_exprs, )*
    //         }
    //     };
    //
    //     let match_exprs_consume = data_enum
    //         .variants
    //         .iter()
    //         .map(|v| {
    //             let ident = &v.ident;
    //             quote! { #name::#ident(inner) => #crate_name::serialization::PlatformSerializable::serialize_consume_with_platform_version(inner, platform_version) }
    //         })
    //         .collect::<Vec<_>>();
    //
    //     serialize_into_consume = quote! {
    //         match self {
    //             #( #match_exprs_consume, )*
    //         }
    //     };
    // } else if untagged {
    //     let match_exprs = data_enum
    //         .variants
    //         .iter()
    //         .map(|v| {
    //             let ident = &v.ident;
    //             quote! { #name::#ident(inner) => bincode::encode_to_vec(inner, config) }
    //         })
    //         .collect::<Vec<_>>();
    //
    //     serialize_into = quote! {
    //         match self {
    //             #( #match_exprs, )*
    //         }#map_err
    //     };
    //
    //     let match_exprs_consume = data_enum
    //         .variants
    //         .iter()
    //         .map(|v| {
    //             let ident = &v.ident;
    //             quote! { #name::#ident(inner) => platform_serialization::platform_encode_to_vec(inner, config, platform_version) }
    //         })
    //         .collect::<Vec<_>>();
    //
    //     serialize_into_consume = quote! {
    //         match self {
    //             #( #match_exprs_consume, )*
    //         }#map_err
    //     };
    // } else {

    // };

    let serialization_body = if unversioned {
        quote! {
            impl #impl_generics #crate_name::serialization::PlatformSerializable for #name #ty_generics #where_clause
            {
                fn serialize(&self) -> Result<Vec<u8>, #error_type> {
                    #config
                    bincode::encode_to_vec(self, config)#limit_err
                }

                fn serialize_consume(self) -> Result<Vec<u8>, #error_type> {
                    #config
                    bincode::encode_to_vec(self, config)#limit_err
                }
            }
        }
    } else {
        quote! {
             impl #impl_generics #crate_name::serialization::PlatformSerializableWithPlatformVersion for #name #ty_generics #where_clause {
                fn serialize_with_platform_version(&self, platform_version: &#crate_name::version::PlatformVersion) -> Result<Vec<u8>, #error_type> {
                    #config
                        platform_serialization::platform_encode_to_vec(self, config, platform_version)#limit_err
                }

                fn serialize_consume_with_platform_version(self, platform_version: &#crate_name::version::PlatformVersion) -> Result<Vec<u8>, #error_type> {
                    #config
                        platform_serialization::platform_encode_to_vec(self, config, platform_version)#limit_err
                }
            }
        }
    };
    //
    // let with_limit_prefix_version_body =
    // };
    //
    // let with_prefix_version_body = quote! {
    //     impl #impl_generics #crate_name::serialization::PlatformSerializableWithPrefixVersion for #name #ty_generics #where_clause {
    //         fn serialize_with_prefix_version(&self, version: #crate_name::version::FeatureVersion) -> Result<Vec<u8>, #error_type> {
    //             let config = bincode::config::standard().with_big_endian().with_no_limit();
    //
    //             let mut serialized = #serialize_into;
    //
    //             let mut encoded_version = bincode::encode_to_vec(&version, config.clone()).map_err(|e| {
    //                 match e {
    //                     bincode::error::EncodeError::Io{inner, index} => #crate_name::#error_type::MaxEncodedBytesReachedError{max_size_kbytes: #limit, size_hit: index},
    //                     _ => #crate_name::#error_type::PlatformSerializationError(format!("unable to serialize version {}: {}", stringify!(#name), e)),
    //                 }
    //             })?;
    //
    //             encoded_version.append(&mut serialized); // prepend the version to the serialized data
    //
    //             Ok(encoded_version)
    //         }
    //
    //         fn serialize_consume_with_prefix_version(self, version: #crate_name::version::FeatureVersion) -> Result<Vec<u8>, #error_type> {
    //             let config = bincode::config::standard().with_big_endian().with_no_limit();
    //
    //             let mut serialized = #serialize_into_consume
    //
    //             let mut encoded_version = bincode::encode_to_vec(&version, config.clone()).map_err(|e| {
    //                 match e {
    //                     bincode::error::EncodeError::Io{inner, index} => #crate_name::#error_type::MaxEncodedBytesReachedError{max_size_kbytes: #limit, size_hit: index},
    //                     _ => #crate_name::#error_type::PlatformSerializationError(format!("unable to serialize version {}: {}", stringify!(#name), e)),
    //                 }
    //             })?;
    //
    //             encoded_version.append(&mut serialized); // prepend the version to the serialized data
    //
    //             Ok(encoded_version)
    //         }
    //     }
    // };
    //
    // let with_platform_version_body = if let Some(platform_version_path) = platform_version_path {
    //     // let platform_version_path_tokens =
    //     //     proc_macro2::TokenStream::from_str(&platform_version_path.value())
    //     //         .expect("Expected a valid field path for 'platform_version_path'");
    //     quote! {
    //
    //     }
    // } else {
    //     quote! {}
    // };

    // let bincode_encode_body =
    // // if it's passthrough we just encode the variants directly
    // if passthrough {
    //     let match_exprs = data_enum
    //         .variants
    //         .iter()
    //         .map(|v| {
    //             let ident = &v.ident;
    //             quote! { #name::#ident(inner) => inner.platform_encode(encoder, platform_version) }
    //         })
    //         .collect::<Vec<_>>();
    //
    //     quote! {
    //         match self {
    //             #( #match_exprs, )*
    //         }
    //     }
    // } else {
    let bincode_encode_body: proc_macro2::TokenStream =
        crate::derive_bincode::derive_encode_inner(token_stream_input)
            .unwrap_or_else(|e| e.into_token_stream())
            .into();
    //     bincode_encode_body
    // };

    let mut expanded = quote! {
        #serialization_body
        #bincode_encode_body
    };
    //
    // if force_prepend_version {
    //     if with_limit {
    //         expanded = quote! {
    //             #with_platform_version_body
    //             #bincode_encode_body
    //         };
    //     } else {
    //         expanded = quote! {
    //             #with_platform_version_body
    //             #bincode_encode_body
    //         };
    //     }
    // } else if allow_prepend_version {
    //     if with_limit {
    //         expanded = quote! {
    //             #with_limit_body
    //             #with_platform_version_body
    //             #bincode_encode_body
    //         };
    //     } else {
    //         expanded = quote! {
    //             #without_limit_body
    //             #with_platform_version_body
    //             #bincode_encode_body
    //         };
    //     }
    // } else if with_limit {
    //     expanded = quote! {
    //         #with_limit_body
    //         #bincode_encode_body
    //     };
    // } else {
    //     expanded = quote! {
    //         #without_limit_body
    //         #bincode_encode_body
    //     };
    // }
    eprintln!("Processing variant: {}", &expanded);

    TokenStream::from(expanded)
}
