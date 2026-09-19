mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::config::moderation::{ContractModerationList, ContractModerationStatus};
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of one identity's status on a moderated contract, built by
    /// [`Drive::prove_contract_moderation_status`] for the same `lists`.
    ///
    /// # Parameters
    ///
    /// * `proof`: The proof.
    /// * `contract_id`: The moderated contract.
    /// * `identity_id`: The identity.
    /// * `lists`: The lists the contract keeps, as its config declares them; at least one.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((RootHash, ContractModerationStatus))` with the root hash and the status.
    /// * `Err(Error)` when the proof is not valid for the query or holds unexpected elements.
    pub fn verify_contract_moderation_status(
        proof: &[u8],
        contract_id: Identifier,
        identity_id: Identifier,
        lists: &[ContractModerationList],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ContractModerationStatus), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract_moderation
            .verify_contract_moderation_status
        {
            0 => Self::verify_contract_moderation_status_v0(
                proof,
                contract_id,
                identity_id,
                lists,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract_moderation_status".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
