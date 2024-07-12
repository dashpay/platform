// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Fee costs
//!
//! Fee costs for Known Platform operations
//!

use crate::block::epoch::{Epoch, EpochIndex};
use crate::fee::Credits;
use platform_version::version::fee::FeeVersion;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

pub mod constants;

pub type CachedEpochIndexFeeVersions = BTreeMap<EpochIndex, FeeVersion>;

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
        self.lookup_cost(&version)
    }
}

/// Costs for Epochs
pub trait EpochCosts {
    /// Get the closest epoch in the past that has a cost table
    /// This is where the base costs last changed
    fn active_fee_version(&self, cached_fee_version: &CachedEpochIndexFeeVersions) -> FeeVersion;
    /// Get the cost for the known cost item
    fn cost_for_known_cost_item(
        &self,
        cached_fee_version: &CachedEpochIndexFeeVersions,
        cost_item: KnownCostItem,
    ) -> Credits;
}

impl EpochCosts for Epoch {
    /// Get the active fee version for an epoch
    fn active_fee_version(&self, cached_fee_version: &CachedEpochIndexFeeVersions) -> FeeVersion {
        // If the exact EpochIndex is matching to a FeeVersion update
        if let Some(fee_version) = cached_fee_version.get(&self.index) {
            return fee_version.clone();
        }
        // else return the FeeVersion at  lower adjacent EpochIndex (if available, else the FeeVersion of first PlatformVersion)
        cached_fee_version
            .range(..=self.index)
            .next_back()
            .map(|(_, fee_version)| fee_version)
            .unwrap_or_else(|| &PlatformVersion::first().fee_version)
            .clone()
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
