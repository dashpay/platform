use heck::AsSnakeCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Ident};

/// Versioned gRPC message derive macro
///
/// This adds implementation of [dapi_grpc::VersionedGrpcMessage] to the message.
/// It should be implemented on all gRPC requests and responses that are versioned.
///
/// It uses the `grpc_versions` attribute to determine implemented versions.
///
/// ## Requirements
///
/// * `crate::platform::VersionedGrpcMessage` must be in scope
///
#[proc_macro_derive(VersionedGrpcMessage, attributes(grpc_versions))]
pub fn versioned_grpc_message_derive(input: TokenStream) -> TokenStream {
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
    let impl_from_arms = (0..=versions).map(|version| {
        let version_ident = format_ident!("V{}", version);
        let version_msg_ident = format_ident!("{}V{}", name, version);
        // Now create an identifier from the constructed string
        quote! {
            impl From<#mod_ident::#version_msg_ident> for #name {
                fn from(inner: #mod_ident::#version_msg_ident) -> Self {
                    Self {
                        version: Some(#mod_ident::Version::#version_ident(inner)),
                    }
                }
            }
            impl crate::platform::VersionedGrpcMessage<#mod_ident::#version_msg_ident> for #name {}
        }
    });

    // Generate the implementation
    let expanded = quote! {
        #( #impl_from_arms )*
    };

    // println!("Expanded code for VersionedGrpcMessage: {}", expanded);

    // Return the generated code
    TokenStream::from(expanded)
}

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
                // TODO: this substitutes any error that could be received instead of a proof, not just version-related
                _ => return Err(::platform_version::error::PlatformVersionError::UnknownVersionError("unknown proof version not known".to_string())),
            },
        }
    });

    // Generate match arms for proof and metadata methods
    let proof_owned_arms = (0..=versions).map(|version| {
        let version_ident = format_ident!("V{}", version);
        // Construct the identifier string for the module
        let version_mod_str = format!("{}_v{}", mod_ident, version);
        // Now create an identifier from the constructed string
        let version_mod_ident = format_ident!("{}", version_mod_str);
        quote! {
            #mod_ident::Version::#version_ident(inner) => match inner.result {
                Some(#mod_ident::#version_mod_ident::Result::Proof(proof)) => Ok(proof),
                // TODO: this substitutes any error that could be received instead of a proof, not just version-related
                _ => return Err(::platform_version::error::PlatformVersionError::UnknownVersionError("unknown proof version not known".to_string())),
            },
        }
    });

    let metadata_arms = (0..=versions).map(|version| {
        let version_ident = format_ident!("V{}", version);
        quote! {
            #mod_ident::Version::#version_ident(inner) => inner.metadata.as_ref().ok_or(platform_version::error::PlatformVersionError:: UnknownVersionError("result did not have metadata".to_string())),
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

            fn proof_owned(self) -> Result<Proof, Self::Error> {
                match self.version {
                    Some(version) => match version {
                        #( #proof_owned_arms )*
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

/// Implement mocking on gRPC messages
///
/// This adds implementation of [dapi_grpc::mock::Mockable] to the message.
/// If the `mocks` feature is enabled, the implementation uses serde_json to serialize/deserialize the message.
/// Otherwise, it returns None.
///
/// ## Requirements
///
/// When `mocks` feature is enabled:
///
/// * The message must implement [serde::Serialize] and [serde::Deserialize].
/// * The crate must depend on `serde` and `serde_json` crates.
///
#[proc_macro_derive(Mockable)]
pub fn mockable_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    // Generate the implementation
    let expanded = quote! {
        impl crate::mock::Mockable for #name {
            #[cfg(feature = "mocks")]
            fn mock_serialize(&self) -> Option<Vec<u8>> {
                Some(serde_json::to_vec_pretty(self).expect("unable to serialize"))
            }

            #[cfg(feature = "mocks")]
            fn mock_deserialize(data: &[u8]) -> Option<Self> {
                Some(serde_json::from_slice(data).expect("unable to deserialize"))
            }
        }
    };

    // println!("Expanded code: {}", expanded);

    // Return the generated code
    TokenStream::from(expanded)
}
