use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_split_count_request::GetDocumentsSplitCountRequestV0;
use dapi_grpc::platform::v0::get_documents_split_count_response::{
    get_documents_split_count_response_v0, GetDocumentsSplitCountResponseV0,
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
use std::collections::BTreeMap;

impl<C> Platform<C> {
    pub(super) fn query_documents_split_count_v0(
        &self,
        GetDocumentsSplitCountRequestV0 {
            data_contract_id,
            document_type: document_type_name,
            r#where,
            split_count_by_index_property,
            prove,
        }: GetDocumentsSplitCountRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsSplitCountResponseV0>, Error> {
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

        // Validate the split property exists in the document type
        if split_count_by_index_property.is_empty() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(
                    "split_count_by_index_property must not be empty".to_string(),
                ),
            ));
        }

        // Check that the property exists in the document type schema
        if document_type
            .properties()
            .get(split_count_by_index_property.as_str())
            .is_none()
        {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "property {} not found in document type {}",
                    split_count_by_index_property, document_type_name
                )),
            ));
        }

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

        // Parse where clauses
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

            GetDocumentsSplitCountResponseV0 {
                result: Some(get_documents_split_count_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            // For no-prove path, use CountTree-based approach.
            //
            // Find a countable index where:
            // 1. The split property is the NEXT property after those covered by where clauses
            // 2. All where clause properties form a prefix of the index properties
            let countable_index = Self::find_countable_index_for_split(
                document_type.indexes(),
                &all_where_clauses,
                &split_count_by_index_property,
            );

            let entries = if let Some(index) = countable_index {
                self.split_count_from_count_tree(
                    contract_id.to_buffer(),
                    document_type_name.as_str(),
                    document_type,
                    index,
                    &all_where_clauses,
                    &split_count_by_index_property,
                    platform_version,
                )?
            } else {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(
                        "split count query requires a countable index where the split property \
                         follows the where clause properties in the index"
                            .to_string(),
                    ),
                ));
            };

            GetDocumentsSplitCountResponseV0 {
                result: Some(get_documents_split_count_response_v0::Result::SplitCounts(
                    get_documents_split_count_response_v0::SplitCounts { entries },
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }

    /// Finds a countable index where:
    /// - The equality where clause fields form a prefix of the index properties
    /// - The split_property is the next property after the covered prefix
    /// - The index has countable=true
    fn find_countable_index_for_split<'a>(
        indexes: &'a BTreeMap<String, Index>,
        where_clauses: &[WhereClause],
        split_property: &str,
    ) -> Option<&'a Index> {
        let equality_fields: std::collections::BTreeSet<&str> = where_clauses
            .iter()
            .filter(|wc| wc.operator == WhereOperator::Equal)
            .map(|wc| wc.field.as_str())
            .collect();

        for index in indexes.values() {
            if !index.countable {
                continue;
            }

            // Check that where clause equality fields form a prefix
            let mut prefix_len = 0;
            for prop in &index.properties {
                if equality_fields.contains(prop.name.as_str()) {
                    prefix_len += 1;
                } else {
                    break;
                }
            }

            if prefix_len < equality_fields.len() {
                continue;
            }

            // The split property must be the next property after the prefix
            if let Some(next_prop) = index.properties.get(prefix_len) {
                if next_prop.name == split_property {
                    return Some(index);
                }
            }
        }

        None
    }

    /// Counts documents split by a property value using CountTree elements.
    ///
    /// Navigates the index tree to the split property level using the where
    /// clause values, then iterates over all values of the split property
    /// and reads the CountTree count for each.
    #[allow(clippy::too_many_arguments)]
    fn split_count_from_count_tree(
        &self,
        contract_id: [u8; 32],
        document_type_name: &str,
        document_type: dpp::data_contract::document_type::DocumentTypeRef,
        index: &Index,
        where_clauses: &[WhereClause],
        split_property: &str,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<get_documents_split_count_response_v0::SplitCountEntry>, Error> {
        let drive_version = &platform_version.drive;

        // Build the base path: [DataContractDocuments, contract_id, 1, doc_type_name]
        let mut path = vec![
            vec![RootTree::DataContractDocuments as u8],
            contract_id.to_vec(),
            vec![1u8],
            document_type_name.as_bytes().to_vec(),
        ];

        // Walk the index properties up to (but not including) the split property,
        // pushing property keys and serialized values from the where clauses.
        for prop in &index.properties {
            if prop.name == split_property {
                break;
            }

            let matching_clause = where_clauses
                .iter()
                .find(|wc| wc.field == prop.name && wc.operator == WhereOperator::Equal);

            if let Some(clause) = matching_clause {
                path.push(prop.name.as_bytes().to_vec());
                let serialized_value = document_type.serialize_value_for_key(
                    prop.name.as_str(),
                    &clause.value,
                    platform_version,
                )?;
                path.push(serialized_value);
            } else {
                break;
            }
        }

        // Now push the split property key name
        path.push(split_property.as_bytes().to_vec());

        // Query all value subtrees at the split property level
        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(path.clone(), SizedQuery::new(query, None, None));

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
                return Ok(vec![]);
            }
            Err(e) => return Err(e.into()),
        };

        let key_elements = elements.to_key_elements();

        if key_elements.is_empty() {
            return Ok(vec![]);
        }

        // Determine how many remaining index properties follow the split property
        let split_prop_idx = index
            .properties
            .iter()
            .position(|p| p.name == split_property)
            .unwrap_or(0);
        let remaining_properties = &index.properties[split_prop_idx + 1..];

        let mut entries = Vec::new();

        for (key, _element) in key_elements {
            // Build path for this specific value: [..., split_property, <value>]
            let mut value_path = path.clone();
            value_path.push(key.clone());

            // Get the count by recursively descending through remaining properties
            let count = if remaining_properties.is_empty() {
                // Terminal level: fetch CountTree directly at key [0]
                let mut ops = vec![];
                let path_refs: Vec<&[u8]> = value_path.iter().map(|p| p.as_slice()).collect();
                let element = self.drive.grove_get_raw_optional(
                    SubtreePath::from(path_refs.as_slice()),
                    &[0],
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut ops,
                    drive_version,
                )?;
                element.map_or(0, |e| e.count_value_or_default())
            } else {
                // Need to iterate remaining levels
                self.count_split_recursive(value_path, remaining_properties, drive_version)?
            };

            if count > 0 {
                entries.push(get_documents_split_count_response_v0::SplitCountEntry { key, count });
            }
        }

        Ok(entries)
    }

    /// Recursively descends through remaining index property levels after the
    /// split property, iterating over all values and summing CountTree counts.
    fn count_split_recursive(
        &self,
        current_path: Vec<Vec<u8>>,
        remaining_properties: &[dpp::data_contract::document_type::IndexProperty],
        drive_version: &dpp::version::drive_versions::DriveVersion,
    ) -> Result<u64, Error> {
        if remaining_properties.is_empty() {
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

        let mut property_path = current_path;
        property_path.push(prop.name.as_bytes().to_vec());

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
                return Ok(0);
            }
            Err(e) => return Err(e.into()),
        };

        let key_elements = elements.to_key_elements();
        let mut total_count: u64 = 0;

        for (key, _element) in key_elements {
            let mut value_path = property_path.clone();
            value_path.push(key);
            let sub_count = self.count_split_recursive(value_path, rest, drive_version)?;
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
    fn test_documents_split_count_no_prove() {
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

        let mut std_rng = StdRng::seed_from_u64(600);
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

        let request = GetDocumentsSplitCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            split_count_by_index_property: "firstName".to_string(),
            prove: false,
        };

        let result = platform
            .query_documents_split_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsSplitCountResponseV0 {
                result:
                    Some(get_documents_split_count_response_v0::Result::SplitCounts(split_counts)),
                metadata: Some(_),
            }) => {
                // The total count across all splits should equal 5
                let total: u64 = split_counts.entries.iter().map(|e| e.count).sum();
                assert_eq!(total, 5, "expected total split count of 5 documents");
                // Each entry should have a non-empty key (firstName is required)
                for entry in &split_counts.entries {
                    assert!(!entry.key.is_empty(), "expected non-empty split key");
                    assert!(entry.count > 0, "expected positive count per split");
                }
            }
            other => panic!("expected split counts result, got {:?}", other),
        }
    }

    #[test]
    fn test_documents_split_count_with_prove() {
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

        let mut std_rng = StdRng::seed_from_u64(600);
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

        let request = GetDocumentsSplitCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            split_count_by_index_property: "firstName".to_string(),
            prove: true,
        };

        let result = platform
            .query_documents_split_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        assert!(matches!(
            result.data,
            Some(GetDocumentsSplitCountResponseV0 {
                result: Some(get_documents_split_count_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_documents_split_count_empty_split_property() {
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

        let request = GetDocumentsSplitCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            split_count_by_index_property: "".to_string(),
            prove: false,
        };

        let result = platform
            .query_documents_split_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg == "split_count_by_index_property must not be empty"
        ));
    }

    #[test]
    fn test_documents_split_count_nonexistent_property() {
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

        let request = GetDocumentsSplitCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            split_count_by_index_property: "nonExistentProp".to_string(),
            prove: false,
        };

        let result = platform
            .query_documents_split_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("property nonExistentProp not found")
        ));
    }
}
