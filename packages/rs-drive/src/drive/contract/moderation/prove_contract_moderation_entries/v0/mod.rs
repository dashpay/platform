use crate::drive::contract::moderation::types::ContractModerationEntriesQuery;
use crate::drive::Drive;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn prove_contract_moderation_entries_v0(
        &self,
        contract_id: Identifier,
        query: &ContractModerationEntriesQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.check_contract_moderation_entries_limit(query.limit)?;
        let path_query = Self::contract_moderation_entries_query(contract_id.to_buffer(), query);
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
