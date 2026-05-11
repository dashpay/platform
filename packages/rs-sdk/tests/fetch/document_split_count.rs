//! Mock-based integration tests for the SDK [`DocumentSplitCounts`] fetch path.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::common::{mock_data_contract, mock_document_type};
use dash_sdk::{
    platform::{documents::document_split_count_query::DocumentSplitCountQuery, Fetch},
    Sdk,
};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use drive_proof_verifier::DocumentSplitCounts;

#[tokio::test]
async fn test_mock_fetch_document_split_counts_returns_expected() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentSplitCountQuery::new(Arc::new(data_contract), document_type.name(), "a")
        .expect("build DocumentSplitCountQuery");

    let mut counts = BTreeMap::new();
    counts.insert(b"alice".to_vec(), 3u64);
    counts.insert(b"bob".to_vec(), 11u64);
    let expected = DocumentSplitCounts(counts);

    sdk.mock()
        .expect_fetch(query.clone(), Some(expected.clone()))
        .await
        .expect("expectation should be added");

    let retrieved = DocumentSplitCounts::fetch(&sdk, query)
        .await
        .expect("fetch should succeed")
        .expect("split counts should be present");

    assert_eq!(retrieved, expected);
    assert_eq!(retrieved.0.get(b"alice".as_slice()), Some(&3u64));
    assert_eq!(retrieved.0.get(b"bob".as_slice()), Some(&11u64));
}

#[tokio::test]
async fn test_mock_fetch_document_split_counts_empty_map() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentSplitCountQuery::new(Arc::new(data_contract), document_type.name(), "a")
        .expect("build DocumentSplitCountQuery");

    let expected = DocumentSplitCounts(BTreeMap::new());

    sdk.mock()
        .expect_fetch(query.clone(), Some(expected.clone()))
        .await
        .expect("expectation should be added");

    let retrieved = DocumentSplitCounts::fetch(&sdk, query)
        .await
        .expect("fetch should succeed")
        .expect("split counts should be present");

    assert!(retrieved.0.is_empty());
}

#[tokio::test]
async fn test_mock_fetch_document_split_counts_not_found() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentSplitCountQuery::new(Arc::new(data_contract), document_type.name(), "a")
        .expect("build DocumentSplitCountQuery");

    sdk.mock()
        .expect_fetch(query.clone(), None as Option<DocumentSplitCounts>)
        .await
        .expect("expectation should be added");

    let retrieved = DocumentSplitCounts::fetch(&sdk, query)
        .await
        .expect("fetch should succeed");

    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_generic_fromproof_for_drive_query_returns_error_not_empty_map() {
    // Regression: the older `FromProof<DriveDocumentQuery> for DocumentSplitCounts`
    // silently returned `Some(DocumentSplitCounts(BTreeMap::new()))` because the
    // split-property name isn't carried by `DriveDocumentQuery`. After the fix
    // the generic impl returns an explicit error so callers can't get silent
    // empty results — only the SDK-side `Fetch` impl on `DocumentSplitCountQuery`
    // (which threads `split_property`) should succeed.
    use dash_context_provider::{ContextProvider, ContextProviderError};
    use dash_sdk::dpp::dashcore::Network;
    use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dash_sdk::dpp::version::PlatformVersion;
    use dash_sdk::platform::proto::GetDocumentsSplitCountResponse;
    use dash_sdk::platform::DriveDocumentQuery;
    use drive_proof_verifier::{DocumentSplitCounts, FromProof};

    struct NoopProvider;
    impl ContextProvider for NoopProvider {
        fn get_data_contract(
            &self,
            _id: &dash_sdk::dpp::prelude::Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<
            Option<std::sync::Arc<dash_sdk::dpp::prelude::DataContract>>,
            ContextProviderError,
        > {
            Ok(None)
        }
        fn get_token_configuration(
            &self,
            _token_id: &dash_sdk::dpp::prelude::Identifier,
        ) -> Result<Option<dash_sdk::dpp::data_contract::TokenConfiguration>, ContextProviderError>
        {
            Ok(None)
        }
        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            Ok([0u8; 48])
        }
        fn get_platform_activation_height(
            &self,
        ) -> Result<dash_sdk::dpp::prelude::CoreBlockHeight, ContextProviderError> {
            Ok(0)
        }
    }

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let drive_query = DriveDocumentQuery {
        contract: &data_contract,
        document_type: data_contract
            .document_type_for_name(document_type.name())
            .unwrap(),
        internal_clauses: Default::default(),
        offset: None,
        limit: None,
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
    };

    let response = GetDocumentsSplitCountResponse { version: None };
    let provider = NoopProvider;

    let result =
        <DocumentSplitCounts as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
            drive_query,
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            &provider,
        );

    let err = result.expect_err(
        "generic FromProof<DriveDocumentQuery> for DocumentSplitCounts must error \
         (split-property unknown) — see fix preventing silent empty maps under prove=true",
    );
    let msg = format!("{}", err);
    assert!(
        msg.contains("split-property"),
        "error should mention the missing split-property contract: {}",
        msg
    );
}
