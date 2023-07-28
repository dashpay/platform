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
        ..
    } = version_attributes;

    // Extract the generics.
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // let fields = &data_struct.fields;
    // let custom_serialized_field_indices: Vec<_> = fields.iter()
    //     .enumerate()
    //     .filter_map(|(idx, field)| {
    //         if field.attrs.iter().any(|attr| {
    //             attr.path().is_ident("platform_versioned_serialized")
    //         }) {
    //             Some(idx)
    //         } else {
    //             None
    //         }
    //     })
    //     .collect();
    //
    // let field_serializations: Vec<_> = fields.iter().enumerate().map(|(idx, field)| {
    //     let field_name = &field.ident;
    //     if custom_serialized_field_indices.contains(&idx) {
    //         quote! {
    //         let #field_name = #field_name.serialize(platform_version)?;
    //     }
    //     } else {
    //         quote! {
    //         let #field_name = bincode::encode(&self.#field_name)?;
    //     }
    //     }
    // }).collect();
    //
    // let field_serializations_consume: Vec<_> = fields.iter().enumerate().map(|(idx, field)| {
    //     let field_name = &field.ident;
    //     if custom_serialized_field_indices.contains(&idx) {
    //         quote! {
    //         let #field_name = #field_name.serialize(platform_version)?;
    //     }
    //     } else {
    //         quote! {
    //         let #field_name = bincode::encode(&#field_name)?;
    //     }
    //     }
    // }).collect();

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

    let serialize_into = match platform_serialize_into.clone() {
        Some(inner) => quote! {
            let inner: #inner = self.clone().into();
            inner.serialize_with_platform_version(platform_version)
        },
        None => quote! {
                    #config
            platform_serialization::platform_encode_to_vec(self, config, platform_version)#limit_err
        },
    };

    let serialize_into_consume = match platform_serialize_into {
        Some(inner) => quote! {
            let inner: #inner = self.into();
            inner.serialize_consume_with_platform_version(platform_version)
        },
        None => quote! {
                #config
            platform_serialization::platform_encode_to_vec(self, config, platform_version)#limit_err
        },
    };

    let bincode_encode_body = if true {
        crate::derive_bincode::derive_encode_inner(token_stream_input)
            .unwrap_or_else(|e| e.into_token_stream())
            .into()
    } else {
        quote! {}
    };

    let expanded = quote! {
        impl #impl_generics #crate_name::serialization::PlatformSerializableWithPlatformVersion for #name #ty_generics #where_clause
        {
            fn serialize_with_platform_version(&self, platform_version: &#crate_name::version::PlatformVersion) -> Result<Vec<u8>, #error_type> {
                #serialize_into
            }

            fn serialize_consume_with_platform_version(self, platform_version: &#crate_name::version::PlatformVersion) -> Result<Vec<u8>, #error_type> {
                #serialize_into_consume
            }
        }
        #bincode_encode_body
    };

    eprintln!("Processing serialize struct: {}", &expanded);

    TokenStream::from(expanded)
}
