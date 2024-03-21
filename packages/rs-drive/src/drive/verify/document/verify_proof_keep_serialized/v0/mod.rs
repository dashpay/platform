use crate::drive::verify::RootHash;

use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::DriveQuery;

use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl<'a> DriveQuery<'a> {
    /// Verifies the given proof and returns the root hash of the GroveDB tree and a vector
    /// of serialized documents if the verification is successful.
    ///
    /// # Arguments
    /// * `proof` - A byte slice representing the proof to be verified.
    ///
    /// # Returns
    /// * On success, returns a tuple containing the root hash of the GroveDB tree and a vector of serialized documents.
    /// * On failure, returns an Error.
    ///
    /// # Errors
    /// This function will return an Error if:
    /// * The start at document is not present in proof and it is expected to be.
    /// * The path query fails to verify against the given proof.
    /// * Converting the element into bytes fails.
    #[inline(always)]
    pub(super) fn verify_proof_keep_serialized_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Vec<u8>>), Error> {
        let path_query = if let Some(start_at) = &self.start_at {
            let (_, start_document) =
                self.verify_start_at_document_in_proof(proof, true, *start_at, platform_version)?;
            let document = start_document.ok_or(Error::Proof(ProofError::IncompleteProof(
                "expected start at document to be present in proof",
            )))?;
            self.construct_path_query(Some(document), platform_version)
        } else {
            self.construct_path_query(None, platform_version)
        }?;
        let (root_hash, proved_key_values) = if self.start_at.is_some() {
            GroveDb::verify_subset_query(proof, &path_query)?
        } else {
            GroveDb::verify_query(proof, &path_query)?
        };

        let documents = proved_key_values
            .into_iter()
            .filter_map(|(_path, _key, element)| element)
            .map(|element| element.into_item_bytes().map_err(Error::GroveDB))
            .collect::<Result<Vec<Vec<u8>>, Error>>()?;
        Ok((root_hash, documents))
    }
}
