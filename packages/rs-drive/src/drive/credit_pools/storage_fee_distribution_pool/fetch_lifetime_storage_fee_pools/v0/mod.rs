use crate::drive::credit_pools::epochs::epochs_root_tree_key_constants::KEY_LIFETIME_STORAGE_FEE_POOLS;
use crate::drive::credit_pools::paths::{lifetime_storage_fee_pools_vec_path, pools_path};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::EpochIndex;
use dpp::fee::fee_result::LifetimeStorageFees;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_lifetime_storage_fee_pools_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<EpochIndex, LifetimeStorageFees>, Error> {
        // Only protocol version 14 on reads the pools, and its chains all hold the tree: one
        // without it is corrupted, not empty. A query under a missing tree returns nothing, so
        // the tree is looked up first.
        let tree_exists = self
            .grove
            .has_raw(
                &pools_path(),
                KEY_LIFETIME_STORAGE_FEE_POOLS,
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::from)?;
        if !tree_exists {
            return Err(Error::Drive(DriveError::CorruptedDriveState(
                "the lifetime storage fee pools tree must exist from protocol version 14"
                    .to_string(),
            )));
        }
        let pools = self.fetch_sum_items(
            lifetime_storage_fee_pools_vec_path(),
            "a lifetime storage fee pool must be a sum item",
            transaction,
            &platform_version.drive,
        )?;
        let mut pools_by_epoch = BTreeMap::<EpochIndex, LifetimeStorageFees>::new();
        for (key, credits) in pools {
            let [epoch_high, epoch_low, lifetime_high, lifetime_low]: [u8; 4] =
                key.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "a lifetime storage fee pool must be keyed by an epoch and a lifetime, \
                         two u16"
                            .to_string(),
                    ))
                })?;
            // A pool only ever receives storage fees: a negative one is corrupted, and
            // spreading its absolute value would create credits.
            let credits = u64::try_from(credits).map_err(|_| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "a lifetime storage fee pool must not be negative".to_string(),
                ))
            })?;
            pools_by_epoch
                .entry(u16::from_be_bytes([epoch_high, epoch_low]))
                .or_default()
                .insert(u16::from_be_bytes([lifetime_high, lifetime_low]), credits);
        }
        Ok(pools_by_epoch)
    }
}
