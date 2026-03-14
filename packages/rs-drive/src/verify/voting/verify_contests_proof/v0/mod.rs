use crate::verify::RootHash;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::platform_value::Value;
use grovedb::GroveDb;

use crate::error::Error;

use crate::error::drive::DriveError;
use crate::query::vote_polls_by_document_type_query::ResolvedVotePollsByDocumentTypeQuery;
use dpp::version::PlatformVersion;

impl ResolvedVotePollsByDocumentTypeQuery<'_> {
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
                        .map_err(Error::from)
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
                        .map_err(Error::from)
                }
            })
            .collect::<Result<Vec<Value>, Error>>()?;

        Ok((root_hash, values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::object_size_info::DataContractResolvedInfo;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::tests::json_document::json_document_to_contract;
    use std::sync::Arc;

    #[test]
    fn should_prove_and_verify_empty_contests_proof() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract(
            "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index.json",
            false,
            platform_version,
        )
        .expect("expected to create a data contract");

        // Insert the DPNS contract so its paths exist in the store
        drive
            .insert_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        let doc_type_name = String::from("domain");
        let index_name = String::from("parentNameAndLabel");
        let start_index_values = vec![Value::Text("dash".to_string())];
        let end_index_values = vec![];
        let start_at_value = None;
        let arc_contract = Arc::new(data_contract);

        let query = ResolvedVotePollsByDocumentTypeQuery {
            contract: DataContractResolvedInfo::ArcDataContract(arc_contract),
            document_type_name: &doc_type_name,
            index_name: &index_name,
            start_index_values: &start_index_values,
            end_index_values: &end_index_values,
            start_at_value: &start_at_value,
            limit: Some(10),
            order_ascending: true,
        };

        // Build path query and generate proof from drive
        let index = query.index().expect("expected to get index");
        let path_query = query
            .construct_path_query_with_known_index(index, platform_version)
            .expect("expected to construct path query");
        let proof = drive
            .grove_get_proved_path_query(
                &path_query,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to get proof");

        let (_, values) = query
            .verify_contests_proof(proof.as_slice(), platform_version)
            .expect("expected proof verification to succeed");

        assert!(values.is_empty());
    }
}
