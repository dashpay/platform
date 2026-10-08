use super::offsets::{assert_no_node_bans, DocumentsQuery};
use super::*;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::SubQuery as ProtoSubQuery;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::{
    sub_query, sub_query::Binding as ProtoBinding,
};
use dapi_grpc::platform::v0::get_documents_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_documents_request::{
    document_field_value, DocumentFieldValue as ProtoDocumentFieldValue, GetDocumentsRequestV1,
    WhereClause as ProtoWhereClause, WhereOperator as ProtoWhereOperator,
};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v1::Result as ResponseResult,
    get_documents_response_v1::{composite_documents, result_data},
    Version as ResponseVersion,
};
use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::Value;
use dpp::prelude::DataContract;
use dpp::tests::json_document::{json_document_to_contract, json_document_to_json_value};
use drive::query::{
    BindingSource, DriveSubQuery, InternalClauses, SubQueryBinding, SubQueryKind, WhereClause,
    WhereOperator,
};
use drive::util::object_size_info::{
    DocumentAndContractInfo, DocumentInfo::DocumentRefInfo, OwnedDocumentInfo,
};
use drive::util::storage_flags::StorageFlags;
use std::collections::BTreeMap;
const FEED_CONTRACT_PATH: &str =
    "../rs-drive/tests/supporting_files/contract/yappr-feed/yappr-feed-contract.json";
const DASHPAY_CONTRACT_PATH: &str =
    "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json";
const POST_A: [u8; 32] = [0xA1; 32];
const POST_B: [u8; 32] = [0xB2; 32];
const POST_D: [u8; 32] = [0xD4; 32];
const OWNER_1: [u8; 32] = [0x11; 32];
const OWNER_2: [u8; 32] = [0x22; 32];
fn store_document(
    fixture: &QueryFixture,
    contract: &DataContract,
    document_type: dpp::data_contract::document_type::DocumentTypeRef,
    document: &Document,
    version: &PlatformVersion,
) {
    fixture
        .platform
        .drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: None,
                },
                contract,
                document_type,
            },
            true,
            BlockInfo::default(),
            true,
            None,
            version,
            None,
        )
        .expect("store seeded document");
}
fn setup_feed(
    fixture: &QueryFixture,
) -> (DataContract, DataContract, BTreeMap<String, Vec<Document>>) {
    let version = fixture.version;
    let feed = if version.protocol_version < 14 {
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
        let mut value = json_document_to_json_value(FEED_CONTRACT_PATH).expect("feed JSON");
        // Materialized likes support raw document lookups as well as counts.
        value["documentSchemas"]["like"]
            .as_object_mut()
            .expect("like schema")
            .remove("indexOnly");
        for index in value["documentSchemas"]["like"]["indices"]
            .as_array_mut()
            .expect("like indexes")
        {
            let index = index.as_object_mut().expect("like index");
            for keyword in ["terminal", "preallocated", "rankedCountable"] {
                index.remove(keyword);
            }
        }
        DataContract::from_json(value, false, version).expect("feed contract")
    };
    let dashpay = json_document_to_contract(
        DASHPAY_CONTRACT_PATH,
        version.protocol_version < 14,
        version,
    )
    .expect("dashpay fixture");
    let mut expected = BTreeMap::<String, Vec<Document>>::new();
    for contract in [&feed, &dashpay] {
        fixture
            .platform
            .drive
            .apply_contract(contract, BlockInfo::default(), true, None, None, version)
            .expect("store contract");
    }
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
        expected
            .entry("post".to_owned())
            .or_default()
            .push(post.clone());
        store_document(fixture, &feed, post_type, &post, version);
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
        expected
            .entry("like".to_owned())
            .or_default()
            .push(like.clone());
        store_document(fixture, &feed, like_type, &like, version);
    }
    let mut profile = profile_type
        .random_document(Some(30), version)
        .expect("profile");
    let mut props = std::collections::BTreeMap::new();
    props.insert("displayName".to_string(), Value::Text("one".to_string()));
    profile.set_properties(props);
    profile.set_owner_id(Identifier::from(OWNER_1));
    expected
        .entry("profile".to_owned())
        .or_default()
        .push(profile.clone());
    store_document(fixture, &dashpay, profile_type, &profile, version);

    (feed, dashpay, expected)
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

fn composite_request(prove: bool, feed_id: Vec<u8>, dashpay_id: Vec<u8>) -> GetDocumentsRequestV1 {
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
    let bound =
        |contract: &'a DataContract, type_name: &str, kind, source_property: &str, field: &str| {
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

fn by_ids(request: &mut GetDocumentsRequestV1, ids: Vec<[u8; 32]>) {
    request.where_clauses = vec![ProtoWhereClause {
        field: "$id".to_owned(),
        operator: ProtoWhereOperator::In as i32,
        value: Some(ProtoDocumentFieldValue {
            variant: Some(document_field_value::Variant::List(
                document_field_value::ValueList {
                    values: ids
                        .into_iter()
                        .map(|id| ProtoDocumentFieldValue {
                            variant: Some(document_field_value::Variant::BytesValue(id.to_vec())),
                        })
                        .collect(),
                },
            )),
        }),
        integer_range: None,
        time_range: None,
    }];
    request.limit = Some(2);
}
fn request_for(
    fixture: &QueryFixture,
    feed: &DataContract,
    dashpay: &DataContract,
    prove: bool,
) -> GetDocumentsRequestV1 {
    let mut request = composite_request(prove, feed.id().to_vec(), dashpay.id().to_vec());
    if fixture.version.protocol_version < 14 {
        request.sub_queries.remove(1);
    }
    request
}
fn wire_request(request: GetDocumentsRequestV1) -> wire::GetDocumentsRequest {
    wire::GetDocumentsRequest {
        version: Some(RequestVersion::V1(request)),
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_refuse_composite_caps_unavailable_on_historical_nodes_over_tonic() {
    let mut failures = Vec::new();
    for version in [
        PlatformVersion::get(12).unwrap(),
        PlatformVersion::get(13).unwrap(),
    ] {
        let fixture = QueryFixture::new_with_version(version);
        let (feed, dashpay, _) = setup_feed(&fixture);
        let (mut client, server) = fixture.client();
        for prove in [false, true] {
            for leaf in [false, true] {
                let mut request = request_for(&fixture, &feed, &dashpay, prove);
                if leaf {
                    by_ids(&mut request, vec![POST_B, POST_A]);
                    request.sub_queries = vec![sub(
                        vec![],
                        "like",
                        sub_query::Kind::Documents,
                        Some(2),
                        Some((0, "$id", "postId")),
                    )];
                }
                let status = client
                    .get_documents(wire_request(request))
                    .await
                    .expect_err("historical capped composition is unsupported");
                println!(
                    "PV{} prove={prove} leaf={leaf}: {status}",
                    version.protocol_version
                );
                let expected_message = if prove && leaf {
                    "unsupported error: the composite query's components cannot be merged into one proof: can not merge pathqueries carrying per-instance limits (Query::limit)"
                } else {
                    "unsupported error: composite query limits require a protocol version that supports per-instance limits"
                };
                if status.code() != Code::InvalidArgument || status.message() != expected_message {
                    failures.push(format!(
                        "PV{} prove={prove} leaf={leaf}: {status}",
                        version.protocol_version
                    ));
                }
            }
        }
        server.abort();
    }
    assert!(
        failures.is_empty(),
        "historical caller capability failures must be InvalidArgument:\n{}",
        failures.join("\n")
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_not_ban_historical_nodes_for_composite_caps() {
    for version in [
        PlatformVersion::get(12).unwrap(),
        PlatformVersion::get(13).unwrap(),
    ] {
        let fixture = Arc::new(QueryFixture::new_with_version(version));
        let (feed, dashpay, _) = setup_feed(&fixture);
        for prove in [false, true] {
            let attempts = Arc::default();
            let request = wire_request(request_for(&fixture, &feed, &dashpay, prove));
            assert_no_node_bans(
                DocumentsQuery {
                    fixture: Arc::clone(&fixture),
                    request,
                    attempts: Arc::clone(&attempts),
                },
                &attempts,
            )
            .await;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_preserve_supported_composites_and_empty_derived_caps_over_tonic() {
    for version in [
        PlatformVersion::get(12).unwrap(),
        PlatformVersion::get(13).unwrap(),
        PlatformVersion::latest(),
    ] {
        let fixture = QueryFixture::new_with_version(version);
        let (feed, dashpay, expected) = setup_feed(&fixture);
        let stored_root = fixture
            .platform
            .drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .value
            .expect("stored root");
        let (mut client, server) = fixture.client();
        for empty in [false, true] {
            for prove in [false, true] {
                let mut request = request_for(&fixture, &feed, &dashpay, prove);
                let mut query = client_query(&feed, &dashpay, version);
                let ids = if empty {
                    vec![[0xFF; 32]]
                } else {
                    vec![POST_B, POST_A]
                };
                by_ids(&mut request, ids.clone());
                query.internal_clauses = InternalClauses::extract_from_clauses(
                    vec![WhereClause {
                        field: "$id".to_owned(),
                        operator: WhereOperator::In,
                        value: Value::Array(ids.into_iter().map(Value::Identifier).collect()),
                    }],
                    version,
                )
                .expect("original selected IDs");
                query.limit = Some(2);
                if version.protocol_version < 14 {
                    query.sub_queries.remove(1);
                }
                if empty {
                    request.sub_queries = vec![sub(
                        vec![],
                        "like",
                        sub_query::Kind::Documents,
                        Some(2),
                        Some((0, "$id", "postId")),
                    )];
                    query.sub_queries = vec![DriveSubQuery {
                        contract: &feed,
                        document_type: feed.document_type_for_name("like").expect("like"),
                        kind: SubQueryKind::Documents,
                        where_clauses: vec![],
                        order_by: vec![],
                        limit: Some(2),
                        binding: Some(SubQueryBinding {
                            source: BindingSource::Page,
                            source_property: "$id".to_owned(),
                            field: "postId".to_owned(),
                        }),
                    }];
                }
                let response = client
                    .get_documents(wire_request(request))
                    .await
                    .expect("supported composition")
                    .into_inner();
                let Some(ResponseVersion::V1(v1)) = response.version else {
                    panic!("V1")
                };
                let expected_page = if empty {
                    vec![]
                } else {
                    [POST_A, POST_B]
                        .into_iter()
                        .map(|id| {
                            expected["post"]
                                .iter()
                                .find(|d| d.id().to_buffer() == id)
                                .unwrap()
                                .clone()
                        })
                        .collect()
                };
                if prove {
                    let Some(ResponseResult::Proof(proof)) = v1.result else {
                        panic!("proof")
                    };
                    let (root, verified) = query
                        .verify_composite_documents_proof(&proof.grovedb_proof, version)
                        .expect("original composition proof");
                    assert_eq!(root, stored_root);
                    assert_eq!(verified.page_documents, expected_page);
                    if empty {
                        assert_eq!(verified.sub_results.len(), 1);
                        assert!(verified.sub_results[0].documents().is_empty());
                    } else {
                        assert_eq!(
                            verified.sub_results[0]
                                .counts()
                                .iter()
                                .map(|e| (e.key.clone(), e.count))
                                .collect::<BTreeMap<_, _>>(),
                            BTreeMap::from([
                                (POST_A.to_vec(), Some(2)),
                                (POST_B.to_vec(), Some(1))
                            ])
                        );
                        let profile_index = if version.protocol_version < 14 { 1 } else { 2 };
                        assert_eq!(
                            verified.sub_results[profile_index].documents(),
                            expected["profile"].as_slice()
                        );
                        if version.protocol_version >= 14 {
                            assert_eq!(
                                verified.sub_results[1].documents(),
                                [expected["post"][0].clone()].as_slice()
                            );
                        }
                    }
                } else {
                    let Some(ResponseResult::Data(data)) = v1.result else {
                        panic!("data")
                    };
                    let Some(result_data::Variant::Composite(composite)) = data.variant else {
                        panic!("composite")
                    };
                    let post_type = feed.document_type_for_name("post").unwrap();
                    assert_eq!(
                        composite
                            .page_documents
                            .iter()
                            .map(|b| Document::from_bytes(b, post_type, version)
                                .expect("post payload"))
                            .collect::<Vec<_>>(),
                        expected_page
                    );
                    if empty {
                        assert_eq!(composite.sub_results.len(), 1);
                        let Some(composite_documents::sub_query_result::Result::Documents(docs)) =
                            &composite.sub_results[0].result
                        else {
                            panic!("docs")
                        };
                        assert!(docs.documents.is_empty());
                    } else {
                        let Some(composite_documents::sub_query_result::Result::Counts(counts)) =
                            &composite.sub_results[0].result
                        else {
                            panic!("counts")
                        };
                        assert_eq!(
                            counts
                                .entries
                                .iter()
                                .map(|e| (e.key.clone(), e.count))
                                .collect::<BTreeMap<_, _>>(),
                            BTreeMap::from([(POST_A.to_vec(), 2), (POST_B.to_vec(), 1)])
                        );
                        let profile_index = if version.protocol_version < 14 { 1 } else { 2 };
                        let Some(composite_documents::sub_query_result::Result::Documents(docs)) =
                            &composite.sub_results[profile_index].result
                        else {
                            panic!("profiles")
                        };
                        let ty = dashpay.document_type_for_name("profile").unwrap();
                        assert_eq!(
                            docs.documents
                                .iter()
                                .map(|b| Document::from_bytes(b, ty, version)
                                    .expect("profile payload"))
                                .collect::<Vec<_>>(),
                            expected["profile"]
                        );
                        if version.protocol_version >= 14 {
                            let Some(composite_documents::sub_query_result::Result::Documents(
                                docs,
                            )) = &composite.sub_results[1].result
                            else {
                                panic!("quoted posts")
                            };
                            assert_eq!(
                                docs.documents
                                    .iter()
                                    .map(|b| Document::from_bytes(b, post_type, version).unwrap())
                                    .collect::<Vec<_>>(),
                                vec![expected["post"][0].clone()]
                            );
                        }
                    }
                }
            }
        }
        server.abort();
        assert_eq!(
            stored_root,
            fixture
                .platform
                .drive
                .grove
                .root_hash(None, &version.drive.grove_version)
                .value
                .expect("unchanged root")
        );
    }
}
