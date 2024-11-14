use crate::verify::RootHash;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::platform_value::Value;
use grovedb::GroveDb;

use crate::error::Error;

use crate::error::drive::DriveError;
use crate::query::vote_polls_by_document_type_query::ResolvedVotePollsByDocumentTypeQuery;
use dpp::version::PlatformVersion;

impl<'a> ResolvedVotePollsByDocumentTypeQuery<'a> {
    /// Verifies a proof for a collection of documents.
    ///
    /// This function takes a slice of bytes `proof` containing a serialized proof,
    /// verifies it, and returns a tuple consisting of the root hash and a vector of deserialized documents.
    ///
    /// # Arguments
    ///
    /// * `proof` - A byte slice representing the proof to be verified.
    /// * `drive_version` - The current active drive version
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// * A tuple with the root hash and a vector of deserialized `Document`s, if the proof is valid.
    /// * An `Error` variant, in case the proof verification fails or deserialization error occurs.
    ///
    /// # Errors
    ///
    /// This function will return an `Error` variant if:
    /// 1. The proof verification fails.
    /// 2. There is a deserialization error when parsing the serialized document(s) into `Document` struct(s).
    #[inline(always)]
    pub(super) fn verify_contests_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Value>), Error> {
        let index = self.index()?;
        let path_query = self.construct_path_query_with_known_index(index, platform_version)?;
        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let result_is_in_key = self.result_is_in_key();
        let result_path_index = if result_is_in_key {
            None
        } else {
            Some(self.result_path_index())
        };
        let document_type = self.document_type()?;
        let property_name_being_searched = self.property_name_being_searched(index)?;
        let values = proved_key_values
            .into_iter()
            .map(|(mut path, key, _)| {
                if result_is_in_key {
                    // the result is in the key because we did not provide any end index values
                    // like this  <------ start index values (path) --->    Key
                    // properties ------- --------- --------- ----------  -------
                    document_type
                        .deserialize_value_for_key(
                            property_name_being_searched.name.as_str(),
                            key.as_slice(),
                            platform_version,
                        )
                        .map_err(Error::Protocol)
                } else if path.len() < result_path_index.unwrap() {
                    Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "the path length should always be bigger or equal to the result path index",
                    )))
                } else {
                    // the result is in the path because we did not provide any end index values
                    // like this  <------ start index values (path) --->    Key
                    // properties ------- --------- --------- ----------  -------
                    let inner_path_value_bytes = path.remove(result_path_index.unwrap());
                    document_type
                        .deserialize_value_for_key(
                            property_name_being_searched.name.as_str(),
                            inner_path_value_bytes.as_slice(),
                            platform_version,
                        )
                        .map_err(Error::Protocol)
                }
            })
            .collect::<Result<Vec<Value>, Error>>()?;

        Ok((root_hash, values))
    }
}
