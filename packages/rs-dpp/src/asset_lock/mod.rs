use crate::asset_lock::reduced_asset_lock_value::ReducedAssetLockValue;

pub mod reduced_asset_lock_value;


pub enum StoredAssetLockInfo {
    Present,
    PresentWithInfo(ReducedAssetLockValue),
    NotPresent,
}
