use crate::drive::tokens::paths::token_balances_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::balances::credits::TokenAmount;
use dpp::version::PlatformVersion;
use grovedb::Element::SumItem;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    pub(super) fn fetch_identity_token_balance_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenAmount>, Error> {
        self.fetch_identity_token_balance_operations_v0(
            token_id,
            identity_id,
            true,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_identity_token_balance_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenAmount>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            // 8 is the size of an i64 used in sum trees
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::SumTree,
                query_target: QueryTargetValue(8),
            }
        };

        let balance_path = token_balances_path(&token_id);

        match self.grove_get_raw_optional(
            (&balance_path).into(),
            identity_id.as_slice(),
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(SumItem(balance, _))) if balance >= 0 => Ok(Some(balance as TokenAmount)),

            Ok(None) | Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_))) => {
                // If we are applying (stateful), no balance means None.
                // If we are estimating (stateless), return Some(0) to indicate no cost or minimal cost scenario.
                if apply {
                    Ok(None)
                } else {
                    Ok(Some(0))
                }
            }

            Ok(Some(SumItem(..))) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity token balance was present but was negative",
            ))),

            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity token balance was present but was not a sum item",
            ))),

            Err(e) => Err(e),
        }
    }
}
