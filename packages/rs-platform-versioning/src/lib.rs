extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Ident, LitStr};
#[proc_macro_derive(
    PlatformVersioned,
    attributes(platform_version_path, platform_version_path_bounds)
)]
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
                Ok(platform_version.#tokens.check_version(self.feature_version()))
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
            pub fn feature_version(&self) -> crate::version::FeatureVersion {
                match self {
                    #(#version_arms),*
                }
            }

            #verify_protocol_version
        }
    };

    // eprintln!("Processing versioning : {}", &output);

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
            is_bounds,
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
