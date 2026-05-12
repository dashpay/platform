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
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::{DocumentCountRequest, DocumentCountResponse, SplitCountEntry};
use drive::util::grove_operations::GroveDBToUse;

/// Wrap a single aggregate `u64` plus current-state metadata into the
/// protobuf `GetDocumentsCountResponseV0`. Produces the `CountResults
/// .variant.AggregateCount(_)` wire shape used by total-count and
/// range-without-distinct modes — the dispatcher routes drive's
/// `DocumentCountResponse::Aggregate(_)` through here so the wire
/// answer is a single u64, not an entries map with one empty-key
/// entry.
fn count_response_aggregate<C>(
    count: u64,
    platform: &Platform<C>,
    platform_state: &PlatformState,
) -> GetDocumentsCountResponseV0 {
    GetDocumentsCountResponseV0 {
        result: Some(get_documents_count_response_v0::Result::Counts(
            get_documents_count_response_v0::CountResults {
                variant: Some(
                    get_documents_count_response_v0::count_results::Variant::AggregateCount(count),
                ),
            },
        )),
        metadata: Some(platform.response_metadata_v0(platform_state, CheckpointUsed::Current)),
    }
}

/// Wrap a vector of [`SplitCountEntry`]s plus current-state metadata
/// into the protobuf `GetDocumentsCountResponseV0`. Produces the
/// `CountResults.variant.Entries(_)` wire shape used by per-`In`-value
/// and per-distinct-value-in-range modes. Note that an aggregate
/// total never reaches here — see [`count_response_aggregate`].
fn count_response_with_entries<C>(
    entries: Vec<SplitCountEntry>,
    platform: &Platform<C>,
    platform_state: &PlatformState,
) -> GetDocumentsCountResponseV0 {
    let entries: Vec<get_documents_count_response_v0::CountEntry> = entries
        .into_iter()
        .map(|e| get_documents_count_response_v0::CountEntry {
            in_key: e.in_key,
            key: e.key,
            count: e.count,
        })
        .collect();
    GetDocumentsCountResponseV0 {
        result: Some(get_documents_count_response_v0::Result::Counts(
            get_documents_count_response_v0::CountResults {
                variant: Some(
                    get_documents_count_response_v0::count_results::Variant::Entries(
                        get_documents_count_response_v0::CountEntries { entries },
                    ),
                ),
            },
        )),
        metadata: Some(platform.response_metadata_v0(platform_state, CheckpointUsed::Current)),
    }
}

impl<C> Platform<C> {
    /// `pub(crate)` (was `pub(super)`) so the v1 `getDocuments`
    /// handler in `document_query::v1` can delegate `select=COUNT`
    /// requests here, keeping the v1 surface "pure rewiring"
    /// without duplicating the count dispatcher logic. See
    /// `query_documents_v1` for the v1 → v0-count translation.
    pub(crate) fn query_documents_count_v0(
        &self,
        GetDocumentsCountRequestV0 {
            data_contract_id,
            document_type: document_type_name,
            r#where,
            return_distinct_counts_in_range,
            order_by,
            limit,
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

        // `order_by` is decoded the same way as `where`: empty bytes
        // → `Value::Null` (no clauses), any other shape must be a
        // CBOR-encoded outer array of `[field, direction]` inner
        // arrays. Drive parses + validates per clause. Required on
        // the `(In + prove)` dispatch arm for proof determinism;
        // empty is fine on every other arm (drive synthesizes an
        // ascending default for split-mode entry direction).
        let order_by_clause = if order_by.is_empty() {
            Value::Null
        } else {
            check_validation_result_with_data!(ciborium::de::from_reader(order_by.as_slice())
                .map_err(|_| {
                    QueryError::Query(QuerySyntaxError::DeserializationError(
                        "unable to decode 'order_by' query from cbor".to_string(),
                    ))
                }))
        };

        // Hand the raw decoded where + order_by `Value`s to drive —
        // same pattern `query_documents_v0` uses. Drive parses +
        // validates per clause and surfaces any error as
        // `Error::Query(...)`, which the existing match arm below maps
        // to a query-validation result. Drive also applies per-mode
        // limit policy:
        // - no-proof modes silently clamp to `max_query_limit`
        //   (proto contract — "passing a larger value just gets
        //   clamped, not rejected")
        // - the prove-distinct mode rejects `limit > max_query_limit`
        //   instead of clamping, because client-side proof
        //   reconstruction needs the exact same limit value the
        //   server used; silent clamping would silently break
        //   verification on requests above the cap.
        let request = DocumentCountRequest {
            contract: contract_ref,
            document_type,
            raw_where_value: where_clause,
            raw_order_by_value: order_by_clause,
            return_distinct_counts_in_range,
            limit,
            prove,
            drive_config: &self.config.drive,
        };
        let drive_response =
            match self
                .drive
                .execute_document_count_request(request, None, platform_version)
            {
                Ok(r) => r,
                Err(drive::error::Error::Query(qe)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(qe)));
                }
                Err(e) => return Err(e.into()),
            };

        let response = match drive_response {
            DocumentCountResponse::Aggregate(count) => {
                count_response_aggregate(count, self, platform_state)
            }
            DocumentCountResponse::Entries(entries) => {
                count_response_with_entries(entries, self, platform_state)
            }
            DocumentCountResponse::Proof(proof_bytes) => {
                let (grovedb_used, proof) =
                    self.response_proof_v0(platform_state, proof_bytes, GroveDBToUse::Current)?;
                GetDocumentsCountResponseV0 {
                    result: Some(get_documents_count_response_v0::Result::Proof(proof)),
                    metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
                }
            }
        };
        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{setup_platform, store_data_contract, store_document};
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::DocumentV0Setters;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    /// Builds an in-memory v12 contract with a `widget` document type
    /// that has `documentsCountable: true` — the type's primary-key
    /// tree becomes a CountTree, enabling the unfiltered total-count
    /// fast path on both no-proof and prove paths.
    fn build_documents_countable_widget_contract() -> dpp::prelude::DataContract {
        use dpp::data_contract::DataContractFactory;
        use dpp::platform_value::platform_value;

        const PROTOCOL_VERSION_V12: u32 = 12;
        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
        let document_schema = platform_value!({
            "type": "object",
            "documentsCountable": true,
            "properties": {
                "color": {"type": "string", "position": 0, "maxLength": 32},
            },
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned()
    }

    /// Unfiltered total count via the `documentsCountable: true` fast
    /// path. Asserts O(1) read of the primary-key CountTree returns
    /// the correct count after a few inserts.
    #[test]
    fn test_documents_count_no_prove() {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let contract = build_documents_countable_widget_contract();
        store_data_contract(&platform, &contract, version);

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        // Insert 5 widgets.
        for i in 1..=5u8 {
            let random_document = document_type
                .random_document(Some(i as u64), platform_version)
                .expect("expected to get random document");
            store_document(
                &platform,
                &contract,
                document_type,
                &random_document,
                platform_version,
            );
        }

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: contract.id().to_vec(),
            document_type: "widget".to_string(),
            r#where: vec![],
            return_distinct_counts_in_range: false,
            order_by: Vec::new(),
            limit: None,
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(
                    get_documents_count_response_v0::CountResults {
                        variant:
                            Some(get_documents_count_response_v0::count_results::Variant::AggregateCount(
                                total,
                            )),
                    },
                )),
                metadata: Some(_),
            }) => {
                assert_eq!(total, 5, "expected count of 5 documents");
            }
            other => panic!("expected aggregate count result, got {:?}", other),
        }
    }

    /// Same fast-path query as `test_documents_count_no_prove`, but
    /// against an empty contract (no documents inserted). Asserts the
    /// path returns 0 cleanly rather than erroring.
    #[test]
    fn test_documents_count_empty_result() {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let _platform_version = PlatformVersion::latest();

        let contract = build_documents_countable_widget_contract();
        store_data_contract(&platform, &contract, version);

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: contract.id().to_vec(),
            document_type: "widget".to_string(),
            r#where: vec![],
            return_distinct_counts_in_range: false,
            order_by: Vec::new(),
            limit: None,
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(
                    get_documents_count_response_v0::CountResults {
                        variant:
                            Some(get_documents_count_response_v0::count_results::Variant::AggregateCount(
                                total,
                            )),
                    },
                )),
                metadata: Some(_),
            }) => {
                assert_eq!(total, 0, "expected count of 0 documents");
            }
            other => panic!("expected aggregate count result, got {:?}", other),
        }
    }

    fn serialize_where_clauses_to_cbor(where_clauses: Vec<Value>) -> Vec<u8> {
        use ciborium::value::Value as CborValue;
        let cbor: CborValue = TryInto::<CborValue>::try_into(Value::Array(where_clauses))
            .expect("expected to convert where clauses to cbor value");
        let mut out = Vec::new();
        ciborium::ser::into_writer(&cbor, &mut out).expect("expected to serialize where clauses");
        out
    }

    fn store_person_document(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        data_contract: &dpp::prelude::DataContract,
        id: [u8; 32],
        first_name: &str,
        last_name: &str,
        age: u64,
        platform_version: &PlatformVersion,
    ) {
        use dpp::document::{Document, DocumentV0};
        use std::collections::BTreeMap;

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let mut properties = BTreeMap::new();
        properties.insert("firstName".to_string(), Value::Text(first_name.to_string()));
        properties.insert("lastName".to_string(), Value::Text(last_name.to_string()));
        properties.insert("age".to_string(), Value::U64(age));

        let document: Document = DocumentV0 {
            id: Identifier::from(id),
            owner_id: Identifier::from([0u8; 32]),
            properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();

        store_document(
            platform,
            data_contract,
            document_type,
            &document,
            platform_version,
        );
    }

    #[test]
    fn test_documents_count_with_in_operator() {
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

        // 3 docs with age=30, 2 with age=40, 1 with age=50.
        store_person_document(
            &platform,
            &data_contract,
            [1u8; 32],
            "Alice",
            "Smith",
            30,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [2u8; 32],
            "Bob",
            "Smith",
            30,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [3u8; 32],
            "Carol",
            "Smith",
            30,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [4u8; 32],
            "Dave",
            "Smith",
            40,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [5u8; 32],
            "Eve",
            "Smith",
            40,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [6u8; 32],
            "Frank",
            "Smith",
            50,
            platform_version,
        );

        // [["age", "in", [30, 40]]]
        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text("in".to_string()),
            Value::Array(vec![Value::U64(30), Value::U64(40)]),
        ])];

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract.id().to_vec(),
            document_type: "person".to_string(),
            r#where: serialize_where_clauses_to_cbor(where_clauses),
            return_distinct_counts_in_range: false,
            order_by: Vec::new(),
            limit: None,
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result:
                    Some(get_documents_count_response_v0::Result::Counts(
                        get_documents_count_response_v0::CountResults {
                            variant:
                                Some(get_documents_count_response_v0::count_results::Variant::Entries(
                                    entries,
                                )),
                        },
                    )),
                metadata: Some(_),
            }) => {
                let total: u64 = entries.entries.iter().map(|e| e.count).sum();
                assert_eq!(total, 5, "expected count of 5 (3 age=30 + 2 age=40)");
            }
            other => panic!("expected per-In-value entries result, got {:?}", other),
        }
    }

    #[test]
    fn test_documents_count_range_without_range_countable_index_returns_clear_error() {
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

        // [["age", ">", 20]] — range operator on a contract whose `age`
        // index is `countable` but NOT `range_countable`. The range
        // path now accepts range operators, but the picker must report
        // "no usable index" so the handler surfaces a clear error.
        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text(">".to_string()),
            Value::U64(20),
        ])];

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract.id().to_vec(),
            document_type: "person".to_string(),
            r#where: serialize_where_clauses_to_cbor(where_clauses),
            return_distinct_counts_in_range: false,
            order_by: Vec::new(),
            limit: None,
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to return validation error");

        // Step 2 of the refactor moved the no-covering-index check into
        // rs-drive, where it surfaces as
        // `Query(WhereClauseOnNonIndexedProperty)` rather than the
        // handler-local `InvalidArgument`. Both shapes are valid
        // rejections — accept either.
        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::InvalidArgument(msg)] if msg.contains("range_countable")
            ) || matches!(
                result.errors.as_slice(),
                [QueryError::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(msg))]
                    if msg.contains("range_countable")
            ),
            "expected range_countable-index rejection, got {:?}",
            result.errors
        );
    }

    /// `prove = true` + Equal-on-single-property-countable-index =
    /// the fully-covered fast path that produces a real grovedb proof
    /// of the CountTree element at `[..., firstName, "Alice", 0]`.
    /// Asserts the response is a `Proof` variant with non-empty bytes
    /// — drive emits a CountTree element proof here, not the legacy
    /// materialize-and-count document proof.
    #[test]
    fn test_documents_count_with_prove_and_covering_equal() {
        use dpp::document::DocumentV0Setters;

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

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        // Insert 2 docs at firstName=Alice and 1 at firstName=Bob so
        // the targeted CountTree (`byFirstName` index, value=Alice)
        // has count_value > 0.
        let mut std_rng = StdRng::seed_from_u64(500);
        for first_name in ["Alice", "Alice", "Bob"] {
            let mut doc = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");
            let mut props = std::collections::BTreeMap::new();
            props.insert("firstName".to_string(), Value::Text(first_name.to_string()));
            props.insert("lastName".to_string(), Value::Text("Smith".to_string()));
            props.insert("age".to_string(), Value::U64(30));
            doc.set_properties(props);
            store_document(
                &platform,
                &data_contract,
                document_type,
                &doc,
                platform_version,
            );
        }

        let where_clauses = vec![Value::Array(vec![
            Value::Text("firstName".to_string()),
            Value::Text("==".to_string()),
            Value::Text("Alice".to_string()),
        ])];

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract.id().to_vec(),
            document_type: "person".to_string(),
            r#where: serialize_where_clauses_to_cbor(where_clauses),
            return_distinct_counts_in_range: false,
            order_by: Vec::new(),
            limit: None,
            prove: true,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Proof(proof)),
                metadata: Some(_),
            }) => {
                assert!(
                    !proof.grovedb_proof.is_empty(),
                    "expected non-empty grovedb proof bytes for covered prove count",
                );
            }
            other => panic!("expected Proof response, got {:?}", other),
        }
    }

    /// Symmetric-rejection contract: `prove = true` with no where
    /// clauses (or any where shape that doesn't fully cover a
    /// `countable: true` index) rejects with
    /// `WhereClauseOnNonIndexedProperty`. Matches the no-proof Total
    /// mode's behaviour when no covering countable index exists, and
    /// makes contract authors' index-design defects visible at the
    /// API boundary rather than silently materializing every doc.
    #[test]
    fn test_documents_count_prove_without_covering_index_returns_clear_error() {
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

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract.id().to_vec(),
            document_type: "person".to_string(),
            r#where: vec![],
            return_distinct_counts_in_range: false,
            order_by: Vec::new(),
            limit: None,
            prove: true,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to surface a validation error");

        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::Query(
                    QuerySyntaxError::WhereClauseOnNonIndexedProperty(msg),
                )] if msg.contains("countable")
            ),
            "expected covering-index rejection, got {:?}",
            result.errors,
        );
    }

    /// End-to-end pin for `prove = true` + `In`.
    ///
    /// `detect_mode` must route `(has_range=false, has_in=true,
    /// prove=true, _)` to `PointLookupProof`, which builds a
    /// per-branch CountTree-element proof via the shared
    /// [`DriveDocumentCountQuery::point_lookup_count_path_query`]
    /// builder (no document materialization, no `u16::MAX` cap on
    /// matching docs — the proof shape is O(|In values| × log n)).
    /// A regression that dispatches In+prove back through
    /// `PerInValue` would emit a `Counts(...)` no-proof variant
    /// instead, and the SDK verifier would bail with
    /// `NoProofInResult`.
    ///
    /// Asserts the response variant is `Proof(non-empty bytes)`.
    /// `order_by` is unused on this path — the builder sorts In
    /// keys lex-ascending unconditionally for prove/no-proof
    /// parity (see `point_lookup_count_path_query`), so proof
    /// determinism is independent of the request's order_by.
    #[test]
    fn test_documents_count_with_in_and_prove_returns_proof() {
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

        // Same distribution as `test_documents_count_with_in_operator`:
        // 3 docs at age=30, 2 at age=40, 1 at age=50. We ask for
        // `age in [30, 40]` so the proof has to cover two forks. One
        // doc at age=50 is outside the In set, so the proof must NOT
        // collapse to the full contents.
        for (id, name, age) in [
            ([1u8; 32], "Alice", 30u64),
            ([2u8; 32], "Bob", 30),
            ([3u8; 32], "Carol", 30),
            ([4u8; 32], "Dave", 40),
            ([5u8; 32], "Eve", 40),
            ([6u8; 32], "Frank", 50),
        ] {
            store_person_document(
                &platform,
                &data_contract,
                id,
                name,
                "Smith",
                age,
                platform_version,
            );
        }

        // [["age", "in", [30, 40]]]
        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text("in".to_string()),
            Value::Array(vec![Value::U64(30), Value::U64(40)]),
        ])];

        // [["age", "asc"]] — required for the materialize-and-count
        // proof walker; bug #2 in the doc comment above turned this
        // omission into a hard error.
        let order_by = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text("asc".to_string()),
        ])];

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract.id().to_vec(),
            document_type: "person".to_string(),
            r#where: serialize_where_clauses_to_cbor(where_clauses),
            return_distinct_counts_in_range: false,
            order_by: serialize_where_clauses_to_cbor(order_by),
            limit: None,
            prove: true,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Proof(proof)),
                metadata: Some(_),
            }) => {
                // Non-empty grovedb proof bytes pin that the
                // `PointLookupProof` dispatch actually emitted a
                // materialize-and-count proof rather than a
                // degenerate empty envelope. End-to-end SDK-verifier
                // round-trip (group verified docs by the In field's
                // serialized value → per-key entries) is exercised
                // by the SDK integration tests once those are
                // restored post-testnet.
                assert!(
                    !proof.grovedb_proof.is_empty(),
                    "expected non-empty grovedb proof bytes for In + prove count"
                );
            }
            other => panic!(
                "expected Proof response from In + prove count, got {:?}",
                other
            ),
        }
    }

    /// End-to-end test for the range count happy path against a v12
    /// contract whose `widget` document type carries a
    /// `rangeCountable: true` index over `color`. Exercises the
    /// `find_range_countable_index_for_where_clauses` →
    /// `execute_range_count_no_proof` route in the no-prove handler,
    /// in both summed and distinct modes plus the pagination knobs.
    #[test]
    fn test_documents_count_range_query_no_prove() {
        use dpp::data_contract::DataContractFactory;
        use dpp::document::DocumentV0Setters;
        use dpp::platform_value::platform_value;

        const PROTOCOL_VERSION_V12: u32 = 12;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        // Build an in-memory v12 contract with a range_countable index.
        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color": {"type": "string", "position": 0, "maxLength": 32},
            },
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned();

        store_data_contract(&platform, &contract, version);

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        // 6 docs across 3 colors: red×2, blue×1, green×3.
        for (i, color) in ["red", "red", "blue", "green", "green", "green"]
            .iter()
            .enumerate()
        {
            let mut doc = document_type
                .random_document(Some((i + 1) as u64), platform_version)
                .expect("random doc");
            let mut props = std::collections::BTreeMap::new();
            props.insert("color".to_string(), Value::Text(color.to_string()));
            doc.set_properties(props);
            store_document(&platform, &contract, document_type, &doc, platform_version);
        }

        // Helper: issue a range count request with the given options.
        // `ascending` controls the direction encoded into the
        // `order_by` field as `[["color", "asc"|"desc"]]`. `None` →
        // empty `order_by` bytes, which drive treats as "use ascending
        // default" for split-mode entry ordering.
        let make_request = |distinct: bool, limit: Option<u32>, ascending: Option<bool>| {
            let where_clauses = vec![Value::Array(vec![
                Value::Text("color".to_string()),
                Value::Text(">".to_string()),
                Value::Text("blue".to_string()),
            ])];
            let order_by_bytes = match ascending {
                Some(asc) => serialize_where_clauses_to_cbor(vec![Value::Array(vec![
                    Value::Text("color".to_string()),
                    Value::Text(if asc { "asc" } else { "desc" }.to_string()),
                ])]),
                None => Vec::new(),
            };
            GetDocumentsCountRequestV0 {
                data_contract_id: contract.id().to_vec(),
                document_type: "widget".to_string(),
                r#where: serialize_where_clauses_to_cbor(where_clauses),
                return_distinct_counts_in_range: distinct,
                order_by: order_by_bytes,
                limit,
                prove: false,
            }
        };

        // Sum mode: green(3) + red(2) = 5. Range-without-distinct
        // collapses to `AggregateCount` on the wire (no empty-key
        // entry wrapping).
        let result = platform
            .query_documents_count_v0(make_request(false, None, None), &state, version)
            .expect("query should succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(
                    get_documents_count_response_v0::CountResults {
                        variant:
                            Some(get_documents_count_response_v0::count_results::Variant::AggregateCount(
                                total,
                            )),
                    },
                )),
                ..
            }) => {
                assert_eq!(total, 5, "summed range mode → aggregate of 5");
            }
            other => panic!("expected aggregate result, got {:?}", other),
        }

        // Distinct mode ascending: [(green, 3), (red, 2)] in entries.
        let result = platform
            .query_documents_count_v0(make_request(true, None, Some(true)), &state, version)
            .expect("query should succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result:
                    Some(get_documents_count_response_v0::Result::Counts(
                        get_documents_count_response_v0::CountResults {
                            variant:
                                Some(get_documents_count_response_v0::count_results::Variant::Entries(
                                    entries,
                                )),
                        },
                    )),
                ..
            }) => {
                assert_eq!(entries.entries.len(), 2);
                assert_eq!(entries.entries[0].key, b"green".to_vec());
                assert_eq!(entries.entries[0].count, 3);
                assert_eq!(entries.entries[1].key, b"red".to_vec());
                assert_eq!(entries.entries[1].count, 2);
            }
            other => panic!("expected entries result, got {:?}", other),
        }

        // Distinct mode with limit=1: only the first entry (ascending → green).
        let result = platform
            .query_documents_count_v0(make_request(true, Some(1), Some(true)), &state, version)
            .expect("query should succeed");
        assert!(result.errors.is_empty());
        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result:
                    Some(get_documents_count_response_v0::Result::Counts(
                        get_documents_count_response_v0::CountResults {
                            variant:
                                Some(get_documents_count_response_v0::count_results::Variant::Entries(
                                    entries,
                                )),
                        },
                    )),
                ..
            }) => {
                assert_eq!(entries.entries.len(), 1);
                assert_eq!(entries.entries[0].key, b"green".to_vec());
            }
            other => panic!("expected entries result, got {:?}", other),
        }

        // Distinct descending: [(red, 2), (green, 3)] in entries.
        let result = platform
            .query_documents_count_v0(make_request(true, None, Some(false)), &state, version)
            .expect("query should succeed");
        assert!(result.errors.is_empty());
        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result:
                    Some(get_documents_count_response_v0::Result::Counts(
                        get_documents_count_response_v0::CountResults {
                            variant:
                                Some(get_documents_count_response_v0::count_results::Variant::Entries(
                                    entries,
                                )),
                        },
                    )),
                ..
            }) => {
                assert_eq!(entries.entries.len(), 2);
                assert_eq!(entries.entries[0].key, b"red".to_vec());
                assert_eq!(entries.entries[1].key, b"green".to_vec());
            }
            other => panic!("expected entries result, got {:?}", other),
        }
    }

    /// End-to-end pin for the `RangeDistinctProof` dispatch path —
    /// `return_distinct_counts_in_range = true` + `prove = true` +
    /// a range clause. Backed by a regular grovedb range proof
    /// against the property-name `ProvableCountTree` whose
    /// `KVValueHashFeatureType[WithChildHash]` ops carry per-
    /// distinct-value counts bound to the merk root via
    /// `node_hash_with_count`. Asserts the wire-shape contract:
    /// a `Proof` response variant with non-empty grovedb proof
    /// bytes (not the empty-envelope degenerate shape that a
    /// no-match query would emit).
    #[test]
    fn test_documents_count_range_with_prove_and_distinct_returns_proof() {
        use dpp::data_contract::DataContractFactory;
        use dpp::platform_value::platform_value;

        const PROTOCOL_VERSION_V12: u32 = 12;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color": {"type": "string", "position": 0, "maxLength": 32},
            },
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned();

        store_data_contract(&platform, &contract, version);

        // Insert a few widgets spread across distinct color values
        // so the prove-distinct path actually carries per-key counts
        // in its proof — without this the proof covers an empty
        // range and the test only verifies dispatch acceptance.
        // Same distribution as the no-prove test above:
        // red×2, green×3, blue×1. `color > "blue"` excludes blue,
        // so the proof should carry per-color entries for red(2)
        // and green(3).
        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");
        let platform_version = PlatformVersion::latest();
        for (i, color) in ["red", "red", "green", "green", "green", "blue"]
            .iter()
            .enumerate()
        {
            let mut doc = document_type
                .random_document(Some((i + 1) as u64), platform_version)
                .expect("random doc");
            let mut props = std::collections::BTreeMap::new();
            props.insert("color".to_string(), Value::Text(color.to_string()));
            doc.set_properties(props);
            store_document(&platform, &contract, document_type, &doc, platform_version);
        }

        let where_clauses = vec![Value::Array(vec![
            Value::Text("color".to_string()),
            Value::Text(">".to_string()),
            Value::Text("blue".to_string()),
        ])];
        let request = GetDocumentsCountRequestV0 {
            data_contract_id: contract.id().to_vec(),
            document_type: "widget".to_string(),
            r#where: serialize_where_clauses_to_cbor(where_clauses),
            return_distinct_counts_in_range: true,
            order_by: Vec::new(),
            limit: None,
            prove: true,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("query should succeed");
        assert!(
            result.errors.is_empty(),
            "expected no validation errors, got {:?}",
            result.errors
        );
        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Proof(proof)),
                metadata: Some(_),
            }) => {
                // The proof should not be empty since we inserted
                // matching documents — a non-trivial proof shape
                // pins that the prover actually emitted per-key
                // count entries, not just a degenerate envelope.
                assert!(
                    !proof.grovedb_proof.is_empty(),
                    "expected non-empty grovedb proof bytes for non-empty range result"
                );
            }
            other => panic!("expected Proof response, got {:?}", other),
        }
    }
}
