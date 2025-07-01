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
pub fn get_quorum_base_url(network: Network, devnet_name: Option<&str>) -> String {
    match network {
        Network::Dash => "https://quorums.mainnet.networks.dash.org".to_string(),
        Network::Testnet => "https://quorums.testnet.networks.dash.org".to_string(),
        Network::Devnet => {
            if let Some(name) = devnet_name {
                format!("https://quorums.devnet.{}.networks.dash.org", name)
            } else {
                panic!("Devnet name must be provided for devnet network")
            }
        }
        Network::Regtest => panic!("Regtest network is not supported by trusted context provider"),
        _ => panic!("Unknown network type"),
    }
}
