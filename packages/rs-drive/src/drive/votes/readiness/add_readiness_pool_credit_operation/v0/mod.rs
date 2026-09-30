use crate::drive::credit_pools::epochs::epoch_key_constants::KEY_POOL_PROCESSING_FEES;
use crate::drive::credit_pools::epochs::paths::EpochProposers;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use crate::util::type_constants::U64_SIZE_U32;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::{Element, TransactionArg, TreeType};

impl Drive {
    pub(super) fn add_readiness_pool_credit_operation_v0(
        &self,
        block_info: &BlockInfo,
        amount: Credits,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<LowLevelDriveOperation, Error> {
        let epoch_tree_path = block_info.epoch.get_path();
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::SumTree,
                query_target: QueryTargetValue(U64_SIZE_U32),
            }
        };
        let existing_value = match self.grove_get_raw_optional(
            (&epoch_tree_path).into(),
            KEY_POOL_PROCESSING_FEES.as_slice(),
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        )? {
            None => 0,
            Some(Element::SumItem(existing_value, _)) => existing_value,
            Some(_) => {
                return Err(Error::Drive(DriveError::UnexpectedElementType(
                    "epochs processing fee must be a sum item",
                )))
            }
        };
        if amount > i64::MAX as u64 {
            return Err(Error::Protocol(Box::new(ProtocolError::Overflow(
                "adding over i64::MAX to the processing fee pool",
            ))));
        }
        let updated_value = if apply {
            existing_value
                .checked_add(amount as i64)
                .ok_or(ProtocolError::Overflow(
                    "overflow when adding to the processing fee pool",
                ))?
        } else {
            i64::MAX
        };
        Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
            epoch_tree_path
                .iter()
                .map(|segment| segment.to_vec())
                .collect(),
            KEY_POOL_PROCESSING_FEES.to_vec(),
            Element::new_sum_item(updated_value),
        ))
    }
}
