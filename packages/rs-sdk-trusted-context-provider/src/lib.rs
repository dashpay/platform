//! # Trusted Context Provider for Dash Platform SDK
//!
//! This crate provides a trusted HTTP-based context provider that fetches quorum
//! information from trusted HTTP endpoints instead of requiring Core RPC access.
//!
//! ## Networks Supported
//! - **Mainnet**: Uses `https://quorums.mainnet.networks.dash.org/`
//! - **Testnet**: Uses `https://quorums.testnet.networks.dash.org/`
//! - **Devnet**: Uses `https://quorums.devnet.<devnet_name>.networks.dash.org/`

pub mod error;
pub mod provider;
pub mod types;

pub use error::TrustedContextProviderError;
pub use provider::TrustedHttpContextProvider;

use dpp::dashcore::Network;

/// Get the base URL for quorum endpoints based on the network
pub fn get_quorum_base_url(
    network: Network,
    devnet_name: Option<&str>,
) -> Result<String, TrustedContextProviderError> {
    match network {
        Network::Dash => Ok("https://quorums.mainnet.networks.dash.org".to_string()),
        Network::Testnet => Ok("https://quorums.testnet.networks.dash.org".to_string()),
        Network::Devnet => {
            if let Some(name) = devnet_name {
                // Validate devnet name format: must be alphanumeric with hyphens allowed
                if name.is_empty() {
                    return Err(TrustedContextProviderError::InvalidDevnetName(
                        "Devnet name cannot be empty".to_string(),
                    ));
                }
                if !name.chars().all(|c| c.is_alphanumeric() || c == '-') {
                    return Err(TrustedContextProviderError::InvalidDevnetName(
                        "Devnet name must contain only alphanumeric characters and hyphens"
                            .to_string(),
                    ));
                }
                if name.starts_with('-') || name.ends_with('-') {
                    return Err(TrustedContextProviderError::InvalidDevnetName(
                        "Devnet name cannot start or end with a hyphen".to_string(),
                    ));
                }
                Ok(format!("https://quorums.devnet.{}.networks.dash.org", name))
            } else {
                Err(TrustedContextProviderError::InvalidDevnetName(
                    "Devnet name must be provided for devnet network".to_string(),
                ))
            }
        }
        Network::Regtest => Err(TrustedContextProviderError::UnsupportedNetwork(
            "Regtest network is not supported by trusted context provider".to_string(),
        )),
        _ => Err(TrustedContextProviderError::UnsupportedNetwork(format!(
            "Unknown network type: {:?}",
            network
        ))),
    }
}
