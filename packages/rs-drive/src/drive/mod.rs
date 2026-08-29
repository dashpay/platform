use std::sync::Arc;

#[cfg(feature = "server")]
use std::collections::BTreeMap;
#[cfg(feature = "server")]
use std::path::PathBuf;

#[cfg(any(feature = "server", feature = "verify"))]
use crate::config::DriveConfig;
#[cfg(feature = "server")]
use arc_swap::ArcSwap;
#[cfg(feature = "server")]
use dpp::prelude::{BlockHeight, TimestampMillis};
#[cfg(any(feature = "server", feature = "verify"))]
use grovedb::GroveDb;
use std::fmt;

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

/// Prefunded specialized balances module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod prefunded_specialized_balances;

/// Vote module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod votes;

/// Group module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod group;
#[cfg(feature = "server")]
mod shared;

/// Address funds module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod address_funds;
/// Saved block transactions module
#[cfg(feature = "server")]
pub mod saved_block_transactions;
/// Shielded pools module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod shielded;
/// Token module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod tokens;

#[cfg(feature = "server")]
use crate::cache::DriveCache;
use crate::error::drive::DriveError;
use crate::error::Error;

/// A checkpoint wrapping a GroveDb instance that can optionally clean up its directory on drop.
#[cfg(feature = "server")]
pub struct Checkpoint {
    /// The GroveDb instance for this checkpoint
    pub grove_db: GroveDb,
    /// The path to the checkpoint directory (for cleanup on drop)
    path: PathBuf,
    /// Whether to delete the checkpoint directory when this checkpoint is dropped
    marked_for_deletion: std::sync::atomic::AtomicBool,
}

#[cfg(feature = "server")]
impl Checkpoint {
    /// Creates a new Checkpoint with the given GroveDb and path.
    /// By default, the checkpoint is not marked for deletion.
    pub fn new(grove_db: GroveDb, path: PathBuf) -> Self {
        Self {
            grove_db,
            path,
            marked_for_deletion: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Returns true if this checkpoint contains the reduced platform state
    /// (`Misc/reduced_saved_state`), which a state-syncing node needs to reconstruct the
    /// platform state. Checkpoints taken before the protocol version that introduced the
    /// reduced state lack the key and cannot be offered as state sync snapshots.
    pub fn has_reduced_platform_state(
        &self,
        grove_version: &grovedb_version::version::GroveVersion,
    ) -> Result<bool, Error> {
        self.grove_db
            .get_raw_optional(
                (&crate::drive::system::misc_path()).into(),
                crate::drive::platform_state::REDUCED_PLATFORM_STATE_KEY,
                None,
                grove_version,
            )
            .unwrap()
            .map(|maybe_element| maybe_element.is_some())
            .map_err(Error::from)
    }

    /// Marks this checkpoint for deletion when it is dropped.
    pub fn mark_for_deletion(&self) {
        self.marked_for_deletion
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

#[cfg(feature = "server")]
impl Drop for Checkpoint {
    fn drop(&mut self) {
        // Only clean up the checkpoint directory if marked for deletion
        if self
            .marked_for_deletion
            .load(std::sync::atomic::Ordering::Acquire)
        {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// Information about a checkpoint including the database
#[cfg(feature = "server")]
#[derive(Clone)]
pub struct CheckpointInfo {
    /// The timestamp when this checkpoint was created
    pub timestamp_ms: TimestampMillis,
    /// The checkpoint database
    pub checkpoint: Arc<Checkpoint>,
}

#[cfg(feature = "server")]
impl CheckpointInfo {
    /// Creates a new CheckpointInfo with the given timestamp and checkpoint
    pub fn new(timestamp_ms: TimestampMillis, checkpoint: Arc<Checkpoint>) -> Self {
        Self {
            timestamp_ms,
            checkpoint,
        }
    }
}

/// Type alias for the checkpoints map wrapped in ArcSwap for atomic updates
#[cfg(feature = "server")]
pub type CheckpointsMap = ArcSwap<BTreeMap<BlockHeight, CheckpointInfo>>;

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

    /// Drive Checkpoints
    #[cfg(feature = "server")]
    pub checkpoints: CheckpointsMap,
}

// The root tree structure is very important!
// It must be constructed in such a way that important information
// is at the top of the tree in order to reduce proof size
// the most import tree is theDataContract Documents tree

//                                                                                DataContract_Documents 64
//                                 /                                                                                                       \
//                       Identities 32                                                                                                 Balances 96
//             /                            \                                                                        /                                                       \
//       Tokens 16                    Pools 48                                                    WithdrawalTransactions 80                                                Votes  112
//       /      \                           /                     \                                         /                           \                            /                          \
//     NUPKH->I 8 UPKH->I 24   PreFundedSpecializedBalances 40  AddressBalances 56              SpentAssetLockTransactions 72    GroupActions 88             Misc 104                        Versions 120
//                                     /                          /
//                           Saved Block Transactions 36       ShieldedBalances 52

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
    /// Saved Block Transactions contains address based transactions that we save for sync purposes
    SavedBlockTransactions = 36,
    /// Shielded credit pools (SumTree of shielded pool subtrees; only the main
    /// pool exists today, but the layout reserves siblings for future pools).
    /// Kept separate from `AddressBalances` so that per-pool internal trees
    /// (notes, nullifiers, anchors, …) cannot contaminate the address-credit
    /// aggregate via sum propagation.
    ShieldedBalances = 52,
    /// Address Balances
    AddressBalances = 56,
    /// Spent Asset Lock Transactions
    SpentAssetLockTransactions = 72,
    /// Misc
    Misc = 104,
    /// Asset Unlock Transactions
    WithdrawalTransactions = 80,
    /// Balances (For identities)
    Balances = 96,
    /// Token Balances and Info
    Tokens = 16,
    /// Versions desired by proposers
    Versions = 120,
    /// Registered votes
    Votes = 112,
    /// Group actions
    GroupActions = 88,
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
            RootTree::SavedBlockTransactions => "SavedBlockTransactions",
            RootTree::ShieldedBalances => "ShieldedBalances",
            RootTree::AddressBalances => "SingleUseKeyBalances",
            // RootTree::MasternodeLists => "MasternodeLists"
            RootTree::SpentAssetLockTransactions => "SpentAssetLockTransactions",
            RootTree::Misc => "Misc",
            RootTree::WithdrawalTransactions => "WithdrawalTransactions",
            RootTree::Balances => "Balances",
            RootTree::Tokens => "TokenBalances",
            RootTree::Versions => "Versions",
            RootTree::Votes => "Votes",
            RootTree::GroupActions => "GroupActions",
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
            // 56 => Ok(RootTree::MasternodeLists), //todo (reserved)
            40 => Ok(RootTree::PreFundedSpecializedBalances),
            36 => Ok(RootTree::SavedBlockTransactions),
            52 => Ok(RootTree::ShieldedBalances),
            72 => Ok(RootTree::SpentAssetLockTransactions),
            104 => Ok(RootTree::Misc),
            80 => Ok(RootTree::WithdrawalTransactions),
            96 => Ok(RootTree::Balances),
            16 => Ok(RootTree::Tokens),
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
            RootTree::SavedBlockTransactions => &[36],
            RootTree::ShieldedBalances => &[52],
            RootTree::AddressBalances => &[56],
            RootTree::Misc => &[104],
            RootTree::WithdrawalTransactions => &[80],
            RootTree::Balances => &[96],
            RootTree::Tokens => &[16],
            RootTree::NonUniquePublicKeyKeyHashesToIdentities => &[8],
            RootTree::Versions => &[120],
            RootTree::Votes => &[112],
            RootTree::GroupActions => &[88],
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
#[cfg(any(feature = "server", feature = "verify"))]
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
#[cfg(any(feature = "server", feature = "verify"))]
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
