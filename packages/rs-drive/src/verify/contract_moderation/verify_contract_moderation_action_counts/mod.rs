mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Verifies a proof of the moderation action counts of the elected contract `contract_id`
    /// and returns them: for each member of its seated team who signed a counted moderation
    /// action since the moderators pot was last settled, how many.
    ///
    /// # Parameters
    ///
    /// * `proof`: The proof, as `prove_contract_moderation_action_counts` produced it.
    /// * `contract_id`: The elected contract.
    /// * `verify_subset_of_proof`: Whether the proof may prove more than this query (verified as
    ///   a subset).
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((RootHash, BTreeMap<Identifier, u32>))` with the proof's root hash and the counts.
    /// * `Err(Error)` when the method version is unknown, the proof fails verification, or a
    ///   proven count is malformed.
    pub fn verify_contract_moderation_action_counts(
        proof: &[u8],
        contract_id: Identifier,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, BTreeMap<Identifier, u32>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .contract_moderation
            .verify_contract_moderation_action_counts
        {
            0 => Self::verify_contract_moderation_action_counts_v0(
                proof,
                contract_id,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_contract_moderation_action_counts".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
