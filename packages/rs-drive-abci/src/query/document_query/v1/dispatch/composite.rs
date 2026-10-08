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
use super::document_serialization_failure;
use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::contract_moderation_queries::removal_entry_to_response;
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
use drive::error::Error as DriveError;
use drive::grovedb::Error as GroveError;
use drive::query::{
    BindingSource, DriveDocumentQuery, DriveSubQuery, SubQueryBinding, SubQueryKind,
    SubQueryResult, MAX_SUB_QUERIES,
};
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

/// Only the per-instance-limit refusal of GroveDB versions without that capability
/// is attributable to the request.
/// Other GroveDB failures still identify a node or storage fault.
fn composite_query_error(
    error: DriveError,
    platform_version: &PlatformVersion,
) -> Result<QueryError, Error> {
    match error {
        DriveError::Query(query_error) => Ok(QueryError::Query(query_error)),
        DriveError::GroveDB(ref grove_error)
            if platform_version.drive.grove_version.grovedb_versions.path_query_methods.per_instance_query_limits == 0
                && matches!(grove_error.as_ref(), GroveError::NotSupported(message)
                    if *message == "per-instance query limits (Query::limit) require a grove version that serves them") =>
        {
            Ok(QueryError::Query(QuerySyntaxError::Unsupported(
                "composite query limits require a protocol version that supports per-instance limits".to_string(),
            )))
        }
        error => Err(error.into()),
    }
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

        // Bound dispatch work before scanning clauses, allocating decoded
        // sub-queries, or fetching any of the request's contracts.
        if proto_sub_queries.len() > MAX_SUB_QUERIES {
            return Ok(unsupported(&format!(
                "a composite query carries at most {} sub-queries, got {}",
                MAX_SUB_QUERIES,
                proto_sub_queries.len(),
            )));
        }

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
            .any(conversions::is_window_selection_clause)
        {
            return Ok(unsupported(
                "a composite request supports no window selection (IN_TIME_RANGE or \
                 IN_INTEGER_RANGE) clauses",
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
        let composite = page.with_sub_queries(sub_queries);
        // Fail the shape checks as query errors (client-attributable),
        // before any execution.
        match composite.validate_composite(platform_version) {
            Ok(()) => {}
            Err(error) => {
                return Ok(QueryValidationResult::new_with_error(
                    composite_query_error(error, platform_version)?,
                ));
            }
        }

        let response = if prove {
            let (merged_proof, _page_documents) = match self
                .drive
                .query_composite_documents_with_proof(&composite, platform_version)
            {
                Ok(result) => result,
                Err(error) => {
                    return Ok(QueryValidationResult::new_with_error(
                        composite_query_error(error, platform_version)?,
                    ));
                }
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
                    Err(error) => {
                        return Ok(QueryValidationResult::new_with_error(
                            composite_query_error(error, platform_version)?,
                        ));
                    }
                };
            let serialize_all = |documents: &[dpp::document::Document],
                                 sub: Option<&DriveSubQuery>|
             -> Result<Vec<Vec<u8>>, dpp::ProtocolError> {
                let (document_type, contract) = match sub {
                    None => (composite.document_type, composite.contract),
                    Some(sub) => (sub.document_type, sub.contract),
                };
                documents
                    .iter()
                    .map(|document| document.serialize(document_type, contract, platform_version))
                    .collect()
            };
            let page_documents = match serialize_all(&outcome.result.page_documents, None) {
                Ok(documents) => documents,
                Err(error) => {
                    return Ok(QueryValidationResult::new_with_error(
                        document_serialization_failure(error, composite.document_type)?,
                    ))
                }
            };
            let mut sub_results = Vec::with_capacity(composite.sub_queries.len());
            for (((sub, result), missing_ids), removed) in composite
                .sub_queries
                .iter()
                .zip(outcome.result.sub_results)
                .zip(outcome.result.sub_result_missing_ids)
                .zip(outcome.result.sub_result_removals)
            {
                let result = match result {
                    SubQueryResult::Documents(documents) => {
                        let documents = match serialize_all(&documents, Some(sub)) {
                            Ok(documents) => documents,
                            Err(error) => {
                                return Ok(QueryValidationResult::new_with_error(
                                    document_serialization_failure(error, sub.document_type)?,
                                ))
                            }
                        };
                        composite_documents::sub_query_result::Result::Documents(Documents {
                            documents,
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
                    missing_ids: missing_ids.iter().map(|id| id.to_vec()).collect(),
                    removed: removed.into_iter().map(removal_entry_to_response).collect(),
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
    use crate::query::service::query_result_into_response;
    use crate::query::tests::{
        assert_invalid_argument_status, removal_of, remove_post_by_moderator, setup_platform,
        store_data_contract, store_document, with_moderated_posts,
    };
    use dapi_grpc::platform::v0::get_documents_request::document_field_value;
    use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::{
        sub_query::Binding as ProtoBinding, ChainedJoin,
    };
    use dapi_grpc::platform::v0::get_documents_request::DocumentFieldValue as ProtoDocumentFieldValue;
    use dapi_grpc::platform::v0::get_documents_request::GetDocumentsRequestV1;
    use dapi_grpc::platform::v0::get_documents_request::Version as RequestVersion;
    use dapi_grpc::platform::v0::get_documents_request::WhereOperator as ProtoWhereOperator;
    use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::Result as ResponseResult;
    use dapi_grpc::platform::v0::get_documents_response::Version as ResponseVersion;
    use dapi_grpc::platform::v0::GetDocumentsRequest;
    use dapi_grpc::tonic::Code;
    use dpp::dashcore::Network;
    use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use dpp::identifier::Identifier;
    use dpp::platform_value::Value;
    use dpp::prelude::DataContract;
    use dpp::tests::json_document::{json_document_to_contract, json_document_to_json_value};
    use drive::drive::contract::moderation::types::ContractDocumentRemovalEntry;
    use drive::query::{InternalClauses, WhereClause, WhereOperator};

    #[test]
    fn should_classify_only_the_exact_historical_composite_capability_failure() {
        const CAPABILITY: &str =
            "per-instance query limits (Query::limit) require a grove version that serves them";
        for version in [
            PlatformVersion::get(12).unwrap(),
            PlatformVersion::get(13).unwrap(),
            PlatformVersion::latest(),
        ] {
            for (grove_error, matches_capability) in [
                (GroveError::NotSupported(CAPABILITY.to_owned()), true),
                (GroveError::NotSupported("different unsupported storage operation".to_owned()), false),
                (GroveError::NotSupported("per-instance query limits (Query::limit) require a grove version that serves them extra".to_owned()), false),
                (GroveError::InvalidQuery(CAPABILITY), false),
                (GroveError::CorruptedData(CAPABILITY.to_owned()), false),
            ] {
                let result: Result<QueryValidationResult<()>, Error> = composite_query_error(
                    DriveError::GroveDB(Box::new(grove_error)), version,
                ).map(QueryValidationResult::new_with_error);
                let status = query_result_into_response(result)
                    .expect_err("classification returns an error response");
                assert_eq!(status.code(), if matches_capability && version.protocol_version < 14 {
                    Code::InvalidArgument
                } else {
                    Code::Internal
                });
            }
            let result: Result<QueryValidationResult<()>, Error> = composite_query_error(
                DriveError::Query(QuerySyntaxError::Unsupported(
                    "existing proof-shape rejection".to_owned(),
                )),
                version,
            )
            .map(QueryValidationResult::new_with_error);
            let status = query_result_into_response(result).unwrap_err();
            assert_eq!(status.code(), Code::InvalidArgument);
            assert_eq!(
                status.message(),
                "unsupported error: existing proof-shape rejection"
            );
        }
    }

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
        setup_feed_state_with(None, |feed, _| feed)
    }

    /// The same state, the feed contract passed through `modify_feed` first
    fn setup_feed_state_with(
        initial_protocol_version: Option<u32>,
        modify_feed: impl FnOnce(DataContract, &PlatformVersion) -> DataContract,
    ) -> (
        crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        std::sync::Arc<PlatformState>,
        &'static PlatformVersion,
        DataContract,
        DataContract,
    ) {
        let (platform, state, version) =
            setup_platform(None, Network::Testnet, initial_protocol_version);
        let feed_contract = if initial_protocol_version.is_some() {
            let mut value = json_document_to_json_value(FEED_CONTRACT_PATH).expect("feed JSON");
            // Historical schemas store likes as documents and do not admit the
            // generation-3 terminal, preallocation or ranked-index keywords.
            // They also predate property references, so the historical composition
            // joins profiles by owner and counts likes, without a quoted-post join.
            for schema in value["documentSchemas"]
                .as_object_mut()
                .expect("document schemas")
                .values_mut()
            {
                schema
                    .as_object_mut()
                    .expect("schema object")
                    .remove("indexOnly");
                for property in schema["properties"]
                    .as_object_mut()
                    .expect("properties")
                    .values_mut()
                {
                    property
                        .as_object_mut()
                        .expect("property object")
                        .remove("refersTo");
                }
                for index in schema["indices"].as_array_mut().expect("indexes") {
                    let index = index.as_object_mut().expect("index object");
                    for keyword in ["terminal", "preallocated", "rankedCountable"] {
                        index.remove(keyword);
                    }
                }
            }
            // A unique lookup is value-bounded and needs no per-instance query
            // cap, which the historical GroveDB versions do not support.
            let quote_index = value["documentSchemas"]["post"]["indices"]
                .as_array_mut()
                .expect("post indexes")
                .iter_mut()
                .find(|index| index["name"] == "quotesOfPost")
                .expect("quote index");
            quote_index["unique"] = serde_json::Value::Bool(true);
            DataContract::from_json(value, true, version).expect("valid historical feed contract")
        } else {
            json_document_to_contract(FEED_CONTRACT_PATH, false, version)
                .expect("expected to parse the feed contract")
        };
        let feed = modify_feed(feed_contract, version);
        let dashpay = json_document_to_contract(
            DASHPAY_CONTRACT_PATH,
            initial_protocol_version.is_some(),
            version,
        )
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
                    integer_range: None,
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
        platform_version: &PlatformVersion,
    ) -> DriveDocumentQuery<'a> {
        let page = DriveDocumentQuery {
            contract: feed,
            document_type: feed.document_type_for_name("post").expect("post"),
            internal_clauses: InternalClauses::extract_from_clauses(
                vec![WhereClause {
                    field: "hashtag".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("dash".to_string()),
                }],
                platform_version,
            )
            .expect("clauses extract"),
            offset: None,
            limit: Some(10),
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
            resolved_time_ranges: vec![],
            sub_queries: vec![],
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
        page.with_sub_queries(vec![
            bound(feed, "like", SubQueryKind::Count, "$id", "postId"),
            bound(feed, "post", SubQueryKind::Documents, "quotedPostId", "$id"),
            bound(
                dashpay,
                "profile",
                SubQueryKind::Documents,
                "$ownerId",
                "$ownerId",
            ),
        ])
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

        let query = client_query(&feed, &dashpay, version);
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

    /// A by-id join off a `moderatedDocument` property reports a quoted post a moderator
    /// removed by its removal record, on both wire modes: the unproven sub-result carries the
    /// record in `removed`, as the removals query writes it, and the proven one verifies to
    /// the same record. Nothing is reported missing.
    #[test]
    fn should_report_a_removed_quoted_post_of_a_moderated_document_join_with_its_record() {
        let (platform, state, version, feed, dashpay) =
            setup_feed_state_with(None, with_moderated_posts);
        let removal = removal_of(POST_D, OWNER_2);
        remove_post_by_moderator(&platform.platform, &feed, POST_D, removal.clone(), version);
        let entry = ContractDocumentRemovalEntry {
            document_id: Identifier::from(POST_D),
            removal,
        };

        let result = platform
            .platform
            .query_documents_v1(
                composite_request(false, feed.id().to_vec(), dashpay.id().to_vec()),
                &state,
                version,
            )
            .expect("query executes");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let Some(ResponseResult::Data(data)) = result.data.expect("response data").result else {
            panic!("expected a data result");
        };
        let Some(result_data::Variant::Composite(composite)) = data.variant else {
            panic!("expected the composite variant");
        };
        let quoted = &composite.sub_results[1];
        let Some(composite_documents::sub_query_result::Result::Documents(documents)) =
            &quoted.result
        else {
            panic!("expected quoted documents");
        };
        assert!(
            documents.documents.is_empty(),
            "the quoted post was removed"
        );
        assert!(quoted.missing_ids.is_empty());
        assert_eq!(
            quoted.removed,
            vec![removal_entry_to_response(entry.clone())]
        );
        let record = &quoted.removed[0];
        assert_eq!(record.document_id, POST_D.to_vec());
        assert_eq!(record.document_owner_id, OWNER_2.to_vec());
        assert_eq!(record.document_hash, POST_D.to_vec());
        for other in [&composite.sub_results[0], &composite.sub_results[2]] {
            assert!(other.removed.is_empty() && other.missing_ids.is_empty());
        }

        let result = platform
            .platform
            .query_documents_v1(
                composite_request(true, feed.id().to_vec(), dashpay.id().to_vec()),
                &state,
                version,
            )
            .expect("query executes");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let Some(ResponseResult::Proof(proof)) = result.data.expect("response data").result else {
            panic!("expected a proof result");
        };
        let query = client_query(&feed, &dashpay, version);
        let (_root_hash, verified) = query
            .verify_composite_documents_proof(proof.grovedb_proof.as_slice(), version)
            .expect("the proof verifies with the removed post's record proven");
        assert!(verified.sub_results[1].documents().is_empty());
        assert_eq!(
            verified.sub_result_removals,
            vec![Vec::new(), vec![entry], Vec::new()]
        );
        assert!(verified.sub_result_missing_ids.iter().all(Vec::is_empty));
    }

    #[test]
    fn should_reject_oversized_sub_queries_before_decoding_or_contract_fetch() {
        let (platform, state, version, feed, dashpay) = setup_feed_state();
        let mut query = client_query(&feed, &dashpay, version);
        query
            .sub_queries
            .resize(MAX_SUB_QUERIES + 1, query.sub_queries[0].clone());
        let drive::error::Error::Query(QuerySyntaxError::Unsupported(expected_message)) = query
            .validate_composite(version)
            .expect_err("too many sub-queries")
        else {
            panic!("expected Drive's sub-query count rejection");
        };

        for prove in [false, true] {
            let mut oversized = composite_request(prove, feed.id().to_vec(), dashpay.id().to_vec());
            oversized
                .sub_queries
                .resize(MAX_SUB_QUERIES + 1, oversized.sub_queries[0].clone());
            let mut missing_page_contract = oversized.clone();
            missing_page_contract.data_contract_id = vec![0x99; 32];
            let mut malformed_page_clause = oversized.clone();
            malformed_page_clause.where_clauses[0].operator = i32::MAX;
            let mut missing_sub_query_contract = oversized.clone();
            missing_sub_query_contract.sub_queries[0].data_contract_id = vec![0x99; 32];
            let mut malformed_sub_query = oversized.clone();
            malformed_sub_query.sub_queries[0].kind = i32::MAX;

            // The count rejection must precede both decoding errors and
            // contract lookup errors, in either response mode.
            for request in [
                oversized,
                missing_page_contract,
                malformed_page_clause,
                missing_sub_query_contract,
                malformed_sub_query,
            ] {
                let result = platform
                    .platform
                    .query_documents_v1(request, &state, version)
                    .expect("query returns a validation error");
                assert!(result.data.is_none());
                assert!(
                    matches!(
                        result.errors.as_slice(),
                        [QueryError::Query(QuerySyntaxError::Unsupported(message))]
                            if message == &expected_message
                    ),
                    "expected the sub-query count rejection, prove={prove}, got {:?}",
                    result.errors
                );
            }
        }
    }

    #[test]
    fn should_accept_sub_query_counts_through_the_maximum() {
        let (platform, state, version, feed, dashpay) = setup_feed_state();
        for count in [1, MAX_SUB_QUERIES] {
            for prove in [false, true] {
                let mut request =
                    composite_request(prove, feed.id().to_vec(), dashpay.id().to_vec());
                request.sub_queries = vec![request.sub_queries[0].clone(); count];
                let result = platform
                    .platform
                    .query_documents_v1(request, &state, version)
                    .expect("query executes");
                assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
                match result.data.expect("response data").result.expect("result") {
                    ResponseResult::Proof(proof) => {
                        assert!(prove);
                        let mut query = client_query(&feed, &dashpay, version);
                        query.sub_queries = vec![query.sub_queries[0].clone(); count];
                        let (_, verified) = query
                            .verify_composite_documents_proof(&proof.grovedb_proof, version)
                            .expect("composite proof verifies");
                        assert_eq!(verified.page_documents.len(), 2);
                        assert_eq!(verified.sub_results.len(), count);
                        assert!(verified
                            .sub_results
                            .iter()
                            .all(|sub| sub.counts().len() == 2));
                    }
                    ResponseResult::Data(data) => {
                        assert!(!prove);
                        let Some(result_data::Variant::Composite(composite)) = data.variant else {
                            panic!("expected composite data");
                        };
                        assert_eq!(composite.page_documents.len(), 2);
                        assert_eq!(composite.sub_results.len(), count);
                        assert!(composite.sub_results.iter().all(|sub| matches!(
                            &sub.result,
                            Some(composite_documents::sub_query_result::Result::Counts(entries))
                                if entries.entries.len() == 2
                        )));
                    }
                }
            }
        }
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

    #[test]
    fn should_preserve_by_id_composite_data_and_proofs_at_supported_protocol_versions() {
        for initial_protocol_version in [Some(12), Some(13), None] {
            let (platform, state, version, feed, dashpay) =
                setup_feed_state_with(initial_protocol_version, |feed, _| feed);
            let stored_root = platform
                .platform
                .drive
                .grove
                .root_hash(None, &version.drive.grove_version)
                .value
                .expect("stored root");
            for prove in [false, true] {
                let mut request =
                    composite_request(prove, feed.id().to_vec(), dashpay.id().to_vec());
                request.where_clauses = vec![ProtoWhereClause {
                    field: "$id".to_string(),
                    operator: ProtoWhereOperator::In as i32,
                    value: Some(ProtoDocumentFieldValue {
                        variant: Some(document_field_value::Variant::List(
                            document_field_value::ValueList {
                                values: [POST_B, POST_A]
                                    .into_iter()
                                    .map(|id| ProtoDocumentFieldValue {
                                        variant: Some(document_field_value::Variant::BytesValue(
                                            id.to_vec(),
                                        )),
                                    })
                                    .collect(),
                            },
                        )),
                    }),
                    integer_range: None,
                    time_range: None,
                }];
                request.limit = Some(2);
                if initial_protocol_version.is_some() {
                    request.sub_queries.remove(1);
                }
                let result = platform
                    .platform
                    .query_documents(
                        GetDocumentsRequest {
                            version: Some(RequestVersion::V1(request)),
                        },
                        &state,
                        version,
                    )
                    .expect("the public query dispatcher executes");
                assert!(
                    result.errors.is_empty(),
                    "PV{}: {:?}",
                    version.protocol_version,
                    result.errors
                );
                let Some(ResponseVersion::V1(response)) = result.data.expect("response").version
                else {
                    panic!("expected the V1 composite response");
                };
                let post_type = feed.document_type_for_name("post").expect("post type");
                let assert_joined_documents = |quoted: &[Document], profiles: &[Document]| {
                    if initial_protocol_version.is_none() {
                        assert_eq!(quoted.len(), 1, "A quotes D");
                        assert_eq!(quoted[0].id().to_buffer(), POST_D);
                        assert_eq!(quoted[0].owner_id().to_buffer(), OWNER_2);
                        assert_eq!(
                            quoted[0].properties().get("message"),
                            Some(&Value::Text("post 4".to_owned()))
                        );
                    }
                    assert_eq!(profiles.len(), 1, "only owner 1 has a profile");
                    assert_eq!(profiles[0].owner_id().to_buffer(), OWNER_1);
                    assert_eq!(
                        profiles[0].properties().get("displayName"),
                        Some(&Value::Text("one".to_owned()))
                    );
                };
                if prove {
                    let Some(ResponseResult::Proof(proof)) = response.result else {
                        panic!("a proved composite request must contain a proof");
                    };
                    let mut query = client_query(&feed, &dashpay, version);
                    query.internal_clauses = InternalClauses::extract_from_clauses(
                        vec![WhereClause {
                            field: "$id".to_string(),
                            operator: WhereOperator::In,
                            value: Value::Array(vec![
                                Value::Identifier(POST_B),
                                Value::Identifier(POST_A),
                            ]),
                        }],
                        version,
                    )
                    .expect("original page clause");
                    query.limit = Some(2);
                    if initial_protocol_version.is_some() {
                        query.sub_queries.remove(1);
                    }
                    let (root, verified) = query
                        .verify_composite_documents_proof(&proof.grovedb_proof, version)
                        .expect("the original by-ID composition verifies");
                    assert_eq!(root, stored_root);
                    assert_eq!(
                        verified
                            .page_documents
                            .iter()
                            .map(|document| document.id().to_buffer())
                            .collect::<Vec<_>>(),
                        vec![POST_A, POST_B]
                    );
                    assert_eq!(
                        verified.sub_results[0]
                            .counts()
                            .iter()
                            .map(|entry| (entry.key.clone(), entry.count))
                            .collect::<std::collections::BTreeMap<_, _>>(),
                        std::collections::BTreeMap::from([
                            (POST_A.to_vec(), Some(2)),
                            (POST_B.to_vec(), Some(1))
                        ])
                    );
                    if initial_protocol_version.is_some() {
                        assert_eq!(verified.sub_results.len(), 2);
                        assert_joined_documents(&[], verified.sub_results[1].documents());
                    } else {
                        assert_eq!(verified.sub_results.len(), 3);
                        assert_joined_documents(
                            verified.sub_results[1].documents(),
                            verified.sub_results[2].documents(),
                        );
                    }
                } else {
                    let Some(ResponseResult::Data(data)) = response.result else {
                        panic!("an unproved composite request must contain data");
                    };
                    let Some(result_data::Variant::Composite(composite)) = data.variant else {
                        panic!("expected composite data");
                    };
                    assert_eq!(
                        composite
                            .page_documents
                            .iter()
                            .map(|bytes| Document::from_bytes(bytes, post_type, version)
                                .expect("post deserializes")
                                .id()
                                .to_buffer())
                            .collect::<Vec<_>>(),
                        vec![POST_A, POST_B]
                    );
                    let Some(composite_documents::sub_query_result::Result::Counts(counts)) =
                        &composite.sub_results[0].result
                    else {
                        panic!("expected like counts");
                    };
                    assert_eq!(
                        counts
                            .entries
                            .iter()
                            .map(|entry| (entry.key.clone(), entry.count))
                            .collect::<std::collections::BTreeMap<_, _>>(),
                        std::collections::BTreeMap::from([
                            (POST_A.to_vec(), 2),
                            (POST_B.to_vec(), 1)
                        ])
                    );
                    let profile_type = dashpay
                        .document_type_for_name("profile")
                        .expect("profile type");
                    let document_types = if initial_protocol_version.is_some() {
                        assert_eq!(composite.sub_results.len(), 2);
                        vec![profile_type]
                    } else {
                        assert_eq!(composite.sub_results.len(), 3);
                        vec![post_type, profile_type]
                    };
                    let joined: Vec<Vec<Document>> = composite.sub_results[1..]
                        .iter()
                        .zip(document_types)
                        .map(|(sub, document_type)| {
                            let Some(composite_documents::sub_query_result::Result::Documents(
                                documents,
                            )) = &sub.result
                            else {
                                panic!("expected joined documents");
                            };
                            documents
                                .documents
                                .iter()
                                .map(|bytes| {
                                    Document::from_bytes(bytes, document_type, version)
                                        .expect("joined document deserializes")
                                })
                                .collect()
                        })
                        .collect();
                    if initial_protocol_version.is_some() {
                        assert_joined_documents(&[], &joined[0]);
                    } else {
                        assert_joined_documents(&joined[0], &joined[1]);
                    }
                }
                assert_eq!(
                    platform
                        .platform
                        .drive
                        .grove
                        .root_hash(None, &version.drive.grove_version)
                        .value
                        .expect("stored root"),
                    stored_root
                );
            }
        }
    }

    #[test]
    fn should_split_overlapping_page_and_lookup_documents_at_supported_protocol_versions() {
        for initial_protocol_version in [Some(12), Some(13), None] {
            let (platform, state, version, feed, dashpay) =
                setup_feed_state_with(initial_protocol_version, |feed, _| feed);
            let stored_root = platform
                .platform
                .drive
                .grove
                .root_hash(None, &version.drive.grove_version)
                .value
                .expect("stored root");
            let post_type = feed.document_type_for_name("post").expect("post type");
            for prove in [false, true] {
                let mut request =
                    composite_request(prove, feed.id().to_vec(), dashpay.id().to_vec());
                request.where_clauses = vec![ProtoWhereClause {
                    field: "$id".to_owned(),
                    operator: ProtoWhereOperator::In as i32,
                    value: Some(ProtoDocumentFieldValue {
                        variant: Some(document_field_value::Variant::List(
                            document_field_value::ValueList {
                                values: [POST_D, POST_A]
                                    .into_iter()
                                    .map(|id| ProtoDocumentFieldValue {
                                        variant: Some(document_field_value::Variant::BytesValue(
                                            id.to_vec(),
                                        )),
                                    })
                                    .collect(),
                            },
                        )),
                    }),
                    integer_range: None,
                    time_range: None,
                }];
                request.limit = Some(2);
                request.sub_queries = vec![sub(
                    Vec::new(),
                    "post",
                    sub_query::Kind::Documents,
                    initial_protocol_version.is_none().then_some(10),
                    Some((0, "$id", "quotedPostId")),
                )];
                let result = platform
                    .platform
                    .query_documents(
                        GetDocumentsRequest {
                            version: Some(RequestVersion::V1(request)),
                        },
                        &state,
                        version,
                    )
                    .expect("public query dispatcher");
                assert!(
                    result.errors.is_empty(),
                    "PV{}: {:?}",
                    version.protocol_version,
                    result.errors
                );
                let Some(ResponseVersion::V1(response)) = result.data.expect("response").version
                else {
                    panic!("expected a V1 response");
                };
                let (page, joined) = if prove {
                    let Some(ResponseResult::Proof(proof)) = response.result else {
                        panic!("a proved request must contain a proof");
                    };
                    let mut query = client_query(&feed, &dashpay, version);
                    query.internal_clauses = InternalClauses::extract_from_clauses(
                        vec![WhereClause {
                            field: "$id".to_owned(),
                            operator: WhereOperator::In,
                            value: Value::Array(vec![
                                Value::Identifier(POST_D),
                                Value::Identifier(POST_A),
                            ]),
                        }],
                        version,
                    )
                    .expect("original page clause");
                    query.limit = Some(2);
                    let mut lookup = query.sub_queries[1].clone();
                    lookup.limit = initial_protocol_version.is_none().then_some(10);
                    lookup.binding = Some(SubQueryBinding {
                        source: BindingSource::Page,
                        source_property: "$id".to_owned(),
                        field: "quotedPostId".to_owned(),
                    });
                    query.sub_queries = vec![lookup];
                    let (root, verified) = query
                        .verify_composite_documents_proof(&proof.grovedb_proof, version)
                        .expect("the original overlapping composition verifies");
                    assert_eq!(root, stored_root);
                    assert_eq!(verified.sub_results.len(), 1);
                    (
                        verified.page_documents,
                        verified.sub_results[0].documents().to_vec(),
                    )
                } else {
                    let Some(ResponseResult::Data(data)) = response.result else {
                        panic!("an unproved request must contain data");
                    };
                    let Some(result_data::Variant::Composite(composite)) = data.variant else {
                        panic!("expected composite data");
                    };
                    assert_eq!(composite.sub_results.len(), 1);
                    let Some(composite_documents::sub_query_result::Result::Documents(joined)) =
                        &composite.sub_results[0].result
                    else {
                        panic!("expected lookup documents");
                    };
                    let decode = |bytes: &Vec<Vec<u8>>| {
                        bytes
                            .iter()
                            .map(|bytes| {
                                Document::from_bytes(bytes, post_type, version)
                                    .expect("post deserializes")
                            })
                            .collect::<Vec<_>>()
                    };
                    (decode(&composite.page_documents), decode(&joined.documents))
                };
                assert_eq!(
                    page.iter()
                        .map(|document| document.id().to_buffer())
                        .collect::<Vec<_>>(),
                    vec![POST_A, POST_D]
                );
                assert_eq!(joined.len(), 1);
                assert_eq!(joined[0].id().to_buffer(), POST_A);
                assert_eq!(joined[0].owner_id().to_buffer(), OWNER_1);
                assert_eq!(
                    joined[0].properties().get("message"),
                    Some(&Value::Text("post 1".to_owned()))
                );
                assert_eq!(
                    platform
                        .platform
                        .drive
                        .grove
                        .root_hash(None, &version.drive.grove_version)
                        .value
                        .expect("stored root"),
                    stored_root
                );
            }
        }
    }

    /// A by-ids page's `$id IN` values are the client's: more than 100 of them, or values that
    /// are not identifiers, are refused as the plain query refuses them, not as a node fault.
    #[test]
    fn should_refuse_a_malformed_by_ids_page_as_invalid_argument() {
        for initial_protocol_version in [Some(13), None] {
            let (platform, state, version, feed, dashpay) =
                setup_feed_state_with(initial_protocol_version, |feed, _| feed);
            let too_many: Vec<ProtoDocumentFieldValue> = (0..101u8)
                .map(|i| ProtoDocumentFieldValue {
                    variant: Some(document_field_value::Variant::BytesValue(vec![i; 32])),
                })
                .collect();
            let not_identifiers: Vec<ProtoDocumentFieldValue> = (0..3u64)
                .map(|i| ProtoDocumentFieldValue {
                    variant: Some(document_field_value::Variant::Uint64Value(i)),
                })
                .collect();

            for (values, expected) in [
                (too_many, "at most 100 values"),
                (not_identifiers, "must contain identifiers"),
            ] {
                for prove in [false, true] {
                    let mut request =
                        composite_request(prove, feed.id().to_vec(), dashpay.id().to_vec());
                    request.where_clauses = vec![ProtoWhereClause {
                        field: "$id".to_string(),
                        operator: ProtoWhereOperator::In as i32,
                        value: Some(ProtoDocumentFieldValue {
                            variant: Some(document_field_value::Variant::List(
                                document_field_value::ValueList {
                                    values: values.clone(),
                                },
                            )),
                        }),
                        integer_range: None,
                        time_range: None,
                    }];
                    request.limit = Some(100);

                    let status = assert_invalid_argument_status(platform.platform.query_documents(
                        GetDocumentsRequest {
                            version: Some(RequestVersion::V1(request)),
                        },
                        &state,
                        version,
                    ));
                    assert!(status.message().contains(expected), "{}", status.message());
                }
            }
        }
    }
}
