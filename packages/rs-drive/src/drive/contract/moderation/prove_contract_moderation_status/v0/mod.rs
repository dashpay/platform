use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn prove_contract_moderation_status_v0(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        lists: &[ContractModerationList],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        if lists.is_empty() {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(
                "a contract moderation status proof needs at least one list".to_string(),
            )));
        }
        let path_query = Self::contract_moderation_status_query(
            contract_id.to_buffer(),
            identity_id.to_buffer(),
            lists,
            &platform_version.drive.grove_version,
        )?;
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
