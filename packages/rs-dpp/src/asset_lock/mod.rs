use crate::asset_lock::reduced_asset_lock_value::AssetLockValue;

pub mod reduced_asset_lock_value;

pub type PastAssetLockStateTransitionHashes = Vec<Vec<u8>>;

/// An enumeration of the possible states when querying platform to get the stored state of an outpoint
/// representing if the asset lock was already used or not.
pub enum StoredAssetLockInfo {
    /// The asset lock was fully consumed in the past
    FullyConsumed,
    /// The asset lock was partially consumed, and we stored the asset lock value in the state
    PartiallyConsumed(AssetLockValue),
    /// The asset lock is not yet known to Platform
    NotPresent,
}
