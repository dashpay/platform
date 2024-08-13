mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;

use crate::error::Error;
use dpp::fee::Credits;

use crate::verify::RootHash;

use dpp::version::PlatformVersion;

use std::iter::FromIterator;

impl Drive {
    /// Verifies the balances of multiple identities by their identity IDs in a range query.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proofs of authentication from the users.
    /// - `is_proof_subset`: A boolean indicating whether we are verifying a subset of a larger proof.
    /// - `start_at`: An optional tuple where the first element is a 32-byte array representing the
    ///   starting identity ID, and the second element is a boolean indicating whether to include the
    ///   starting identity in the range.
    /// - `ascending`: A boolean indicating the order of the range query. If `true`, the query is
    ///   ascending; otherwise, it is descending.
    /// - `limit`: A 16-bit unsigned integer representing the maximum number of identities to query.
    /// - `platform_version`: A reference to the platform version against which to verify the balances.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` containing a tuple:
    /// - `RootHash`: The root hash of the verified proof.
    /// - `T`: A generic collection of tuples where each tuple consists of a 32-byte array representing
    ///   an identity ID and a `Credits`. The `Credits` represents the balance of the
    ///   respective identities.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - An unknown or unsupported platform version is provided.
    /// - Any other error as documented in the specific versioned function.
    pub fn verify_identity_balances_for_range_query<
        T: FromIterator<([u8; 32], Credits)>,
    >(
        proof: &[u8],
        is_proof_subset: bool,
        start_at: Option<([u8; 32], bool)>,
        ascending: bool,
        limit: u16,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .identity
            .verify_identity_balances_for_range_query
        {
            0 => Self::verify_identity_balances_for_range_query_v0(
                proof,
                is_proof_subset,
                start_at,
                ascending,
                limit,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_identity_balances_for_range_query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
