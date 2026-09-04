//! Chained-mode dispatch — the provable semi-join:
//! `SELECT * FROM <outer> WHERE $id IN (SELECT <join_property> FROM
//! <document_type> WHERE …)`, selected by the request's `chained`
//! message. The request's own type/clauses/limit describe the INNER
//! indexOnly query; the outer half is DERIVED from its results, and on
//! the proof path both halves ride ONE merged grovedb proof (drive
//! brackets its materialize/prove sequence with root-hash reads, since
//! grovedb proves committed state only) with the join-value bootstrap
//! hint riding beside the envelope.

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::document_query::v1::conversions;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::{
    select, ChainedJoin, Select as ProtoSelect, Start as RequestV1Start,
};
use dapi_grpc::platform::v0::get_documents_request::{
    HavingClause as ProtoHavingClause, OrderClause as ProtoOrderClause,
    WhereClause as ProtoWhereClause,
};
use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
    result_data, ChainedDocuments, ResultData,
};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v1, GetDocumentsResponseV1,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::drive_chained_document_query::DriveChainedDocumentQuery;
use drive::query::DriveDocumentQuery;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Serve a chained-mode v1 request. Runs before select routing:
    /// the chained surface owns its own (deliberately narrow) shape.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::query::document_query::v1) fn dispatch_chained_v1(
        &self,
        data_contract_id: Vec<u8>,
        document_type: String,
        chained: ChainedJoin,
        proto_where_clauses: Vec<ProtoWhereClause>,
        proto_order_by: Vec<ProtoOrderClause>,
        limit: Option<u32>,
        start: Option<RequestV1Start>,
        prove: bool,
        proto_selects: Vec<ProtoSelect>,
        group_by: Vec<String>,
        having: Vec<ProtoHavingClause>,
        offset: Option<u32>,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        let unsupported = |message: &str| {
            QueryValidationResult::new_with_error(QueryError::Query(QuerySyntaxError::Unsupported(
                message.to_string(),
            )))
        };

        // The chained surface is documents-shaped by construction:
        // an empty `selects` (the v0-style default) or a single
        // DOCUMENTS projection; every SQL-shaped knob and every
        // cursor is rejected — pagination is a range clause on the
        // join property.
        let selects_are_documents = match proto_selects.as_slice() {
            [] => true,
            [single] => {
                single.function == select::Function::Documents as i32 && single.field.is_empty()
            }
            _ => false,
        };
        if !selects_are_documents {
            return Ok(unsupported(
                "a chained request supports the DOCUMENTS projection only",
            ));
        }
        if !group_by.is_empty() || !having.is_empty() {
            return Ok(unsupported(
                "a chained request supports no group_by or having clauses",
            ));
        }
        if start.is_some() {
            return Ok(unsupported(
                "a chained request supports no cursor; paginate with a range clause on \
                 the join property",
            ));
        }
        if offset.is_some() {
            return Ok(unsupported("a chained request supports no offset"));
        }
        if proto_where_clauses
            .iter()
            .any(conversions::is_time_range_clause)
        {
            return Ok(unsupported(
                "a chained request supports no time-range (IN_TIME_RANGE) clauses",
            ));
        }

        // The inner limit is REQUIRED — it bounds the derived outer
        // query. No server-default fallback on this surface.
        let inner_limit = match limit {
            Some(n) if n >= 1 && n <= self.config.drive.max_query_limit as u32 => n as u16,
            other => {
                return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                    QuerySyntaxError::InvalidLimit(format!(
                        "chained requests require an explicit limit in [1, {}], got {:?}",
                        self.config.drive.max_query_limit, other
                    )),
                )));
            }
        };

        let where_clauses = match conversions::where_clauses_from_proto(proto_where_clauses) {
            Ok(c) => c,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };
        let order_by_clauses = match conversions::order_clauses_from_proto(proto_order_by) {
            Ok(c) => c,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        let (_, contract_fetch_info) = check_validation_result_with_data!(
            self.fetch_contract_for_document_query_v1(data_contract_id, platform_version)?
        );
        let contract_ref = &contract_fetch_info.contract;

        let inner_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(document_type.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for the queried contract",
                document_type
            ))));
        let outer_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(chained.outer_document_type.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for the queried contract",
                chained.outer_document_type
            ))));

        let inner_query =
            check_validation_result_with_data!(DriveDocumentQuery::from_typed_clauses(
                where_clauses,
                order_by_clauses,
                Some(inner_limit),
                None,
                true,
                None,
                contract_ref,
                inner_type,
                &self.config.drive,
                platform_version,
            ));

        let chained_query = DriveChainedDocumentQuery {
            inner: inner_query,
            join_property: chained.join_property,
            outer_document_type: outer_type,
        };
        // Fail the shape checks as query errors (client-attributable),
        // before any execution.
        match chained_query.validate(platform_version) {
            Ok(()) => {}
            Err(drive::error::Error::Query(query_error)) => {
                return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                    query_error,
                )));
            }
            Err(e) => return Err(e.into()),
        }

        let response = if prove {
            // The proof is self-sufficient for the verifier: it
            // subset-verifies the inner query against it to extract
            // the join values, then re-derives and verifies the whole
            // merged composition.
            let (merged_proof, _inner_documents) = match self
                .drive
                .query_chained_documents_with_proof(&chained_query, platform_version)
            {
                Ok(result) => result,
                Err(drive::error::Error::Query(query_error)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        query_error,
                    )));
                }
                Err(e) => return Err(e.into()),
            };

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, merged_proof, GroveDBToUse::Current)?;

            GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let outcome = match self.drive.query_chained_documents(
                &chained_query,
                None,
                None,
                platform_version,
            ) {
                Ok(outcome) => outcome,
                Err(drive::error::Error::Query(query_error)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        query_error,
                    )));
                }
                Err(e) => return Err(e.into()),
            };

            let serialize_all =
                |documents: &[dpp::document::Document],
                 document_type: dpp::data_contract::document_type::DocumentTypeRef|
                 -> Result<Vec<Vec<u8>>, Error> {
                    documents
                        .iter()
                        .map(|document| {
                            document
                                .serialize(document_type, contract_ref, platform_version)
                                .map_err(Error::Protocol)
                        })
                        .collect()
                };
            let inner_documents = serialize_all(
                &outcome.result.inner_documents,
                chained_query.inner.document_type,
            )?;
            let outer_documents = serialize_all(
                &outcome.result.outer_documents,
                chained_query.outer_document_type,
            )?;

            GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Data(ResultData {
                    variant: Some(result_data::Variant::Chained(ChainedDocuments {
                        inner_documents,
                        outer_documents,
                    })),
                })),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{setup_platform, store_data_contract, store_document};
    use dapi_grpc::platform::v0::get_documents_request::document_field_value;
    use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::select::Function as SelectFunction;
    use dapi_grpc::platform::v0::get_documents_request::DocumentFieldValue as ProtoDocumentFieldValue;
    use dapi_grpc::platform::v0::get_documents_request::GetDocumentsRequestV1;
    use dapi_grpc::platform::v0::get_documents_request::WhereOperator as ProtoWhereOperator;
    use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::Result as ResponseResult;
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use dpp::identifier::Identifier;
    use dpp::platform_value::Value;
    use dpp::tests::json_document::json_document_to_contract;

    const YAPPR_CONTRACT_PATH: &str =
        "../rs-drive/tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json";
    const POST_A: [u8; 32] = [0xA1; 32];
    const POST_B: [u8; 32] = [0xB2; 32];
    const OWNER_1: [u8; 32] = [0x11; 32];

    fn setup_yappr_state() -> (
        crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        std::sync::Arc<PlatformState>,
        &'static PlatformVersion,
        dpp::prelude::DataContract,
    ) {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = json_document_to_contract(YAPPR_CONTRACT_PATH, false, version)
            .expect("expected to parse the yappr-likes contract");
        store_data_contract(&platform.platform, &contract, version);

        let post_type = contract
            .document_type_for_name("post")
            .expect("post doctype");
        let like_type = contract
            .document_type_for_name("like")
            .expect("like doctype");

        for (id, message, seed) in [(POST_A, "post a", 10u64), (POST_B, "post b", 11)] {
            let mut post = post_type
                .random_document(Some(seed), version)
                .expect("post");
            let mut props = std::collections::BTreeMap::new();
            props.insert("hashtag".to_string(), Value::Text("dash".to_string()));
            props.insert("message".to_string(), Value::Text(message.to_string()));
            post.set_properties(props);
            post.set_id(Identifier::from(id));
            post.set_owner_id(Identifier::from(OWNER_1));
            store_document(&platform.platform, &contract, post_type, &post, version);
        }
        for (post, seed) in [(POST_A, 1u64), (POST_B, 2)] {
            let mut like = like_type
                .random_document(Some(seed), version)
                .expect("like");
            let mut props = std::collections::BTreeMap::new();
            props.insert("hashtag".to_string(), Value::Text("dash".to_string()));
            props.insert("postId".to_string(), Value::Identifier(post));
            like.set_properties(props);
            like.set_owner_id(Identifier::from(OWNER_1));
            store_document(&platform.platform, &contract, like_type, &like, version);
        }

        (platform, state, version, contract)
    }

    fn chained_request(prove: bool, contract_id: Vec<u8>) -> GetDocumentsRequestV1 {
        GetDocumentsRequestV1 {
            data_contract_id: contract_id,
            document_type: "like".to_string(),
            where_clauses: vec![
                dapi_grpc::platform::v0::get_documents_request::WhereClause {
                    field: "$ownerId".to_string(),
                    operator: ProtoWhereOperator::Equal as i32,
                    value: Some(ProtoDocumentFieldValue {
                        // Identifiers cross the v1 wire as bytes; the
                        // value layer coerces them against identifier-typed
                        // fields.
                        variant: Some(document_field_value::Variant::BytesValue(OWNER_1.to_vec())),
                    }),
                    time_range: None,
                },
            ],
            order_by: Vec::new(),
            limit: Some(10),
            start: None,
            prove,
            selects: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            offset: None,
            sub_queries: Vec::new(),
            chained: Some(ChainedJoin {
                join_property: "postId".to_string(),
                outer_document_type: "post".to_string(),
            }),
        }
    }

    #[test]
    fn should_return_both_halves_without_proof() {
        let (platform, state, version, contract) = setup_yappr_state();

        let result = platform
            .platform
            .query_documents_v1(
                chained_request(false, contract.id().to_vec()),
                &state,
                version,
            )
            .expect("query executes");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let response = result.data.expect("response data");
        let Some(ResponseResult::Data(data)) = response.result else {
            panic!("expected a data result");
        };
        let Some(result_data::Variant::Chained(chained)) = data.variant else {
            panic!("expected the chained variant");
        };
        assert_eq!(chained.inner_documents.len(), 2);
        assert_eq!(chained.outer_documents.len(), 2);

        let post_type = contract
            .document_type_for_name("post")
            .expect("post doctype");
        let posts: Vec<Document> = chained
            .outer_documents
            .iter()
            .map(|bytes| {
                Document::from_bytes(bytes, post_type, version).expect("post deserializes")
            })
            .collect();
        assert_eq!(
            posts.iter().map(|p| p.id().to_buffer()).collect::<Vec<_>>(),
            vec![POST_A, POST_B],
            "posts in inner (postId) order"
        );
    }

    #[test]
    fn should_prove_end_to_end_through_the_v1_wire() {
        let (platform, state, version, contract) = setup_yappr_state();

        let result = platform
            .platform
            .query_documents_v1(
                chained_request(true, contract.id().to_vec()),
                &state,
                version,
            )
            .expect("query executes");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let response = result.data.expect("response data");
        let Some(ResponseResult::Proof(proof)) = response.result else {
            panic!("expected a proof result");
        };

        // Client-side composition: rebuild the same chained query and
        // verify the single merged proof.
        let like_type = contract
            .document_type_for_name("like")
            .expect("like doctype");
        let inner = DriveDocumentQuery {
            contract: &contract,
            document_type: like_type,
            internal_clauses: drive::query::InternalClauses::extract_from_clauses(
                vec![drive::query::WhereClause {
                    field: "$ownerId".to_string(),
                    operator: drive::query::WhereOperator::Equal,
                    value: Value::Identifier(OWNER_1),
                }],
                version,
            )
            .expect("clauses extract"),
            offset: None,
            limit: Some(10),
            order_by: Default::default(),
            start_at: None,
            start_at_included: true,
            block_time_ms: None,
            resolved_time_ranges: vec![],
        };
        let chained = DriveChainedDocumentQuery {
            inner,
            join_property: "postId".to_string(),
            outer_document_type: contract
                .document_type_for_name("post")
                .expect("post doctype"),
        };
        let (_root_hash, verified) = chained
            .verify_chained_documents_proof(proof.grovedb_proof.as_slice(), version)
            .expect("chained proof verifies — the proof alone carries everything");
        assert_eq!(verified.outer_documents.len(), 2);
        assert_eq!(
            verified
                .outer_documents
                .iter()
                .map(|p| p.id().to_buffer())
                .collect::<Vec<_>>(),
            vec![POST_A, POST_B]
        );
    }

    #[test]
    fn should_require_an_explicit_inner_limit() {
        let (platform, state, version, contract) = setup_yappr_state();

        let mut no_limit = chained_request(false, contract.id().to_vec());
        no_limit.limit = None;
        let result = platform
            .platform
            .query_documents_v1(no_limit, &state, version)
            .expect("query executes");
        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::Query(QuerySyntaxError::InvalidLimit(_))]
            ),
            "expected InvalidLimit, got {:?}",
            result.errors
        );
    }

    #[test]
    fn should_reject_non_refers_to_join_properties() {
        let (platform, state, version, contract) = setup_yappr_state();

        let mut bad_join = chained_request(false, contract.id().to_vec());
        bad_join.chained = Some(ChainedJoin {
            join_property: "hashtag".to_string(),
            outer_document_type: "post".to_string(),
        });
        let result = platform
            .platform
            .query_documents_v1(bad_join, &state, version)
            .expect("query executes");
        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::Query(QuerySyntaxError::Unsupported(_))]
            ),
            "expected Unsupported, got {:?}",
            result.errors
        );
    }

    #[test]
    fn should_reject_sql_shaped_knobs_in_chained_mode() {
        let (platform, state, version, contract) = setup_yappr_state();

        let mut grouped = chained_request(false, contract.id().to_vec());
        grouped.group_by = vec!["hashtag".to_string()];
        let result = platform
            .platform
            .query_documents_v1(grouped, &state, version)
            .expect("query executes");
        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::Query(QuerySyntaxError::Unsupported(_))]
            ),
            "expected Unsupported for group_by, got {:?}",
            result.errors
        );

        let mut counted = chained_request(false, contract.id().to_vec());
        counted.selects = vec![ProtoSelect {
            function: SelectFunction::Count as i32,
            field: String::new(),
        }];
        let result = platform
            .platform
            .query_documents_v1(counted, &state, version)
            .expect("query executes");
        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::Query(QuerySyntaxError::Unsupported(_))]
            ),
            "expected Unsupported for a COUNT projection, got {:?}",
            result.errors
        );
    }
}
