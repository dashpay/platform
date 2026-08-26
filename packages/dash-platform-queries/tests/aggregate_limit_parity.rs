//! The aggregate verify paths (COUNT / SUM / AVG) must refuse every
//! request limit the server's aggregate dispatchers refuse — before
//! any proof or context-provider machinery runs. See
//! `src/documents/aggregate_limit.rs` for the server counterpart.

use std::sync::Arc;

use dapi_grpc::platform::v0::GetDocumentsResponse;
use dash_context_provider::{ContextProvider, ContextProviderError};
use dash_platform_queries::documents::document_query::DocumentQuery;
use dpp::dashcore::Network;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::query::SelectProjection;
use drive_proof_verifier::{DocumentAverage, DocumentCount, DocumentSum, FromProof};

fn test_contract() -> Arc<DataContract> {
    let platform_version = PlatformVersion::latest();
    Arc::new(
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned(),
    )
}

fn documents_query() -> DocumentQuery {
    DocumentQuery::new(test_contract(), "niceDocument").expect("document type exists")
}

/// Fails the test if proof verification is reached at all.
struct NeverCalledProvider;

impl ContextProvider for NeverCalledProvider {
    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        panic!("request must be rejected before proof verification starts")
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        panic!("request must be rejected before proof verification starts")
    }

    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        panic!("request must be rejected before proof verification starts")
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        panic!("request must be rejected before proof verification starts")
    }
}

/// The aggregate verify paths (COUNT / SUM / AVG) share the same
/// server-side limit cap — drive's aggregate dispatchers refuse
/// over-`max_query_limit` requests with `InvalidLimit` before
/// producing proof bytes — so their verifiers must refuse the
/// same requests before any proof machinery. Exercised through
/// all three `FromProof` entry points, so dropping the shared
/// `aggregate_limit::check_within_server_cap` gate from any one
/// of them fails this test. The panicking provider pins that the
/// rejection precedes all proof machinery, and the asserted
/// message pins that the limit gate (not the missing proof in
/// the default response) is what fired.
#[test]
fn rejects_aggregate_limit_above_server_cap() {
    fn assert_over_cap_rejected<T>(select: SelectProjection, surface: &str)
    where
        T: FromProof<DocumentQuery, Request = DocumentQuery, Response = GetDocumentsResponse>
            + std::fmt::Debug,
    {
        for limit in [101u32, 65_535, u32::MAX] {
            let query = documents_query()
                .with_select(select.clone())
                .with_limit(limit);
            let error = T::maybe_from_proof_with_metadata(
                query,
                GetDocumentsResponse::default(),
                Network::Testnet,
                PlatformVersion::latest(),
                &NeverCalledProvider,
            )
            .expect_err("an over-cap limit on an aggregate verify path must be rejected");
            assert!(
                error
                    .to_string()
                    .contains("exceeds the server's max_query_limit 100"),
                "unexpected error for {surface} limit {limit}: {error}"
            );
        }
    }

    assert_over_cap_rejected::<DocumentCount>(SelectProjection::count_star(), "COUNT");
    assert_over_cap_rejected::<DocumentSum>(SelectProjection::sum("age"), "SUM");
    assert_over_cap_rejected::<DocumentAverage>(SelectProjection::avg("age"), "AVG");
}
