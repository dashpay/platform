use crate::VersionAttributes;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{Data, DataEnum, DeriveInput, LitInt, LitStr, Meta, Path, Type};

pub(super) fn derive_platform_deserialize_enum(
    token_stream_input: TokenStream,
    input: &DeriveInput,
    version_attributes: VersionAttributes,
    data_enum: &DataEnum,
    error_type: Path,
    name: &Ident,
) -> TokenStream {
    let VersionAttributes {
        passthrough,
        derive_bincode: nested,
        platform_version_path,
        platform_serialize_limit,
        untagged,
        crate_name,
        ..
    } = version_attributes;

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let map_err = quote! {.map_err(|e| {
        #crate_name::#error_type::PlatformDeserializationError(format!("unable to deserialize {} : {}", stringify!(#name), e))
    })};

    // if we have passthrough or untagged we can't decode directly
    let bincode_decode_body = if nested && !passthrough && !untagged {
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

    let platform_deserialize_body = if passthrough || untagged {
        // If we deserialize with passthrough or untagged, this means that we previously discarded
        // information about the variant, meaning that the variant is code structure only.
        // A passthrough serialization doesn't look at the current variant and instead just
        // serializes the underlying type with .serialize
        // An untagged serialization serializes with bincode encode very similarly.
        // When deserializing we should deserialize based on the protocol version.

        // These variations only work with versioned deserialize and versioned limit deserialize.
        // These are the following traits.

        // pub trait PlatformDeserializableFromVersionedStructure {
        //     /// We will deserialize a versioned structure into a code structure
        //     /// For example we have DataContractV0 and DataContractV1
        //     /// The system version will tell which version to deserialize into
        //     /// This happens by first deserializing the data into a potentially versioned structure
        //     /// For example we could have DataContractSerializationFormatV0 and DataContractSerializationFormatV1
        //     /// Both of the structures will be valid in perpetuity as they are saved into the state.
        //     /// So from the bytes we could get DataContractSerializationFormatV0.
        //     /// Then the system_version given will tell to transform DataContractSerializationFormatV0 into
        //     /// DataContractV1 (if system version is 1)
        //     fn versioned_deserialize(
        //         data: &[u8],
        //         platform_version: &PlatformVersion,
        //     ) -> Result<Self, ProtocolError>
        //     where
        //         Self: Sized;
        // }
        //
        // pub trait PlatformLimitDeserializableFromVersionedStructure {
        //     fn versioned_limit_deserialize(
        //         data: &[u8],
        //         platform_version: &PlatformVersion,
        //     ) -> Result<Self, ProtocolError>
        //     where
        //         Self: Sized;
        // }

        // The platform_version_path describes how to get the feature version based on the
        // PlatformVersion, (by doing platform_version.<#platform_version_path>)

        // If we are untagged, we will call Decode knowing this subversion.
        // If we are passthrough we will instead call Platform deserialize on the subversion.

        if let Some(platform_version_path) = platform_version_path {
            // Generate the match arms for each variant of the enum
            let match_arms_no_limit: Vec<_> = data_enum.variants.iter().enumerate().map(|(index, variant)| {
                let variant_name = &variant.ident;
                let versioned_variant_name = format_ident!("{}{}", name, variant_name);
                if passthrough {
                    quote! {
                        #index => {
                            let deserialized = #versioned_variant_name::platform_deserialize(data, platform_version)?;
                            deserialized.into()
                        },
                    }
                } else {
                    quote! {
                        #index => {
                            let config = bincode::config::standard().with_big_endian().with_no_limit();
                            let deserialized : #versioned_variant_name = bincode::decode_from_slice(&data, config)#map_err?;
                            deserialized.into()
                        },
                    }
                }

                        }).collect();

            let deserialize_block_no_limit = quote! {
                let sub_version = platform_version.#platform_version_path;
                match sub_version {
                    #(#match_arms_no_limit)*
                    _ => Err(#crate_name::#error_type::PlatformDeserializationError(format!("Unsupported version for {}: {}", stringify!(#name), sub_version))),
                }
            };

            let without_limit = quote! {
                impl #impl_generics #crate_name::serialization::PlatformDeserializableFromVersionedStructure for #name #ty_generics #where_clause {
                    fn versioned_deserialize(
                        data: &[u8],
                        platform_version: &PlatformVersion,
                    ) -> Result<Self, ProtocolError>
                    where
                        Self: Sized {
                        deserialize_block_no_limit
                    }
                }
            };

            let with_limit = if let Some(limit) = platform_serialize_limit {
                let match_arms_with_limit: Vec<_> = data_enum.variants.iter().enumerate().map(|(index, variant)| {
                    let variant_name = &variant.ident;
                    let versioned_variant_name = format_ident!("{}{}", name, variant_name);
                    if passthrough {
                        quote! {
                        #index => {
                            let deserialized = #versioned_variant_name::platform_deserialize(data, platform_version)?;
                            deserialized.into()
                        },
                    }
                    } else {
                        quote! {
                        #index => {
                            let config = bincode::config::standard().with_big_endian().with_big_endian().with_limit::<{ #limit }>();
                            let deserialized : #versioned_variant_name = bincode::decode_from_slice(&data, config)#map_err?;
                            deserialized.into()
                        },
                    }
                    }

                }).collect();

                let deserialize_block_with_limit = quote! {
                    let sub_version = platform_version.#platform_version_path;
                    match sub_version {
                        #(#match_arms_with_limit)*
                        _ => Err(#crate_name::#error_type::PlatformDeserializationError(format!("Unsupported version for {}: {}", stringify!(#name), sub_version))),
                    }
                };

                quote! {
                    impl #impl_generics #crate_name::serialization::PlatformLimitDeserializableFromVersionedStructure for #name #ty_generics #where_clause {
                        fn versioned_limit_deserialize(
                            data: &[u8],
                            platform_version: &PlatformVersion,
                        ) -> Result<Self, ProtocolError>
                        where
                            Self: Sized {
                            #deserialize_block_with_limit
                        }
                    }
                }
            } else {
                quote! {}
            };

            quote! {
                #without_limit
                #with_limit
            }
        } else {
            quote! {}
        }
    } else {
        quote! {
            impl #impl_generics #crate_name::serialization::PlatformDeserializable for #name #ty_generics #where_clause {
                        fn deserialize(
                            data: &[u8]
                        ) -> Result<Self, ProtocolError>
                        where
                            Self: Sized {
                            let config = bincode::config::standard().with_big_endian().with_no_limit();
                            bincode::decode_from_slice(&data, config).map(|(a,_)| a)#map_err
                        }

                        fn deserialize_no_limit(
                            data: &[u8]
                        ) -> Result<Self, ProtocolError>
                        where
                            Self: Sized {
                            let config = bincode::config::standard().with_big_endian().with_no_limit();
                            bincode::decode_from_slice(&data, config).map(|(a,_)| a)#map_err
                        }
                    }
        }
    };

    let expanded = quote! {
        #bincode_decode_body
        #platform_deserialize_body
    };

    // eprintln!("Processing deserialize variant: {}", &expanded);

    TokenStream::from(expanded)
}
