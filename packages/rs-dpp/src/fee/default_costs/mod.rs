//! Fee costs
//!
//! Fee costs for Known Platform operations
//!

use crate::block::epoch::{Epoch, EpochIndex};
use crate::fee::Credits;
use platform_version::version::fee::{
    FeeVersion, FeeVersionFieldsBeforeVersion4, FeeVersionNumber,
};
use std::collections::BTreeMap;

pub mod constants;

pub type CachedEpochIndexFeeVersions = BTreeMap<EpochIndex, &'static FeeVersion>;
pub type EpochIndexFeeVersionsForStorage = BTreeMap<EpochIndex, FeeVersionNumber>;

// This is type only meant for deserialization because of an issue
// The issue was that the platform state was stored with FeeVersions in it before version 1.4
// When we would add new fields we would be unable to deserialize
// This FeeProcessingVersionFieldsBeforeVersion4 is how things were before version 1.4 was released
pub type CachedEpochIndexFeeVersionsFieldsBeforeVersion4 =
    BTreeMap<EpochIndex, FeeVersionFieldsBeforeVersion4>;

/// A Known Cost Item is an item that changes costs depending on the Epoch
#[derive(Eq, PartialEq, Copy, Clone, Hash)]
pub enum KnownCostItem {
    /// The storage cost used when writing bytes
    StorageDiskUsageCreditPerByte,
    /// The processing cost used when writing bytes
    StorageProcessingCreditPerByte,
    /// The processing cost used when loading bytes from storage
    StorageLoadCreditPerByte,
    /// The processing cost used when loading bytes not from storage
    NonStorageLoadCreditPerByte,
    /// The cost used when performing a disk seek
    StorageSeekCost,
    // The following are set costs of routine operations
    /// The cost for fetching an identity balance
    FetchIdentityBalanceProcessingCost,
    /// The cost for fetching an identity key
    FetchSingleIdentityKeyProcessingCost,
    /// The cost for a Single SHA256 operation, with a specific size
    SingleSHA256(usize),
    /// The cost for a Blake3 operation, with a specific size
    Blake3(usize),
    /// The cost for a EcdsaSecp256k1 signature verification
    VerifySignatureEcdsaSecp256k1,
    /// The cost for a BLS12_381 signature verification
    VerifySignatureBLS12_381,
    /// The cost for a EcdsaHash160 signature verification
    VerifySignatureEcdsaHash160,
    /// The cost for a Bip13ScriptHash signature verification
    VerifySignatureBip13ScriptHash,
    /// The cost for a Eddsa25519Hash160 signature verification
    VerifySignatureEddsa25519Hash160,
}

impl KnownCostItem {
    #[inline]
    pub fn lookup_cost(&self, fee_version: &FeeVersion) -> Credits {
        match self {
            KnownCostItem::StorageDiskUsageCreditPerByte => {
                fee_version.storage.storage_disk_usage_credit_per_byte
            }
            KnownCostItem::StorageProcessingCreditPerByte => {
                fee_version.storage.storage_processing_credit_per_byte
            }
            KnownCostItem::StorageLoadCreditPerByte => {
                fee_version.storage.storage_load_credit_per_byte
            }
            KnownCostItem::NonStorageLoadCreditPerByte => {
                fee_version.storage.non_storage_load_credit_per_byte
            }
            KnownCostItem::StorageSeekCost => fee_version.storage.storage_seek_cost,
            KnownCostItem::FetchIdentityBalanceProcessingCost => {
                fee_version
                    .processing
                    .fetch_identity_balance_processing_cost
            }
            KnownCostItem::FetchSingleIdentityKeyProcessingCost => {
                fee_version
                    .processing
                    .fetch_single_identity_key_processing_cost
            }
            KnownCostItem::Blake3(size) => {
                fee_version.hashing.blake3_base
                    + fee_version.hashing.blake3_per_block * *size as u64
            }
            KnownCostItem::SingleSHA256(size) => {
                fee_version.hashing.single_sha256_base
                    + fee_version.hashing.sha256_per_block * *size as u64
            }
            KnownCostItem::VerifySignatureEcdsaSecp256k1 => {
                fee_version.signature.verify_signature_ecdsa_secp256k1
            }
            KnownCostItem::VerifySignatureBLS12_381 => {
                fee_version.signature.verify_signature_bls12_381
            }
            KnownCostItem::VerifySignatureEcdsaHash160 => {
                fee_version.signature.verify_signature_ecdsa_hash160
            }
            KnownCostItem::VerifySignatureBip13ScriptHash => {
                fee_version.signature.verify_signature_bip13_script_hash
            }
            KnownCostItem::VerifySignatureEddsa25519Hash160 => {
                fee_version.signature.verify_signature_eddsa25519_hash160
            }
        }
    }

    pub fn lookup_cost_on_epoch<T: EpochCosts>(
        &self,
        epoch: &T,
        cached_fee_version: &CachedEpochIndexFeeVersions,
    ) -> Credits {
        let version = epoch.active_fee_version(cached_fee_version);
        self.lookup_cost(version)
    }
}

/// Costs for Epochs
pub trait EpochCosts {
    /// Get the closest epoch in the past that has a cost table
    /// This is where the base costs last changed
    fn active_fee_version(
        &self,
        cached_fee_version: &CachedEpochIndexFeeVersions,
    ) -> &'static FeeVersion;
    /// Get the cost for the known cost item
    fn cost_for_known_cost_item(
        &self,
        cached_fee_version: &CachedEpochIndexFeeVersions,
        cost_item: KnownCostItem,
    ) -> Credits;
}

impl EpochCosts for Epoch {
    /// Get the active fee version for an epoch
    fn active_fee_version(
        &self,
        cached_fee_version: &CachedEpochIndexFeeVersions,
    ) -> &'static FeeVersion {
        // If the exact EpochIndex is matching to a FeeVersion update
        if let Some(fee_version) = cached_fee_version.get(&self.index) {
            return fee_version;
        }
        // else return the FeeVersion at  lower adjacent EpochIndex (if available, else the FeeVersion of first PlatformVersion)
        cached_fee_version
            .range(..=self.index)
            .next_back()
            .map(|(_, fee_version)| *fee_version)
            .unwrap_or_else(|| FeeVersion::first())
    }

    /// Get the cost for the known cost item
    fn cost_for_known_cost_item(
        &self,
        cached_fee_version: &CachedEpochIndexFeeVersions,
        cost_item: KnownCostItem,
    ) -> Credits {
        cost_item.lookup_cost_on_epoch(self, cached_fee_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_version::version::PLATFORM_VERSIONS;

    /// Every known cost item, the hashing ones at a few sizes, with a name
    /// for failure messages. The match stops a new item compiling until
    /// it is listed here.
    fn every_known_cost_item() -> Vec<(String, KnownCostItem)> {
        let mut items = vec![
            KnownCostItem::StorageDiskUsageCreditPerByte,
            KnownCostItem::StorageProcessingCreditPerByte,
            KnownCostItem::StorageLoadCreditPerByte,
            KnownCostItem::NonStorageLoadCreditPerByte,
            KnownCostItem::StorageSeekCost,
            KnownCostItem::FetchIdentityBalanceProcessingCost,
            KnownCostItem::FetchSingleIdentityKeyProcessingCost,
            KnownCostItem::VerifySignatureEcdsaSecp256k1,
            KnownCostItem::VerifySignatureBLS12_381,
            KnownCostItem::VerifySignatureEcdsaHash160,
            KnownCostItem::VerifySignatureBip13ScriptHash,
            KnownCostItem::VerifySignatureEddsa25519Hash160,
        ];
        for size in [0, 1, 64, 1_000] {
            items.push(KnownCostItem::SingleSHA256(size));
            items.push(KnownCostItem::Blake3(size));
        }
        items
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let name = match item {
                    KnownCostItem::StorageDiskUsageCreditPerByte
                    | KnownCostItem::StorageProcessingCreditPerByte
                    | KnownCostItem::StorageLoadCreditPerByte
                    | KnownCostItem::NonStorageLoadCreditPerByte
                    | KnownCostItem::StorageSeekCost
                    | KnownCostItem::FetchIdentityBalanceProcessingCost
                    | KnownCostItem::FetchSingleIdentityKeyProcessingCost
                    | KnownCostItem::VerifySignatureEcdsaSecp256k1
                    | KnownCostItem::VerifySignatureBLS12_381
                    | KnownCostItem::VerifySignatureEcdsaHash160
                    | KnownCostItem::VerifySignatureBip13ScriptHash
                    | KnownCostItem::VerifySignatureEddsa25519Hash160 => {
                        format!("item {index}")
                    }
                    KnownCostItem::SingleSHA256(size) => format!("SingleSHA256({size})"),
                    KnownCostItem::Blake3(size) => format!("Blake3({size})"),
                };
                (name, item)
            })
            .collect()
    }

    /// Saved platform state keeps only a fee version's number and reloads
    /// the schedule registered under it, so every cost the fee history
    /// serves must be the same in each protocol version's schedule and in
    /// the one its number resolves to. A schedule that changes one needs
    /// the next number.
    #[test]
    fn should_serve_every_known_cost_of_a_protocol_version_from_its_fee_version_number() {
        for platform_version in PLATFORM_VERSIONS {
            let fee_version = &platform_version.fee_version;
            let registered = FeeVersion::get(fee_version.fee_version_number)
                .expect("expected the fee version number to be registered");
            for (name, item) in every_known_cost_item() {
                assert_eq!(
                    item.lookup_cost(fee_version),
                    item.lookup_cost(registered),
                    "protocol version {}: {name} differs from fee version number {}",
                    platform_version.protocol_version,
                    fee_version.fee_version_number
                );
            }
        }
    }
}
