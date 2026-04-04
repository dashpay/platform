use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_count_request::GetDocumentsCountRequestV0;
use dapi_grpc::platform::v0::get_documents_count_response::{
    get_documents_count_response_v0, GetDocumentsCountResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::Index;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::RootTree;
use drive::error::query::QuerySyntaxError;
use drive::grovedb::query_result_type::QueryResultType;
use drive::grovedb::{PathQuery, Query, SizedQuery};
use drive::grovedb_path::SubtreePath;
use drive::query::{DriveDocumentQuery, WhereClause, WhereOperator};
use drive::util::grove_operations::{DirectQueryType, GroveDBToUse};

impl<C> Platform<C> {
    pub(super) fn query_documents_count_v0(
        &self,
        GetDocumentsCountRequestV0 {
            data_contract_id,
            document_type: document_type_name,
            r#where,
            prove,
        }: GetDocumentsCountRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsCountResponseV0>, Error> {
        let contract_id: Identifier = check_validation_result_with_data!(data_contract_id
            .try_into()
            .map_err(|_| QueryError::InvalidArgument(
                "id must be a valid identifier (32 bytes long)".to_string()
            )));

        let (_, contract) = self.drive.get_contract_with_fetch_info_and_fee(
            contract_id.to_buffer(),
            None,
            true,
            None,
            platform_version,
        )?;

        let contract = check_validation_result_with_data!(contract.ok_or(QueryError::Query(
            QuerySyntaxError::DataContractNotFound(
                "contract not found when querying from value with contract info",
            )
        )));

        let contract_ref = &contract.contract;

        let document_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(document_type_name.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                document_type_name, contract_id
            ))));

        let where_clause = if r#where.is_empty() {
            Value::Null
        } else {
            check_validation_result_with_data!(ciborium::de::from_reader(r#where.as_slice())
                .map_err(|_| {
                    QueryError::Query(QuerySyntaxError::DeserializationError(
                        "unable to decode 'where' query from cbor".to_string(),
                    ))
                }))
        };

        // Parse where clauses into WhereClause structs so we can match them against
        // index properties for the CountTree path.
        let all_where_clauses: Vec<WhereClause> =
            check_validation_result_with_data!(match &where_clause {
                Value::Null => Ok(vec![]),
                Value::Array(clauses) => clauses
                    .iter()
                    .map(|wc| {
                        if let Value::Array(components) = wc {
                            WhereClause::from_components(components).map_err(|e| match e {
                                drive::error::Error::Query(qe) => QueryError::Query(qe),
                                other => QueryError::InvalidArgument(format!(
                                    "error parsing where clauses: {}",
                                    other
                                )),
                            })
                        } else {
                            Err(QueryError::Query(
                                QuerySyntaxError::InvalidFormatWhereClause(
                                    "where clause must be an array",
                                ),
                            ))
                        }
                    })
                    .collect::<Result<Vec<WhereClause>, QueryError>>(),
                _ => Err(QueryError::Query(
                    QuerySyntaxError::InvalidFormatWhereClause("where clause must be an array"),
                )),
            });

        let response = if prove {
            // For prove path, use the standard DriveDocumentQuery approach.
            // We still need the full path query structure for proof generation.
            let mut drive_query =
                check_validation_result_with_data!(DriveDocumentQuery::from_decomposed_values(
                    where_clause,
                    None,
                    Some(self.config.drive.default_query_limit),
                    None,
                    true,
                    None,
                    contract_ref,
                    document_type,
                    &self.config.drive,
                ));

            drive_query.limit = None;

            let proof =
                match drive_query.execute_with_proof(&self.drive, None, None, platform_version) {
                    Ok(result) => result.0,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            // For no-prove path, use CountTree-based O(1) counting when possible.
            //
            // Find a countable index that matches the where clause properties.
            // The index must have countable=true, and any where clause properties
            // must match prefix properties of the index with equality operators.
            let countable_index = Self::find_countable_index_for_where_clauses(
                document_type.indexes(),
                &all_where_clauses,
            );

            let count = if let Some(index) = countable_index {
                // Build the path to the CountTree(s) and fetch count(s).
                self.count_from_count_tree(
                    contract_id.to_buffer(),
                    document_type_name.as_str(),
                    document_type,
                    index,
                    &all_where_clauses,
                    platform_version,
                )?
            } else {
                // No countable index found. Return an error telling the caller
                // that count queries require a countable index.
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(
                        "count query requires a countable index on the document type that \
                         matches the where clause properties"
                            .to_string(),
                    ),
                ));
            };

            GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Count(count)),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }

    /// Finds a countable index whose properties form a prefix that matches the
    /// equality where clauses. For a count query:
    /// - All where clause fields must appear as a prefix of the index properties
    /// - The index must have countable=true
    /// - Among matching indexes, we prefer the one with the most properties
    ///   matched by where clauses (most specific)
    fn find_countable_index_for_where_clauses<'a>(
        indexes: &'a std::collections::BTreeMap<String, Index>,
        where_clauses: &[WhereClause],
    ) -> Option<&'a Index> {
        let equality_fields: std::collections::BTreeSet<&str> = where_clauses
            .iter()
            .filter(|wc| wc.operator == WhereOperator::Equal)
            .map(|wc| wc.field.as_str())
            .collect();

        let mut best_match: Option<(&Index, usize)> = None;

        for index in indexes.values() {
            if !index.countable {
                continue;
            }

            // Check that where clause equality fields form a prefix of the index properties.
            // For example, if index has properties [A, B, C]:
            // - WHERE A = x -> matches prefix of length 1
            // - WHERE A = x AND B = y -> matches prefix of length 2
            // - WHERE B = y -> does NOT match (A is not covered)
            // - No where clause -> matches prefix of length 0 (count all)
            let mut prefix_len = 0;
            for prop in &index.properties {
                if equality_fields.contains(prop.name.as_str()) {
                    prefix_len += 1;
                } else {
                    break;
                }
            }

            // All equality where clause fields must be consumed as a prefix
            if prefix_len < equality_fields.len() {
                continue;
            }

            // Prefer the index with the longest matching prefix (most specific)
            match &best_match {
                None => best_match = Some((index, prefix_len)),
                Some((_, best_len)) if prefix_len > *best_len => {
                    best_match = Some((index, prefix_len));
                }
                _ => {}
            }
        }

        best_match.map(|(index, _)| index)
    }

    /// Counts documents using the CountTree elements in the index path.
    ///
    /// When all index properties are covered by equality where clauses, this is
    /// O(1) -- a single GroveDB fetch of the CountTree element.
    ///
    /// When some properties are not covered (e.g., no where clause), this
    /// iterates over distinct values at the unspecified levels and sums
    /// their CountTree counts. Still much cheaper than fetching all documents.
    fn count_from_count_tree(
        &self,
        contract_id: [u8; 32],
        document_type_name: &str,
        document_type: dpp::data_contract::document_type::DocumentTypeRef,
        index: &Index,
        where_clauses: &[WhereClause],
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let drive_version = &platform_version.drive;

        // Build the base path: [DataContractDocuments, contract_id, 1, doc_type_name]
        let mut path = vec![
            vec![RootTree::DataContractDocuments as u8],
            contract_id.to_vec(),
            vec![1u8],
            document_type_name.as_bytes().to_vec(),
        ];

        // Walk the index properties, pushing property keys and values for
        // each equality where clause.
        let mut covered_count = 0;
        for prop in &index.properties {
            let matching_clause = where_clauses
                .iter()
                .find(|wc| wc.field == prop.name && wc.operator == WhereOperator::Equal);

            if let Some(clause) = matching_clause {
                // Push the index property key
                path.push(prop.name.as_bytes().to_vec());
                // Serialize and push the property value
                let serialized_value = document_type.serialize_value_for_key(
                    prop.name.as_str(),
                    &clause.value,
                    platform_version,
                )?;
                path.push(serialized_value);
                covered_count += 1;
            } else {
                // This property and all subsequent ones are not covered by where clauses
                break;
            }
        }

        if covered_count == index.properties.len() {
            // All index properties are covered -- O(1) fetch of the single CountTree.
            // The CountTree element is at key [0] under the fully specified path.
            let mut drive_operations = vec![];
            let path_refs: Vec<&[u8]> = path.iter().map(|p| p.as_slice()).collect();
            let element = self.drive.grove_get_raw_optional(
                SubtreePath::from(path_refs.as_slice()),
                &[0],
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut drive_operations,
                drive_version,
            )?;

            Ok(element.map_or(0, |e| e.count_value_or_default()))
        } else {
            // Not all properties covered. We need to iterate over the values at the
            // next unspecified property level.
            //
            // The path is currently at the level of the last covered property value.
            // We need to descend into the next property key level and query all value
            // subtrees, fetching their CountTree at key [0].
            //
            // For a single-property index with no where clause, the path is just
            // [2, contract_id, 1, doc_type_name] and we need to iterate all values
            // under the first property key.
            //
            // For a multi-property index with partial coverage, we iterate the
            // remaining levels.

            let remaining_properties = &index.properties[covered_count..];

            self.count_from_count_tree_recursive(path, remaining_properties, drive_version)
        }
    }

    /// Recursively descends through remaining index property levels,
    /// iterating over all values at each level, and sums the CountTree
    /// counts at the terminal level.
    fn count_from_count_tree_recursive(
        &self,
        current_path: Vec<Vec<u8>>,
        remaining_properties: &[dpp::data_contract::document_type::IndexProperty],
        drive_version: &dpp::version::drive_versions::DriveVersion,
    ) -> Result<u64, Error> {
        if remaining_properties.is_empty() {
            // We've navigated through all index properties.
            // The CountTree element is at key [0] under the current path.
            let mut drive_operations = vec![];
            let path_refs: Vec<&[u8]> = current_path.iter().map(|p| p.as_slice()).collect();
            let element = self.drive.grove_get_raw_optional(
                SubtreePath::from(path_refs.as_slice()),
                &[0],
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut drive_operations,
                drive_version,
            )?;

            return Ok(element.map_or(0, |e| e.count_value_or_default()));
        }

        let prop = &remaining_properties[0];
        let rest = &remaining_properties[1..];

        // Push the index property key to descend into that level
        let mut property_path = current_path.clone();
        property_path.push(prop.name.as_bytes().to_vec());

        // Query all children (value subtrees) at this property level
        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(property_path.clone(), SizedQuery::new(query, None, None));

        let mut drive_operations = vec![];
        let result = self.drive.grove_get_raw_path_query(
            &path_query,
            None,
            QueryResultType::QueryKeyElementPairResultType,
            &mut drive_operations,
            drive_version,
        );

        let (elements, _) = match result {
            Ok(result) => result,
            Err(drive::error::Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    drive::grovedb::Error::PathNotFound(_)
                        | drive::grovedb::Error::PathParentLayerNotFound(_)
                        | drive::grovedb::Error::PathKeyNotFound(_)
                ) =>
            {
                // Path doesn't exist -- no documents have been inserted yet
                return Ok(0);
            }
            Err(e) => return Err(e.into()),
        };

        let key_elements = elements.to_key_elements();

        if key_elements.is_empty() {
            return Ok(0);
        }

        let mut total_count: u64 = 0;

        for (key, _element) in key_elements {
            // Build the path for this value: [..., prop_name, <value>]
            let mut value_path = property_path.clone();
            value_path.push(key);

            // Recurse into the remaining property levels
            let sub_count =
                self.count_from_count_tree_recursive(value_path, rest, drive_version)?;
            total_count = total_count.saturating_add(sub_count);
        }

        Ok(total_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{setup_platform, store_data_contract, store_document};
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_documents_count_no_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        store_data_contract(&platform, &data_contract, version);

        let data_contract_id = data_contract.id();
        let document_type_name = "person";
        let document_type = data_contract
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(500);
        for _ in 0..5 {
            let random_document = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");
            store_document(
                &platform,
                &data_contract,
                document_type,
                &random_document,
                platform_version,
            );
        }

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Count(count)),
                metadata: Some(_),
            }) => {
                assert_eq!(count, 5, "expected count of 5 documents");
            }
            other => panic!("expected count result, got {:?}", other),
        }
    }

    #[test]
    fn test_documents_count_empty_result() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        store_data_contract(&platform, &data_contract, version);

        let data_contract_id = data_contract.id();
        let document_type_name = "person";

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Count(count)),
                metadata: Some(_),
            }) => {
                assert_eq!(count, 0, "expected count of 0 documents");
            }
            other => panic!("expected count result, got {:?}", other),
        }
    }

    #[test]
    fn test_documents_count_with_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        store_data_contract(&platform, &data_contract, version);

        let data_contract_id = data_contract.id();
        let document_type_name = "person";
        let document_type = data_contract
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(500);
        for _ in 0..3 {
            let random_document = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");
            store_document(
                &platform,
                &data_contract,
                document_type,
                &random_document,
                platform_version,
            );
        }

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            prove: true,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        assert!(matches!(
            result.data,
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
