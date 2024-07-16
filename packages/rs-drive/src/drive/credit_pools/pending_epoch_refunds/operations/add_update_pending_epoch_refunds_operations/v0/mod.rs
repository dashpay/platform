use crate::drive::credit_pools::pending_epoch_refunds::pending_epoch_refunds_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::{DriveOperation, GroveDbOpBatch};
use dpp::balances::credits::Creditable;
use dpp::fee::epoch::CreditsPerEpoch;
use grovedb::Element;

impl Drive {
    /// Adds GroveDB batch operations to update pending epoch storage pool updates
    pub(super) fn add_update_pending_epoch_refunds_operations_v0(
        batch: &mut Vec<DriveOperation>,
        refunds_per_epoch: CreditsPerEpoch,
    ) -> Result<(), Error> {
        if !refunds_per_epoch.is_empty() {
            let mut inner_batch = GroveDbOpBatch::new();
            for (epoch_index, credits) in refunds_per_epoch {
                let epoch_index_key = epoch_index.to_be_bytes().to_vec();

                let element = Element::new_sum_item(-credits.to_signed()?);

                inner_batch.add_insert(pending_epoch_refunds_path_vec(), epoch_index_key, element);
            }

            batch.push(DriveOperation::GroveDBOpBatch(inner_batch));
        }

        Ok(())
    }
}
