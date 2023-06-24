extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Ident, Lit, Meta};

#[proc_macro_derive(PlatformSerdeVersioned)]
pub fn derive_platform_versions(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let data_enum = match &input.data {
        Data::Enum(data_enum) => data_enum,
        _ => panic!("PlatformSerdeVersioned can only be used with enums"),
    };

    let variant_idents: Vec<&Ident> = data_enum
        .variants
        .iter()
        .map(|variant| &variant.ident)
        .collect();

    let serialize_arms = generate_serialize_arms(&variant_idents);
    let deserialize_arms = generate_deserialize_arms(&variant_idents);

    let output = quote! {
            impl ::serde::ser::Serialize for #name {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: ::serde::ser::Serializer,
                {
                    use ::serde::ser::SerializeStruct;

                    let mut state = serializer.serialize_struct(stringify!(#name), 2)?;
                    match self {
                        #(#serialize_arms),*
                    };
                    state.end()
                }
            }

            impl<'de> ::serde::Deserialize<'de> for #name {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            enum Field {
                Version,
            }

            impl<'de> ::serde::de::Deserialize<'de> for Field {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: ::serde::Deserializer<'de>,
                {
                    struct FieldVisitor;

                    impl<'de> ::serde::de::Visitor<'de> for FieldVisitor {
                        type Value = Field;

                        fn expecting(
                            &self,
                            formatter: &mut ::std::fmt::Formatter,
                        ) -> ::std::fmt::Result {
                            formatter.write_str("field identifier")
                        }

                        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                        where
                            E: ::serde::de::Error,
                        {
                            match value {
                                "$version" => Ok(Field::Version),
                                _ => Err(::serde::de::Error::unknown_field(value, FIELDS)),
                            }
                        }
                    }

                    deserializer.deserialize_identifier(FieldVisitor)
                }
            }

            struct NameVisitor;

            impl<'de> ::serde::de::Visitor<'de> for NameVisitor {
                type Value = #name;

                fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    formatter.write_str("struct DataContract")
                }

                fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: ::serde::de::MapAccess<'de>,
                {
                    let mut version: Option<u16> = None;
                    while let Some(key) = map.next_key()? {
                        match key {
                            Field::Version => {
                                if version.is_some() {
                                    return Err(::serde::de::Error::duplicate_field("$version"));
                                }
                                version = Some(map.next_value()?);
                            }
                        }
                    }
                    let version = version.ok_or_else(|| ::serde::de::Error::missing_field("$version"))?;
                    match version {
                        #(#deserialize_arms),*
                        _ => Err(::serde::de::Error::custom("Invalid version value")),
                    }
                }
            }

            const FIELDS: &'static [&'static str] = &["$version"];
            deserializer.deserialize_struct(stringify!(#name), FIELDS, NameVisitor)
        }
    }
    };
    TokenStream::from(output)
}

fn generate_serialize_arms(variant_idents: &[&Ident]) -> Vec<proc_macro2::TokenStream> {
    variant_idents
        .iter()
        .enumerate()
        .map(|(index, ident)| {
            let index_u16 = index as u16;
            quote! {
                Self::#ident(inner) => {
                    state.serialize_field("$version", &#index_u16)?;
                    state.serialize_field(stringify!(#ident), inner)?;
                }
            }
        })
        .collect()
}

fn generate_deserialize_arms(variant_idents: &[&Ident]) -> Vec<proc_macro2::TokenStream> {
    variant_idents
        .iter()
        .enumerate()
        .map(|(index, ident)| {
            let index_u16 = index as u16;
            quote! {
                #index_u16 => {
                    let inner: #ident = map.next_value()?;
                    Ok(Self::#ident(inner))
                }
            }
        })
        .collect()
}

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
