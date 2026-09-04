//! Composite-mode dispatch: a page plus sub-queries derived from its
//! results, answered as ONE merged proof. The request's own type,
//! clauses and limit describe the PAGE; each `sub_queries` entry is a
//! join, a lookup, a count, or a sibling (see the proto), whose `IN`
//! clause the node derives from the proven page (or an earlier
//! sub-query). On the proof path everything rides one merged grovedb
//! proof (drive brackets its materialize/prove sequence with root-hash
//! reads, since grovedb proves committed state only), and the verifier
//! re-derives the whole composition from the proven page.

use super::count::into_v1_entry;
use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::document_query::v1::conversions;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::{
    select, sub_query, Select as ProtoSelect, Start as RequestV1Start, SubQuery as ProtoSubQuery,
};
use dapi_grpc::platform::v0::get_documents_request::{
    HavingClause as ProtoHavingClause, OrderClause as ProtoOrderClause,
    WhereClause as ProtoWhereClause,
};
use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
    composite_documents, result_data, CompositeDocuments, CountEntries, Documents, ResultData,
};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v1, GetDocumentsResponseV1,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::contract::DataContractFetchInfo;
use drive::error::query::QuerySyntaxError;
use drive::query::drive_composite_document_query::{
    BindingSource, DriveCompositeDocumentQuery, DriveSubQuery, SubQueryBinding, SubQueryKind,
    SubQueryResult,
};
use drive::query::DriveDocumentQuery;
use drive::util::grove_operations::GroveDBToUse;
use std::sync::Arc;

/// A sub-query's wire fields decoded into drive's typed forms, before
/// the contract it targets is bound.
struct DecodedSubQuery {
    contract_index: usize,
    document_type: String,
    kind: SubQueryKind,
    where_clauses: Vec<drive::query::WhereClause>,
    order_by: Vec<drive::query::OrderClause>,
    limit: Option<u16>,
    binding: Option<SubQueryBinding>,
}

impl<C> Platform<C> {
    /// Serve a composite-mode v1 request. Runs before select routing:
    /// the composite surface owns its own (deliberately narrow) shape.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::query::document_query::v1) fn dispatch_composite_v1(
        &self,
        data_contract_id: Vec<u8>,
        document_type: String,
        proto_sub_queries: Vec<ProtoSubQuery>,
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

        // The composite surface is documents-shaped by construction:
        // an empty `selects` or a single DOCUMENTS projection; every
        // SQL-shaped knob and every cursor is rejected — pagination
        // is a range clause on the page's ordering property.
        let selects_are_documents = match proto_selects.as_slice() {
            [] => true,
            [single] => {
                single.function == select::Function::Documents as i32 && single.field.is_empty()
            }
            _ => false,
        };
        if !selects_are_documents {
            return Ok(unsupported(
                "a composite request supports the DOCUMENTS projection only",
            ));
        }
        if !group_by.is_empty() || !having.is_empty() {
            return Ok(unsupported(
                "a composite request supports no group_by or having clauses",
            ));
        }
        if start.is_some() {
            return Ok(unsupported(
                "a composite request supports no cursor; paginate with a range clause on \
                 the page's ordering property",
            ));
        }
        if offset.is_some() {
            return Ok(unsupported("a composite request supports no offset"));
        }
        if proto_where_clauses
            .iter()
            .chain(
                proto_sub_queries
                    .iter()
                    .flat_map(|sub| sub.where_clauses.iter()),
            )
            .any(conversions::is_time_range_clause)
        {
            return Ok(unsupported(
                "a composite request supports no time-range (IN_TIME_RANGE) clauses",
            ));
        }

        // The page limit is REQUIRED — it bounds every derived clause.
        let max_query_limit = self.config.drive.max_query_limit as u32;
        let page_limit = match limit {
            Some(n) if n >= 1 && n <= max_query_limit => n as u16,
            other => {
                return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                    QuerySyntaxError::InvalidLimit(format!(
                        "composite requests require an explicit page limit in [1, {}], got {:?}",
                        max_query_limit, other
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

        // Every contract the composition touches, fetched once: the
        // page's first, then each distinct sub-query contract.
        let (page_contract_id, page_contract) = check_validation_result_with_data!(
            self.fetch_contract_for_document_query_v1(data_contract_id, platform_version)?
        );
        let mut contracts: Vec<Arc<DataContractFetchInfo>> = vec![page_contract];
        let mut contract_ids: Vec<Vec<u8>> = vec![page_contract_id.to_vec()];

        let mut decoded: Vec<DecodedSubQuery> = Vec::with_capacity(proto_sub_queries.len());
        for (index, proto) in proto_sub_queries.into_iter().enumerate() {
            let label = |message: String| {
                QueryValidationResult::new_with_error(QueryError::InvalidArgument(format!(
                    "sub-query {}: {}",
                    index, message
                )))
            };
            let contract_index = if proto.data_contract_id.is_empty() {
                0
            } else if let Some(position) = contract_ids
                .iter()
                .position(|id| *id == proto.data_contract_id)
            {
                position
            } else {
                let (id, fetched) = check_validation_result_with_data!(self
                    .fetch_contract_for_document_query_v1(
                        proto.data_contract_id.clone(),
                        platform_version
                    )?);
                contracts.push(fetched);
                contract_ids.push(id.to_vec());
                contracts.len() - 1
            };
            let kind = match sub_query::Kind::try_from(proto.kind) {
                Ok(sub_query::Kind::Documents) => SubQueryKind::Documents,
                Ok(sub_query::Kind::Count) => SubQueryKind::Count,
                Err(_) => return Ok(label(format!("unknown kind {}", proto.kind))),
            };
            let limit = match proto.limit {
                None => None,
                Some(n) if n >= 1 && n <= max_query_limit => Some(n as u16),
                Some(n) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        QuerySyntaxError::InvalidLimit(format!(
                            "sub-query {}: limit must be in [1, {}], got {}",
                            index, max_query_limit, n
                        )),
                    )));
                }
            };
            let where_clauses = match conversions::where_clauses_from_proto(proto.where_clauses) {
                Ok(c) => c,
                Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
            };
            let order_by = match conversions::order_clauses_from_proto(proto.order_by) {
                Ok(c) => c,
                Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
            };
            let binding = proto.bind.map(|bind| SubQueryBinding {
                source: match bind.source {
                    0 => BindingSource::Page,
                    n => BindingSource::SubQuery(n as usize - 1),
                },
                source_property: bind.source_property,
                field: bind.field,
            });
            decoded.push(DecodedSubQuery {
                contract_index,
                document_type: proto.document_type,
                kind,
                where_clauses,
                order_by,
                limit,
                binding,
            });
        }

        // Bind the typed shapes to the fetched contracts.
        let page_contract_ref = &contracts[0].contract;
        let page_type = check_validation_result_with_data!(page_contract_ref
            .document_type_for_name(document_type.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for the queried contract",
                document_type
            ))));
        let page = check_validation_result_with_data!(DriveDocumentQuery::from_typed_clauses(
            where_clauses,
            order_by_clauses,
            Some(page_limit),
            None,
            true,
            None,
            page_contract_ref,
            page_type,
            &self.config.drive,
            platform_version,
        ));
        let mut sub_queries: Vec<DriveSubQuery> = Vec::with_capacity(decoded.len());
        for (index, sub) in decoded.into_iter().enumerate() {
            let contract_ref = &contracts[sub.contract_index].contract;
            let document_type = check_validation_result_with_data!(contract_ref
                .document_type_for_name(sub.document_type.as_str())
                .map_err(|_| QueryError::InvalidArgument(format!(
                    "sub-query {}: document type {} not found for its contract",
                    index, sub.document_type
                ))));
            sub_queries.push(DriveSubQuery {
                contract: contract_ref,
                document_type,
                kind: sub.kind,
                where_clauses: sub.where_clauses,
                order_by: sub.order_by,
                limit: sub.limit,
                binding: sub.binding,
            });
        }
        let composite = DriveCompositeDocumentQuery { page, sub_queries };
        // Fail the shape checks as query errors (client-attributable),
        // before any execution.
        match composite.validate(platform_version) {
            Ok(()) => {}
            Err(drive::error::Error::Query(query_error)) => {
                return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                    query_error,
                )));
            }
            Err(e) => return Err(e.into()),
        }

        let response = if prove {
            let (merged_proof, _page_documents) = match self
                .drive
                .query_composite_documents_with_proof(&composite, platform_version)
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
            let outcome =
                match self
                    .drive
                    .query_composite_documents(&composite, None, None, platform_version)
                {
                    Ok(outcome) => outcome,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };
            let serialize_all = |documents: &[dpp::document::Document],
                                 sub: Option<&DriveSubQuery>|
             -> Result<Vec<Vec<u8>>, Error> {
                let (document_type, contract) = match sub {
                    None => (composite.page.document_type, composite.page.contract),
                    Some(sub) => (sub.document_type, sub.contract),
                };
                documents
                    .iter()
                    .map(|document| {
                        document
                            .serialize(document_type, contract, platform_version)
                            .map_err(Error::Protocol)
                    })
                    .collect()
            };
            let page_documents = serialize_all(&outcome.result.page_documents, None)?;
            let mut sub_results = Vec::with_capacity(composite.sub_queries.len());
            for (sub, result) in composite.sub_queries.iter().zip(outcome.result.sub_results) {
                let result = match result {
                    SubQueryResult::Documents(documents) => {
                        composite_documents::sub_query_result::Result::Documents(Documents {
                            documents: serialize_all(&documents, Some(sub))?,
                        })
                    }
                    SubQueryResult::Counts(entries) => {
                        composite_documents::sub_query_result::Result::Counts(CountEntries {
                            entries: entries.into_iter().map(into_v1_entry).collect(),
                        })
                    }
                };
                sub_results.push(composite_documents::SubQueryResult {
                    result: Some(result),
                });
            }
            GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Data(ResultData {
                    variant: Some(result_data::Variant::Composite(CompositeDocuments {
                        page_documents,
                        sub_results,
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
    use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::{
        sub_query::Binding as ProtoBinding, ChainedJoin,
    };
    use dapi_grpc::platform::v0::get_documents_request::DocumentFieldValue as ProtoDocumentFieldValue;
    use dapi_grpc::platform::v0::get_documents_request::GetDocumentsRequestV1;
    use dapi_grpc::platform::v0::get_documents_request::WhereOperator as ProtoWhereOperator;
    use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::Result as ResponseResult;
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use dpp::identifier::Identifier;
    use dpp::platform_value::Value;
    use dpp::prelude::DataContract;
    use dpp::tests::json_document::json_document_to_contract;
    use drive::query::{InternalClauses, WhereClause, WhereOperator};

    const FEED_CONTRACT_PATH: &str =
        "../rs-drive/tests/supporting_files/contract/yappr-feed/yappr-feed-contract.json";
    const DASHPAY_CONTRACT_PATH: &str =
        "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json";
    const POST_A: [u8; 32] = [0xA1; 32];
    const POST_B: [u8; 32] = [0xB2; 32];
    const POST_D: [u8; 32] = [0xD4; 32];
    const OWNER_1: [u8; 32] = [0x11; 32];
    const OWNER_2: [u8; 32] = [0x22; 32];

    /// Two `dash` posts (A by owner 1 quoting D, B by owner 2), the
    /// quoted `btc` post D, two likes on A and one on B, and a profile
    /// for owner 1 only.
    fn setup_feed_state() -> (
        crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        std::sync::Arc<PlatformState>,
        &'static PlatformVersion,
        DataContract,
        DataContract,
    ) {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let feed = json_document_to_contract(FEED_CONTRACT_PATH, false, version)
            .expect("expected to parse the feed contract");
        let dashpay = json_document_to_contract(DASHPAY_CONTRACT_PATH, false, version)
            .expect("expected to parse the dashpay contract");
        store_data_contract(&platform.platform, &feed, version);
        store_data_contract(&platform.platform, &dashpay, version);

        let post_type = feed.document_type_for_name("post").expect("post doctype");
        let like_type = feed.document_type_for_name("like").expect("like doctype");
        let profile_type = dashpay
            .document_type_for_name("profile")
            .expect("profile doctype");

        for (id, owner, hashtag, quoted, seed) in [
            (POST_D, OWNER_2, "btc", None, 4u64),
            (POST_A, OWNER_1, "dash", Some(POST_D), 1),
            (POST_B, OWNER_2, "dash", None, 2),
        ] {
            let mut post = post_type
                .random_document(Some(seed), version)
                .expect("post");
            let mut props = std::collections::BTreeMap::new();
            props.insert("hashtag".to_string(), Value::Text(hashtag.to_string()));
            props.insert("message".to_string(), Value::Text(format!("post {seed}")));
            if let Some(quoted) = quoted {
                props.insert("quotedPostId".to_string(), Value::Identifier(quoted));
            }
            post.set_properties(props);
            post.set_id(Identifier::from(id));
            post.set_owner_id(Identifier::from(owner));
            store_document(&platform.platform, &feed, post_type, &post, version);
        }
        for (owner, post, seed) in [
            (OWNER_1, POST_A, 10u64),
            (OWNER_2, POST_A, 11),
            (OWNER_1, POST_B, 12),
        ] {
            let mut like = like_type
                .random_document(Some(seed), version)
                .expect("like");
            let mut props = std::collections::BTreeMap::new();
            props.insert("hashtag".to_string(), Value::Text("dash".to_string()));
            props.insert("postId".to_string(), Value::Identifier(post));
            like.set_properties(props);
            like.set_owner_id(Identifier::from(owner));
            store_document(&platform.platform, &feed, like_type, &like, version);
        }
        let mut profile = profile_type
            .random_document(Some(30), version)
            .expect("profile");
        let mut props = std::collections::BTreeMap::new();
        props.insert("displayName".to_string(), Value::Text("one".to_string()));
        profile.set_properties(props);
        profile.set_owner_id(Identifier::from(OWNER_1));
        store_document(
            &platform.platform,
            &dashpay,
            profile_type,
            &profile,
            version,
        );

        (platform, state, version, feed, dashpay)
    }

    fn sub(
        contract_id: Vec<u8>,
        document_type: &str,
        kind: sub_query::Kind,
        limit: Option<u32>,
        bind: Option<(u32, &str, &str)>,
    ) -> ProtoSubQuery {
        ProtoSubQuery {
            data_contract_id: contract_id,
            document_type: document_type.to_string(),
            where_clauses: Vec::new(),
            order_by: Vec::new(),
            limit,
            kind: kind as i32,
            bind: bind.map(|(source, source_property, field)| ProtoBinding {
                source,
                source_property: source_property.to_string(),
                field: field.to_string(),
            }),
        }
    }

    /// Page: `dash` posts; sub-queries: like counts, the quoted posts
    /// (by-id join), the authors' profiles (cross-contract lookup).
    fn composite_request(
        prove: bool,
        feed_id: Vec<u8>,
        dashpay_id: Vec<u8>,
    ) -> GetDocumentsRequestV1 {
        GetDocumentsRequestV1 {
            data_contract_id: feed_id,
            document_type: "post".to_string(),
            where_clauses: vec![
                dapi_grpc::platform::v0::get_documents_request::WhereClause {
                    field: "hashtag".to_string(),
                    operator: ProtoWhereOperator::Equal as i32,
                    value: Some(ProtoDocumentFieldValue {
                        variant: Some(document_field_value::Variant::Text("dash".to_string())),
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
            chained: None,
            sub_queries: vec![
                sub(
                    Vec::new(),
                    "like",
                    sub_query::Kind::Count,
                    None,
                    Some((0, "$id", "postId")),
                ),
                sub(
                    Vec::new(),
                    "post",
                    sub_query::Kind::Documents,
                    None,
                    Some((0, "quotedPostId", "$id")),
                ),
                sub(
                    dashpay_id,
                    "profile",
                    sub_query::Kind::Documents,
                    None,
                    Some((0, "$ownerId", "$ownerId")),
                ),
            ],
        }
    }

    /// The same composition built directly against drive, as the SDK
    /// builds it to verify a proof.
    fn client_query<'a>(
        feed: &'a DataContract,
        dashpay: &'a DataContract,
    ) -> DriveCompositeDocumentQuery<'a> {
        let page = DriveDocumentQuery {
            contract: feed,
            document_type: feed.document_type_for_name("post").expect("post"),
            internal_clauses: InternalClauses::extract_from_clauses(
                vec![WhereClause {
                    field: "hashtag".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("dash".to_string()),
                }],
                PlatformVersion::latest(),
            )
            .expect("clauses extract"),
            offset: None,
            limit: Some(10),
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
            resolved_time_ranges: vec![],
        };
        let bound = |contract: &'a DataContract,
                     type_name: &str,
                     kind,
                     source_property: &str,
                     field: &str| {
            DriveSubQuery {
                contract,
                document_type: contract.document_type_for_name(type_name).expect("doctype"),
                kind,
                where_clauses: vec![],
                order_by: vec![],
                limit: None,
                binding: Some(SubQueryBinding {
                    source: BindingSource::Page,
                    source_property: source_property.to_string(),
                    field: field.to_string(),
                }),
            }
        };
        DriveCompositeDocumentQuery {
            page,
            sub_queries: vec![
                bound(feed, "like", SubQueryKind::Count, "$id", "postId"),
                bound(feed, "post", SubQueryKind::Documents, "quotedPostId", "$id"),
                bound(
                    dashpay,
                    "profile",
                    SubQueryKind::Documents,
                    "$ownerId",
                    "$ownerId",
                ),
            ],
        }
    }

    #[test]
    fn should_return_the_page_and_every_sub_result_without_proof() {
        let (platform, state, version, feed, dashpay) = setup_feed_state();

        let result = platform
            .platform
            .query_documents_v1(
                composite_request(false, feed.id().to_vec(), dashpay.id().to_vec()),
                &state,
                version,
            )
            .expect("query executes");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let response = result.data.expect("response data");
        let Some(ResponseResult::Data(data)) = response.result else {
            panic!("expected a data result");
        };
        let Some(result_data::Variant::Composite(composite)) = data.variant else {
            panic!("expected the composite variant");
        };

        let post_type = feed.document_type_for_name("post").expect("post");
        let page: Vec<[u8; 32]> = composite
            .page_documents
            .iter()
            .map(|bytes| {
                Document::from_bytes(bytes, post_type, version)
                    .expect("post deserializes")
                    .id()
                    .to_buffer()
            })
            .collect();
        assert_eq!(page, vec![POST_A, POST_B]);
        assert_eq!(composite.sub_results.len(), 3);

        let Some(composite_documents::sub_query_result::Result::Counts(counts)) =
            &composite.sub_results[0].result
        else {
            panic!("expected count entries");
        };
        let like_counts: std::collections::BTreeMap<Vec<u8>, u64> = counts
            .entries
            .iter()
            .map(|entry| (entry.key.clone(), entry.count))
            .collect();
        assert_eq!(
            like_counts,
            std::collections::BTreeMap::from([(POST_A.to_vec(), 2), (POST_B.to_vec(), 1)])
        );

        let Some(composite_documents::sub_query_result::Result::Documents(quoted)) =
            &composite.sub_results[1].result
        else {
            panic!("expected quoted documents");
        };
        assert_eq!(quoted.documents.len(), 1, "A quotes D");
        assert_eq!(
            Document::from_bytes(&quoted.documents[0], post_type, version)
                .expect("post deserializes")
                .id()
                .to_buffer(),
            POST_D
        );

        let Some(composite_documents::sub_query_result::Result::Documents(profiles)) =
            &composite.sub_results[2].result
        else {
            panic!("expected profile documents");
        };
        let profile_type = dashpay.document_type_for_name("profile").expect("profile");
        assert_eq!(
            profiles.documents.len(),
            1,
            "owner 2 has no profile: a proven absence"
        );
        assert_eq!(
            Document::from_bytes(&profiles.documents[0], profile_type, version)
                .expect("profile deserializes")
                .owner_id()
                .to_buffer(),
            OWNER_1
        );
    }

    #[test]
    fn should_prove_end_to_end_through_the_v1_wire() {
        let (platform, state, version, feed, dashpay) = setup_feed_state();

        let result = platform
            .platform
            .query_documents_v1(
                composite_request(true, feed.id().to_vec(), dashpay.id().to_vec()),
                &state,
                version,
            )
            .expect("query executes");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let response = result.data.expect("response data");
        let Some(ResponseResult::Proof(proof)) = response.result else {
            panic!("expected a proof result");
        };

        let query = client_query(&feed, &dashpay);
        let (_root_hash, verified) = query
            .verify_composite_documents_proof(proof.grovedb_proof.as_slice(), version)
            .expect("the composite proof verifies from the proof alone");
        assert_eq!(
            verified
                .page_documents
                .iter()
                .map(|post| post.id().to_buffer())
                .collect::<Vec<_>>(),
            vec![POST_A, POST_B]
        );
        assert_eq!(verified.sub_results[0].counts().len(), 2);
        assert_eq!(verified.sub_results[1].documents().len(), 1);
        assert_eq!(verified.sub_results[2].documents().len(), 1);
    }

    #[test]
    fn should_require_an_explicit_page_limit() {
        let (platform, state, version, feed, dashpay) = setup_feed_state();

        let mut no_limit = composite_request(false, feed.id().to_vec(), dashpay.id().to_vec());
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
    fn should_reject_chained_and_composite_together() {
        let (platform, state, version, feed, dashpay) = setup_feed_state();

        let mut both = composite_request(false, feed.id().to_vec(), dashpay.id().to_vec());
        both.chained = Some(ChainedJoin {
            join_property: "quotedPostId".to_string(),
            outer_document_type: "post".to_string(),
        });
        let result = platform
            .platform
            .query_documents_v1(both, &state, version)
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
    fn should_reject_an_unknown_sub_query_contract() {
        let (platform, state, version, feed, dashpay) = setup_feed_state();

        let mut unknown = composite_request(false, feed.id().to_vec(), dashpay.id().to_vec());
        unknown.sub_queries[2].data_contract_id = vec![0x99; 32];
        let result = platform
            .platform
            .query_documents_v1(unknown, &state, version)
            .expect("query executes");
        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::Query(QuerySyntaxError::DataContractNotFound(_))]
            ),
            "expected DataContractNotFound, got {:?}",
            result.errors
        );
    }

    #[test]
    fn should_surface_shape_rejections_as_query_errors() {
        let (platform, state, version, feed, dashpay) = setup_feed_state();

        // A count with a limit is a shape the drive validator refuses;
        // it must come back as a client-attributable query error.
        let mut counted_with_limit =
            composite_request(false, feed.id().to_vec(), dashpay.id().to_vec());
        counted_with_limit.sub_queries[0].limit = Some(5);
        let result = platform
            .platform
            .query_documents_v1(counted_with_limit, &state, version)
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
