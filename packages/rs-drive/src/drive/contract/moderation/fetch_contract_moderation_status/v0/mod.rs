use crate::drive::contract::moderation::types::{decode_ban, decode_suspension, decode_warnings};
use crate::drive::contract::paths::contract_moderation_list_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::data_contract::config::moderation::{ContractModerationList, ContractModerationStatus};
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_contract_moderation_status_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        lists: &[ContractModerationList],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ContractModerationStatus, Error> {
        let mut status = ContractModerationStatus::default();
        for list in lists {
            let path = contract_moderation_list_path(contract_id.as_slice(), *list);
            let entry = self.grove_get_raw_optional_item(
                (&path).into(),
                identity_id.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                &platform_version.drive,
            )?;
            let malformed = |description: String| {
                Error::Drive(DriveError::CorruptedDriveState(format!(
                    "entry of {} on contract {} {} is malformed: {}",
                    identity_id, contract_id, list, description
                )))
            };
            match (list, entry) {
                (_, None) => {}
                (ContractModerationList::Banlist, Some(value)) => {
                    status.ban = Some(decode_ban(&value).map_err(malformed)?);
                }
                (ContractModerationList::Suspensions, Some(value)) => {
                    status.suspension = Some(decode_suspension(&value).map_err(malformed)?);
                }
                (ContractModerationList::Warnings, Some(value)) => {
                    status.warnings = decode_warnings(&value).map_err(malformed)?;
                }
            }
        }
        Ok(status)
    }
}
