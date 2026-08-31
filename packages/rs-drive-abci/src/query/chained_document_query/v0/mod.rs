use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_chained_documents_request::GetChainedDocumentsRequestV0;
use dapi_grpc::platform::v0::get_chained_documents_response::{
    get_chained_documents_response_v0, GetChainedDocumentsResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::drive_chained_document_query::DriveChainedDocumentQuery;
use drive::query::{DriveDocumentQuery, OrderClause, WhereClause};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// v0 of the chained documents query.
    ///
    /// The proof path returns ONE merged grovedb proof covering both
    /// halves (drive brackets its materialize/prove sequence with
    /// root-hash reads, since grovedb proves committed state only),
    /// plus the join-value bootstrap hint the verifier re-derives the
    /// outer component from.
    pub(super) fn query_chained_documents_v0(
        &self,
        GetChainedDocumentsRequestV0 {
            data_contract_id,
            inner_document_type,
            inner_where,
            inner_order_by,
            inner_limit,
            join_property,
            outer_document_type,
            prove,
        }: GetChainedDocumentsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetChainedDocumentsResponseV0>, Error> {
        // CBOR-decode the wire clauses — identical treatment to
        // `GetDocumentsRequestV0.where` / `.order_by`.
        let where_value = if inner_where.is_empty() {
            Value::Null
        } else {
            check_validation_result_with_data!(ciborium::de::from_reader(inner_where.as_slice())
                .map_err(|_| {
                    QueryError::Query(QuerySyntaxError::DeserializationError(
                        "unable to decode 'inner_where' query from cbor".to_string(),
                    ))
                }))
        };
        let order_by_value: Option<Value> = if !inner_order_by.is_empty() {
            check_validation_result_with_data!(ciborium::de::from_reader(inner_order_by.as_slice())
                .map_err(|_| {
                    QueryError::Query(QuerySyntaxError::DeserializationError(
                        "unable to decode 'inner_order_by' query from cbor".to_string(),
                    ))
                }))
        } else {
            None
        };

        let where_clauses: Vec<WhereClause> = match where_value {
            Value::Null => Vec::new(),
            Value::Array(clauses) => {
                let parsed: Result<Vec<WhereClause>, _> = clauses
                    .iter()
                    .map(|wc| match wc {
                        Value::Array(components) => WhereClause::from_components(components),
                        _ => Err(drive::error::Error::Query(
                            QuerySyntaxError::InvalidFormatWhereClause(
                                "where clause must be an array".to_string(),
                            ),
                        )),
                    })
                    .collect();
                check_validation_result_with_data!(parsed.map_err(|e| QueryError::Query(
                    QuerySyntaxError::InvalidFormatWhereClause(format!(
                        "invalid where clause components: {e}"
                    ))
                )))
            }
            _ => {
                return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                    QuerySyntaxError::InvalidFormatWhereClause(
                        "where clause must be an array".to_string(),
                    ),
                )));
            }
        };
        let order_by_clauses: Vec<OrderClause> = match order_by_value {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(clauses)) => {
                let parsed: Result<Vec<OrderClause>, _> = clauses
                    .iter()
                    .map(|oc| match oc {
                        Value::Array(components) => OrderClause::from_components(components)
                            .map_err(|_| {
                                QueryError::Query(QuerySyntaxError::InvalidOrderByProperties(
                                    "invalid order_by clause components",
                                ))
                            }),
                        _ => Err(QueryError::Query(
                            QuerySyntaxError::InvalidOrderByProperties(
                                "order_by clause must be an array",
                            ),
                        )),
                    })
                    .collect();
                check_validation_result_with_data!(parsed)
            }
            _ => {
                return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                    QuerySyntaxError::InvalidOrderByProperties("order_by must be an array"),
                )));
            }
        };

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
                "contract not found when querying chained documents",
            )
        )));
        let contract_ref = &contract.contract;

        let inner_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(inner_document_type.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                inner_document_type, contract_id
            ))));
        let outer_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(outer_document_type.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                outer_document_type, contract_id
            ))));

        // The inner limit is REQUIRED — it bounds the derived outer
        // query. No `0 = server default` sentinel here: an explicit
        // bound is part of the chained contract. Enforce the server's
        // max up front so the message states the bound this check
        // applies (the drive layer caps again at the outer `$id IN`
        // clause's value limit).
        if inner_limit == 0 || inner_limit > self.config.drive.max_query_limit as u32 {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidLimit(format!(
                    "chained queries require an inner limit in [1, {}], got {}",
                    self.config.drive.max_query_limit, inner_limit
                )),
            )));
        }

        let inner_query =
            check_validation_result_with_data!(DriveDocumentQuery::from_typed_clauses(
                where_clauses,
                order_by_clauses,
                Some(inner_limit as u16),
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
            join_property,
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
            // ONE merged proof for both halves; the drive call brackets
            // its materialize/prove sequence with root-hash reads
            // (grovedb proves committed state only) and retries if a
            // block commit interleaves.
            let (merged_proof, inner_documents) = match self
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

            // The bootstrap hint the verifier re-derives the outer
            // component from — untrusted by design.
            let proven_join_values = chained_query
                .join_values(&inner_documents)?
                .into_iter()
                .map(|id| id.to_vec())
                .collect();

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, merged_proof, GroveDBToUse::Current)?;

            GetChainedDocumentsResponseV0 {
                result: Some(get_chained_documents_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
                proven_join_values,
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

            GetChainedDocumentsResponseV0 {
                result: Some(get_chained_documents_response_v0::Result::Documents(
                    get_chained_documents_response_v0::ChainedDocuments {
                        inner_documents,
                        outer_documents,
                    },
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
                proven_join_values: Vec::new(),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{setup_platform, store_data_contract, store_document};
    use ciborium::value::Value as CborValue;
    use dapi_grpc::platform::v0::get_chained_documents_response::get_chained_documents_response_v0::Result as ResponseResult;
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
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

    fn owner_where_cbor(owner: [u8; 32]) -> Vec<u8> {
        let clauses = CborValue::Array(vec![CborValue::Array(vec![
            CborValue::Text("$ownerId".to_string()),
            CborValue::Text("==".to_string()),
            CborValue::Bytes(owner.to_vec()),
        ])]);
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&clauses, &mut bytes).expect("cbor encode");
        bytes
    }

    fn request(prove: bool, contract_id: Vec<u8>) -> GetChainedDocumentsRequestV0 {
        GetChainedDocumentsRequestV0 {
            data_contract_id: contract_id,
            inner_document_type: "like".to_string(),
            inner_where: owner_where_cbor(OWNER_1),
            inner_order_by: vec![],
            inner_limit: 10,
            join_property: "postId".to_string(),
            outer_document_type: "post".to_string(),
            prove,
        }
    }

    #[test]
    fn test_chained_documents_no_proof_returns_both_halves() {
        let (platform, state, version, contract) = setup_yappr_state();

        let result = platform
            .platform
            .query_chained_documents_v0(request(false, contract.id().to_vec()), &state, version)
            .expect("query executes");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let response = result.data.expect("response data");
        assert!(response.proven_join_values.is_empty());
        let Some(ResponseResult::Documents(documents)) = response.result else {
            panic!("expected a documents result");
        };
        assert_eq!(documents.inner_documents.len(), 2);
        assert_eq!(documents.outer_documents.len(), 2);

        let post_type = contract
            .document_type_for_name("post")
            .expect("post doctype");
        let posts: Vec<Document> = documents
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
    fn test_chained_documents_proof_verifies_end_to_end() {
        let (platform, state, version, contract) = setup_yappr_state();

        let result = platform
            .platform
            .query_chained_documents_v0(request(true, contract.id().to_vec()), &state, version)
            .expect("query executes");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let response = result.data.expect("response data");
        let Some(ResponseResult::Proof(proof)) = response.result else {
            panic!("expected a proof result");
        };
        assert_eq!(
            response.proven_join_values,
            vec![POST_A.to_vec(), POST_B.to_vec()],
            "the bootstrap hint carries the proven join values in first-appearance order"
        );

        // Client-side composition: rebuild the same chained query and
        // verify both proofs as one statement.
        let like_type = contract
            .document_type_for_name("like")
            .expect("like doctype");
        let inner = DriveDocumentQuery {
            contract: &contract,
            document_type: like_type,
            internal_clauses: drive::query::InternalClauses::extract_from_clauses(
                vec![WhereClause {
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
        let hint: Vec<Identifier> = response
            .proven_join_values
            .iter()
            .map(|bytes| {
                Identifier::from_bytes(bytes).expect("hint entries are 32-byte identifiers")
            })
            .collect();
        let (_root_hash, verified) = chained
            .verify_chained_documents_proof(proof.grovedb_proof.as_slice(), &hint, version)
            .expect("chained proof verifies");
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
    fn test_chained_documents_requires_inner_limit() {
        let (platform, state, version, contract) = setup_yappr_state();

        let mut zero_limit = request(false, contract.id().to_vec());
        zero_limit.inner_limit = 0;
        let result = platform
            .platform
            .query_chained_documents_v0(zero_limit, &state, version)
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
    fn test_chained_documents_rejects_bad_join_property() {
        let (platform, state, version, contract) = setup_yappr_state();

        let mut bad_join = request(false, contract.id().to_vec());
        bad_join.join_property = "hashtag".to_string();
        let result = platform
            .platform
            .query_chained_documents_v0(bad_join, &state, version)
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
}
