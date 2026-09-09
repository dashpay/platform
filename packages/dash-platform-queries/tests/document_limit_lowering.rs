//! The `DocumentQuery` → `DriveDocumentQuery` lowering must reproduce
//! the server's limit contract exactly: `SizedQuery::limit` is
//! proof-sensitive, so any divergence lets an untrusted transport pair
//! a request the server would refuse (or answer differently) with a
//! genuine proof produced for another query.

use std::sync::Arc;

use dash_platform_queries::documents::document_query::DocumentQuery;
use dash_platform_queries::Error;
use dpp::prelude::DataContract;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::query::DriveDocumentQuery;

fn test_contract() -> Arc<DataContract> {
    let platform_version = PlatformVersion::latest();
    Arc::new(
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned(),
    )
}

/// The lowering mirrors `DriveDocumentQuery::from_typed_clauses`'
/// limit contract exactly: `0` = unset → the concrete server default
/// (`Some(DEFAULT_QUERY_LIMIT)`, never `None` — `None` is unbounded in
/// `SizedQuery`, which no honest server produces),
/// `1..=DEFAULT_QUERY_LIMIT` passes through, and anything above the
/// cap — including 101..=65535, which fits a `u16` but is
/// server-invalid, and 65537, which the old `as u16` cast silently
/// truncated to 1 — is refused with the server's `InvalidLimit`.
#[test]
fn limit_cap_mirrors_the_server() {
    let contract = test_contract();
    let query = |limit: u32| {
        DocumentQuery::new(Arc::clone(&contract), "niceDocument")
            .expect("document type exists")
            .with_limit(limit)
    };

    let unset_query = query(0);
    let unset = DriveDocumentQuery::try_from(&unset_query).expect("limit 0 is the unset sentinel");
    assert_eq!(
        unset.limit,
        Some(100),
        "0 must lower to the concrete server default, not unbounded"
    );

    let at_cap_query = query(100);
    let at_cap =
        DriveDocumentQuery::try_from(&at_cap_query).expect("the server serves limits up to 100");
    assert_eq!(at_cap.limit, Some(100));

    for limit in [101u32, 65_535, 65_537, u32::MAX] {
        let error = DriveDocumentQuery::try_from(&query(limit))
            .expect_err("a limit the server refuses must not reach a DriveDocumentQuery");
        assert!(
            matches!(
                &error,
                Error::Drive(drive::error::Error::Query(
                    drive::error::query::QuerySyntaxError::InvalidLimit(message)
                )) if message.contains("greater than max limit 100")
            ),
            "expected the server's InvalidLimit for limit {limit}: {error}"
        );
    }
}
