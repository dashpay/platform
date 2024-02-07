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
use std::sync::RwLock;

#[cfg(any(feature = "full", feature = "verify"))]
use grovedb::GroveDb;

#[cfg(any(feature = "full", feature = "verify"))]
use crate::drive::config::DriveConfig;

#[cfg(feature = "full")]
use crate::fee::op::LowLevelDriveOperation;

#[cfg(any(feature = "full", feature = "verify"))]
pub mod balances;
/// Batch module
#[cfg(feature = "full")]
pub mod batch;
/// Drive Cache
#[cfg(any(feature = "full", feature = "verify"))]
pub mod cache;
#[cfg(any(feature = "full", feature = "verify"))]
pub mod config;
///DataContract module
#[cfg(any(feature = "full", feature = "verify", feature = "fixtures-and-mocks"))]
pub mod contract;
/// Fee pools module
#[cfg(any(feature = "full", feature = "verify"))]
pub mod credit_pools;
#[cfg(any(feature = "full", feature = "verify"))]
pub mod defaults;
/// Document module
#[cfg(any(feature = "full", feature = "verify", feature = "fixtures-and-mocks"))]
pub mod document;
#[cfg(any(feature = "full", feature = "verify"))]
pub mod flags;

/// Low level GroveDB operations
#[cfg(feature = "full")]
pub mod grove_operations;
/// Identity module
#[cfg(any(feature = "full", feature = "verify"))]
pub mod identity;
#[cfg(feature = "full")]
pub mod initialization;
#[cfg(feature = "full")]
pub mod object_size_info;

/// Protocol upgrade module
#[cfg(any(feature = "full", feature = "verify"))]
pub mod protocol_upgrade;
#[cfg(feature = "full")]
mod shared_estimation_costs;
#[cfg(feature = "full")]
mod system;
#[cfg(test)]
mod test_utils;

#[cfg(feature = "full")]
mod asset_lock;
#[cfg(feature = "full")]
pub(crate) mod fee;
#[cfg(feature = "full")]
mod open;
#[cfg(feature = "full")]
mod operations;
#[cfg(feature = "full")]
mod platform_state;
#[cfg(feature = "full")]
mod prove;
/// Contains a set of useful grovedb proof verification functions
#[cfg(any(feature = "full", feature = "verify"))]
pub mod verify;

#[cfg(feature = "full")]
use crate::drive::cache::DriveCache;

/// Drive struct
#[cfg(any(feature = "full", feature = "verify"))]
pub struct Drive {
    /// GroveDB
    pub grove: GroveDb,
    /// Drive config
    pub config: DriveConfig,
    /// Drive Cache
    #[cfg(feature = "full")]
    pub cache: RwLock<DriveCache>,
}

// The root tree structure is very important!
// It must be constructed in such a way that important information
// is at the top of the tree in order to reduce proof size
// the most import tree is theDataContract Documents tree

//                        DataContract_Documents 64
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
    ///DataContract Documents
    DataContractDocuments = 64,
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
            RootTree::DataContractDocuments => &[64],
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
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
        &[1],
    ]
}
