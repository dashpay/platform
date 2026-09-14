use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn prove_contracts_versions_v0(
        &self,
        contract_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        if contract_ids.is_empty() {
            return Err(Error::Query(QuerySyntaxError::NoQueryItems(
                "no contract ids to prove versions for",
            )));
        }

        let path_query = Self::fetch_contracts_versions_query(contract_ids);

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
