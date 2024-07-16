use crate::drive::Drive;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies the identity IDs of multiple identities by their public key hashes.
    ///
    /// `is_proof_subset` is used to indicate if we want to verify a subset of a bigger proof.
    /// For example, if the proof can prove the identity IDs and revisions, but here we are only
    /// interested in verifying the identity IDs.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proofs of authentication from the users.
    /// - `is_proof_subset`: A boolean indicating whether we are verifying a subset of a larger proof.
    /// - `public_key_hashes`: A slice of 20-byte arrays representing the public key hashes of the users.
    ///
    /// # Returns
    ///
    /// If the verification is successful, it returns a `Result` with a tuple of `RootHash` and
    /// a generic collection `T` of tuples. Each tuple in `T` consists of a 20-byte array
    /// representing a public key hash and an `Option<[u8; 32]>`. The `Option<[u8; 32]>` represents
    /// the identity ID of the respective identity if it exists.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof of authentication is not valid.
    /// - Any of the public key hashes does not correspond to a valid identity ID.
    /// - The number of proved key values does not match the number of public key hashes provided.
    /// - The value size of the identity ID is incorrect.
    ///
    pub(crate) fn verify_identity_ids_by_public_key_hashes_v0<
        T: FromIterator<([u8; 20], Option<[u8; 32]>)>,
    >(
        proof: &[u8],
        is_proof_subset: bool,
        public_key_hashes: &[[u8; 20]],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let mut path_query = Self::identity_ids_by_unique_public_key_hash_query(public_key_hashes);
        path_query.query.limit = Some(public_key_hashes.len() as u16);
        let (root_hash, proved_key_values) = if is_proof_subset {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };
        if proved_key_values.len() == public_key_hashes.len() {
            let values = proved_key_values
                .into_iter()
                .map(|proved_key_value| {
                    let key: [u8; 20] = proved_key_value
                        .1
                        .try_into()
                        .map_err(|_| Error::Proof(ProofError::IncorrectValueSize("value size")))?;
                    let maybe_element = proved_key_value.2;
                    match maybe_element {
                        None => Ok((key, None)),
                        Some(element) => {
                            let identity_id = element
                                .into_item_bytes()
                                .map_err(Error::GroveDB)?
                                .try_into()
                                .map_err(|_| {
                                    Error::Proof(ProofError::IncorrectValueSize(
                                        "value size is incorrect",
                                    ))
                                })?;
                            Ok((key, Some(identity_id)))
                        }
                    }
                })
                .collect::<Result<T, Error>>()?;
            Ok((root_hash, values))
        } else {
            Err(Error::Proof(ProofError::WrongElementCount {
                expected: public_key_hashes.len(),
                got: proved_key_values.len(),
            }))
        }
    }
}
