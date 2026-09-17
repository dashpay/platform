use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_for_readiness_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use grovedb::Element::SumItem;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    pub(super) fn fetch_readiness_fund_operations_v0(
        &self,
        fund_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Credits>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            // 8 is the size of a i64 used in sum trees
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::SumTree,
                query_target: QueryTargetValue(8),
            }
        };

        let path = prefunded_specialized_balances_for_readiness_path();

        match self.grove_get_raw_optional(
            (&path).into(),
            fund_id.as_slice(),
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(SumItem(balance, _))) if balance >= 0 => Ok(Some(balance as Credits)),
            Ok(None) => {
                if apply {
                    Ok(None)
                } else {
                    Ok(Some(0))
                }
            }
            Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathKeyNotFound(_)) => {
                if apply {
                    Ok(None)
                } else {
                    Ok(Some(0))
                }
            }
            Ok(Some(SumItem(..))) => Err(Error::Drive(DriveError::CorruptedElementType(
                "readiness fund was present but was negative",
            ))),
            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "readiness fund was present but was not identified as a sum item",
            ))),
            Err(e) => Err(e),
        }
    }
}
