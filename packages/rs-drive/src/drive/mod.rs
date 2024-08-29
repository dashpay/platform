use std::sync::Arc;

#[cfg(any(feature = "server", feature = "verify"))]
use grovedb::GroveDb;
use std::fmt;

#[cfg(any(feature = "server", feature = "verify"))]
use crate::config::DriveConfig;

#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;

#[cfg(any(feature = "server", feature = "verify"))]
pub mod balances;
#[cfg(any(feature = "server", feature = "verify"))]
pub mod constants;
///DataContract module
#[cfg(any(feature = "server", feature = "verify", feature = "fixtures-and-mocks"))]
pub mod contract;
/// Fee pools module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod credit_pools;
/// Document module
#[cfg(any(feature = "server", feature = "verify", feature = "fixtures-and-mocks"))]
pub mod document;

/// Identity module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod identity;
#[cfg(feature = "server")]
pub mod initialization;

/// Protocol upgrade module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod protocol_upgrade;

/// System module
#[cfg(feature = "server")]
pub mod system;

#[cfg(feature = "server")]
mod asset_lock;
#[cfg(feature = "server")]
mod platform_state;
pub(crate) mod prefunded_specialized_balances;

/// Vote module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod votes;

#[cfg(feature = "server")]
mod shared;

#[cfg(feature = "server")]
use crate::cache::DriveCache;
use crate::error::drive::DriveError;
use crate::error::Error;

/// Drive struct
#[cfg(any(feature = "server", feature = "verify"))]
pub struct Drive {
    /// GroveDB
    pub grove: Arc<GroveDb>,

    /// Drive config
    pub config: DriveConfig,

    /// Drive Cache
    #[cfg(feature = "server")]
    pub cache: DriveCache,
}

// The root tree structure is very important!
// It must be constructed in such a way that important information
// is at the top of the tree in order to reduce proof size
// the most import tree is theDataContract Documents tree

//                                                      DataContract_Documents 64
//                                 /                                                                         \
//                       Identities 32                                                                        Balances 96
//             /                            \                                              /                                               \
//   Token_Balances 16                    Pools 48                    WithdrawalTransactions 80                                        Votes  112
//       /      \                           /                                      /                                                    /                          \
//     NUPKH->I 8 UPKH->I 24   PreFundedSpecializedBalances 40          SpentAssetLockTransactions 72                             Misc 104                          Versions 120

/// Keys for the root tree.
#[cfg(any(feature = "server", feature = "verify"))]
#[repr(u8)]
pub enum RootTree {
    // Input data errors
    ///DataContract Documents
    DataContractDocuments = 64,
    /// Identities
    Identities = 32,
    /// Unique Public Key Hashes to Identities
    UniquePublicKeyHashesToIdentities = 24, // UPKH->I above
    /// Non-Unique Public Key Hashes to Identities, useful for Masternode Identities
    NonUniquePublicKeyKeyHashesToIdentities = 8, // NUPKH->I
    /// Pools
    Pools = 48,
    /// PreFundedSpecializedBalances are balances that can fund specific state transitions that match
    /// predefined criteria
    PreFundedSpecializedBalances = 40,
    /// Spent Asset Lock Transactions
    SpentAssetLockTransactions = 72,
    /// Misc
    Misc = 104,
    /// Asset Unlock Transactions
    WithdrawalTransactions = 80,
    /// Balances (For identities)
    Balances = 96,
    /// Token Balances
    TokenBalances = 16,
    /// Versions desired by proposers
    Versions = 120,
    /// Registered votes
    Votes = 112,
}

#[cfg(any(feature = "server", feature = "verify"))]
impl fmt::Display for RootTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let variant_name = match self {
            RootTree::DataContractDocuments => "DataContractAndDocumentsRoot",
            RootTree::Identities => "Identities",
            RootTree::UniquePublicKeyHashesToIdentities => "UniquePublicKeyHashesToIdentities",
            RootTree::NonUniquePublicKeyKeyHashesToIdentities => {
                "NonUniquePublicKeyKeyHashesToIdentities"
            }
            RootTree::Pools => "Pools",
            RootTree::PreFundedSpecializedBalances => "PreFundedSpecializedBalances",
            RootTree::SpentAssetLockTransactions => "SpentAssetLockTransactions",
            RootTree::Misc => "Misc",
            RootTree::WithdrawalTransactions => "WithdrawalTransactions",
            RootTree::Balances => "Balances",
            RootTree::TokenBalances => "TokenBalances",
            RootTree::Versions => "Versions",
            RootTree::Votes => "Votes",
        };
        write!(f, "{}", variant_name)
    }
}

/// Storage cost
#[cfg(feature = "server")]
pub const STORAGE_COST: i32 = 50;

#[cfg(any(feature = "server", feature = "verify"))]
impl From<RootTree> for u8 {
    fn from(root_tree: RootTree) -> Self {
        root_tree as u8
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
impl From<RootTree> for [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        [root_tree as u8]
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
impl TryFrom<u8> for RootTree {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            64 => Ok(RootTree::DataContractDocuments),
            32 => Ok(RootTree::Identities),
            24 => Ok(RootTree::UniquePublicKeyHashesToIdentities),
            8 => Ok(RootTree::NonUniquePublicKeyKeyHashesToIdentities),
            48 => Ok(RootTree::Pools),
            40 => Ok(RootTree::PreFundedSpecializedBalances),
            72 => Ok(RootTree::SpentAssetLockTransactions),
            104 => Ok(RootTree::Misc),
            80 => Ok(RootTree::WithdrawalTransactions),
            96 => Ok(RootTree::Balances),
            16 => Ok(RootTree::TokenBalances),
            120 => Ok(RootTree::Versions),
            112 => Ok(RootTree::Votes),
            _ => Err(Error::Drive(DriveError::NotSupported(
                "unknown root tree item",
            ))),
        }
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
impl From<RootTree> for &'static [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        match root_tree {
            RootTree::Identities => &[32],
            RootTree::DataContractDocuments => &[64],
            RootTree::UniquePublicKeyHashesToIdentities => &[24],
            RootTree::SpentAssetLockTransactions => &[72],
            RootTree::Pools => &[48],
            RootTree::PreFundedSpecializedBalances => &[40],
            RootTree::Misc => &[104],
            RootTree::WithdrawalTransactions => &[80],
            RootTree::Balances => &[96],
            RootTree::TokenBalances => &[16],
            RootTree::NonUniquePublicKeyKeyHashesToIdentities => &[8],
            RootTree::Versions => &[120],
            RootTree::Votes => &[112],
        }
    }
}

/// Returns the path to the identities
#[cfg(feature = "server")]
pub(crate) fn identity_tree_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Identities)]
}

/// Returns the path to the identities as a vec
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn identity_tree_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Identities as u8]]
}

/// Returns the path to the key hashes.
#[cfg(feature = "server")]
pub(crate) fn unique_key_hashes_tree_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(
        RootTree::UniquePublicKeyHashesToIdentities,
    )]
}

/// Returns the path to the key hashes.
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn unique_key_hashes_tree_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::UniquePublicKeyHashesToIdentities as u8]]
}

/// Returns the path to the masternode key hashes.
#[cfg(feature = "server")]
pub(crate) fn non_unique_key_hashes_tree_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(
        RootTree::NonUniquePublicKeyKeyHashesToIdentities,
    )]
}

/// Returns the path to the masternode key hashes.
#[cfg(feature = "server")]
pub(crate) fn non_unique_key_hashes_tree_path_vec() -> Vec<Vec<u8>> {
    vec![vec![
        RootTree::NonUniquePublicKeyKeyHashesToIdentities as u8,
    ]]
}

/// Returns the path to the masternode key hashes sub tree.
#[cfg(feature = "server")]
pub(crate) fn non_unique_key_hashes_sub_tree_path(public_key_hash: &[u8]) -> [&[u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::NonUniquePublicKeyKeyHashesToIdentities),
        public_key_hash,
    ]
}

/// Returns the path to the masternode key hashes sub tree.
#[cfg(feature = "server")]
pub(crate) fn non_unique_key_hashes_sub_tree_path_vec(public_key_hash: [u8; 20]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::NonUniquePublicKeyKeyHashesToIdentities as u8],
        public_key_hash.to_vec(),
    ]
}

/// Returns the path to a contract's document types.
#[cfg(feature = "server")]
fn contract_documents_path(contract_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        contract_id,
        &[1],
    ]
}
