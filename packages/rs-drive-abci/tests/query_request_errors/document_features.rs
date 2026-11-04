//! The document features the SDK refuses below their protocol version
//! (`DocumentQuery::required_protocol_version`), sent to a node one version below it.
//!
//! Most of them are refused with a status clients do not retry, so sending them would only
//! fail the call. A start cursor on a type whose documents are summed fails with a status
//! clients retry on every other node, banning each one, so it must never be sent there; from
//! its requirement on, it is served.

use super::*;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dash_platform_queries::documents::document_query::DocumentQuery;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::DocumentV0Getters;
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::TryFromPlatformVersioned;
use drive::query::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
    SelectProjection, TimeRangeSelector, WhereClause, WhereOperator,
};
use drive::util::object_size_info::{
    DocumentAndContractInfo, DocumentInfo::DocumentRefInfo, OwnedDocumentInfo,
};
use rs_dapi_client::CanRetry;

/// Its `tip` documents are summed by `amount`.
const TIP_JAR_CONTRACT_PATH: &str =
    "../rs-drive/tests/supporting_files/contract/tip-jar/tip-jar-contract.json";

/// The fixture contract at `version`, stored in `fixture`.
fn fixture_contract(fixture: &QueryFixture, version: &PlatformVersion) -> Arc<DataContract> {
    let contract =
        get_data_contract_fixture(Some(Identifier::new([9; 32])), 0, version.protocol_version)
            .data_contract_owned();
    fixture
        .platform
        .drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .expect("store the fixture contract");
    Arc::new(contract)
}

/// The tip jar contract at `version`, stored in `fixture` with one tip, and that tip's id.
fn tip_jar(fixture: &QueryFixture, version: &PlatformVersion) -> (Arc<DataContract>, [u8; 32]) {
    let contract = json_document_to_contract(TIP_JAR_CONTRACT_PATH, false, version)
        .expect("the tip jar contract");
    let drive = &fixture.platform.drive;
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .expect("store the tip jar contract");
    let document_type = contract.document_type_for_name("tip").expect("tip");
    let tip = document_type
        .random_document(Some(7), version)
        .expect("a random tip");
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&tip, None)),
                    owner_id: Some(tip.owner_id().to_buffer()),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            version,
            None,
        )
        .expect("store a tip");
    (Arc::new(contract), tip.id().to_buffer())
}

/// Builds a query over the fixture contract it is given.
type BuildQuery = Box<dyn Fn(Arc<DataContract>) -> DocumentQuery>;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_refuse_each_document_feature_below_its_protocol_version_without_a_ban() {
    let fixture_at = |query: &DocumentQuery| {
        let requirement = query
            .required_protocol_version()
            .expect("the query uses a feature");
        PlatformVersion::get(requirement.protocol_version - 1).expect("the version below")
    };
    let count = SelectProjection::count_star;
    let cases: Vec<(&str, BuildQuery)> = vec![
        (
            "contains",
            Box::new(|contract| {
                DocumentQuery::new(contract, "niceDocument")
                    .unwrap()
                    .with_where(WhereClause {
                        field: "name".to_string(),
                        operator: WhereOperator::Contains,
                        value: Value::Text("dash".into()),
                    })
            }),
        ),
        (
            "time range",
            Box::new(move |contract| {
                DocumentQuery::new(contract, "niceDocument")
                    .unwrap()
                    .with_select(count())
                    .with_time_range("$createdAt", TimeRangeSelector::Newest)
            }),
        ),
        (
            "integer range",
            Box::new(move |contract| {
                DocumentQuery::new(contract, "niceDocument")
                    .unwrap()
                    .with_select(count())
                    .with_integer_range("$revision", Value::U64(1))
            }),
        ),
        (
            "ranked",
            Box::new(move |contract| {
                DocumentQuery::new(contract, "niceDocument")
                    .unwrap()
                    .with_select(count())
                    .with_group_by("$ownerId")
                    .order_by_selected_aggregate(
                        dash_platform_queries::documents::document_query::RankingDirection::Descending,
                    )
                    .with_limit(5)
            }),
        ),
        (
            "having",
            Box::new(move |contract| {
                DocumentQuery::new(contract, "niceDocument")
                    .unwrap()
                    .with_select(count())
                    .with_group_by("$ownerId")
                    .with_having(vec![HavingClause {
                        aggregate: HavingAggregate {
                            function: HavingAggregateFunction::Count,
                            field: String::new(),
                        },
                        operator: HavingOperator::GreaterThan,
                        right: HavingRightOperand::Value(Value::U64(1)),
                    }])
                    .with_limit(5)
            }),
        ),
        (
            "several in clauses",
            Box::new(|contract| {
                DocumentQuery::new(contract, "niceDocument")
                    .unwrap()
                    .with_where(WhereClause {
                        field: "$ownerId".to_string(),
                        operator: WhereOperator::In,
                        value: Value::Array(vec![
                            Value::Identifier([1; 32]),
                            Value::Identifier([2; 32]),
                        ]),
                    })
                    .with_where(WhereClause {
                        field: "name".to_string(),
                        operator: WhereOperator::In,
                        value: platform_value!(["a", "b"]),
                    })
            }),
        ),
    ];
    let mut failures = Vec::new();
    for (feature, build) in cases {
        let probe = build(Arc::new(
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
                .data_contract_owned(),
        ));
        let below = fixture_at(&probe);
        let fixture = QueryFixture::new_with_version(below);
        let query = build(fixture_contract(&fixture, below));
        let request = wire::GetDocumentsRequest::try_from_platform_versioned(query, below)
            .expect("the query encodes");
        let (mut client, server) = fixture.client();
        match client.get_documents(request).await {
            Err(status) if status.code() == Code::InvalidArgument && !status.can_retry() => {}
            other => failures.push(format!(
                "{feature} at protocol {}: {:?}",
                below.protocol_version,
                other.map(|_| ())
            )),
        }
        server.abort();
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_fail_a_cursor_on_summed_documents_with_a_retried_status_only_below_its_protocol_version(
) {
    let probe = {
        let fixture = QueryFixture::new();
        let (contract, _) = tip_jar(&fixture, PlatformVersion::latest());
        let mut query = DocumentQuery::new(contract, "tip").unwrap().with_limit(10);
        query.start = Some(Start::StartAfter(vec![1; 32]));
        query
            .required_protocol_version()
            .expect("a cursor on summed documents needs a version")
    };
    for (version, served) in [
        (probe.protocol_version - 1, false),
        (probe.protocol_version, true),
    ] {
        let version = PlatformVersion::get(version).expect("a known protocol version");
        let fixture = QueryFixture::new_with_version(version);
        let (contract, tip) = tip_jar(&fixture, version);
        let mut query = DocumentQuery::new(contract, "tip").unwrap().with_limit(10);
        query.start = Some(Start::StartAt(tip.to_vec()));
        let request = wire::GetDocumentsRequest::try_from_platform_versioned(query, version)
            .expect("the query encodes");
        let (mut client, server) = fixture.client();
        let result = client.get_documents(request).await;
        if served {
            result.unwrap_or_else(|status| {
                panic!("protocol {}: served: {status:?}", version.protocol_version)
            });
        } else {
            let status = result.expect_err("refused below its version");
            assert!(
                status.can_retry(),
                "protocol {}: {status:?} would ban no node; the SDK's refusal guards a ban",
                version.protocol_version
            );
        }
        server.abort();
    }
}

/// Its `post` indexes the elements of `hashtags`, and ranks them (`byHashtag`).
const ARRAY_INDEX_CONTRACT_PATH: &str =
    "../rs-drive/tests/supporting_files/contract/array-index/array-index-contract.json";

/// An `item` whose compound index takes an `In` on each of its two properties.
fn two_in_contract(version: &PlatformVersion) -> DataContract {
    let schemas = platform_value!({"item": {
        "type": "object",
        "properties": {
            "a": {"type": "string", "minLength": 1, "maxLength": 32, "position": 0},
            "b": {"type": "string", "minLength": 1, "maxLength": 32, "position": 1}
        },
        "required": ["a", "b"],
        "indices": [{"name": "byAB", "properties": [{"a": "asc"}, {"b": "asc"}]}],
        "additionalProperties": false
    }});
    dpp::data_contract::DataContractFactory::new(version.protocol_version)
        .expect("factory")
        .create_with_value_config(Identifier::new([10; 32]), 0, schemas, None, None)
        .expect("the item contract is well-formed")
        .data_contract_owned()
}

fn two_ins(contract: Arc<DataContract>) -> DocumentQuery {
    DocumentQuery::new(contract, "item")
        .unwrap()
        .with_where(WhereClause {
            field: "a".to_string(),
            operator: WhereOperator::In,
            value: platform_value!(["x", "y"]),
        })
        .with_where(WhereClause {
            field: "b".to_string(),
            operator: WhereOperator::In,
            value: platform_value!(["p", "q"]),
        })
        .with_order_by(drive::query::OrderClause {
            field: "a".to_string(),
            ascending: true,
        })
        .with_order_by(drive::query::OrderClause {
            field: "b".to_string(),
            ascending: true,
        })
        .with_limit(10)
}

/// A contract a feature's query is served against, and the query.
struct Served {
    feature: &'static str,
    contract: fn(&PlatformVersion) -> Option<DataContract>,
    query: fn(Arc<DataContract>) -> DocumentQuery,
}

fn array_index_contract(version: &PlatformVersion) -> Option<DataContract> {
    json_document_to_contract(ARRAY_INDEX_CONTRACT_PATH, false, version).ok()
}

fn served_cases() -> Vec<Served> {
    vec![
        Served {
            feature: "contains",
            contract: array_index_contract,
            query: |contract| {
                DocumentQuery::new(contract, "post")
                    .unwrap()
                    .with_where(WhereClause {
                        field: "hashtags".to_string(),
                        operator: WhereOperator::Contains,
                        value: Value::Text("dash".into()),
                    })
                    .with_limit(10)
            },
        },
        Served {
            feature: "ranked",
            contract: array_index_contract,
            query: |contract| {
                DocumentQuery::new(contract, "post")
                    .unwrap()
                    .with_select(SelectProjection::count_star())
                    .with_group_by("hashtags")
                    .order_by_selected_aggregate(
                        dash_platform_queries::documents::document_query::RankingDirection::Descending,
                    )
                    .with_limit(5)
            },
        },
        Served {
            feature: "having",
            contract: array_index_contract,
            query: |contract| {
                DocumentQuery::new(contract, "post")
                    .unwrap()
                    .with_select(SelectProjection::count_star())
                    .with_group_by("hashtags")
                    .with_having(vec![HavingClause {
                        aggregate: HavingAggregate {
                            function: HavingAggregateFunction::Count,
                            field: String::new(),
                        },
                        operator: HavingOperator::GreaterThan,
                        right: HavingRightOperand::Value(Value::U64(0)),
                    }])
                    .with_limit(5)
            },
        },
        Served {
            feature: "several in clauses",
            contract: |version| Some(two_in_contract(version)),
            query: two_ins,
        },
    ]
}

/// From its protocol version on, a node serves each feature on a contract that indexes it;
/// one version below, the same query on the same contract is refused without a ban, when the
/// contract can be registered there at all. So the requirement is neither later nor earlier
/// than the version that serves the feature.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_serve_each_document_feature_from_its_protocol_version_and_not_before() {
    let mut failures = Vec::new();
    for case in served_cases() {
        let latest = Arc::new((case.contract)(PlatformVersion::latest()).expect("the contract"));
        let required = (case.query)(latest)
            .required_protocol_version()
            .expect("the query uses a feature")
            .protocol_version;
        for (version, served) in [(required - 1, false), (required, true)] {
            let version = PlatformVersion::get(version).expect("a known protocol version");
            let Some(contract) = (case.contract)(version) else {
                // The contract's indexes do not exist at this version, so neither can the
                // query it serves.
                assert!(
                    !served,
                    "{}: the contract exists from its version",
                    case.feature
                );
                continue;
            };
            let fixture = QueryFixture::new_with_version(version);
            fixture
                .platform
                .drive
                .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
                .expect("store the contract");
            let request = wire::GetDocumentsRequest::try_from_platform_versioned(
                (case.query)(Arc::new(contract)),
                version,
            )
            .expect("the query encodes");
            let (mut client, server) = fixture.client();
            let result = client.get_documents(request).await.map(|_| ());
            let outcome_ok = match (&result, served) {
                (Ok(()), true) => true,
                (Err(status), false) => {
                    status.code() == Code::InvalidArgument && !status.can_retry()
                }
                _ => false,
            };
            if !outcome_ok {
                failures.push(format!(
                    "{} at protocol {} (served expected: {served}): {result:?}",
                    case.feature, version.protocol_version
                ));
            }
            server.abort();
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
