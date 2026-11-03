use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn prove_contract_fee_pots_v0(
        &self,
        contract_id: Identifier,
        pots: &[ContractFeePot],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        if pots.is_empty() {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(
                "a contract fee pots proof needs at least one pot".to_string(),
            )));
        }
        let path_query = Self::contract_fee_pots_query(
            contract_id.to_buffer(),
            pots,
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
