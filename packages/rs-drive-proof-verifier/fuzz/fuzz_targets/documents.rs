//! DPNS `domain` documents through the SDK's `DocumentQuery`: the exact-label
//! lookup and the label-prefix search.
//!
//! The corpus stores a placeholder payload at the proven document position, so
//! the recorded proof of the signed root fails document decoding and the
//! verifier must never accept a response for either query.

#![no_main]

use std::sync::{Arc, LazyLock};

use dapi_grpc::platform::v0::{get_documents_response, GetDocumentsResponse};
use dash_platform_queries::documents::document_query::DocumentQuery;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use drive::query::{OrderClause, WhereClause, WhereOperator};
use drive_proof_verifier::types::Documents;
use drive_proof_verifier::Error;
use drive_proof_verifier_fuzz::{metadata, proof, verify, DPNS_CONTRACT, PLATFORM_VERSIONS};
use libfuzzer_sys::fuzz_target;

const DOMAIN: &str = "domain";

fn text_clause(field: &str, operator: WhereOperator, value: &str) -> WhereClause {
    WhereClause {
        field: field.to_string(),
        operator,
        value: Value::Text(value.to_string()),
    }
}

fn dpns_query() -> DocumentQuery {
    DocumentQuery::new(Arc::clone(&DPNS_CONTRACT), DOMAIN)
        .expect("DPNS defines the domain document type")
        .with_where(text_clause(
            "normalizedParentDomainName",
            WhereOperator::Equal,
            "dash",
        ))
}

static RESOLVE_NAME: LazyLock<DocumentQuery> = LazyLock::new(|| {
    dpns_query()
        .with_where(text_clause(
            "normalizedLabel",
            WhereOperator::Equal,
            "alice",
        ))
        .with_limit(1)
});

static SEARCH_NAMES: LazyLock<DocumentQuery> = LazyLock::new(|| {
    dpns_query()
        .with_where(text_clause(
            "normalizedLabel",
            WhereOperator::StartsWith,
            "ali",
        ))
        .with_order_by(OrderClause {
            field: "normalizedLabel".to_string(),
            ascending: true,
        })
        .with_limit(25)
});

fn documents(
    platform_version: &PlatformVersion,
    query: &DocumentQuery,
    grovedb_proof: &[u8],
) -> Result<Option<Documents>, Error> {
    verify::<DocumentQuery, Documents>(
        platform_version,
        query.clone(),
        GetDocumentsResponse {
            version: Some(get_documents_response::Version::V0(
                get_documents_response::GetDocumentsResponseV0 {
                    metadata: Some(metadata()),
                    result: Some(
                        get_documents_response::get_documents_response_v0::Result::Proof(proof(
                            grovedb_proof,
                        )),
                    ),
                },
            )),
        },
    )
}

fuzz_target!(|data: &[u8]| {
    for platform_version in PLATFORM_VERSIONS {
        for query in [&*RESOLVE_NAME, &*SEARCH_NAMES] {
            if let Ok(documents) = documents(platform_version, query, data) {
                panic!(
                    "a proof of the signed root verified to documents it does not hold: {documents:?}"
                );
            }
        }
    }
});
