extern crate proc_macro;

use bincode;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};
#[proc_macro_derive(
    PlatformSerialize,
    attributes(platform_error_type, platform_serialize_limit, platform_serialize_into)
)]
pub fn derive_platform_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Extract the error type from the attribute.
    let error_type = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("platform_error_type") {
                Some(attr.parse_args::<syn::Path>().unwrap())
            } else {
                None
            }
        })
        .expect("Missing platform_error_type attribute");

    // Extract the serialization limit from the attribute, if it exists.
    let limit = input.attrs.iter().find_map(|attr| {
        if attr.path().is_ident("platform_serialize_limit") {
            Some(attr.parse_args::<syn::LitInt>().unwrap())
        } else {
            None
        }
    });

    let platform_serialize_into: Option<syn::Path> = input.attrs.iter().find_map(|attr| {
        if attr.path().is_ident("platform_serialize_into") {
            Some(attr.parse_args::<syn::Path>().unwrap())
        } else {
            None
        }
    });

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

    let expanded = if let Some(limit) = limit {
        quote! {
            impl #impl_generics #name #ty_generics #where_clause
            {
                pub fn serialize(&self) -> Result<Vec<u8>, #error_type> {
                    let config = config::standard().with_big_endian().with_limit::<{ #limit }>();
                    #serialize_into.map_err(|e| {
                        #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e))
                    })
                }

                pub fn serialize_consume(self) -> Result<Vec<u8>, #error_type> {
                    let config = config::standard().with_big_endian().with_limit::<{ #limit }>();
                    #serialize_into_consume.map_err(|e| {
                        #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e))
                    })
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics #name #ty_generics #where_clause
            {
                pub fn serialize(&self) -> Result<Vec<u8>, #error_type> {
                    let config = config::standard().with_big_endian().with_no_limit();
                    #serialize_into.map_err(|e| {
                        #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e))
                    })
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(
    PlatformDeserialize,
    attributes(
        platform_error_type,
        platform_deserialize_limit,
        platform_deserialize_from
    )
)]
pub fn derive_platform_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Extract the generics.
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Extract the platform_error_type attribute, if provided.
    let platform_error_type = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("platform_error_type") {
                Some(attr.parse_args::<syn::Type>().unwrap())
            } else {
                None
            }
        })
        .unwrap_or_else(|| syn::parse_str("Error").unwrap());

    // Extract the platform_deserialize_limit attribute, if provided.
    let platform_deserialize_limit: Option<usize> = input.attrs.iter().find_map(|attr| {
        if attr.path().is_ident("platform_deserialize_limit") {
            Some(
                attr.parse_args::<syn::LitInt>()
                    .unwrap()
                    .base10_parse()
                    .unwrap(),
            )
        } else {
            None
        }
    });

    let platform_deserialize_from: Option<syn::Path> = input.attrs.iter().find_map(|attr| {
        if attr.path().is_ident("platform_deserialize_from") {
            Some(attr.parse_args::<syn::Path>().unwrap())
        } else {
            None
        }
    });

    let deserialize_from_inner = match platform_deserialize_from {
        Some(inner) => quote! {
            let inner: #inner = bincode::decode_from_slice(data, config).map_err(|e| {
                #platform_error_type::PlatformDeserializationError(format!("unable to deserialize {}: {}", stringify!(#name), e))
            }).map(|(a, _)| a)?;
            inner.try_into().map_err(|e: #platform_error_type| {
                #platform_error_type::PlatformDeserializationError(format!("unable to deserialize {}: {}", stringify!(#name), e))
            })
        },
        None => quote! {
            bincode::decode_from_slice(data, config).map(|(a, _)| a).map_err(|e| {
                #platform_error_type::PlatformDeserializationError(format!("unable to deserialize {}: {}", stringify!(#name), e))
            })
        },
    };

    let config = match platform_deserialize_limit {
        Some(limit) => quote! { config::standard().with_big_endian().with_limit::<{ #limit }>() },
        None => quote! { config::standard().with_big_endian().with_no_limit() },
    };

    let expanded = quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            pub fn deserialize(data: &[u8]) -> Result<Self, #platform_error_type> {
                let config = #config;
                #deserialize_from_inner.map_err(|e| {
                    #platform_error_type::PlatformDeserializationError(format!("unable to deserialize {}: {}", stringify!(#name), e))
                })
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(
    PlatformDeserializeNoLimit,
    attributes(platform_error_type, platform_deserialize_from)
)]
pub fn derive_platform_deserialize_no_limit(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Extract the generics.
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Extract the platform_error_type attribute, if provided.
    let platform_error_type = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("platform_error_type") {
                Some(attr.parse_args::<syn::Type>().unwrap())
            } else {
                None
            }
        })
        .unwrap_or_else(|| syn::parse_str("Error").unwrap());

    let platform_deserialize_from: Option<syn::Path> = input.attrs.iter().find_map(|attr| {
        if attr.path().is_ident("platform_deserialize_from") {
            Some(attr.parse_args::<syn::Path>().unwrap())
        } else {
            None
        }
    });

    let deserialize_from_inner = match platform_deserialize_from {
        Some(inner) => quote! {
            let inner: #inner = bincode::decode_from_slice(data, config).map_err(|e| {
                #platform_error_type::PlatformDeserializationError(format!("unable to deserialize {}: {}", stringify!(#name), e))
            }).map(|(a, _)| a)?;
            inner.try_into().map_err(|e: #platform_error_type| {
                #platform_error_type::PlatformDeserializationError(format!("unable to deserialize {}: {}", stringify!(#name), e))
            })
        },
        None => quote! {
            bincode::decode_from_slice(data, config).map(|(a, _)| a).map_err(|e| {
                #platform_error_type::PlatformDeserializationError(format!("unable to deserialize {}: {}", stringify!(#name), e))
            })
        },
    };

    let expanded = quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            pub fn deserialize_no_limit(data: &[u8]) -> Result<Self, #platform_error_type> {
                let config = config::standard().with_big_endian().with_no_limit();
                #deserialize_from_inner.map_err(|e| {
                    #platform_error_type::PlatformDeserializationError(format!("unable to deserialize {}: {}", stringify!(#name), e))
                })
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(
PlatformSignable,
attributes(platform_error_type, exclude_from_sig_hash)
)]
pub fn derive_platform_signable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let error_type = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("platform_error_type") {
                Some(attr.parse_args::<syn::Path>().unwrap())
            } else {
                None
            }
        })
        .expect("Missing platform_error_type attribute");

    let fields = match &input.data {
        syn::Data::Struct(data) => &data.fields,
        _ => panic!("PlatformSignable can only be derived for structs"),
    };

    let filtered_fields: Vec<&syn::Field> = fields
        .iter()
        .filter(|field| {
            !field.attrs.iter().any(|attr| attr.path().is_ident("exclude_from_sig_hash"))
        })
        .collect();

    let intermediate_name = syn::Ident::new(&format!("{}Intermediate", name), name.span());
    let intermediate_fields: Vec<_> = filtered_fields.iter().map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;
        quote! { #ident: #ty }
    }).collect();

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let field_mapping = filtered_fields.iter().map(|field| {
        let ident = field.ident.as_ref().expect("Expected named field");
        quote! { #ident: self.#ident.clone() }
    });

    let field_mapping_clone = field_mapping.clone();

    let expanded = quote! {
    struct #intermediate_name #impl_generics {
        #( #intermediate_fields, )*
    }

    // impl #impl_generics Clone for #intermediate_name #ty_generics #where_clause {
    //     fn clone(&self) -> Self {
    //         #intermediate_name {
    //             #( #field_mapping_clone.clone(), )*
    //         }
    //     }
    // }

    impl #impl_generics bincode::Encode for #intermediate_name #ty_generics #where_clause {
        fn encode<E>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError>
        where
            E: bincode::Encoder,
        {
            #( encoder.encode_field(&self.#field_mapping_clone.clone())?; )*
            Ok(())
        }
    }

    impl #impl_generics #name #ty_generics #where_clause {
        pub fn sig_hash(&self) -> Result<Vec<u8>, #error_type> {
            let config = config::standard().with_big_endian();

            let intermediate = #intermediate_name {
                #( #field_mapping.clone(), )*
            };

            bincode::encode_to_vec(&intermediate, config).map_err(|e| {
                #error_type::PlatformSerializationError(format!("unable to serialize to produce sig hash {}: {}", stringify!(#name), e))
            })
        }
    }
};

    println!("Expanded code: {}", expanded.to_string());

    TokenStream::from(expanded)
}