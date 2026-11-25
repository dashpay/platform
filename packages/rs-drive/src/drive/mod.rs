use std::sync::Arc;

#[cfg(feature = "server")]
use std::collections::BTreeMap;
#[cfg(feature = "server")]
use std::path::{Path, PathBuf};

#[cfg(any(feature = "server", feature = "verify"))]
use crate::config::DriveConfig;
#[cfg(feature = "server")]
use arc_swap::ArcSwap;
#[cfg(feature = "server")]
use dpp::prelude::{BlockHeight, TimestampMillis};
#[cfg(any(feature = "server", feature = "verify"))]
use grovedb::GroveDb;
use std::fmt;
#[cfg(feature = "full")]
use tempfile::TempDir;

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
#[cfg(all(feature = "server", any(test, feature = "structure")))]
pub(crate) mod structure;

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
/// The saved platform state: its record, the small per-block record, and the
/// collections kept as one aux entry per member.
pub mod platform_state;

/// Prefunded specialized balances module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod prefunded_specialized_balances;

/// Vote module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod votes;

/// Contract groups: identity-owned sets of contracts, document types and tokens.
#[cfg(any(feature = "server", feature = "verify"))]
pub mod contract_groups;
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
    /// The GroveDb instance for this checkpoint. It is private, so that it cannot be moved
    /// out of the checkpoint and stay open while the directory is deleted.
    grove_db: GroveDb,
    /// The checkpoint's directory, deleted on drop once marked. Fields drop in declaration
    /// order, so this one stays after `grove_db`: the database is closed before its
    /// directory is deleted, and a storage engine never has files open in a directory that
    /// is going away.
    directory: CheckpointDirectory,
}

/// The directory of a [`Checkpoint`], deleted when it is dropped if it was marked.
#[cfg(feature = "server")]
struct CheckpointDirectory {
    /// The path to the checkpoint directory
    path: PathBuf,
    /// Whether to delete the checkpoint directory on drop
    marked_for_deletion: std::sync::atomic::AtomicBool,
}

#[cfg(feature = "server")]
impl Checkpoint {
    /// Creates a new Checkpoint with the given GroveDb and path.
    /// By default, the checkpoint is not marked for deletion.
    ///
    /// `path` must be the directory `grove_db` was opened at: once the checkpoint is marked,
    /// dropping it deletes the database at that path.
    pub fn new(grove_db: GroveDb, path: PathBuf) -> Self {
        Self {
            grove_db,
            directory: CheckpointDirectory {
                path,
                marked_for_deletion: std::sync::atomic::AtomicBool::new(false),
            },
        }
    }

    /// The checkpoint's database.
    pub fn grove_db(&self) -> &GroveDb {
        &self.grove_db
    }

    /// Marks this checkpoint for deletion when it is dropped.
    pub fn mark_for_deletion(&self) {
        self.directory
            .marked_for_deletion
            .store(true, std::sync::atomic::Ordering::Release);
    }

    /// Deletes the directory of a checkpoint of this node on which no database is open.
    ///
    /// A directory that is not there is left so: a storage engine that is asked to open a
    /// missing path can make the directory, and an empty directory named after a height
    /// would be loaded as a checkpoint at the next start. A symbolic link is unlinked, and
    /// what it names is neither opened nor changed. Where the path cannot be looked at,
    /// for another reason than that nothing is there, that error is returned and no storage
    /// engine is asked. GroveDB deletes the checkpoint
    /// where it can open it as a database of its storage engine. Where it cannot, because
    /// the checkpoint is damaged or was left half made, the directory is removed all the
    /// same: it is this node's own, and what is left in it would only stand in the way of
    /// the next checkpoint at that height.
    pub fn delete_closed(path: &Path) -> std::io::Result<()> {
        let gone = |error: &std::io::Error| error.kind() == std::io::ErrorKind::NotFound;
        match std::fs::symlink_metadata(path) {
            Err(error) if gone(&error) => return Ok(()),
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return match std::fs::remove_file(path) {
                    Err(error) if !gone(&error) => Err(error),
                    _ => Ok(()),
                };
            }
            Err(error) => return Err(error),
            Ok(_) => {}
        }
        let Err(refused) = GroveDb::delete_checkpoint(path) else {
            return Ok(());
        };
        tracing::warn!(
            error = ?refused,
            ?path,
            "grovedb did not delete a checkpoint; removing its directory"
        );
        match std::fs::remove_dir_all(path) {
            Err(error) if !gone(&error) => Err(error),
            _ => Ok(()),
        }
    }
}

#[cfg(feature = "server")]
impl Drop for CheckpointDirectory {
    fn drop(&mut self) {
        // Only clean up the checkpoint directory if marked for deletion
        if !self
            .marked_for_deletion
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return;
        }
        if let Err(error) = Checkpoint::delete_closed(&self.path) {
            tracing::error!(
                ?error,
                path = ?self.path,
                "failed to delete a pruned grovedb checkpoint"
            );
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

    /// The temporary directory holding the database, when a test helper such as
    /// [`setup_drive`](crate::util::test_helpers::setup::setup_drive) created one; `None` for a
    /// Drive opened at a caller's path. RocksDB creates files in this directory for as long as
    /// the database is open (memtable flushes, new log files, compactions), so the Drive owns
    /// it and it is removed only when the Drive is dropped. Fields drop in declaration order,
    /// so this one stays last: `grove` is dropped before the directory is removed.
    #[cfg(feature = "full")]
    pub(crate) temp_dir: Option<TempDir>,
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
//                                     /                          /                                                                                                                                         \
//                           Saved Block Transactions 36       ShieldedBalances 52                                                                                                                     ContractGroups 124
//
// This is the shape of a fresh chain. `drive::structure` describes every level below the root as
// code, and `packages/rs-drive/grovedb-structure.json` records this shape from a real GroveDB
// (`layer_shapes.root`), so a test fails when the two drift apart.

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
    /// Contract groups: identity-owned sets of contracts, contract document types and contract
    /// tokens, with a backwards index from each member contract to its groups (protocol
    /// version 14).
    ContractGroups = 124,
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
            RootTree::ContractGroups => "ContractGroups",
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
            88 => Ok(RootTree::GroupActions),
            124 => Ok(RootTree::ContractGroups),
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
            RootTree::ContractGroups => &[124],
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

#[cfg(all(test, feature = "server"))]
mod checkpoint_tests {
    use super::Checkpoint;
    use grovedb::GroveDb;
    use std::path::{Path, PathBuf};

    /// A database with a checkpoint of it at `<directory>/checkpoints/7`, opened.
    fn checkpoint_in(directory: &Path) -> (GroveDb, Checkpoint, PathBuf) {
        let live = GroveDb::open(directory.join("db")).expect("a database");
        let path = directory.join("checkpoints").join("7");
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the parent");
        live.create_checkpoint(&path).expect("a checkpoint");
        let checkpoint =
            Checkpoint::new(GroveDb::open(&path).expect("the checkpoint"), path.clone());
        (live, checkpoint, path)
    }

    #[test]
    fn a_checkpoint_keeps_its_directory_unless_it_is_marked() {
        let directory = tempfile::tempdir().expect("a directory");
        let (_live, checkpoint, path) = checkpoint_in(directory.path());
        drop(checkpoint);
        assert!(path.is_dir(), "an unmarked checkpoint stays on disk");

        let checkpoint =
            Checkpoint::new(GroveDb::open(&path).expect("the checkpoint"), path.clone());
        checkpoint.mark_for_deletion();
        assert!(path.is_dir(), "a marked checkpoint stays while it is held");
        drop(checkpoint);
        assert!(
            !path.exists(),
            "a marked checkpoint goes when it is dropped"
        );
    }

    /// A marked checkpoint whose directory somebody removed is not made again by its drop:
    /// a directory named after a height would be loaded as a checkpoint at the next start.
    #[test]
    fn a_marked_checkpoint_whose_directory_is_gone_leaves_none() {
        let directory = tempfile::tempdir().expect("a directory");
        let (_live, checkpoint, path) = checkpoint_in(directory.path());
        checkpoint.mark_for_deletion();
        std::fs::remove_dir_all(&path).expect("the directory removed");
        drop(checkpoint);
        assert!(!path.exists(), "the drop made the directory again");

        Checkpoint::delete_closed(&path).expect("nothing to delete");
        assert!(!path.exists(), "the deletion made the directory");
    }

    /// A directory that holds a file of the node and no database, as a checkpoint whose
    /// creation was cut short can be, cannot be opened as a checkpoint, and goes all the
    /// same.
    #[test]
    fn a_closed_checkpoint_that_cannot_be_opened_is_deleted_all_the_same() {
        let directory = tempfile::tempdir().expect("a directory");
        let (_live, checkpoint, path) = checkpoint_in(directory.path());
        drop(checkpoint);
        std::fs::remove_dir_all(&path).expect("the checkpoint removed");
        std::fs::create_dir(&path).expect("a directory in its place");
        std::fs::write(path.join("platform_state.bin"), b"state").expect("a file of the node");
        assert!(
            GroveDb::open_checkpoint(&path).is_err(),
            "a directory with no database was opened as a checkpoint"
        );

        Checkpoint::delete_closed(&path).expect("the directory removed");
        assert!(!path.exists());
    }

    /// A symbolic link at a checkpoint's path is unlinked. What it names is not opened as
    /// a database, so nothing is written there.
    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_is_unlinked_and_what_it_names_is_left_as_it_was() {
        let directory = tempfile::tempdir().expect("a directory");
        let named = directory.path().join("named");
        std::fs::create_dir(&named).expect("the directory the link names");
        let path = directory.path().join("8");
        std::os::unix::fs::symlink(&named, &path).expect("a link");

        Checkpoint::delete_closed(&path).expect("the link removed");
        assert_eq!(
            std::fs::symlink_metadata(&path)
                .expect_err("the link is still there")
                .kind(),
            std::io::ErrorKind::NotFound
        );
        assert_eq!(
            std::fs::read_dir(&named)
                .expect("the named directory")
                .count(),
            0,
            "the directory the link names was written to"
        );
    }
}
