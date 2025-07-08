use grovedb::batch::QualifiedGroveDbOp;
use grovedb::Element;

use crate::drive::Drive;
use crate::error::Error;

use crate::drive::credit_pools::epochs::epoch_key_constants;
use crate::drive::credit_pools::epochs::paths::EpochProposers;
use dpp::block::epoch::Epoch;
use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
use dpp::serialization::PlatformSerializable;

impl Drive {
    /// Serializes and stores the epoch final info
    pub(super) fn add_epoch_final_info_operation_v0(
        &self,
        epoch: &Epoch,
        finalized_epoch_info: FinalizedEpochInfo,
    ) -> Result<QualifiedGroveDbOp, Error> {
        let epoch_tree_path = epoch.get_path_vec();

        let serialized = finalized_epoch_info.serialize_consume_to_bytes()?;

        Ok(QualifiedGroveDbOp::insert_or_replace_op(
            epoch_tree_path,
            epoch_key_constants::KEY_FINISHED_EPOCH_INFO.to_vec(),
            Element::new_item(serialized),
        ))
    }
}
