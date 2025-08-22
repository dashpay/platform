use crate::drive::tokens::paths::token_statuses_root_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::serialization::PlatformDeserializable;
use dpp::tokens::status::TokenStatus;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    pub(super) fn fetch_token_status_v0(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenStatus>, Error> {
        self.fetch_token_status_operations_v0(
            token_id,
            true,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_token_status_operations_v0(
        &self,
        token_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenStatus>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTargetValue(8),
            }
        };

        let token_statuses_root_path = token_statuses_root_path();

        match self.grove_get_raw_optional(
            (&token_statuses_root_path).into(),
            &token_id,
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(info, _))) => {
                Ok(Some(TokenStatus::deserialize_from_bytes(info.as_slice())?))
            }

            Ok(None) | Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_))) => Ok(None),

            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "token status was present but was not an item",
            ))),

            Err(e) => Err(e),
        }
    }
}
