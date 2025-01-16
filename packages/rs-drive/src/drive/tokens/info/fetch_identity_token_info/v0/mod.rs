use crate::drive::tokens::paths::token_identity_infos_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::serialization::PlatformDeserializable;
use dpp::tokens::info::IdentityTokenInfo;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    pub(super) fn fetch_identity_token_info_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IdentityTokenInfo>, Error> {
        self.fetch_identity_token_info_operations_v0(
            token_id,
            identity_id,
            true,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_identity_token_info_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IdentityTokenInfo>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTargetValue(8),
            }
        };

        let info_path = token_identity_infos_path(&token_id);

        match self.grove_get_raw_optional(
            (&info_path).into(),
            identity_id.as_slice(),
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(info, _))) => Ok(Some(IdentityTokenInfo::deserialize_from_bytes(
                info.as_slice(),
            )?)),

            Ok(None) | Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_))) => Ok(None),

            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity token info was present but was not an item",
            ))),

            Err(e) => Err(e),
        }
    }
}
