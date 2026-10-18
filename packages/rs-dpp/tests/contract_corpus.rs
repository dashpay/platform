#[path = "fixtures/contract_corpus/mod.rs"]
mod contract_corpus;

use contract_corpus::{contracts, CorpusContract, Network};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted,
    PlatformSerializableWithPlatformVersion,
};
use dpp::version::PlatformVersion;

/// Loads a stored contract the way Drive does: trusted bytes, no full validation.
fn load(contract: &CorpusContract, platform_version: &PlatformVersion) -> DataContract {
    DataContract::versioned_deserialize_trusted(&contract.bytes, false, platform_version)
        .unwrap_or_else(|error| {
            panic!(
                "{:?} contract {} failed to load at protocol version {}: {error}",
                contract.network, contract.id, platform_version.protocol_version
            )
        })
}

#[test]
fn should_hold_every_mainnet_contract_and_every_distinct_testnet_schema() {
    let corpus = contracts();
    let count = |network| corpus.iter().filter(|c| c.network == network).count();
    assert_eq!(count(Network::Mainnet), 55);
    assert_eq!(count(Network::Testnet), 694);
}

#[test]
fn should_load_every_corpus_contract_at_protocol_version_14() {
    let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
    let document_types: usize = contracts()
        .iter()
        .map(|contract| {
            let loaded = load(contract, platform_version);
            assert_eq!(
                loaded
                    .id()
                    .to_string(dpp::platform_value::string_encoding::Encoding::Base58),
                contract.id
            );
            assert_eq!(loaded.version(), contract.version);
            loaded.document_types().len()
        })
        .sum();
    assert_eq!(document_types, 106 + 1397);
}

#[test]
fn should_load_every_corpus_contract_at_the_latest_protocol_version() {
    let platform_version = PlatformVersion::latest();
    for contract in contracts() {
        let loaded = load(&contract, platform_version);
        assert_eq!(loaded.version(), contract.version);
    }
}

#[test]
fn should_reload_every_corpus_contract_reserialized_at_the_latest_protocol_version() {
    let platform_version = PlatformVersion::latest();
    for contract in contracts() {
        let loaded = load(&contract, platform_version);
        let bytes = loaded
            .serialize_to_bytes_with_platform_version(platform_version)
            .unwrap_or_else(|error| {
                panic!(
                    "{:?} contract {} failed to serialize: {error}",
                    contract.network, contract.id
                )
            });
        let reloaded = DataContract::versioned_deserialize_trusted(&bytes, false, platform_version)
            .unwrap_or_else(|error| {
                panic!(
                    "{:?} contract {} failed to reload: {error}",
                    contract.network, contract.id
                )
            });
        assert_eq!(
            reloaded, loaded,
            "{:?} contract {} changed through a serialization round trip",
            contract.network, contract.id
        );
    }
}
