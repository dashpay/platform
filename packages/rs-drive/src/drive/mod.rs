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

#[cfg(any(feature = "full", feature = "verify"))]
use std::cell::RefCell;
#[cfg(feature = "full")]
use std::collections::HashMap;
#[cfg(feature = "full")]
use std::path::Path;

#[cfg(feature = "full")]
use dpp::data_contract::DriveContractExt;
#[cfg(feature = "full")]
use grovedb::batch::KeyInfoPath;
#[cfg(any(feature = "full", feature = "verify"))]
use grovedb::GroveDb;
#[cfg(feature = "full")]
use grovedb::{EstimatedLayerInformation, Transaction, TransactionArg};

#[cfg(feature = "full")]
use object_size_info::DocumentAndContractInfo;
#[cfg(feature = "full")]
use object_size_info::DocumentInfo::DocumentEstimatedAverageSize;

#[cfg(feature = "full")]
use crate::contract::Contract;
#[cfg(feature = "full")]
use crate::drive::batch::GroveDbOpBatch;
#[cfg(any(feature = "full", feature = "verify"))]
use crate::drive::config::DriveConfig;
#[cfg(feature = "full")]
use crate::error::Error;
#[cfg(feature = "full")]
use crate::fee::op::LowLevelDriveOperation;
#[cfg(feature = "full")]
use crate::fee::op::LowLevelDriveOperation::GroveOperation;

#[cfg(any(feature = "full", feature = "verify"))]
pub mod balances;
/// Batch module
#[cfg(feature = "full")]
pub mod batch;
/// Block info module
#[cfg(feature = "full")]
pub mod block_info;
/// Drive Cache
#[cfg(any(feature = "full", feature = "verify"))]
pub mod cache;
#[cfg(any(feature = "full", feature = "verify"))]
pub mod config;
/// Contract module
#[cfg(any(feature = "full", feature = "verify"))]
pub mod contract;
#[cfg(any(feature = "full", feature = "verify"))]
pub mod defaults;
/// Document module
#[cfg(feature = "full")]
pub mod document;
#[cfg(feature = "full")]
mod estimation_costs;
/// Fee pools module
#[cfg(feature = "full")]
pub mod fee_pools;
#[cfg(any(feature = "full", feature = "verify"))]
pub mod flags;
/// Genesis time module
#[cfg(feature = "full")]
pub mod genesis_time;
#[cfg(feature = "full")]
pub(crate) mod grove_operations;
/// Identity module
#[cfg(any(feature = "full", feature = "verify"))]
pub mod identity;
#[cfg(feature = "full")]
pub mod initialization;
#[cfg(feature = "full")]
pub mod object_size_info;
#[cfg(feature = "full")]
mod protocol_upgrade;
#[cfg(feature = "full")]
pub mod query;
#[cfg(feature = "full")]
mod system;
#[cfg(test)]
mod test_utils;
#[cfg(any(feature = "full", feature = "verify"))]
mod verify;

#[cfg(feature = "full")]
use crate::drive::block_info::BlockInfo;
#[cfg(feature = "full")]
use crate::drive::cache::DataContractCache;
#[cfg(any(feature = "full", feature = "verify"))]
use crate::drive::cache::DriveCache;
#[cfg(feature = "full")]
use crate::drive::object_size_info::OwnedDocumentInfo;
#[cfg(feature = "full")]
use crate::fee::result::FeeResult;
#[cfg(feature = "full")]
use crate::fee_pools::epochs::Epoch;

/// Drive struct
#[cfg(any(feature = "full", feature = "verify"))]
pub struct Drive {
    /// GroveDB
    pub grove: GroveDb,
    /// Drive config
    pub config: DriveConfig,
    /// Drive Cache
    pub cache: RefCell<DriveCache>,
}

// The root tree structure is very important!
// It must be constructed in such a way that important information
// is at the top of the tree in order to reduce proof size
// the most import tree is the Contract Documents tree

//                         Contract_Documents 64
//                  /                               \
//             Identities 32                           Balances 96
//             /        \                         /                   \
//   Token_Balances 16    Pools 48      WithdrawalTransactions 80    Misc  112
//       /      \                                /                       \
//     NUPKH->I 8 UPKH->I 24        SpentAssetLockTransactions 72        Versions 120

/// Keys for the root tree.
#[cfg(any(feature = "full", feature = "verify"))]
#[repr(u8)]
pub enum RootTree {
    // Input data errors
    /// Contract Documents
    ContractDocuments = 64,
    /// Identities
    Identities = 32,
    /// Unique Public Key Hashes to Identities
    UniquePublicKeyHashesToIdentities = 24, // UPKH->I above
    /// Non Unique Public Key Hashes to Identities, useful for Masternode Identities
    NonUniquePublicKeyKeyHashesToIdentities = 8, // NUPKH->I
    /// Pools
    Pools = 48,
    /// Spent Asset Lock Transactions
    SpentAssetLockTransactions = 72,
    /// Misc
    Misc = 112,
    /// Asset Unlock Transactions
    WithdrawalTransactions = 80,
    /// Balances
    Balances = 96,
    /// Token Balances
    TokenBalances = 16,
    /// Versions desired by proposers
    Versions = 120,
}

/// Storage cost
#[cfg(feature = "full")]
pub const STORAGE_COST: i32 = 50;

#[cfg(any(feature = "full", feature = "verify"))]
impl From<RootTree> for u8 {
    fn from(root_tree: RootTree) -> Self {
        root_tree as u8
    }
}

#[cfg(any(feature = "full", feature = "verify"))]
impl From<RootTree> for [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        [root_tree as u8]
    }
}

#[cfg(any(feature = "full", feature = "verify"))]
impl From<RootTree> for &'static [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        match root_tree {
            RootTree::Identities => &[32],
            RootTree::ContractDocuments => &[64],
            RootTree::UniquePublicKeyHashesToIdentities => &[24],
            RootTree::SpentAssetLockTransactions => &[72],
            RootTree::Pools => &[48],
            RootTree::Misc => &[112],
            RootTree::WithdrawalTransactions => &[80],
            RootTree::Balances => &[96],
            RootTree::TokenBalances => &[16],
            RootTree::NonUniquePublicKeyKeyHashesToIdentities => &[8],
            RootTree::Versions => &[120],
        }
    }
}

/// Returns the path to the identities
#[cfg(feature = "full")]
pub(crate) fn identity_tree_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Identities)]
}

/// Returns the path to the key hashes.
#[cfg(feature = "full")]
pub(crate) fn unique_key_hashes_tree_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(
        RootTree::UniquePublicKeyHashesToIdentities,
    )]
}

/// Returns the path to the key hashes.
#[cfg(any(feature = "full", feature = "verify"))]
pub(crate) fn unique_key_hashes_tree_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::UniquePublicKeyHashesToIdentities as u8]]
}

/// Returns the path to the masternode key hashes.
#[cfg(feature = "full")]
pub(crate) fn non_unique_key_hashes_tree_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(
        RootTree::NonUniquePublicKeyKeyHashesToIdentities,
    )]
}

/// Returns the path to the masternode key hashes.
#[cfg(feature = "full")]
pub(crate) fn non_unique_key_hashes_tree_path_vec() -> Vec<Vec<u8>> {
    vec![vec![
        RootTree::NonUniquePublicKeyKeyHashesToIdentities as u8,
    ]]
}

/// Returns the path to the masternode key hashes sub tree.
#[cfg(feature = "full")]
pub(crate) fn non_unique_key_hashes_sub_tree_path(public_key_hash: &[u8]) -> [&[u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::NonUniquePublicKeyKeyHashesToIdentities),
        public_key_hash,
    ]
}

/// Returns the path to the masternode key hashes sub tree.
#[cfg(feature = "full")]
pub(crate) fn non_unique_key_hashes_sub_tree_path_vec(public_key_hash: [u8; 20]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::NonUniquePublicKeyKeyHashesToIdentities as u8],
        public_key_hash.to_vec(),
    ]
}

/// Returns the path to a contract's document types.
#[cfg(feature = "full")]
fn contract_documents_path(contract_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
        contract_id,
        &[1],
    ]
}

#[cfg(feature = "full")]
impl Drive {
    /// Opens a path in groveDB.
    pub fn open<P: AsRef<Path>>(path: P, config: Option<DriveConfig>) -> Result<Self, Error> {
        match GroveDb::open(path) {
            Ok(grove) => {
                let config = config.unwrap_or_default();
                let genesis_time_ms = config.default_genesis_time;
                let data_contracts_global_cache_size = config.data_contracts_global_cache_size;
                let data_contracts_block_cache_size = config.data_contracts_block_cache_size;

                Ok(Drive {
                    grove,
                    config,
                    cache: RefCell::new(DriveCache {
                        cached_contracts: DataContractCache::new(
                            data_contracts_global_cache_size,
                            data_contracts_block_cache_size,
                        ),
                        genesis_time_ms,
                        protocol_versions_counter: None,
                    }),
                })
            }
            Err(e) => Err(Error::GroveDB(e)),
        }
    }

    /// Drops the drive cache
    pub fn drop_cache(&self) {
        let genesis_time_ms = self.config.default_genesis_time;
        let data_contracts_global_cache_size = self.config.data_contracts_global_cache_size;
        let data_contracts_block_cache_size = self.config.data_contracts_block_cache_size;
        self.cache.replace(DriveCache {
            cached_contracts: DataContractCache::new(
                data_contracts_global_cache_size,
                data_contracts_block_cache_size,
            ),
            genesis_time_ms,
            protocol_versions_counter: None,
        });
    }

    /// Commits a transaction.
    pub fn commit_transaction(&self, transaction: Transaction) -> Result<(), Error> {
        self.grove
            .commit_transaction(transaction)
            .unwrap() // TODO: discuss what to do with transaction cost as costs are
            // returned in advance on transaction operations not on commit
            .map_err(Error::GroveDB)
    }

    /// Rolls back a transaction.
    pub fn rollback_transaction(&self, transaction: &Transaction) -> Result<(), Error> {
        self.grove
            .rollback_transaction(transaction)
            .map_err(Error::GroveDB)
    }

    /// Applies a batch of Drive operations to groveDB.
    fn apply_batch_low_level_drive_operations(
        &self,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: Vec<LowLevelDriveOperation>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let grove_db_operations =
            LowLevelDriveOperation::grovedb_operations_batch(&batch_operations);
        self.apply_batch_grovedb_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            grove_db_operations,
            drive_operations,
        )?;
        batch_operations.into_iter().for_each(|op| match op {
            GroveOperation(_) => (),
            _ => drive_operations.push(op),
        });
        Ok(())
    }

    /// Applies a batch of groveDB operations if apply is True, otherwise gets the cost of the operations.
    fn apply_batch_grovedb_operations(
        &self,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: GroveDbOpBatch,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        if let Some(estimated_layer_info) = estimated_costs_only_with_layer_info {
            // Leave this for future debugging
            // for (k, v) in estimated_layer_info.iter() {
            //     let path = k
            //         .to_path()
            //         .iter()
            //         .map(|k| hex::encode(k.as_slice()))
            //         .join("/");
            //     dbg!(path, v);
            // }
            self.grove_batch_operations_costs(
                batch_operations,
                estimated_layer_info,
                false,
                drive_operations,
            )?;
        } else {
            self.grove_apply_batch_with_add_costs(
                batch_operations,
                false,
                transaction,
                drive_operations,
            )?;
        }
        Ok(())
    }

    /// Returns the worst case fee for a contract document type.
    pub fn worst_case_fee_for_document_type_with_name(
        &self,
        contract: &Contract,
        document_type_name: &str,
        epoch_index: u16,
    ) -> Result<FeeResult, Error> {
        let document_type = contract.document_type_for_name(document_type_name)?;
        self.add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentEstimatedAverageSize(document_type.max_size() as u32),
                    owner_id: None,
                },
                contract,
                document_type,
            },
            false,
            BlockInfo::default_with_epoch(Epoch::new(epoch_index)),
            false,
            None,
        )
    }
}
