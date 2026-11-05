//! Document requests no index serves. A node answers one with a query error, so the SDK
//! refuses it before sending, by running the request-only half of the request's own verifier:
//! the documents path query, or the count, sum or average proof plan, which the node builds
//! its answer from too. The node and the check must agree on every shape below: a request the
//! check refuses is one no node answers, and one the check sends is one the node serves.

use super::*;
use dash_platform_queries::documents::document_query::DocumentQuery;
use dpp::data_contract::DataContractFactory;
use dpp::platform_value::{platform_value, Value};
use dpp::version::TryFromPlatformVersioned;
use drive::query::{SelectProjection, TimeRangeSelector, WhereClause, WhereOperator};
use drive_proof_verifier::types::Documents;
use drive_proof_verifier::{
    DocumentAverage, DocumentCount, DocumentSplitAverages, DocumentSplitCounts, DocumentSplitSums,
    DocumentSum, FromProof,
};

type Check = fn(&DocumentQuery, &PlatformVersion) -> Result<(), drive_proof_verifier::Error>;

// The SDK's error type is large; the check returns it as the view's check does.
#[allow(clippy::result_large_err)]
fn check<T: FromProof<DocumentQuery, Request = DocumentQuery>>(
    query: &DocumentQuery,
    version: &PlatformVersion,
) -> Result<(), drive_proof_verifier::Error> {
    T::check_request(query, version)
}

fn contract(id: u8, schemas: Value, version: &PlatformVersion) -> Arc<dpp::prelude::DataContract> {
    Arc::new(
        DataContractFactory::new(version.protocol_version)
            .expect("factory")
            .create_with_value_config(Identifier::new([id; 32]), 0, schemas, None, None)
            .expect("the contract is well-formed")
            .data_contract_owned(),
    )
}

/// A `listing` whose `price` is bucketed in windows of 100 and whose `$createdAt` in daily
/// windows, each under an index that counts listings and sums their `quantity`; nothing sums
/// `price`.
fn listings(version: &PlatformVersion) -> Arc<dpp::prelude::DataContract> {
    contract(
        31,
        platform_value!({"listing": {
            "type": "object",
            "properties": {
                "price": {"type": "integer", "minimum": 0, "maximum": 1_000_000, "position": 0},
                "category": {"type": "string", "minLength": 1, "maxLength": 63, "position": 1},
                "quantity": {"type": "integer", "minimum": 0, "maximum": 1_000_000, "position": 2}
            },
            "required": ["price", "category", "quantity", "$createdAt"],
            "indices": [
                {"name": "byPriceBand",
                 "properties": [{"price": "asc"}, {"category": "asc"}],
                 "integerRange": {"on": "price", "range": 100u64, "step": 100u64},
                 "averageable": "quantity"},
                {"name": "byDay",
                 "properties": [{"$createdAt": "asc"}, {"category": "asc"}],
                 "timeRange": {"on": "$createdAt", "range": 86400, "step": 86400},
                 "averageable": "quantity"}
            ],
            "additionalProperties": false
        }}),
        version,
    )
}

/// A `review` whose index sums and counts `grade` per restaurant, and a `note` that counts and
/// sums nothing.
fn reviews(version: &PlatformVersion) -> Arc<dpp::prelude::DataContract> {
    contract(
        32,
        platform_value!({
            "review": {
                "type": "object",
                "properties": {
                    "restaurantId": {"type": "string", "minLength": 1, "maxLength": 59, "position": 0},
                    "grade": {"type": "integer", "minimum": 0, "maximum": 100, "position": 1}
                },
                "required": ["restaurantId", "grade"],
                "indices": [
                    {"name": "byRestaurant",
                     "properties": [{"restaurantId": "asc"}],
                     "averageable": "grade"}
                ],
                "additionalProperties": false
            },
            "note": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "minLength": 1, "maxLength": 59, "position": 0}
                },
                "required": ["name"],
                "additionalProperties": false
            }
        }),
        version,
    )
}

fn equal(field: &str, value: Value) -> WhereClause {
    WhereClause {
        field: field.to_string(),
        operator: WhereOperator::Equal,
        value,
    }
}

fn any_of(field: &str, values: Value) -> WhereClause {
    WhereClause {
        field: field.to_string(),
        operator: WhereOperator::In,
        value: values,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_refuse_before_sending_exactly_the_requests_no_index_serves() {
    // Windows ride protocol version 14, which testnet runs, and the latest version.
    let mut versions = vec![
        PlatformVersion::get(14).expect("protocol version 14"),
        PlatformVersion::latest(),
    ];
    versions.dedup_by_key(|version| version.protocol_version);
    for version in versions {
        agree_at(version).await;
    }
}

async fn agree_at(version: &'static PlatformVersion) {
    let fixture = QueryFixture::new_with_version(version);
    let listings = listings(version);
    let reviews = reviews(version);
    for contract in [&listings, &reviews] {
        fixture
            .platform
            .drive
            .apply_contract(contract, BlockInfo::default(), true, None, None, version)
            .expect("store the contract");
    }
    let (mut client, server) = fixture.client();
    let listing = || DocumentQuery::new(Arc::clone(&listings), "listing").expect("listing");
    let review = || DocumentQuery::new(Arc::clone(&reviews), "review").expect("review");
    let note = || DocumentQuery::new(Arc::clone(&reviews), "note").expect("note");
    let price_window = || listing().with_integer_range("price", Value::U64(200));
    let restaurants_in = || any_of("restaurantId", platform_value!(["a", "b"]));
    let categories_in = || any_of("category", platform_value!(["tools", "toys"]));

    // (what it is, the view's check, the request, whether an index serves it)
    let shapes: Vec<(&str, Check, DocumentQuery, bool)> = vec![
        // No index covers a price window beside an equality on `$createdAt`.
        (
            "documents: price window + $createdAt equality",
            check::<Documents>,
            price_window().with_where(equal("$createdAt", Value::U64(1_700_000_000_000))),
            false,
        ),
        (
            "documents: price window",
            check::<Documents>,
            price_window(),
            true,
        ),
        (
            "documents: price window + category",
            check::<Documents>,
            price_window().with_where(equal("category", Value::Text("tools".to_string()))),
            true,
        ),
        // No index serves two windows.
        (
            "documents: two windows",
            check::<Documents>,
            price_window().with_time_range(
                "$createdAt",
                TimeRangeSelector::ByStart {
                    start_ms: 1_700_006_400_000,
                },
            ),
            false,
        ),
        // Nothing sums `price`.
        (
            "sum(price)",
            check::<DocumentSum>,
            listing().with_select(SelectProjection::sum("price")),
            false,
        ),
        (
            "sum(price) per category",
            check::<DocumentSplitSums>,
            listing()
                .with_select(SelectProjection::sum("price"))
                .with_where(categories_in())
                .with_group_by("category"),
            false,
        ),
        (
            "avg(price)",
            check::<DocumentAverage>,
            listing().with_select(SelectProjection::avg("price")),
            false,
        ),
        (
            "avg(price) per category",
            check::<DocumentSplitAverages>,
            listing()
                .with_select(SelectProjection::avg("price"))
                .with_where(categories_in())
                .with_group_by("category"),
            false,
        ),
        (
            "sum(grade) of restaurants",
            check::<DocumentSum>,
            review()
                .with_select(SelectProjection::sum("grade"))
                .with_where(restaurants_in()),
            true,
        ),
        (
            "avg(grade) per restaurant",
            check::<DocumentSplitAverages>,
            review()
                .with_select(SelectProjection::avg("grade"))
                .with_where(restaurants_in())
                .with_group_by("restaurantId"),
            true,
        ),
        (
            "sum(quantity) in a price window + category",
            check::<DocumentSum>,
            price_window()
                .with_select(SelectProjection::sum("quantity"))
                .with_where(equal("category", Value::Text("tools".to_string()))),
            true,
        ),
        // A `note` counts nothing.
        (
            "count of notes",
            check::<DocumentCount>,
            note().with_select(SelectProjection::count_star()),
            false,
        ),
        (
            "count of notes per name",
            check::<DocumentSplitCounts>,
            note()
                .with_select(SelectProjection::count_star())
                .with_where(any_of("name", platform_value!(["a", "b"])))
                .with_group_by("name"),
            false,
        ),
        (
            "count in a price window + category",
            check::<DocumentCount>,
            price_window()
                .with_select(SelectProjection::count_star())
                .with_where(equal("category", Value::Text("tools".to_string()))),
            true,
        ),
        (
            "count in a price window per category",
            check::<DocumentSplitCounts>,
            price_window()
                .with_select(SelectProjection::count_star())
                .with_where(categories_in())
                .with_group_by("category"),
            true,
        ),
    ];
    let protocol_version = version.protocol_version;
    for (shape, check, query, served) in shapes {
        let request =
            wire::GetDocumentsRequest::try_from_platform_versioned(query.clone(), version)
                .expect("the query encodes");
        let answer = client.get_documents(request).await.map(drop);
        let checked = check(&query, version);
        assert_eq!(
            (answer.is_ok(), checked.is_ok()),
            (served, served),
            "protocol {protocol_version}, {shape}: the node answers {answer:?}, the check \
             before sending says {checked:?}"
        );
        if !served {
            assert_eq!(
                code(answer),
                Code::InvalidArgument,
                "protocol {protocol_version}, {shape}: a query error"
            );
        }
    }
    server.abort();
}
