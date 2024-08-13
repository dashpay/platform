use crate::drive::Drive;

use crate::error::proof::ProofError;
use crate::error::Error;
use dpp::fee::Credits;

use crate::verify::RootHash;

use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies the balances of multiple identities by their identity IDs within a specified range.
    ///
    /// This function is used to verify the balances of identities based on a provided proof. The proof
    /// can be a subset of a larger proof, indicated by the `is_proof_subset` parameter. This is useful
    /// when the proof includes more information than what is being verified (e.g., verifying only balances
    /// when the proof includes balances and revisions).
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proofs of authentication from the users.
    /// - `is_proof_subset`: A boolean indicating whether we are verifying a subset of a larger proof.
    /// - `start_at`: An optional tuple containing a 32-byte array representing the starting identity ID and
    ///   a boolean indicating whether to include the starting identity in the range.
    /// - `ascending`: A boolean indicating the order of the range query. If `true`, the query is ascending;
    ///   otherwise, it is descending.
    /// - `limit`: A 16-bit unsigned integer representing the maximum number of identities to query.
    /// - `platform_version`: A reference to the platform version against which to verify the balances.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` containing a tuple:
    /// - `RootHash`: The root hash of the verified proof.
    /// - `T`: A generic collection of tuples where each tuple consists of a 32-byte array representing
    ///   an identity ID and a `Credits` value. The `Credits` value represents the balance of the respective
    ///   identity.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - Any of the identity IDs does not correspond to a valid balance.
    /// - The number of proved key values does not match the number of identity IDs provided.
    /// - The value size of the balance is incorrect.
    ///
    pub(crate) fn verify_identity_balances_for_range_query_v0<
        T: FromIterator<([u8; 32], Credits)>,
    >(
        proof: &[u8],
        is_proof_subset: bool,
        start_at: Option<([u8; 32], bool)>,
        ascending: bool,
        limit: u16,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let path_query = Self::balances_for_range_query(start_at, ascending, limit);
        let (root_hash, proved_key_values) = if is_proof_subset {
            GroveDb::verify_subset_query(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };
        let values = proved_key_values
            .into_iter()
            .map(|(_, key, maybe_element)| {
                let key: [u8; 32] = key
                    .try_into()
                    .map_err(|_| Error::Proof(ProofError::IncorrectValueSize("value size")))?;
                match maybe_element {
                    None => Err(Error::Proof(ProofError::CorruptedProof("range proof can not have absent elements".to_string()))),
                    Some(element) => {
                        let balance: Credits = element
                            .as_sum_item_value()
                            .map_err(Error::GroveDB)?
                            .try_into()
                            .map_err(|_| {
                                Error::Proof(ProofError::IncorrectValueSize(
                                    "balance was negative",
                                ))
                            })?;
                        Ok((key, balance))
                    }
                }
            })
            .collect::<Result<T, Error>>()?;
        Ok((root_hash, values))
    }
}
