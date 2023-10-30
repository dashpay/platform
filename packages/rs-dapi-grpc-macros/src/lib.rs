use heck::AsSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
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
#[proc_macro_derive(VersionedGrpcResponse)]
pub fn versioned_grpc_response_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the struct
    let name = input.ident.clone();

    let v0_name = format!("{}V0", name);
    let v0_ident = syn::parse_str::<Ident>(&v0_name).expect("parse v0 ident");

    // Generate the names of the nested message and enum types
    let mod_name = AsSnakeCase(name.to_string()).to_string();
    let mod_ident = syn::parse_str::<Ident>(&mod_name).expect("parse response ident");

    let mod_v0_name = format!("{}_v0", mod_name);
    let mod_v0_ident = syn::parse_str::<Ident>(&mod_v0_name).expect("parse v0 ident");

    // Generate the implementation
    let expanded = quote! {
        impl crate::platform::VersionedGrpcResponse for #name {
            type Error = ::platform_version::error::PlatformVersionError;

            fn get_proof(&self, version: &::platform_version::version::PlatformVersion) -> Result<Proof, Self::Error> {
                // TODO add assertion to check that version is correct
                use ::platform_version::TryFromPlatformVersioned;

                let item = #mod_ident::#v0_ident::try_from_platform_versioned(self.clone(), version)?;

                match &item.result {
                    Some(#mod_ident::#mod_v0_ident::Result::Proof(ref proof)) => {
                        Ok(proof.clone())
                    },
                    _ => Err(::platform_version::error::PlatformVersionError::UnknownVersionError("result is not proof".to_string())),
                }
            }

            fn get_metadata(&self, version: &::platform_version::version::PlatformVersion)  -> Result<ResponseMetadata, Self::Error>  {
                use ::platform_version::TryFromPlatformVersioned;

                let item = #mod_ident::#v0_ident::try_from_platform_versioned(self.clone(), version)?;

                let metadata = item.metadata.ok_or(::platform_version::error::PlatformVersionError::UnknownVersionError("metadata is not set".to_string()))?;
                Ok(metadata)
            }
        }
    };
    println!("Expanded code: {}", expanded);

    // Return the generated code
    TokenStream::from(expanded)
}

/// Implement versioning on gRPC messages
///
/// This adds implementation of version 0 to the message:
///
/// * impl From<MessageNameV0> for MessageName
/// * impl [TryFromPlatformVersioned<MessageName>](::platform_version::TryFromPlatformVersioned) for MessageNameV0
///
/// where `MessageName` is the name of the object on which the derive is declared.
#[proc_macro_derive(GrpcMessageV0)]
pub fn grpc_message_v0_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the struct
    let name = input.ident.clone();

    let v0_name = format!("{}V0", name);
    let v0_ident = syn::parse_str::<Ident>(&v0_name).expect("parse v0 ident");

    // Generate the names of the nested message and enum types
    let mod_name = AsSnakeCase(name.to_string()).to_string();
    let mod_ident = syn::parse_str::<Ident>(&mod_name).expect("parse response ident");

    // let mod_v0_name = format!("{}_v0", mod_name);
    // let mod_v0_ident = syn::parse_str::<Ident>(&mod_v0_name).expect("parse v0 ident");

    // Generate the implementation of the IntoProof trait
    let expanded = quote! {
        impl From<#mod_ident::#v0_ident> for #name {
            fn from(item: #mod_ident::#v0_ident) -> Self {
                Self {
                    version: Some(#mod_ident::Version::V0(item)),
                }
            }
        }

        impl ::platform_version::TryFromPlatformVersioned<#name> for #mod_ident::#v0_ident {
            type Error = ::platform_version::error::PlatformVersionError;

            fn try_from_platform_versioned(
                value: #name,
                platform_version: &::platform_version::version::PlatformVersion,
            ) -> Result<Self, Self::Error> {
                // TODO check if version matches `platform_version`
                match value.version {
                    Some(#mod_ident::Version::V0(item)) => {
                        Ok(item)
                    },
                    _ => Err(platform_version::error::PlatformVersionError::UnknownVersionError("unsupported version".to_string())),
                }
            }
        }
    };
    // uncomment below to debug
    // println!("Expanded code: {}", expanded);

    // Return the generated code
    TokenStream::from(expanded)
}
