mod v0;

use crate::drive::contract::moderation::types::{
    ContractModerationEntriesQuery, ContractModerationEntry,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of one page of one moderation list of a contract, built by
    /// [`Drive::prove_contract_moderation_entries`] for the same query.
    ///
    /// # Parameters
    ///
    /// * `proof`: The proof.
    /// * `contract_id`: The moderated contract.
    /// * `query`: The list, the cursor and the limit.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((RootHash, Vec<ContractModerationEntry>))` with the root hash and the page.
    /// * `Err(Error)` when the proof is not valid for the query or holds malformed entries.
    pub fn verify_contract_moderation_entries(
        proof: &[u8],
        contract_id: Identifier,
        query: &ContractModerationEntriesQuery,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<ContractModerationEntry>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract_moderation
            .verify_contract_moderation_entries
        {
            0 => Self::verify_contract_moderation_entries_v0(
                proof,
                contract_id,
                query,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract_moderation_entries".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
