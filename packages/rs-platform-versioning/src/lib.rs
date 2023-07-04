extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Fields, Ident, Lit, LitInt, Meta, Type,
};
#[proc_macro_derive(PlatformSerdeVersionedDeserialize, attributes(versioned))]
pub fn derive_platform_versioned_deserialize(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
    let name_str = ident.to_string();

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
                let value = <#variant_ident_sub as ::serde::Deserialize>::deserialize(::serde_json::Value::Object(map))?;
                Ok(#ident::#variant_ident(value))
            }
        }
    }).collect::<Vec<_>>();

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
                        // Make a copy of the whole map
                        let mut map_clone = ::std::collections::HashMap::new();
                        while let Some((key, value)) = map.next_entry::<String, ::serde_json::Value>()? {
                            map_clone.insert(key, value);
                        }

                        let version = match map_clone.get("$version") {
                            Some(::serde_json::Value::Number(n)) if n.is_u64() => n.as_u64().unwrap() as u16,
                            _ => return Err(::serde::de::Error::missing_field("$version")),
                        };

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

    eprintln!("Processing variant: {}", &output);

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
    let verify_arms = generate_verify_arms(&variant_idents, &path);

    let output = quote! {
        impl #name {
            pub fn version(&self) -> FeatureVersion {
                match self {
                    #(#version_arms),*
                }
            }

            pub fn verify_protocol_version(&self, protocol_version: u32) -> Result<bool, ProtocolError> {
                let protocol_version = PlatformVersion::get(protocol_version)?;
                Ok(protocol_version.#path.check_version(self.version()))
            }
        }
    };

    TokenStream::from(output)
}

fn parse_path(attrs: &[Attribute]) -> proc_macro2::TokenStream {
    for attr in attrs {
        if attr.path().is_ident("platform_version_path") {
            let path: syn::Path = attr.parse_args().expect("Failed to parse path");
            return quote! { #path };
        }
    }
    panic!("platform_version_path attribute not found");
}

fn generate_version_arms(variant_idents: &[&Ident]) -> Vec<proc_macro2::TokenStream> {
    variant_idents
        .iter()
        .enumerate()
        .map(|(index, ident)| {
            let index_feature = index as u32;
            quote! {
                Self::#ident(_) => #index_feature
            }
        })
        .collect()
}

fn generate_verify_arms(
    variant_idents: &[&Ident],
    path: &proc_macro2::TokenStream,
) -> Vec<proc_macro2::TokenStream> {
    variant_idents
        .iter()
        .map(|ident| {
            quote! {
                Self::#ident(_) => protocol_version.#path.check_version(self.version())
            }
        })
        .collect()
}
