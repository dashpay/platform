extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Fields, Ident, Lit, LitInt, LitStr, Meta, Type,
};

/// `#[proc_macro_derive(PlatformSerdeVersionedDeserialize, attributes(versioned, platform_serde_versioned))]`
///
/// A procedural macro that generates an implementation of the `serde::Deserialize` trait
/// for a versioned enum, allowing it to be deserialized from JSON or other formats supported by Serde.
///
/// This macro expects the enum to be annotated with a `#[versioned]` attribute on each variant.
/// The attribute should contain an integer value that corresponds to the version of that variant.
/// For example:
/// ```rust
/// #[derive(PlatformSerdeVersionedDeserialize)]
/// pub enum MyEnum {
///     #[versioned(1)]
///     V1 { /* fields go here */ },
///     #[versioned(2)]
///     V2 { /* fields go here */ },
///     // and so on
/// }
/// ```
///
/// The `deserialize` function that the macro generates expects to receive a JSON object (or equivalent in other formats)
/// with a field specified by the `#[platform_serde_versioned(version_field = "name")]` attribute on the enum, where "name"
/// is the desired version field name. By default, this field is `$version`.
///
/// The version field's value should match one of the variant's `#[versioned]` attribute values,
/// and the remainder of the object should match the structure of that variant.
///
/// If the version field is missing or doesn't match any of the variant versions, the `deserialize` function will return an error.
///
/// # Params
///
/// * `input: TokenStream`: A TokenStream that contains the enum definition.
///
/// # Returns
///
/// * `TokenStream`: A TokenStream containing the generated `serde::Deserialize` implementation for the enum.
///
/// # Panics
///
/// * If the input does not represent an enum.
/// * If any of the enum's variants lacks a `#[versioned]` attribute.
/// * If any `#[versioned]` attribute is not a name-value attribute.
/// * If the value of a `#[versioned]` attribute is not an integer.
///
/// # Example
/// ```rust
/// #[derive(PlatformSerdeVersionedDeserialize)]
/// #[platform_serde_versioned(version_field = "structure_version")]
/// pub enum MyEnum {
///     #[versioned(1)]
///     V1 { /* fields go here */ },
///     #[versioned(2)]
///     V2 { /* fields go here */ },
///     // and so on
/// }
///
/// let json_str = r#"{
///   "structure_version": 2,
///   /* fields for the V2 variant go here */
/// }"#;
///
/// let my_enum: MyEnum = serde_json::from_str(json_str).unwrap();
///
/// match my_enum {
///     MyEnum::V1 { /* fields go here */ } => {
///         // Handle the V1 variant
///     },
///     MyEnum::V2 { /* fields go here */ } => {
///         // Handle the V2 variant
///     },
///     // and so on
/// }
/// ```
///
/// Note that in the example, we use `#[platform_serde_versioned(version_field = "structure_version")]` to specify a custom version field name.
/// If the `platform_serde_versioned` attribute is not present, the default version field name will be `$version`.
#[proc_macro_derive(
    PlatformSerdeVersionedDeserialize,
    attributes(
        versioned,
        platform_serde_versioned,
        platform_serialize,
        platform_version_path,
        platform_version_path_bounds
    )
)]
pub fn derive_platform_versioned_deserialize(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident, data, attrs, ..
    } = parse_macro_input!(input);
    let name_str = ident.to_string();

    let (path, is_bounds) = parse_path(&attrs).expect("expected a path");

    let path_tokens: proc_macro2::TokenStream = {
        let mut tokens = proc_macro2::TokenStream::new();
        for (i, ident) in path.iter().enumerate() {
            if i != 0 {
                tokens.extend(quote! { . });
            }
            tokens.extend(quote! { #ident });
        }
        tokens
    };

    let mut version_field_name = String::from("$version");

    if let Some(platform_serde_versioned_attr) = attrs
        .iter()
        .find(|attr| attr.path().is_ident("platform_serde_versioned"))
    {
        platform_serde_versioned_attr
            .parse_nested_meta(|meta| {
                if meta.path.is_ident("version_field") {
                    let value = meta.value()?;
                    let parsed_version_field: LitStr = value.parse()?;
                    version_field_name = parsed_version_field.value();
                } else {
                    return Err(meta.error(
                        format!("unsupported parameter {:?}", meta.path.get_ident()).as_str(),
                    ));
                }
                Ok(())
            })
            .expect("Expected to parse nested meta");
    }

    let variants = match data {
        Data::Enum(e) => e.variants,
        _ => panic!("PlatformSerdeVersionedDeserialize can only be used with enums"),
    };

    let arms = variants.into_iter().map(|v| {
        let variant_ident = &v.ident;
        let attrs = &v.attrs;
        let version_attr = attrs.iter().find(|attr| attr.path().is_ident("versioned")).expect("Each variant must have a #[versioned] attribute");
        let lit_int: LitInt = version_attr.parse_args().expect("Each #[versioned] attribute must be a name-value attribute");
        let version = lit_int.base10_parse::<u16>().expect("The #[versioned] attribute must have an integer value");
        let variant_ident_str = format!("{}V{}",name_str, version);
        let variant_ident_sub = Ident::new(&variant_ident_str, v.ident.span());

        quote! {
            #version => {
                let value = #variant_ident_sub::from_object(map_clone).map_err(|e|serde::de::Error::custom(e.to_string()))?;
                Ok(#ident::#variant_ident(value))
            }
        }
    }).collect::<Vec<_>>();

    let version_check = if is_bounds {
        quote! {
            current_platform_version.#path_tokens.check_version(version)
        }
    } else {
        quote! {
            current_platform_version.#path_tokens != version
        }
    };

    let output = quote! {
        impl<'de> ::serde::de::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                struct MapVisitor;

                impl<'de> ::serde::de::Visitor<'de> for MapVisitor {
                    type Value = #ident;

                    fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        formatter.write_str("enum ");
                        formatter.write_str(#name_str)
                    }

                    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                    where
                        A: ::serde::de::MapAccess<'de>,
                    {
                        let mut map_clone = platform_value::Value::Map(platform_value::ValueMap::new());
                        while let Some((key, value)) = map.next_entry::<String, platform_value::Value>()? {
                            map_clone.insert(key, value).expect("expected a value map");
                        }
                        let version: crate::version::FeatureVersion = map_clone.get_integer(#version_field_name).map_err(|_|serde::de::Error::missing_field(#version_field_name))?;
                        let current_platform_version = crate::version::PlatformVersion::get_current().map_err(|e|serde::de::Error::custom(e.to_string()))?;
                       if #version_check {
                        return Err(::serde::de::Error::custom("Invalid version value"));
                    }
                        match version {
                            #(#arms,)*
                            _ => Err(::serde::de::Error::custom("Invalid version value")),
                        }
                    }
                }

                deserializer.deserialize_map(MapVisitor)
            }
        }
    };

    eprintln!(
        "Processing variant for platform version deserialize: {}",
        &output
    );

    TokenStream::from(output)
}
//
// #[proc_macro_derive(PlatformSerdeVersioned)]
// pub fn derive_platform_versions(input: TokenStream) -> TokenStream {
//     let input = parse_macro_input!(input as DeriveInput);
//
//     let name = &input.ident;
//     let name_str = name.to_string();
//     let data_enum = match &input.data {
//         Data::Enum(data_enum) => data_enum,
//         _ => panic!("PlatformSerdeVersioned can only be used with enums"),
//     };
//
//     let variant_idents: Vec<&Ident> = data_enum
//         .variants
//         .iter()
//         .map(|variant| &variant.ident)
//         .collect();
//
//     let variant_types: Vec<Type> = data_enum
//         .variants
//         .iter()
//         .map(|variant| {
//             // assuming the variant is of the form `Variant(Type)`
//             match &variant.fields {
//                 Fields::Unnamed(fields_unnamed) => fields_unnamed.unnamed.first().unwrap().ty.clone(),
//                 _ => panic!("PlatformSerdeVersioned can only be used with enums of the form `Variant(Type)`"),
//             }
//         })
//         .collect();
//
//     let serialize_arms =
//         variant_idents
//             .iter()
//             .zip(variant_types.iter())
//             .map(|(variant_ident, variant_type)| {
//                 let variant_index = variant_ident
//                     .to_string()
//                     .trim_start_matches('V')
//                     .parse::<u16>()
//                     .unwrap();
//                 quote! {
//                     #name::#variant_ident(inner) => {
//                         use ::serde::ser::SerializeMap;
//                         let mut map = serializer.serialize_map(None)?;
//                         map.serialize_entry("$version", &#variant_index)?;
//                         inner.serialize_flattened(&mut map)?;
//                         map.end()
//                     }
//                 }
//             });
//
//     let deserialize_arms =
//         variant_idents
//             .iter()
//             .zip(variant_types.iter())
//             .map(|(variant_ident, variant_type)| {
//                 let variant_index = variant_ident
//                     .to_string()
//                     .trim_start_matches('V')
//                     .parse::<u16>()
//                     .unwrap();
//                 quote! {
//                     #variant_index => {
//                         let inner = <#variant_type as ::serde::Deserialize>::deserialize(&mut map)?;
//                         Ok(#name::#variant_ident(inner))
//                     }
//                 }
//             });
//
//     let output = quote! {
//         impl ::serde::ser::Serialize for #name {
//             fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//             where
//                 S: ::serde::ser::Serializer,
//             {
//                 match self {
//                     #(#serialize_arms),*
//                 }
//             }
//         }
//
//         impl<'de> ::serde::de::Deserialize<'de> for #name {
//             fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//             where
//                 D: ::serde::Deserializer<'de>,
//             {
//                 struct MapVisitor;
//
//                 impl<'de> ::serde::de::Visitor<'de> for MapVisitor {
//                     type Value = #name;
//
//                     fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
//                         formatter.write_str("enum ");
//                         formatter.write_str(#name_str)
//                     }
//
//                     fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
//                     where
//                         A: ::serde::de::MapAccess<'de>,
//                     {
//                         let mut version: Option<u16> = None;
//                         while let Some(key) = map.next_key::<String>()? {
//                             if key == "$version" {
//                                 if version.is_some() {
//                                     return Err(::serde::de::Error::duplicate_field("$version"));
//                                 }
//                                 version = Some(map.next_value()?);
//                             }
//                         }
//                         let version = version.ok_or_else(|| ::serde::de::Error::missing_field("$version"))?;
//                         match version {
//                             #(#deserialize_arms),*
//                             _ => Err(::serde::de::Error::custom("Invalid version value")),
//                         }
//                     }
//                 }
//
//                 deserializer.deserialize_map(MapVisitor)
//             }
//         }
//     };
//
//     TokenStream::from(output)
// }

#[proc_macro_derive(PlatformVersioned, attributes(platform_version_path))]
pub fn derive_platform_versioned(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let path = parse_path(&input.attrs);

    let verify_protocol_version: proc_macro2::TokenStream = if let Some((path, _)) = path {
        let mut tokens = proc_macro2::TokenStream::new();
        for (i, ident) in path.iter().enumerate() {
            if i != 0 {
                tokens.extend(quote! { . });
            }
            tokens.extend(quote! { #ident });
        }
        quote! {
            pub fn verify_protocol_version(&self, protocol_version: u32) -> Result<bool, ProtocolError> {
                let platform_version = crate::version::PlatformVersion::get(protocol_version)?;
                Ok(platform_version.#tokens.check_version(self.version()))
            }
        }
    } else {
        quote! {}
    };

    let data_enum = match &input.data {
        Data::Enum(data_enum) => data_enum,
        _ => panic!("PlatformVersioned can only be used with enums"),
    };

    let variant_idents: Vec<&Ident> = data_enum
        .variants
        .iter()
        .map(|variant| &variant.ident)
        .collect();

    let version_arms = generate_version_arms(&variant_idents);

    let output = quote! {
        impl #name {
            pub fn version(&self) -> crate::version::FeatureVersion {
                match self {
                    #(#version_arms),*
                }
            }

            #verify_protocol_version
        }
    };

    eprintln!("Processing versioning : {}", &output);

    TokenStream::from(output)
}

fn parse_path(attrs: &[Attribute]) -> Option<(Vec<Ident>, bool)> {
    let mut platform_version_path = None::<LitStr>;
    let mut is_bounds = false;
    //
    // if let Some(platform_serialize_attr) = attrs
    //     .iter()
    //     .find(|attr| attr.path().is_ident("platform_serialize"))
    // {
    //     platform_serialize_attr
    //         .parse_nested_meta(|meta| {
    //             if meta.path.is_ident("platform_version_path") {
    //                 let value = meta.value()?;
    //                 platform_version_path = Some(value.parse::<LitStr>()?);
    //             }
    //             Ok(())
    //         })
    //         .expect("expected to parse nested meta");
    // }
    for attr in attrs {
        if attr.path().is_ident("platform_version_path") {
            platform_version_path = attr.parse_args().expect("Failed to parse path");
        } else if attr.path().is_ident("platform_version_path_bounds") {
            platform_version_path = attr.parse_args().expect("Failed to parse path");
            is_bounds = true;
        }
    }

    if let Some(platform_version_path) = platform_version_path {
        let path_string = platform_version_path.value();
        return Some((
            path_string
                .split('.')
                .map(|s| Ident::new(s, Span::call_site()))
                .collect(),
            is_bounds
        ));
    }

    None
}

fn generate_version_arms(variant_idents: &[&Ident]) -> Vec<proc_macro2::TokenStream> {
    variant_idents
        .iter()
        .enumerate()
        .map(|(index, ident)| {
            let index_feature = index as u16;
            quote! {
                Self::#ident(_) => #index_feature
            }
        })
        .collect()
}

fn generate_verify_arms(variant_idents: &[&Ident], path: &LitStr) -> Vec<proc_macro2::TokenStream> {
    variant_idents
        .iter()
        .map(|ident| {
            quote! {
                Self::#ident(_) => protocol_version.#path.check_version(self.version())
            }
        })
        .collect()
}
