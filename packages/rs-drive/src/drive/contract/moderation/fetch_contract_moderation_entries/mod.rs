mod v0;

use crate::drive::contract::moderation::types::{
    ContractModerationEntriesQuery, ContractModerationEntry,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Reads one page of one moderation list of a contract, in identity id order.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The moderated contract.
    /// * `query`: The list, the cursor and the limit.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ContractModerationEntry>)` with the page; empty when the contract keeps no
    ///   such list.
    /// * `Err(Error)` when the limit is out of bounds, the version is unknown or a read fails.
    pub fn fetch_contract_moderation_entries(
        &self,
        contract_id: Identifier,
        query: &ContractModerationEntriesQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ContractModerationEntry>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .fetch_contract_moderation_entries
        {
            0 => self.fetch_contract_moderation_entries_v0(
                contract_id,
                query,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_moderation_entries".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Checks a moderation entries page limit: at least one entry, at most the platform
    /// version's `max_returned_elements` (the number the proof verifier reads too, so that a
    /// node setting cannot make a default request unanswerable), so that no page and no proof
    /// grows with the size of the list.
    pub(in crate::drive::contract::moderation) fn check_contract_moderation_entries_limit(
        limit: u16,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let max_limit = platform_version.drive_abci.query.max_returned_elements;
        if limit == 0 || limit > max_limit {
            return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                "contract moderation entries limit must be between 1 and {}, got {}",
                max_limit, limit
            ))));
        }
        Ok(())
    }
}
