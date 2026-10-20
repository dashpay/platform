use crate::drive::Drive;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves one page of the contract enumeration.
    ///
    /// Selects the ids-only or the full path query (both built next to the other contract
    /// queries so that the verifier rebuilds the identical query) and proves it.
    #[inline(always)]
    pub(super) fn prove_contracts_by_range_v0(
        &self,
        start_at: Option<([u8; 32], bool)>,
        limit: u16,
        ids_only: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = if ids_only {
            Self::fetch_contract_ids_by_range_query(start_at, limit)
        } else {
            Self::fetch_contracts_by_range_query(start_at, limit)
        };

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
