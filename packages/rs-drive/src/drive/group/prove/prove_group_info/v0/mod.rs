use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_group_info_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_group_info_operations_v0(
            contract_id,
            group_contract_position,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_group_info_operations_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Drive::group_info_for_contract_id_and_group_contract_position_query(
            contract_id.to_buffer(),
            group_contract_position,
        );
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
