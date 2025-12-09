//! Example showing how to use TrustedHttpContextProvider with a custom fallback provider

use dash_context_provider::{ContextProvider, ContextProviderError};
use dpp::dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
use std::num::NonZeroUsize;
use std::sync::Arc;

/// Example fallback provider that could fetch data contracts from another source
struct MyFallbackProvider;

impl ContextProvider for MyFallbackProvider {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        // This would not be called as the trusted provider handles quorum keys
        Err(ContextProviderError::Generic("Not implemented".to_string()))
    }

    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        // In a real implementation, this would fetch from Core RPC or another source
        println!("Fallback provider: get_data_contract called");
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        // In a real implementation, this would fetch from Core RPC or another source
        println!("Fallback provider: get_token_configuration called");
        Ok(None)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        // This would not be called as the trusted provider handles this
        Ok(1320)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the trusted HTTP provider with custom fallback
    let _trusted_provider =
        TrustedHttpContextProvider::new(Network::Testnet, None, NonZeroUsize::new(100).unwrap())?
            .with_fallback_provider(MyFallbackProvider);

    println!("Created trusted HTTP provider with custom fallback!");

    // The provider can now:
    // 1. Fetch quorum public keys from HTTP endpoints (trusted provider)
    // 2. Delegate data contract requests to MyFallbackProvider
    // 3. Delegate token configuration requests to MyFallbackProvider

    // In a real application, you would use this provider with the SDK:
    // sdk.set_context_provider(trusted_provider);

    Ok(())
}
