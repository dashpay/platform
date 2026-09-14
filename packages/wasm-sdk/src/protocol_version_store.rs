//! Persist the protocol version auto-detect learns, per network, so the next
//! SDK instance seeds at it instead of at the per-network floor.
//!
//! rs-sdk keeps the detected version in the instance only: every page load
//! starts at the floor and ratchets up once a proof verifies. A first request
//! that only parses at the network's real version (a contract using newer
//! index grammar, say) fails at the floor instead of ratcheting. Writing the
//! learned version to `localStorage` under `dash-sdk.protocol-version.<network>`
//! and seeding it back through `SdkBuilder::with_initial_version` removes that
//! first-request gap for mainnet and testnet. Devnets are re-cut from the
//! development line and already seed at the current version; regtest is local.
//! Both are left alone.
//!
//! The seed keeps auto-detect on, so a network that has moved further still
//! ratchets the SDK (and this store) upward. A version this build does not
//! know is ignored, as is a value at or below the floor.

use crate::browser_storage;
use dash_sdk::dpp::dashcore::Network;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::sdk::min_protocol_version;

const KEY_PREFIX: &str = "dash-sdk.protocol-version.";

/// The storage key for a network, or `None` for networks that are not persisted.
pub(crate) fn storage_key(network: Network) -> Option<String> {
    match network {
        Network::Mainnet => Some(format!("{KEY_PREFIX}mainnet")),
        Network::Testnet => Some(format!("{KEY_PREFIX}testnet")),
        Network::Devnet | Network::Regtest => None,
    }
}

/// The persisted version for `network`, when one is stored, this build knows it,
/// and it lies above the network's floor (a seed at or below the floor is a no-op).
pub(crate) fn load(network: Network) -> Option<&'static PlatformVersion> {
    let key = storage_key(network)?;
    let raw = browser_storage::get_item(&key)?;
    let version = raw.trim().parse::<u32>().ok()?;
    if version <= min_protocol_version(network) {
        return None;
    }
    PlatformVersion::get(version).ok()
}

/// Persist `version` for `network`. Silent when the network is not persisted or
/// no storage is reachable (workers, Node, blocked site data).
pub(crate) fn store(network: Network, version: u32) {
    if let Some(key) = storage_key(network) {
        browser_storage::set_item(&key, &version.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_public_networks_are_persisted() {
        assert!(storage_key(Network::Mainnet).is_some());
        assert!(storage_key(Network::Testnet).is_some());
        assert!(storage_key(Network::Devnet).is_none());
        assert!(storage_key(Network::Regtest).is_none());
        store(Network::Devnet, 14);
        assert!(load(Network::Devnet).is_none());
    }

    #[test]
    fn load_ignores_floor_unknown_and_garbage_values() {
        let key = storage_key(Network::Testnet).expect("testnet is persisted");
        let floor = min_protocol_version(Network::Testnet);
        browser_storage::set_item(&key, &floor.to_string());
        assert!(
            load(Network::Testnet).is_none(),
            "a value at the floor seeds nothing"
        );
        browser_storage::set_item(&key, &(floor - 1).to_string());
        assert!(
            load(Network::Testnet).is_none(),
            "a value below the floor seeds nothing"
        );
        browser_storage::set_item(
            &key,
            &(dash_sdk::dpp::version::LATEST_VERSION + 1).to_string(),
        );
        assert!(
            load(Network::Testnet).is_none(),
            "an unknown version seeds nothing"
        );
        browser_storage::set_item(&key, "not a number");
        assert!(load(Network::Testnet).is_none());
    }

    #[test]
    fn store_then_load_round_trips_above_the_floor() {
        let latest = dash_sdk::dpp::version::LATEST_VERSION;
        assert!(latest > min_protocol_version(Network::Testnet));
        store(Network::Testnet, latest);
        assert_eq!(
            load(Network::Testnet).map(|v| v.protocol_version),
            Some(latest)
        );
    }
}
