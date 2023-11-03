use heck::AsSnakeCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Ident};

/// Implement versioning on gRPC responses
///
/// This adds implementation of [dapi_grpc::VersionedGrpcResponse] to the message:
///
/// * impl [VersionedGrpcResponse](::platform_version::VersionedGrpcResponse) for ResponseName
///
/// where `ResponseName` is the name of the object on which the derive is declared.
///
/// ## Requirements
///
/// The response must be versioned and contain proof and metadata fields.
#[proc_macro_derive(VersionedGrpcResponse, attributes(grpc_versions))]
pub fn versioned_grpc_response_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Extract attributes to find the number of versions
    let versions: usize = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("grpc_versions") {
                // Parse the attribute into a literal integer
                attr.parse_args::<syn::LitInt>()
                    .ok()
                    .and_then(|lit| lit.base10_parse().ok())
            } else {
                None
            }
        })
        .expect("Expected a grpc_versions attribute with an integer");

    let name = input.ident;
    // Generate the names of the nested message and enum types
    let mod_name = AsSnakeCase(name.to_string()).to_string();
    let mod_ident = syn::parse_str::<Ident>(&mod_name).expect("parse response ident");

    // Generate match arms for proof and metadata methods
    let proof_arms = (0..=versions).map(|version| {
        let version_ident = format_ident!("V{}", version);
        // Construct the identifier string for the module
        let version_mod_str = format!("{}_v{}", mod_ident, version);
        // Now create an identifier from the constructed string
        let version_mod_ident = format_ident!("{}", version_mod_str);
        quote! {
            #mod_ident::Version::#version_ident(inner) => match &inner.result {
                Some(#mod_ident::#version_mod_ident::Result::Proof(proof)) => Ok(proof),
                _ => return Err(::platform_version::error::PlatformVersionError::UnknownVersionError("unknown proof version not known".to_string())),
            },
        }
    });

    let metadata_arms = (0..=versions).map(|version| {
        let version_ident = format_ident!("V{}", version);
        quote! {
            #mod_ident::Version::#version_ident(inner) => inner.metadata.as_ref().ok_or(platform_version::error::PlatformVersionError::
                UnknownVersionError("result did not have metadata".to_string())),
        }
    });

    // Generate the implementation
    let expanded = quote! {
        impl crate::platform::VersionedGrpcResponse for #name {
            type Error = ::platform_version::error::PlatformVersionError;

            fn proof(&self) -> Result<&Proof, Self::Error> {
                match &self.version {
                    Some(version) => match version {
                        #( #proof_arms )*
                    },
                    _ => Err(::platform_version::error::PlatformVersionError::UnknownVersionError("result did not have a version".to_string())),
                }
            }

            fn metadata(&self) -> Result<&ResponseMetadata, Self::Error> {
                match &self.version {
                    Some(version) => match version {
                        #( #metadata_arms )*
                    },
                    None =>  Err(::platform_version::error::PlatformVersionError::UnknownVersionError("result did not have a version".to_string())),
                }
            }
        }
    };

    // println!("Expanded code: {}", expanded);

    // Return the generated code
    TokenStream::from(expanded)
}
