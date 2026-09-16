use crate::drive::contract_groups::types::ContractGroupMembersQuery;
use crate::drive::Drive;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn prove_contract_group_members_v0(
        &self,
        contract_group_id: Identifier,
        query: &ContractGroupMembersQuery,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.check_contract_group_members_limit(limit)?;
        let path_query =
            Self::contract_group_members_query(contract_group_id.to_buffer(), query, limit);
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
