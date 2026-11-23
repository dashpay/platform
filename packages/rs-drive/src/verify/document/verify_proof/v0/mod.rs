use crate::verify::RootHash;

use crate::error::Error;
use crate::query::DriveDocumentQuery;
use dpp::data_contract::document_type::{
    DecodedValueBudget, DEFAULT_CLIENT_PAGE_DECODED_VALUE_BUDGET,
};
use dpp::document::Document;

use dpp::version::PlatformVersion;

impl DriveDocumentQuery<'_> {
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
    pub(super) fn verify_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Document>), Error> {
        // indexOnly documents have no stored serialization — the entry's
        // proved (path, key) position IS the document, so it is synthesized
        // directly rather than round-tripping through serialized bytes
        // (which a subset index's projection could not produce at all).
        {
            use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
            if self.document_type.index_only() {
                return self.verify_index_only_proof(proof, platform_version);
            }
        }

        self.verify_proof_keep_serialized(proof, platform_version)
            .map(|(root_hash, documents)| {
                let mut budget = DecodedValueBudget::new(DEFAULT_CLIENT_PAGE_DECODED_VALUE_BUDGET);
                let documents = documents
                    .into_iter()
                    .map(|serialized| {
                        Document::from_bytes_with_budget(
                            serialized.as_slice(),
                            self.document_type,
                            &mut budget,
                            platform_version,
                        )
                        .map_err(Error::from)
                    })
                    .collect::<Result<Vec<Document>, Error>>()?;
                Ok((root_hash, documents))
            })?
    }
}

#[cfg(test)]
mod tests {
    use crate::query::DriveDocumentQuery;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contracts::SystemDataContract;
    use dpp::document::DocumentV0Getters;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_document_collection() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("expected to load DPNS contract");

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("preorder")
            .expect("expected preorder document type");

        // Insert a few documents
        let mut inserted_doc_ids = vec![];
        for seed in 1u64..=3 {
            let document = document_type
                .random_document(Some(seed), platform_version)
                .expect("expected a random document");
            let doc_id = document.id();
            inserted_doc_ids.push(doc_id);

            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&document, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to insert document");
        }

        // Build a query that fetches all documents (any_item_query has limit=1)
        let query = DriveDocumentQuery::all_items_query(&contract, document_type, None);

        // Get proof
        let (proof, _cost) = query
            .clone()
            .execute_with_proof(&drive, None, None, platform_version)
            .expect("expected to execute query with proof");

        // Verify proof via the v0 method (called through the public verify_proof)
        let (_root_hash, verified_docs) = query
            .verify_proof(proof.as_slice(), platform_version)
            .expect("expected proof verification to succeed");

        assert_eq!(verified_docs.len(), 3);

        // Verify all inserted doc IDs are present
        for doc_id in &inserted_doc_ids {
            assert!(
                verified_docs.iter().any(|d| d.id() == *doc_id),
                "expected document {:?} to be present in verified results",
                doc_id
            );
        }
    }

    /// A padded `startAfter` secondary-index page (`pads_cursor_page`) must compare the first
    /// row's ID from the serialized header rather than decoding its properties: even when the
    /// cursor falls outside the query's clauses and the first matching row would exceed the
    /// decoded-value budget, `verify_proof_keep_serialized` returns the serialized row without
    /// expanding its properties, and `verify_proof` rejects it with the typed budget error.
    #[test]
    fn should_compare_start_after_cursor_from_header_without_expanding_oversized_first_row() {
        use crate::error::Error;
        use crate::query::{InternalClauses, OrderClause, WhereClause, WhereOperator};
        use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
        use dpp::data_contract::errors::DataContractError;
        use dpp::data_contract::DataContract;
        use dpp::document::{Document, DocumentV0};
        use dpp::platform_value::{platform_value, Identifier, Value};
        use dpp::ProtocolError;
        use indexmap::IndexMap;
        use std::collections::BTreeMap;

        let drive = setup_drive_with_initial_state_structure(None);
        let pv14 = PlatformVersion::get(14).expect("PV14 exists");
        let latest = PlatformVersion::latest();

        // Build a PV14-validated contract with a secondary index and 4 nested objects of 65
        // zero-width typed-array properties each (260 * 1025 + 4 = 266,504 > 262,144).
        let groups = 4usize;
        let fields_per_group = 65usize;
        let mut properties = vec![
            (
                Value::Text("category".to_string()),
                platform_value!({ "type": "string", "maxLength": 16, "position": 0 }),
            ),
            (
                Value::Text("rank".to_string()),
                platform_value!({ "type": "integer", "minimum": 0, "maximum": 1000, "position": 1 }),
            ),
        ];
        for g in 0..groups {
            let mut inner = Vec::with_capacity(fields_per_group);
            for i in 0..fields_per_group {
                inner.push((
                    Value::Text(format!("f{i:02}")),
                    platform_value!({
                        "type": "array",
                        "maxItems": 1024,
                        "items": {
                            "type": "array",
                            "byteArray": true,
                            "minItems": 0,
                            "maxItems": 0,
                        },
                        "position": i as u32,
                    }),
                ));
            }
            properties.push((
                Value::Text(format!("g{g}")),
                platform_value!({
                    "type": "object",
                    "properties": Value::Map(inner),
                    "additionalProperties": false,
                    "position": (g + 2) as u32,
                }),
            ));
        }

        let contract = DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": Identifier::new([0x51; 32]),
                "ownerId": Identifier::new([0x52; 32]),
                "version": 1u32,
                "documentSchemas": {
                    "item": {
                        "type": "object",
                        "indices": [
                            {
                                "name": "byCategoryRank",
                                "properties": [
                                    { "category": "asc" },
                                    { "rank": "asc" },
                                ],
                            }
                        ],
                        "properties": Value::Map(properties),
                        "required": ["category", "rank"],
                        "additionalProperties": false,
                    }
                }
            }),
            true,
            pv14,
        )
        .expect("PV14 contract registers");

        drive
            .apply_contract(&contract, BlockInfo::default(), true, None, None, latest)
            .expect("contract applies");
        let document_type = contract.document_type_for_name("item").expect("item type");

        // Small cursor document outside the query's `category == "target"` clause (rank = 1).
        let cursor_doc = Document::V0(DocumentV0 {
            id: Identifier::new([0x10; 32]),
            owner_id: Identifier::new([0x52; 32]),
            revision: Some(1),
            properties: BTreeMap::from([
                ("category".to_string(), Value::Text("other".to_string())),
                ("rank".to_string(), Value::U16(1)),
            ]),
            ..Default::default()
        });

        // Oversized first matching row in `category == "target"` (rank = 2): 266,506 values.
        let mut oversized_props = BTreeMap::from([
            ("category".to_string(), Value::Text("target".to_string())),
            ("rank".to_string(), Value::U16(2)),
        ]);
        for g in 0..groups {
            let group_entries: Vec<(Value, Value)> = (0..fields_per_group)
                .map(|i| {
                    (
                        Value::Text(format!("f{i:02}")),
                        Value::Array(vec![Value::Bytes(vec![]); 1024]),
                    )
                })
                .collect();
            oversized_props.insert(format!("g{g}"), Value::Map(group_entries));
        }
        let oversized_doc = Document::V0(DocumentV0 {
            id: Identifier::new([0x20; 32]),
            owner_id: Identifier::new([0x52; 32]),
            revision: Some(1),
            properties: oversized_props,
            ..Default::default()
        });

        for document in [&cursor_doc, &oversized_doc] {
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((document, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    latest,
                )
                .expect("document inserts");
        }

        let query = DriveDocumentQuery {
            contract: &contract,
            document_type,
            internal_clauses: InternalClauses {
                equal_clauses: BTreeMap::from([(
                    "category".to_string(),
                    WhereClause {
                        field: "category".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Text("target".to_string()),
                    },
                )]),
                ..Default::default()
            },
            offset: None,
            limit: Some(10),
            order_by: IndexMap::from([(
                "rank".to_string(),
                OrderClause {
                    field: "rank".to_string(),
                    ascending: true,
                },
            )]),
            start_at: Some(cursor_doc.id().to_buffer()),
            start_at_included: false,
            block_time_ms: None,
            resolved_time_ranges: Vec::new(),
            sub_queries: Vec::new(),
        };
        assert!(
            query.pads_cursor_page(latest),
            "secondary-index startAfter query must pad the cursor page"
        );

        let (proof, _) = query
            .clone()
            .execute_with_proof(&drive, None, None, latest)
            .expect("proof executes");

        // Header-only cursor comparison succeeds without expanding the oversized row's properties.
        let (_, serialized_docs) = query
            .verify_proof_keep_serialized(&proof, latest)
            .expect("keep_serialized must compare cursor ID from header only");
        assert_eq!(serialized_docs.len(), 1);
        assert_eq!(
            dpp::document::Document::id_from_bytes(&serialized_docs[0], latest)
                .expect("header id reads"),
            oversized_doc.id()
        );

        // Full verification decodes under the page budget and refuses the oversized row.
        let error = query
            .verify_proof(&proof, latest)
            .expect_err("oversized row must exceed decoded value budget");
        assert!(
            matches!(
                error,
                Error::Protocol(ref protocol_error)
                    if matches!(
                        protocol_error.as_ref(),
                        ProtocolError::DataContractError(
                            DataContractError::DecodedValueBudgetExceeded(_)
                        )
                    )
            ),
            "expected DecodedValueBudgetExceeded, got: {error:?}"
        );
    }

    #[test]
    fn should_strip_cursor_from_distinct_page_without_decoding_document_body() {
        use crate::query::{InternalClauses, OrderClause, WhereClause, WhereOperator};
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
        use dpp::data_contract::DataContract;
        use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
        use dpp::document::Document;
        use dpp::platform_value::{platform_value, Identifier, Value};
        use indexmap::IndexMap;
        use std::collections::BTreeMap;

        let latest = PlatformVersion::latest();
        let contract = DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": Identifier::new([0x33; 32]),
                "version": 1u32,
                "ownerId": Identifier::new([0x44; 32]),
                "config": {
                    "$formatVersion": "1",
                    "canBeDeleted": false,
                    "readonly": false,
                    "keepsHistory": false,
                    "documentsKeepHistoryContractDefault": false,
                    "documentsMutableContractDefault": true,
                    "documentsCanBeDeletedContractDefault": true,
                    "requiresIdentityEncryptionBoundedKey": Value::Null,
                    "requiresIdentityDecryptionBoundedKey": Value::Null,
                    "sizedIntegerTypes": true,
                },
                "documentSchemas": {
                    "item": {
                        "type": "object",
                        "indices": [
                            {
                                "name": "byCategoryRank",
                                "properties": [
                                    { "category": "asc" },
                                    { "rank": "asc" }
                                ]
                            }
                        ],
                        "properties": {
                            "category": { "type": "string", "maxLength": 32, "position": 0 },
                            "rank": { "type": "integer", "minimum": 0, "maximum": 1000, "position": 1 }
                        },
                        "required": ["category", "rank"],
                        "additionalProperties": false
                    }
                }
            }),
            false,
            latest,
        )
        .expect("contract builds");
        let document_type = contract.document_type_for_name("item").expect("item type");

        let cursor_id = Identifier::new([0xAA; 32]);
        let next_id = Identifier::new([0xBB; 32]);

        // Synthetic format-3 header prefixes (format = 3, stamp = 0, 32-byte $id) with NO
        // decodable document body. Full `Document::from_bytes` must reject both slices.
        let mut cursor_header_only = vec![3u8, 0u8];
        cursor_header_only.extend_from_slice(cursor_id.as_bytes());
        let mut next_header_only = vec![3u8, 0u8];
        next_header_only.extend_from_slice(next_id.as_bytes());
        assert!(Document::from_bytes(&cursor_header_only, document_type, latest).is_err());
        assert!(Document::from_bytes(&next_header_only, document_type, latest).is_err());

        let query = DriveDocumentQuery {
            contract: &contract,
            document_type,
            internal_clauses: InternalClauses {
                equal_clauses: BTreeMap::from([(
                    "category".to_string(),
                    WhereClause {
                        field: "category".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Text("target".to_string()),
                    },
                )]),
                ..Default::default()
            },
            offset: None,
            limit: Some(10),
            order_by: IndexMap::from([(
                "rank".to_string(),
                OrderClause {
                    field: "rank".to_string(),
                    ascending: true,
                },
            )]),
            start_at: Some(cursor_id.to_buffer()),
            start_at_included: false,
            block_time_ms: None,
            resolved_time_ranges: Vec::new(),
            sub_queries: Vec::new(),
        };
        assert!(query.pads_cursor_page(latest));

        // When the leading row matches the cursor ID, it is stripped from its header alone.
        let (stripped, skipped) = query
            .strip_cursor_from_distinct_page(
                vec![cursor_header_only.clone(), next_header_only.clone()],
                latest,
            )
            .expect("strip_cursor_from_distinct_page must only inspect the header");
        assert_eq!(skipped, 0);
        assert_eq!(stripped, vec![next_header_only.clone()]);

        // When the leading row does not match the cursor ID, it is kept from its header alone.
        let (kept, skipped) = query
            .strip_cursor_from_distinct_page(vec![next_header_only.clone()], latest)
            .expect("non-cursor leading row must be checked from header alone");
        assert_eq!(skipped, 0);
        assert_eq!(kept, vec![next_header_only]);
    }
}
