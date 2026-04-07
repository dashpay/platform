use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "server")]
use crate::drive::Drive;
#[cfg(feature = "server")]
use crate::error::Error;
#[cfg(feature = "server")]
use crate::util::grove_operations::DirectQueryType;
#[cfg(feature = "server")]
use dpp::version::drive_versions::DriveVersion;
#[cfg(feature = "server")]
use grovedb::query_result_type::QueryResultType;
#[cfg(feature = "server")]
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
#[cfg(feature = "server")]
use grovedb_path::SubtreePath;

#[cfg(feature = "server")]
use crate::drive::RootTree;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::IndexProperty;
use dpp::data_contract::document_type::{DocumentTypeRef, Index};
#[cfg(feature = "server")]
use dpp::version::PlatformVersion;

use super::conditions::{WhereClause, WhereOperator};

/// A query to count documents using CountTree elements in the index path.
///
/// This struct encapsulates all the information needed to perform a count
/// query on a document type's countable index, including optional split-by
/// functionality for getting per-value counts.
#[derive(Debug, Clone)]
pub struct DriveDocumentCountQuery<'a> {
    /// The document type to count
    pub document_type: DocumentTypeRef<'a>,
    /// The contract id (32 bytes)
    pub contract_id: [u8; 32],
    /// The document type name
    pub document_type_name: String,
    /// The countable index to use
    pub index: &'a Index,
    /// The equality where clauses that match index prefix properties
    pub where_clauses: Vec<WhereClause>,
    /// Optional property to split counts by. When set, returns per-value
    /// counts for this property instead of a single total count.
    pub split_by_property: Option<String>,
}

/// An entry in a split count result, containing the serialized key
/// and the count of documents matching that key value.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitCountEntry {
    /// The serialized key bytes for this value
    pub key: Vec<u8>,
    /// The count of documents matching this key value
    pub count: u64,
}

impl<'a> DriveDocumentCountQuery<'a> {
    /// Finds a countable index whose properties form a prefix that matches the
    /// equality where clauses. For a count query:
    /// - All where clause fields must appear as a prefix of the index properties
    /// - The index must have countable=true
    /// - Among matching indexes, we prefer the one with the most properties
    ///   matched by where clauses (most specific)
    pub fn find_countable_index_for_where_clauses<'b>(
        indexes: &'b BTreeMap<String, Index>,
        where_clauses: &[WhereClause],
    ) -> Option<&'b Index> {
        let equality_fields: BTreeSet<&str> = where_clauses
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

    /// Finds a countable index where:
    /// - The equality where clause fields form a prefix of the index properties
    /// - The split_property is the next property after the covered prefix
    /// - The index has countable=true
    pub fn find_countable_index_for_split<'b>(
        indexes: &'b BTreeMap<String, Index>,
        where_clauses: &[WhereClause],
        split_property: &str,
    ) -> Option<&'b Index> {
        let equality_fields: BTreeSet<&str> = where_clauses
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

    /// Executes the count query without generating a proof.
    ///
    /// When `split_by_property` is `None`, returns the total count as a single
    /// `SplitCountEntry` with an empty key.
    ///
    /// When `split_by_property` is `Some`, returns per-value counts for the
    /// split property.
    #[cfg(feature = "server")]
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        if self.split_by_property.is_some() {
            self.execute_split_count(drive, transaction, platform_version)
        } else {
            let count = self.execute_total_count(drive, transaction, platform_version)?;
            Ok(vec![SplitCountEntry { key: vec![], count }])
        }
    }

    /// Executes the count query and generates a GroveDB proof.
    ///
    /// Returns the raw proof bytes. The caller is responsible for verifying
    /// the proof and extracting the count from the verified result.
    #[cfg(feature = "server")]
    pub fn execute_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;

        // Build the same path as execute_no_proof
        let mut path = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];

        // Walk the index properties, pushing property keys and equality values
        for prop in &self.index.properties {
            let matching_clause = self
                .where_clauses
                .iter()
                .find(|wc| wc.field == prop.name && wc.operator == WhereOperator::Equal);

            if let Some(clause) = matching_clause {
                path.push(prop.name.as_bytes().to_vec());
                let serialized_value = self.document_type.serialize_value_for_key(
                    prop.name.as_str(),
                    &clause.value,
                    platform_version,
                )?;
                path.push(serialized_value);
            } else {
                break;
            }
        }

        // Build a path query that covers the count tree and its contents
        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(path, SizedQuery::new(query, None, None));

        let proof = drive
            .grove
            .get_proved_path_query(&path_query, None, transaction, &drive_version.grove_version)
            .unwrap()
            .map_err(|e| Error::GroveDB(Box::new(e)))?;

        Ok(proof)
    }

    /// Executes the total count query, returning a single u64 count.
    #[cfg(feature = "server")]
    fn execute_total_count(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let drive_version = &platform_version.drive;

        // Build the base path: [DataContractDocuments, contract_id, 1, doc_type_name]
        let mut path = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];

        // Walk the index properties, pushing property keys and values for
        // each equality where clause.
        let mut covered_count = 0;
        for prop in &self.index.properties {
            let matching_clause = self
                .where_clauses
                .iter()
                .find(|wc| wc.field == prop.name && wc.operator == WhereOperator::Equal);

            if let Some(clause) = matching_clause {
                // Push the index property key
                path.push(prop.name.as_bytes().to_vec());
                // Serialize and push the property value
                let serialized_value = self.document_type.serialize_value_for_key(
                    prop.name.as_str(),
                    &clause.value,
                    platform_version,
                )?;
                path.push(serialized_value);
                covered_count += 1;
            } else {
                break;
            }
        }

        if covered_count == self.index.properties.len() {
            // All index properties are covered -- O(1) fetch of the single CountTree.
            Self::fetch_count_at_path(drive, &path, transaction, drive_version)
        } else {
            // Not all properties covered. Iterate over remaining levels.
            let remaining_properties = &self.index.properties[covered_count..];
            Self::count_recursive(
                drive,
                path,
                remaining_properties,
                transaction,
                drive_version,
            )
        }
    }

    /// Executes a split count query, returning per-value counts for the
    /// split property.
    #[cfg(feature = "server")]
    fn execute_split_count(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let drive_version = &platform_version.drive;
        let split_property = self
            .split_by_property
            .as_deref()
            .expect("split_by_property must be Some when calling execute_split_count");

        // Build the base path: [DataContractDocuments, contract_id, 1, doc_type_name]
        let mut path = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];

        // Walk the index properties up to (but not including) the split property,
        // pushing property keys and serialized values from the where clauses.
        for prop in &self.index.properties {
            if prop.name == split_property {
                break;
            }

            let matching_clause = self
                .where_clauses
                .iter()
                .find(|wc| wc.field == prop.name && wc.operator == WhereOperator::Equal);

            if let Some(clause) = matching_clause {
                path.push(prop.name.as_bytes().to_vec());
                let serialized_value = self.document_type.serialize_value_for_key(
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
        let result = drive.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut drive_operations,
            drive_version,
        );

        let (elements, _) = match result {
            Ok(result) => result,
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathNotFound(_)
                        | grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathKeyNotFound(_)
                ) =>
            {
                return Ok(vec![]);
            }
            Err(e) => return Err(e),
        };

        let key_elements = elements.to_key_elements();

        if key_elements.is_empty() {
            return Ok(vec![]);
        }

        // Determine how many remaining index properties follow the split property
        let split_prop_idx = self
            .index
            .properties
            .iter()
            .position(|p| p.name == split_property)
            .unwrap_or(0);
        let remaining_properties = &self.index.properties[split_prop_idx + 1..];

        let mut entries = Vec::new();

        for (key, _element) in key_elements {
            let mut value_path = path.clone();
            value_path.push(key.clone());

            let count = if remaining_properties.is_empty() {
                Self::fetch_count_at_path(drive, &value_path, transaction, drive_version)?
            } else {
                Self::count_recursive(
                    drive,
                    value_path,
                    remaining_properties,
                    transaction,
                    drive_version,
                )?
            };

            if count > 0 {
                entries.push(SplitCountEntry { key, count });
            }
        }

        Ok(entries)
    }

    /// Fetches the CountTree element count at the given path.
    /// The CountTree element is at key [0] under the path.
    #[cfg(feature = "server")]
    fn fetch_count_at_path(
        drive: &Drive,
        path: &[Vec<u8>],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<u64, Error> {
        let mut drive_operations = vec![];
        let path_refs: Vec<&[u8]> = path.iter().map(|p| p.as_slice()).collect();
        let element = drive.grove_get_raw_optional(
            SubtreePath::from(path_refs.as_slice()),
            &[0],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;

        Ok(element.map_or(0, |e| e.count_value_or_default()))
    }

    /// Recursively descends through remaining index property levels,
    /// iterating over all values at each level, and sums the CountTree
    /// counts at the terminal level.
    #[cfg(feature = "server")]
    fn count_recursive(
        drive: &Drive,
        current_path: Vec<Vec<u8>>,
        remaining_properties: &[IndexProperty],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<u64, Error> {
        if remaining_properties.is_empty() {
            return Self::fetch_count_at_path(drive, &current_path, transaction, drive_version);
        }

        let prop = &remaining_properties[0];
        let rest = &remaining_properties[1..];

        // Push the index property key to descend into that level
        let mut property_path = current_path;
        property_path.push(prop.name.as_bytes().to_vec());

        // Query all children (value subtrees) at this property level
        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(property_path.clone(), SizedQuery::new(query, None, None));

        let mut drive_operations = vec![];
        let result = drive.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut drive_operations,
            drive_version,
        );

        let (elements, _) = match result {
            Ok(result) => result,
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathNotFound(_)
                        | grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathKeyNotFound(_)
                ) =>
            {
                return Ok(0);
            }
            Err(e) => return Err(e),
        };

        let key_elements = elements.to_key_elements();

        if key_elements.is_empty() {
            return Ok(0);
        }

        let mut total_count: u64 = 0;

        for (key, _element) in key_elements {
            let mut value_path = property_path.clone();
            value_path.push(key);

            let sub_count =
                Self::count_recursive(drive, value_path, rest, transaction, drive_version)?;
            total_count = total_count.saturating_add(sub_count);
        }

        Ok(total_count)
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::Drive;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::platform_value::Value;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use dpp::version::PlatformVersion;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::borrow::Cow;

    fn setup_drive_and_contract() -> (Drive, dpp::prelude::DataContract) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, data_contract)
    }

    fn insert_random_documents(
        drive: &Drive,
        data_contract: &dpp::prelude::DataContract,
        document_type_name: &str,
        count: usize,
        seed: u64,
    ) {
        let platform_version = PlatformVersion::latest();
        let document_type = data_contract
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(seed);
        for _ in 0..count {
            let random_document = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");

            let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&random_document, storage_flags)),
                            owner_id: None,
                        },
                        contract: data_contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to insert document");
        }
    }

    #[test]
    fn test_count_query_total_count_with_documents() {
        let (drive, data_contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();

        insert_random_documents(&drive, &data_contract, "person", 5, 500);

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &[],
        )
        .expect("expected to find countable index");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "person".to_string(),
            index,
            where_clauses: vec![],
            split_by_property: None,
        };

        let results = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("expected query to succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].count, 5, "expected count of 5 documents");
        assert!(
            results[0].key.is_empty(),
            "expected empty key for total count"
        );

        // Also verify proof generation works
        let proof = query
            .execute_with_proof(&drive, None, platform_version)
            .expect("expected proof generation to succeed");
        assert!(!proof.is_empty(), "expected non-empty proof");
    }

    #[test]
    fn test_count_query_total_count_empty() {
        let (drive, data_contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &[],
        )
        .expect("expected to find countable index");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "person".to_string(),
            index,
            where_clauses: vec![],
            split_by_property: None,
        };

        let results = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("expected query to succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].count, 0, "expected count of 0 documents");

        // Also verify proof generation works on empty index
        let proof = query
            .execute_with_proof(&drive, None, platform_version)
            .expect("expected proof generation to succeed");
        assert!(!proof.is_empty(), "expected non-empty proof");
    }

    #[test]
    fn test_count_query_split_by_property() {
        let (drive, data_contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();

        insert_random_documents(&drive, &data_contract, "person", 5, 600);

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let index = DriveDocumentCountQuery::find_countable_index_for_split(
            document_type.indexes(),
            &[],
            "firstName",
        )
        .expect("expected to find countable index for split");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "person".to_string(),
            index,
            where_clauses: vec![],
            split_by_property: Some("firstName".to_string()),
        };

        let results = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("expected query to succeed");

        let total: u64 = results.iter().map(|e| e.count).sum();
        assert_eq!(total, 5, "expected total split count of 5 documents");

        for entry in &results {
            assert!(!entry.key.is_empty(), "expected non-empty split key");
            assert!(entry.count > 0, "expected positive count per split");
        }

        // Also verify proof generation works for split query
        let proof = query
            .execute_with_proof(&drive, None, platform_version)
            .expect("expected proof generation to succeed");
        assert!(!proof.is_empty(), "expected non-empty proof");
    }

    #[test]
    fn test_find_countable_index_for_where_clauses_no_match() {
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        // Create a where clause for a field that doesn't appear as a prefix of any index
        let where_clause = WhereClause {
            field: "nonExistentField".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("test".to_string()),
        };

        let result = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &[where_clause],
        );

        assert!(
            result.is_none(),
            "expected no countable index for non-existent field"
        );
    }

    #[test]
    fn test_find_countable_index_for_split_no_match() {
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let result = DriveDocumentCountQuery::find_countable_index_for_split(
            document_type.indexes(),
            &[],
            "nonExistentField",
        );

        assert!(
            result.is_none(),
            "expected no countable index for non-existent split field"
        );
    }
}
