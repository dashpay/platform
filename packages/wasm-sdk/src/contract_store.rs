//! Persist fetched data contracts per network, so the next SDK instance seeds
//! its trusted-context cache from the page's storage instead of fetching the
//! same contracts again on every load.
//!
//! An app's first paint needs its contracts before its first proved document
//! query can be built or verified. Fetching them is one round trip per load
//! (or one batched request), typically 0.7 to 1.3 s ahead of any page data.
//! Contracts are versioned and change rarely, so the fetched bytes are kept
//! under `dash-sdk.contracts.<network>` as one JSON map of contract id to
//! `{version, bytes}` and seeded back in `WasmSdkBuilder::build`.
//!
//! Staleness fails closed: a document stamped with a `$contractVersion` above
//! the cached contract's version drops the entry (see
//! `WasmSdk::drop_stale_contract`), and explicit cache removals remove the
//! stored entry too. An entry this build cannot deserialize is skipped.
//!
//! Persisted for mainnet and testnet only, like the protocol version: devnets
//! are re-cut and a contract id (a hash of owner and nonce) can outlive the
//! chain that minted it; regtest is local.

use std::collections::BTreeMap;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use dash_sdk::dpp::dashcore::Network;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted,
    PlatformSerializableWithPlatformVersion,
};
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::platform::{DataContract, Identifier};
use serde::{Deserialize, Serialize};

use crate::browser_storage;

const KEY_PREFIX: &str = "dash-sdk.contracts.";

/// Entries per network. Well above what an app configures; guards the
/// storage quota against a runaway caller.
const MAX_ENTRIES: usize = 100;

#[derive(Serialize, Deserialize)]
struct StoredContract {
    /// The contract's own `version`, kept alongside so a stale entry can be
    /// recognised without deserializing it.
    version: u32,
    /// Platform-serialized contract bytes, base64.
    bytes: String,
}

type StoredContracts = BTreeMap<String, StoredContract>;

/// The storage key for a network, or `None` for networks that are not persisted.
pub(crate) fn storage_key(network: Network) -> Option<String> {
    match network {
        Network::Mainnet => Some(format!("{KEY_PREFIX}mainnet")),
        Network::Testnet => Some(format!("{KEY_PREFIX}testnet")),
        Network::Devnet | Network::Regtest => None,
    }
}

fn read(key: &str) -> StoredContracts {
    browser_storage::get_item(key)
        .and_then(|raw| serde_json::from_str::<StoredContracts>(&raw).ok())
        .unwrap_or_default()
}

fn write(key: &str, contracts: &StoredContracts) {
    if let Ok(raw) = serde_json::to_string(contracts) {
        browser_storage::set_item(key, &raw);
    }
}

/// Every persisted contract for `network` this build can deserialize, in id
/// order. Entries that fail to decode are skipped, not returned.
pub(crate) fn load_all(network: Network, platform_version: &PlatformVersion) -> Vec<DataContract> {
    let Some(key) = storage_key(network) else {
        return Vec::new();
    };
    read(&key)
        .into_values()
        .filter_map(|stored| {
            let bytes = BASE64.decode(stored.bytes).ok()?;
            DataContract::versioned_deserialize_trusted(&bytes, true, platform_version).ok()
        })
        .collect()
}

/// Persist `contract` for `network`, replacing any entry under its id.
/// Silent when the network is not persisted, the contract does not
/// serialize, or the map is full.
pub(crate) fn store(network: Network, contract: &DataContract, platform_version: &PlatformVersion) {
    let Some(key) = storage_key(network) else {
        return;
    };
    let Ok(bytes) = contract.serialize_to_bytes_with_platform_version(platform_version) else {
        return;
    };
    let id = contract.id().to_string(Encoding::Base58);
    let mut contracts = read(&key);
    if !contracts.contains_key(&id) && contracts.len() >= MAX_ENTRIES {
        return;
    }
    contracts.insert(
        id,
        StoredContract {
            version: contract.version(),
            bytes: BASE64.encode(bytes),
        },
    );
    write(&key, &contracts);
}

/// Remove the entry for `id`, if any.
pub(crate) fn remove(network: Network, id: &Identifier) {
    let Some(key) = storage_key(network) else {
        return;
    };
    let mut contracts = read(&key);
    if contracts.remove(&id.to_string(Encoding::Base58)).is_some() {
        write(&key, &contracts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};

    fn contract(id_byte: u8, version: u32) -> DataContract {
        let mut contract =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("DPNS fixture");
        contract.set_id(Identifier::new([id_byte; 32]));
        contract.set_version(version);
        contract
    }

    #[test]
    fn only_public_networks_are_persisted() {
        assert!(storage_key(Network::Mainnet).is_some());
        assert!(storage_key(Network::Testnet).is_some());
        assert!(storage_key(Network::Devnet).is_none());
        assert!(storage_key(Network::Regtest).is_none());
        store(Network::Devnet, &contract(1, 1), PlatformVersion::latest());
        assert!(load_all(Network::Devnet, PlatformVersion::latest()).is_empty());
    }

    #[test]
    fn store_load_replace_and_remove_round_trip() {
        let pv = PlatformVersion::latest();
        assert!(load_all(Network::Testnet, pv).is_empty());

        store(Network::Testnet, &contract(0x11, 1), pv);
        store(Network::Testnet, &contract(0x22, 3), pv);
        let loaded = load_all(Network::Testnet, pv);
        assert_eq!(loaded.len(), 2);
        assert!(loaded
            .iter()
            .any(|c| c.id() == Identifier::new([0x11; 32]) && c.version() == 1));
        assert!(loaded
            .iter()
            .any(|c| c.id() == Identifier::new([0x22; 32]) && c.version() == 3));

        // A newer fetch of the same id replaces the entry.
        store(Network::Testnet, &contract(0x11, 2), pv);
        let loaded = load_all(Network::Testnet, pv);
        assert_eq!(loaded.len(), 2);
        assert!(loaded
            .iter()
            .any(|c| c.id() == Identifier::new([0x11; 32]) && c.version() == 2));

        remove(Network::Testnet, &Identifier::new([0x11; 32]));
        let loaded = load_all(Network::Testnet, pv);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id(), Identifier::new([0x22; 32]));
    }

    #[test]
    fn undecodable_entries_are_skipped_and_garbage_is_ignored() {
        let pv = PlatformVersion::latest();
        let key = storage_key(Network::Testnet).expect("testnet is persisted");

        browser_storage::set_item(&key, "not json");
        assert!(load_all(Network::Testnet, pv).is_empty());
        // A garbage store does not block new entries.
        store(Network::Testnet, &contract(0x33, 1), pv);
        assert_eq!(load_all(Network::Testnet, pv).len(), 1);

        // One bad entry next to a good one: only the good one comes back.
        let mut contracts = read(&key);
        contracts.insert(
            Identifier::new([0x44; 32]).to_string(Encoding::Base58),
            StoredContract {
                version: 1,
                bytes: BASE64.encode(b"not a contract"),
            },
        );
        write(&key, &contracts);
        let loaded = load_all(Network::Testnet, pv);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id(), Identifier::new([0x33; 32]));
    }

    #[test]
    fn a_full_map_refuses_new_ids_but_still_replaces_known_ones() {
        let pv = PlatformVersion::latest();
        for i in 0..MAX_ENTRIES {
            store(Network::Mainnet, &contract(i as u8, 1), pv);
        }
        assert_eq!(load_all(Network::Mainnet, pv).len(), MAX_ENTRIES);
        store(Network::Mainnet, &contract(0xFF, 1), pv);
        assert_eq!(
            load_all(Network::Mainnet, pv).len(),
            MAX_ENTRIES,
            "an unknown id must not grow a full map"
        );
        store(Network::Mainnet, &contract(0, 7), pv);
        assert!(
            load_all(Network::Mainnet, pv)
                .iter()
                .any(|c| c.id() == Identifier::new([0; 32]) && c.version() == 7),
            "a known id must still be replaced"
        );
    }
}
