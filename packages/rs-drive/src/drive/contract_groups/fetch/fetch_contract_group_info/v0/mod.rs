use crate::drive::contract_groups::paths::{contract_group_path, CONTRACT_GROUP_INFO_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::block::epoch::Epoch;
use dpp::contract_group::ContractGroupInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializableTrusted;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn fetch_contract_group_info_v0(
        &self,
        contract_group_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractGroupInfo>, Error> {
        self.fetch_contract_group_info_add_to_operations_v0(
            contract_group_id,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_contract_group_info_with_fee_v0(
        &self,
        contract_group_id: Identifier,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, Option<ContractGroupInfo>), Error> {
        let mut drive_operations = vec![];
        let info = self.fetch_contract_group_info_add_to_operations_v0(
            contract_group_id,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fee = Drive::calculate_fee(
            None,
            Some(drive_operations),
            epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok((fee, info))
    }

    pub(super) fn fetch_contract_group_info_add_to_operations_v0(
        &self,
        contract_group_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractGroupInfo>, Error> {
        let path = contract_group_path(contract_group_id.as_slice());
        self.grove_get_raw_optional_item(
            (&path).into(),
            CONTRACT_GROUP_INFO_KEY,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?
        .map(|bytes| ContractGroupInfo::deserialize_from_bytes_trusted(&bytes))
        .transpose()
        .map_err(Error::from)
    }
}
