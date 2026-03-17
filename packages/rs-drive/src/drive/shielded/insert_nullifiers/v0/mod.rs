use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::{Element, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 0 implementation of inserting nullifiers.
    ///
    /// For each nullifier:
    /// 1. Inserts into the permanent nullifiers tree to prevent double-spend
    ///
    /// Then stores all nullifiers into per-block sync storage for catch-up RPCs.
    pub(in crate::drive) fn insert_nullifiers_v0(
        &self,
        nullifiers: &[[u8; 32]],
        block_height: u64,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let nullifiers_path = shielded_credit_pool_nullifiers_path_vec();

        // Build permanent nullifier tree ops
        let ops = nullifiers
            .iter()
            .map(|nullifier| {
                GroveOperation(
                    QualifiedGroveDbOp::insert_only_known_to_not_already_exist_op(
                        nullifiers_path.clone(),
                        nullifier.to_vec(),
                        Element::new_item(vec![]),
                    ),
                )
            })
            .collect();

        // Store to per-block sync storage for catch-up RPCs
        //
        // SAFETY: Both the permanent nullifier ops (returned for batch application)
        // and this sync storage write use the same GroveDB transaction. If the block
        // is rolled back (transaction aborted), both are rolled back atomically.
        // There is no inconsistency window because GroveDB transactions are atomic.
        self.store_nullifiers_for_block(
            nullifiers,
            block_height,
            block_time_ms,
            transaction,
            platform_version,
        )?;

        Ok(ops)
    }
}
