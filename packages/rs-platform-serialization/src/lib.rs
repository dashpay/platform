extern crate proc_macro;

use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, DeriveInput, Expr, Lit, Meta};

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
            impl #impl_generics PlatformSerializable for #name #ty_generics #where_clause
            {
                fn serialize(&self) -> Result<Vec<u8>, #error_type> {
                    let config = config::standard().with_big_endian().with_limit::<{ #limit }>();
                    #serialize_into.map_err(|e| {
                    match e {
                        bincode::error::EncodeError::Io{inner, index} => #error_type::MaxEncodedBytesReachedError{max_size_kbytes: #limit, size_hit: index},
                        _ => #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e)),
                    }})
                }

                fn serialize_consume(self) -> Result<Vec<u8>, #error_type> {
                    let config = config::standard().with_big_endian().with_limit::<{ #limit }>();
                    #serialize_into_consume.map_err(|e| {
                    match e {
                        bincode::error::EncodeError::Io{inner, index} => #error_type::MaxEncodedBytesReachedError{max_size_kbytes: #limit, size_hit: index},
                        _ => #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e)),
                    }})
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics PlatformSerializable for #name #ty_generics #where_clause
            {
                fn serialize(&self) -> Result<Vec<u8>, #error_type> {
                    let config = config::standard().with_big_endian().with_no_limit();
                    #serialize_into.map_err(|e| {
                        #error_type::PlatformSerializationError(format!("unable to serialize {}: {}", stringify!(#name), e))
                    })
                }

                fn serialize_consume(self) -> Result<Vec<u8>, #error_type> {
                    let config = config::standard().with_big_endian().with_no_limit();
                    #serialize_into_consume.map_err(|e| {
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
        impl PlatformDeserializable for #impl_generics #name #ty_generics #where_clause {
            fn deserialize(data: &[u8]) -> Result<Self, #platform_error_type> {
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

#[proc_macro_derive(PlatformSignable, attributes(platform_error_type, platform_signable))]
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
            !field.attrs.iter().any(|attr| {
                if attr.path().is_ident("platform_signable") {
                    let meta: Meta = attr.parse_args().expect("Unable to parse attribute");
                    meta.path().is_ident("exclude_from_sig_hash")
                } else {
                    false
                }
            })
        })
        .collect();

    let intermediate_name = syn::Ident::new(&format!("{}Signable", name), name.span());

    let mut intermediate_fields = Vec::new();
    let mut field_conversions = Vec::new();
    let mut field_mapping = Vec::new();

    for field in &filtered_fields {
        let ident = &field.ident;
        let ty = &field.ty;

        let conversion = field.attrs.iter().find_map(|attr| {
            if attr.path().is_ident("platform_signable") {
                let meta: Meta = attr
                    .parse_args()
                    .expect("Failed to parse 'platform_signable' attribute arguments");
                match meta {
                    Meta::Path(_) => None,
                    Meta::List(meta_list) => meta_list
                        .tokens
                        .into_iter()
                        .filter_map(|token| {
                            if let proc_macro2::TokenTree::Group(group) = token {
                                Some(group.stream())
                            } else {
                                None
                            }
                        })
                        .find_map(|stream| {
                            let mut iter = stream.into_iter();
                            if let Some(proc_macro2::TokenTree::Ident(ident)) = iter.next() {
                                if ident == "into" {
                                    if let Some(proc_macro2::TokenTree::Literal(lit)) = iter.next()
                                    {
                                        let lit_str =
                                            syn::LitStr::new(&lit.to_string(), lit.span());
                                        Some(
                                            lit_str
                                                .parse::<syn::Type>()
                                                .expect("Failed to parse type"),
                                        )
                                    } else {
                                        panic!("Expected a string literal for 'into' attribute");
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }),
                    Meta::NameValue(name_value) => {
                        if name_value.path.is_ident("into") {
                            if let Expr::Lit(lit) = name_value.value {
                                if let Lit::Str(lit_str) = lit.lit {
                                    Some(
                                        lit_str.parse::<syn::Type>().expect("Failed to parse type"),
                                    )
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                }
            } else {
                None
            }
        });

        if let Some(into_ty) = conversion {
            if let syn::Type::Path(type_path) = &into_ty {
                if let Some(segment) = type_path.path.segments.first() {
                    if segment.ident == "Vec" {
                        if let syn::PathArguments::AngleBracketed(angle_bracketed) =
                            &segment.arguments
                        {
                            let inner_ty = angle_bracketed.args.first().unwrap();
                            intermediate_fields.push(quote! { #ident: Vec<#inner_ty<'a>> });
                            field_conversions.push(quote! { #ident: original.#ident.iter().map(|x| x.into()).collect() });
                            let ident = field.ident.as_ref().expect("Expected named field");
                            field_mapping.push(quote! {
                            (self.#ident.len() as u64).encode(encoder)?;
                            for item in self.#ident.iter() {
                                item.encode(encoder) ? ;
                            } });
                        } else {
                            panic!("Expected a type inside the vector");
                        }
                    } else {
                        intermediate_fields.push(quote! { #ident: std::borrow::Cow<'a, #ty> });
                        field_conversions.push(quote! { #ident: std::borrow::Cow::<'a, #into_ty>::from(&original.#ident).into() });
                        let ident = field.ident.as_ref().expect("Expected named field");
                        field_mapping.push(quote! { self.#ident.encode(encoder)?; });
                    }
                } else {
                    intermediate_fields.push(quote! { #ident: std::borrow::Cow<'a, #ty> });
                    field_conversions
                        .push(quote! { #ident: std::borrow::Cow::Borrowed(&original.#ident) });
                    let ident = field.ident.as_ref().expect("Expected named field");
                    field_mapping.push(quote! { self.#ident.encode(encoder)?; });
                }
            } else {
                intermediate_fields.push(quote! { #ident: std::borrow::Cow<'a, #ty> });
                field_conversions
                    .push(quote! { #ident: std::borrow::Cow::Borrowed(&original.#ident) });
                let ident = field.ident.as_ref().expect("Expected named field");
                field_mapping.push(quote! { self.#ident.encode(encoder)?; });
            }
        } else {
            intermediate_fields.push(quote! { #ident: std::borrow::Cow<'a, #ty> });
            field_conversions.push(quote! { #ident: std::borrow::Cow::Borrowed(&original.#ident) });
            let ident = field.ident.as_ref().expect("Expected named field");
            field_mapping.push(quote! { self.#ident.encode(encoder)?; });
        }
    }

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        #[derive(Debug, Clone)]
        pub struct #intermediate_name<'a> #impl_generics {
            #( #intermediate_fields, )*
        }

        impl #impl_generics <'a> bincode::Encode for #intermediate_name<'a> #ty_generics #where_clause {
            fn encode<E>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError>
            where
                E: bincode::enc::Encoder,
            {

                #(#field_mapping)*
                Ok(())
            }
        }

        impl #impl_generics <'a> From<&'a #name #ty_generics> for #intermediate_name<'a> #ty_generics #where_clause {
            fn from(original: &'a #name #ty_generics) -> Self {
                #intermediate_name {
                    #( #field_conversions, )*
                }
            }
        }

        impl #impl_generics Signable for #name #ty_generics #where_clause {
            fn signable_bytes(&self) -> Result<Vec<u8>, #error_type> {
                let config = config::standard().with_big_endian();

                let intermediate : #intermediate_name = self.into();

                bincode::encode_to_vec(intermediate, config).map_err(|e| {
                    #error_type::PlatformSerializationError(format!("unable to serialize to produce sig hash {}: {}", stringify!(#name), e))
                })
            }
        }
    };

    TokenStream::from(expanded)
}
