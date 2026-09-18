use crate::drive::contract::moderation::types::decode_until;
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
            match (list, entry) {
                (_, None) => {}
                (ContractModerationList::Banlist, Some(_)) => status.banned = true,
                (ContractModerationList::Suspensions, Some(value)) => {
                    status.suspended_until = Some(decode_until(&value).map_err(|description| {
                        Error::Drive(DriveError::CorruptedDriveState(format!(
                            "suspension of {} on contract {} is malformed: {}",
                            identity_id, contract_id, description
                        )))
                    })?);
                }
            }
        }
        Ok(status)
    }
}
