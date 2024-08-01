use grovedb::GroveDb;
use crate::drive::Drive;
use crate::error::Error;
use crate::verify::RootHash;
use platform_version::version::PlatformVersion;
use integer_encoding::VarInt;
use dpp::fee::Credits;
use crate::drive::balances::{total_credits_on_platform_path_query, TOTAL_SYSTEM_CREDITS_STORAGE_KEY};
use crate::error::proof::ProofError;

impl Drive {
    /// Verifies a proof for the total credits in the system and returns
    /// them if they are in the proof.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `path`: The path where elements should be.
    /// - `keys`: The requested keys.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `Credits`.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    #[inline(always)]
    pub(crate) fn verify_total_credits_in_system_v0(
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Credits), Error> {
        let path_query = total_credits_on_platform_path_query();

        let (root_hash, mut proved_path_key_values) =
            GroveDb::verify_query_raw(proof, &path_query, &platform_version.drive.grove_version)?;
        if proved_path_key_values.len() > 1 {
            return Err(Error::Proof(ProofError::TooManyElements("We should only get back at most 1 element in the proof for the total credits in the system")));
        }

        let Some(proved_path_key_value) = proved_path_key_values.pop() else {
            return Err(Error::Proof(ProofError::IncorrectProof("This proof would show that Platform has not yet been initialized".to_string())));
        };

        if proved_path_key_value.path != path_query.path {
            return Err(Error::Proof(ProofError::CorruptedProof("The result of this proof is not what we asked for (path)".to_string())));
        }

        if proved_path_key_value.key != TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec() {
            return Err(Error::Proof(ProofError::CorruptedProof("The result of this proof is not what we asked for (key)".to_string())));
        }

        let credits = Credits::decode_var(proved_path_key_value.value.as_slice()).ok_or(Error::Proof(ProofError::CorruptedProof("The result of this proof does not contain an encoded var integer for total credits".to_string())))?.0;
        
        Ok((root_hash, credits))
    }
}
