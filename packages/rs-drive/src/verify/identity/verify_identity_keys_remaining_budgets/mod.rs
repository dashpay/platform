mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::fee::Credits;
use dpp::identity::KeyID;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies a proof of what is left of the budgets of several keys of one identity.
    ///
    /// # Parameters
    /// - `proof`: The proof returned by the node.
    /// - `identity_id`: The identity the keys belong to.
    /// - `key_ids`: The key ids that were asked for, which the proof must answer one by one.
    /// - `verify_subset_of_proof`: Whether the proof may cover more than this query.
    /// - `platform_version`: The platform version selecting the implementation.
    ///
    /// # Returns
    /// - The root hash the proof commits to, and one entry per requested key id: `Some(credits)`
    ///   for a budgeted key, `None` when the key has no budget entry.
    pub fn verify_identity_keys_remaining_budgets<T: FromIterator<(KeyID, Option<Credits>)>>(
        proof: &[u8],
        identity_id: [u8; 32],
        key_ids: &[KeyID],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identity_keys_remaining_budgets
        {
            0 => Self::verify_identity_keys_remaining_budgets_v0(
                proof,
                identity_id,
                key_ids,
                verify_subset_of_proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identity_keys_remaining_budgets".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
