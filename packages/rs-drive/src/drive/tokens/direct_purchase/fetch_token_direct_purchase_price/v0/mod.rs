use crate::drive::tokens::paths::token_direct_purchase_root_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::serialization::PlatformDeserializable;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    pub(super) fn fetch_token_direct_purchase_price_v0(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenPricingSchedule>, Error> {
        self.fetch_token_direct_purchase_price_operations_v0(
            token_id,
            true,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_token_direct_purchase_price_operations_v0(
        &self,
        token_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenPricingSchedule>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTargetValue(8),
            }
        };

        let token_direct_purchase_prices_root_path = token_direct_purchase_root_path();

        match self.grove_get_raw_optional(
            (&token_direct_purchase_prices_root_path).into(),
            &token_id,
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(info, _))) => Ok(Some(TokenPricingSchedule::deserialize_from_bytes(
                info.as_slice(),
            )?)),

            Ok(None) | Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_))) => Ok(None),

            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "token direct_purchase_price was present but was not an item",
            ))),

            Err(e) => Err(e),
        }
    }
}
