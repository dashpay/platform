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

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use dpp::data_contract::DriveContractExt;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, GroveDb, Transaction, TransactionArg};

use object_size_info::DocumentAndContractInfo;
use object_size_info::DocumentInfo::DocumentEstimatedAverageSize;

use crate::contract::Contract;
use crate::drive::batch::GroveDbOpBatch;
use crate::drive::config::DriveConfig;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use crate::fee::op::DriveOperation::GroveOperation;

/// Batch module
pub mod batch;
/// Block info module
pub mod block_info;
/// Drive Cache
pub mod cache;
pub mod config;
/// Contract module
pub mod contract;
pub mod defaults;
/// Document module
pub mod document;
mod estimation_costs;
/// Fee pools module
pub mod fee_pools;
pub mod flags;
/// Genesis time module
pub mod genesis_time;
pub(crate) mod grove_operations;
/// Identity module
pub mod identity;
pub mod initialization;
pub mod object_size_info;
pub mod query;
#[cfg(test)]
mod test_utils;

use crate::drive::block_info::BlockInfo;
use crate::drive::cache::{DataContractCache, DriveCache};
use crate::drive::object_size_info::OwnedDocumentInfo;
use crate::fee::result::FeeResult;
use crate::fee_pools::epochs::Epoch;

/// Drive struct
pub struct Drive {
    /// GroveDB
    pub grove: GroveDb,
    /// Drive config
    pub config: DriveConfig,
    /// Drive Cache
    pub cache: RefCell<DriveCache>,
}

/// Keys for the root tree.
#[repr(u8)]
pub enum RootTree {
    // Input data errors
    /// Identities
    Identities = 0,
    /// Contract Documents
    ContractDocuments = 1,
    /// Public Key Hashes to Identities
    PublicKeyHashesToIdentities = 2,
    /// Spent Asset Lock Transactions
    SpentAssetLockTransactions = 3,
    /// Pools
    Pools = 4,
    /// Misc
    Misc = 5,
    /// Asset Unlock Transactions
    WithdrawalTransactions = 6,
}

/// Storage cost
pub const STORAGE_COST: i32 = 50;

impl From<RootTree> for u8 {
    fn from(root_tree: RootTree) -> Self {
        root_tree as u8
    }
}

impl From<RootTree> for [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        [root_tree as u8]
    }
}

impl From<RootTree> for &'static [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        match root_tree {
            RootTree::Identities => &[0],
            RootTree::ContractDocuments => &[1],
            RootTree::PublicKeyHashesToIdentities => &[2],
            RootTree::SpentAssetLockTransactions => &[3],
            RootTree::Pools => &[4],
            RootTree::Misc => &[5],
            RootTree::WithdrawalTransactions => &[6],
        }
    }
}

/// Returns the path to a contract's document types.
fn contract_documents_path(contract_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
        contract_id,
        &[1],
    ]
}

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
    fn apply_batch_drive_operations(
        &self,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: Vec<DriveOperation>,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let grove_db_operations = DriveOperation::grovedb_operations_batch(&batch_operations);
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
        drive_operations: &mut Vec<DriveOperation>,
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
