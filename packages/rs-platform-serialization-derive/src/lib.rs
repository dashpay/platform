mod attribute;
mod derive_bincode;
mod derive_bincode_enum;
mod derive_bincode_struct;
mod deserialize_enum;
mod deserialize_struct;
mod serialize_enum;
mod serialize_struct;

extern crate proc_macro;

use crate::deserialize_enum::derive_platform_deserialize_enum;
use crate::deserialize_struct::derive_platform_deserialize_struct;
use crate::serialize_enum::derive_platform_serialize_enum;
use crate::serialize_struct::derive_platform_serialize_struct;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Expr, Lit, LitInt, LitStr, Meta, Path};

struct VersionAttributes {
    crate_name: Ident,
    passthrough: bool,
    platform_serialize_limit: Option<usize>,
    untagged: bool,
    unversioned: bool,
    platform_serialize_into: Option<Path>,
    platform_version_path: Option<LitStr>,
    #[allow(dead_code)] // TODO this is never read
    allow_prepend_version: bool,
    #[allow(dead_code)] // TODO this is never read
    force_prepend_version: bool,
}

impl VersionAttributes {
    fn check_for_enum(&self) {
        if self.platform_serialize_into.is_some() {
            panic!("'into' can not be used for platform versioning of enums")
        }
    }

    fn check_for_struct(&self) {
        if self.passthrough {
            panic!("'passthrough' can not be used for platform versioning of structs")
        }
        if self.untagged {
            panic!("'untagged' can not be used for platform versioning of structs")
        }
    }
}

// TODO serialize_consume and serialize aren't actually adding
/// This proc macro derives the `PlatformSerialize` trait for the input data structure.
/// The derived trait implementation will provide methods to serialize the data into a binary format, with some customization options.
/// The `platform_error_type` attribute specifies the error type to be used in the `Result` types of the serialization methods.
/// The `platform_serialize` attribute specifies optional serialization behaviors, which include the following:
///
/// - `passthrough`: If the attribute is an enum, it serializes the inner data directly, bypassing the enum's own serialization.
///   This means the serialization process is handled by the inner data itself rather than the enum.
///
///   If `MyEnum::Variant1(inner)` is serialized, it will call `inner.serialize()` directly instead of `MyEnum::Variant1(inner).serialize()`.
///
/// - `untagged`: If the attribute is an enum, it makes the enum untagged. This means the enum variants are serialized directly without their variant names or prefix number.
///   Unlike `passthrough`, `untagged` still uses the enum's serialization method, but it does not include the variant name in the serialized data.
///   This is useful for cases where the variant names are not necessary or desired in the serialized output.
///
///   Note: `passthrough` and `untagged` have different use-cases and cannot be used together. If an enum is marked with `passthrough`, its inner types handle the serialization.
///   If it's marked with `untagged`, the enum itself handles the serialization, but without including variant names.
///
/// - `limit`: This sets a maximum limit on the serialized size. The value of `limit` should be a number.
/// - `into`: This attribute is used to specify a conversion before serialization. The value of `into` should be the path of the target type. The input data will be converted into this type before serialization. This can only be used on structs.
/// - `derive_bincode`: If this attribute is set, we will automatically derive bincode encode. If passthrough will encode the inner variant
/// - `allow_prepend_version`: If this attribute is set, we allow serialization with version prefix and without.
/// - `force_prepend_version`: If this attribute is set, we allow serialization only with version prefix.
///
/// Note that the `passthrough` attribute cannot be used with any other attribute except derive_bincode.
///
/// The derived trait will include these methods:
///
/// - `serialize`: This method serializes the data into a `Vec<u8>`. If the `limit` is specified, it will enforce the limit on the serialized size.
/// - `serialize_consume`: This method is similar to `serialize`, but it takes `self` by value.
///
/// The implementation uses the `bincode` library for serialization. The configuration of `bincode` is set to use big endian and to limit the size according to the `limit` attribute. If the `limit` attribute is not set, the size is unlimited.
///
/// Errors from the `bincode` library are converted into the specified platform error type. If a size limit is exceeded, the error will be `MaxEncodedBytesReachedError`.
///
/// This macro is intended to be used for platform-specific serialization where it is necessary to control the serialization process more closely than what is provided by the standard `Serialize` trait.
#[proc_macro_derive(PlatformSerialize, attributes(platform_error_type, platform_serialize))]
pub fn derive_platform_serialize(input: TokenStream) -> TokenStream {
    let cloned_input = input.clone();

    let derive_input = parse_macro_input!(input as DeriveInput);
    let name = &derive_input.ident;

    // Extract the error type from the attribute.
    let error_type = derive_input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("platform_error_type") {
                Some(attr.parse_args::<syn::Path>().unwrap())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            syn::parse_str::<syn::Path>("ProtocolError")
                .expect("Failed to parse default error type")
        });

    let mut passthrough = false;
    let mut platform_serialize_limit = None;
    let mut untagged = false;
    let mut unversioned = false;
    let mut platform_serialize_into = None;
    let mut platform_version_path = None;
    let mut crate_name: Ident = Ident::new("crate", Span::call_site()); // default value is "crate"
    let mut allow_prepend_version = false;
    let mut force_prepend_version = false;

    if let Some(platform_serialize_attr) = derive_input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("platform_serialize"))
    {
        platform_serialize_attr
            .parse_nested_meta(|meta| {
                if meta.path.is_ident("crate_name") {
                    let value = meta.value()?;
                    let crate_name_str: LitStr = value.parse()?;
                    crate_name = syn::parse_str(&crate_name_str.value()).unwrap();
                } else if meta.path.is_ident("passthrough") {
                    passthrough = true;
                } else if meta.path.is_ident("unversioned") {
                    unversioned = true;
                } else if meta.path.is_ident("allow_prepend_version") {
                    allow_prepend_version = true;
                } else if meta.path.is_ident("force_prepend_version") {
                    force_prepend_version = true;
                } else if meta.path.is_ident("limit") {
                    let value = meta.value()?;
                    let parsed_limit: LitInt = value.parse()?;
                    platform_serialize_limit = Some(
                        parsed_limit
                            .base10_parse::<usize>()
                            .expect("Expected a number for 'limit'"),
                    );
                } else if meta.path.is_ident("untagged") {
                    untagged = true;
                } else if meta.path.is_ident("into") {
                    let value = meta.value()?;
                    let parsed_into: LitStr = value.parse()?;
                    platform_serialize_into = Some(
                        parsed_into
                            .parse::<syn::Path>()
                            .expect("Expected a valid path for 'into'"),
                    );
                } else if meta.path.is_ident("platform_version_path") {
                    let value = meta.value()?;
                    platform_version_path = Some(value.parse()?);
                } else {
                    return Err(meta.error(
                        format!("unsupported parameter {:?}", meta.path.get_ident()).as_str(),
                    ));
                }
                Ok(())
            })
            .expect("expected to parse nested meta");
    }

    if passthrough
        && (platform_serialize_limit.is_some() || untagged || platform_serialize_into.is_some())
    {
        panic!("The 'passthrough' attribute cannot be used with untagged, limit or into");
    }

    if force_prepend_version && allow_prepend_version {
        panic!("The 'allow_prepend_version' attribute cannot be used with 'force_prepend_version', only one should be chosen");
    }

    let version_attributes = VersionAttributes {
        crate_name,
        passthrough,
        platform_serialize_limit,
        untagged,
        unversioned,
        platform_serialize_into,
        platform_version_path,
        allow_prepend_version,
        force_prepend_version,
    };

    match &derive_input.data {
        Data::Struct(data_struct) => {
            version_attributes.check_for_struct();
            derive_platform_serialize_struct(
                cloned_input,
                &derive_input,
                version_attributes,
                data_struct,
                error_type,
                name,
            )
        }
        Data::Enum(data_enum) => {
            version_attributes.check_for_enum();
            derive_platform_serialize_enum(
                cloned_input,
                &derive_input,
                version_attributes,
                data_enum,
                error_type,
                name,
            )
        }
        _ => {
            panic!("can only derive serialize on a struct or an enum")
        }
    }
}

#[proc_macro_derive(
    PlatformDeserialize,
    attributes(platform_error_type, platform_serialize)
)]
pub fn derive_platform_deserialize(input: TokenStream) -> TokenStream {
    let cloned_token_stream_input = input.clone();

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
        .unwrap_or_else(|| {
            syn::parse_str::<syn::Path>("ProtocolError")
                .expect("Failed to parse default error type")
        });

    let mut passthrough = false;
    let mut unversioned = false;
    let mut platform_serialize_limit = None;
    let mut untagged = false;
    let mut platform_serialize_into = None;
    let mut platform_version_path = None;
    let mut crate_name: Ident = Ident::new("crate", Span::call_site()); // default value is "crate"
    let mut allow_prepend_version = false;
    let mut force_prepend_version = false;

    if let Some(platform_serialize_attr) = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("platform_serialize"))
    {
        platform_serialize_attr
            .parse_nested_meta(|meta| {
                if meta.path.is_ident("crate_name") {
                    let value = meta.value()?;
                    let crate_name_str: LitStr = value.parse()?;
                    crate_name = syn::parse_str(&crate_name_str.value()).unwrap();
                } else if meta.path.is_ident("passthrough") {
                    passthrough = true;
                } else if meta.path.is_ident("unversioned") {
                    unversioned = true;
                } else if meta.path.is_ident("allow_prepend_version") {
                    allow_prepend_version = true;
                } else if meta.path.is_ident("force_prepend_version") {
                    force_prepend_version = true;
                } else if meta.path.is_ident("limit") {
                    let value = meta.value()?;
                    let parsed_limit: LitInt = value.parse()?;
                    platform_serialize_limit = Some(
                        parsed_limit
                            .base10_parse::<usize>()
                            .expect("Expected a number for 'limit'"),
                    );
                } else if meta.path.is_ident("untagged") {
                    untagged = true;
                } else if meta.path.is_ident("into") {
                    let value = meta.value()?;
                    let parsed_into: LitStr = value.parse()?;
                    platform_serialize_into = Some(
                        parsed_into
                            .parse::<syn::Path>()
                            .expect("Expected a valid path for 'into'"),
                    );
                } else if meta.path.is_ident("platform_version_path") {
                    let value = meta.value()?;
                    platform_version_path = Some(value.parse()?);
                } else {
                    return Err(meta.error(
                        format!("unsupported parameter {:?}", meta.path.get_ident()).as_str(),
                    ));
                }
                Ok(())
            })
            .expect("expected to parse nested meta");
    }

    if passthrough
        && (platform_serialize_limit.is_some() || untagged || platform_serialize_into.is_some())
    {
        panic!("The 'passthrough' attribute cannot be used with untagged, limit or into");
    }

    if force_prepend_version && allow_prepend_version {
        panic!("The 'allow_prepend_version' attribute cannot be used with 'force_prepend_version', only one should be chosen");
    }

    let version_attributes = VersionAttributes {
        crate_name,
        passthrough,
        platform_serialize_limit,
        untagged,
        unversioned,
        platform_serialize_into,
        platform_version_path,
        allow_prepend_version,
        force_prepend_version,
    };

    match &input.data {
        Data::Struct(data_struct) => {
            version_attributes.check_for_struct();
            derive_platform_deserialize_struct(
                cloned_token_stream_input,
                &input,
                version_attributes,
                data_struct,
                error_type,
                name,
            )
        }
        Data::Enum(data_enum) => {
            version_attributes.check_for_enum();
            derive_platform_deserialize_enum(
                cloned_token_stream_input,
                &input,
                version_attributes,
                data_enum,
                error_type,
                name,
            )
        }
        _ => {
            panic!("can only derive serialize on a struct or an enum")
        }
    }
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
        .unwrap_or_else(|| {
            syn::parse_str::<syn::Path>("ProtocolError")
                .expect("Failed to parse default error type")
        });

    let mut derive_into = false;
    let mut derive_bincode_with_borrowed_vec = false;

    if let Some(platform_serialize_attr) = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("platform_signable"))
    {
        platform_serialize_attr
            .parse_nested_meta(|meta| {
                if meta.path.is_ident("derive_into") {
                    derive_into = true;
                } else if meta.path.is_ident("derive_bincode_with_borrowed_vec") {
                    derive_bincode_with_borrowed_vec = true;
                }
                Ok(())
            })
            .expect("expected to parse nested meta");
    }

    let expanded = match &input.data {
        Data::Struct(data) => {
            let fields = &data.fields;

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
                                intermediate_fields
                                    .push(quote! { #ident: std::borrow::Cow<'a, #ty> });
                                field_conversions.push(quote! { #ident: std::borrow::Cow::<'a, #into_ty>::from(&original.#ident).into() });
                                let ident = field.ident.as_ref().expect("Expected named field");
                                field_mapping.push(quote! { self.#ident.encode(encoder)?; });
                            }
                        } else {
                            intermediate_fields.push(quote! { #ident: std::borrow::Cow<'a, #ty> });
                            field_conversions.push(
                                quote! { #ident: std::borrow::Cow::Borrowed(&original.#ident) },
                            );
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
            }

            let generics = &input.generics;
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

            let (auto_derive_bincode, manually_derive_bincode) = if derive_bincode_with_borrowed_vec
            {
                (
                    quote! {},
                    quote! {
                                            impl #impl_generics <'a> bincode::Encode for #intermediate_name<'a> #ty_generics #where_clause {
                            fn encode<E>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError>
                            where
                            E: bincode::enc::Encoder,
                            {

                            #(#field_mapping)*
                            Ok(())
                        }
                    }
                            },
                )
            } else {
                (quote! {bincode::Encode}, quote! {})
            };

            quote! {
                #[derive(Debug, Clone, #auto_derive_bincode)]
                pub struct #intermediate_name<'a> #impl_generics {
                    #( #intermediate_fields, )*
                }

                impl #impl_generics <'a> From<&'a #name #ty_generics> for #intermediate_name<'a> #ty_generics #where_clause {
                    fn from(original: &'a #name #ty_generics) -> Self {
                        #intermediate_name {
                            #( #field_conversions, )*
                        }
                    }
                }

                #manually_derive_bincode

                impl #impl_generics crate::serialization::Signable for #name #ty_generics #where_clause {
                    fn signable_bytes(&self) -> Result<Vec<u8>, #error_type> {
                        let config = bincode::config::standard().with_big_endian();

                        let intermediate : #intermediate_name = self.into();

                        bincode::encode_to_vec(intermediate, config).map_err(|e| {
                            #error_type::PlatformSerializationError(format!("unable to serialize to produce sig hash {}: {}", stringify!(#name), e))
                        })
                    }
                }
            }
        }
        Data::Enum(data) => {
            let intermediate_name = syn::Ident::new(&format!("{}Signable", name), name.span());

            let transformed_variants = data
                .variants
                .iter()
                .map(|variant| {
                    let variant_ident = &variant.ident;
                    match &variant.fields {
                        syn::Fields::Unnamed(fields) => {
                            let transformed_fields: Vec<_> = fields
                                .unnamed
                                .iter()
                                .map(|field| {
                                    if let syn::Type::Path(type_path) = &field.ty {
                                        let last_segment = type_path.path.segments.last().unwrap();
                                        let new_name = format!("{}Signable", last_segment.ident);
                                        let new_ident =
                                            syn::Ident::new(&new_name, last_segment.ident.span());
                                        quote! { #new_ident<'a> }
                                    } else {
                                        panic!("Unsupported field type in enum variant");
                                    }
                                })
                                .collect::<Vec<_>>();

                            quote! {
                                #variant_ident( #( #transformed_fields ),* )
                            }
                        }
                        _ => panic!("Only tuple-style enum variants are supported"),
                    }
                })
                .collect::<Vec<_>>();

            let variants = &data.variants;

            let variant_arms = variants.iter().enumerate().map(|(i, variant)| {
                let variant_ident = &variant.ident;
                let variant_fields = match &variant.fields {
                    syn::Fields::Unnamed(fields) => fields.unnamed.iter().collect::<Vec<_>>(),
                    _ => panic!("Only tuple-style enum variants are supported"),
                };

                if variant_fields.len() != 1 {
                    panic!("Each enum variant must contain exactly one field");
                }

                quote! {
                    #name::#variant_ident(ref inner) => {
                        let mut buf = bincode::encode_to_vec(&(#i as u16), config).unwrap();
                        let inner_signable_bytes = inner.signable_bytes()?;
                        buf.extend(inner_signable_bytes);
                        buf
                    }
                }
            });

            let generics = &input.generics;
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

            let derive_into_clause = if derive_into {
                let variant_into = variants.iter().map(|variant| {
                    let variant_ident = &variant.ident;
                    let variant_fields = match &variant.fields {
                        syn::Fields::Unnamed(fields) => fields.unnamed.iter().collect::<Vec<_>>(),
                        _ => panic!("Only tuple-style enum variants are supported"),
                    };

                    if variant_fields.len() != 1 {
                        panic!("Each enum variant must contain exactly one field");
                    }

                    let intermediate_ident = syn::Ident::new(
                        &format!("{}{}Signable", name, variant_ident),
                        variant_ident.span(),
                    );

                    quote! {
                        #name::#variant_ident(inner) => {
                            let signable  : #intermediate_ident = inner.into();
                            signable.into()
                        }
                    }
                });
                quote! {
                                    impl #impl_generics <'a> From<&'a #name #ty_generics> for #intermediate_name<'a> #ty_generics #where_clause {
                    fn from(original: &'a #name #ty_generics) -> Self {
                        match original {
                            #( #variant_into, )*
                        }
                    }
                }
                }
            } else {
                quote! {}
            };

            quote! {
                #[derive(Debug, Clone, bincode::Encode, derive_more::From)]
                pub enum #intermediate_name<'a> #impl_generics {
                    #( #transformed_variants, )*
                }

                #derive_into_clause

                impl #impl_generics crate::serialization::Signable for #name #ty_generics #where_clause {
                    fn signable_bytes(&self) -> Result<Vec<u8>, #error_type> {
                        let config = bincode::config::standard().with_big_endian();

                        let signable_bytes = match self {
                            #( #variant_arms, )*
                        };

                        Ok(signable_bytes)
                    }
                }
            }
        }
        Data::Union(_) => panic!("PlatformSignable cannot be derived for unions"),
    };

    // eprintln!("Signable variant: {}", &expanded);

    TokenStream::from(expanded)
}
