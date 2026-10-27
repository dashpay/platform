use super::super::{Sdk, SdkBuilder, SdkInstance};
use crate::platform::{DocumentHistoryQuery, Fetch};
use dapi_grpc::platform::v0::get_document_history_request::get_document_history_request_v1::Filter;
use dapi_grpc::platform::v0::get_document_history_request::Version as RequestVersion;
use dapi_grpc::platform::v0::get_document_history_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetDocumentHistoryRequest, GetDocumentHistoryResponse};
use dapi_grpc::platform::VersionedGrpcResponse;
use dapi_grpc::tonic::{Code, Status};
use dash_context_provider::{ContextProvider, ContextProviderError};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::document::DocumentV0Getters;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted;
use dpp::version::PlatformVersion;
use drive::query::document_history_drive_query::DocumentHistoryFilter;
use drive_proof_verifier::types::{
    DocumentHistory, DocumentHistoryProofInfo, DocumentHistoryState,
};
use drive_proof_verifier::FromProof;
use rs_dapi_client::transport::TransportError;
use rs_dapi_client::{AddressList, DapiClientError, ExecutionError, ExecutionResponse};
use serde::Deserialize;
use std::sync::Arc;
use test_case::test_case;

#[derive(Deserialize)]
struct Fixture {
    contract: String,
    quorum_public_key: String,
    request: GetDocumentHistoryRequest,
    response: GetDocumentHistoryResponse,
}

impl Fixture {
    fn load(cursor: bool) -> Self {
        let bytes = if cursor {
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/vectors/document_history/pv15_cursor.json"
            ))
        } else {
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/vectors/document_history/pv15_revision.json"
            ))
        };
        serde_json::from_str(bytes).expect("signed history fixture")
    }

    fn provider(&self) -> Provider {
        Provider {
            contract: Arc::new(
                DataContract::versioned_deserialize_trusted(
                    &hex::decode(&self.contract).expect("contract hex"),
                    false,
                    PlatformVersion::latest(),
                )
                .expect("fixture contract"),
            ),
            key: hex::decode(&self.quorum_public_key)
                .expect("quorum key hex")
                .try_into()
                .expect("48-byte key"),
        }
    }

    fn query(&self) -> DocumentHistoryQuery {
        let Some(RequestVersion::V1(request)) = &self.request.version else {
            panic!("versioned history request");
        };
        DocumentHistoryQuery {
            data_contract_id: Identifier::from_bytes(&request.data_contract_id).unwrap(),
            document_type_name: request.document_type_name.clone(),
            document_id: Identifier::from_bytes(&request.document_id).unwrap(),
            filter: match request.filter.as_ref().unwrap() {
                Filter::Revision(revision) => DocumentHistoryFilter::Revision(*revision),
                Filter::StartAfter(cursor) => DocumentHistoryFilter::StartAfter {
                    time_ms: cursor.time_ms,
                    revision: cursor.revision,
                },
                _ => panic!("revision or cursor fixture"),
            },
            limit: request.limit,
        }
    }

    fn sdk(&self, network: Network, mock: bool) -> Sdk {
        let builder = if mock {
            SdkBuilder::new_mock()
        } else {
            SdkBuilder::new(AddressList::new())
        };
        let sdk = builder
            .with_network(network)
            .with_context_provider(self.provider())
            .with_time_tolerance(None)
            .with_height_tolerance(Some(1))
            .with_trusted_initial_height(self.response.metadata().unwrap().height)
            .build()
            .expect("fresh unpinned SDK");
        assert!(!sdk.version_pinned);
        assert_eq!(
            sdk.protocol_version_number(),
            if network == Network::Devnet { 14 } else { 13 }
        );
        sdk
    }

    async fn mock_sdk(&self, network: Network) -> Sdk {
        let sdk = self.sdk(network, true);
        let SdkInstance::Mock { dapi, .. } = &sdk.inner else {
            panic!("mock transport");
        };
        dapi.lock()
            .await
            .expect(
                &self.request,
                &Ok(ExecutionResponse {
                    inner: self.response.clone(),
                    ..Default::default()
                }),
            )
            .expect("transport response only");
        sdk
    }

    fn assert_history(&self, history: &DocumentHistory) {
        let expected = if matches!(self.query().filter, DocumentHistoryFilter::Revision(_)) {
            vec![2]
        } else {
            vec![2, 3]
        };
        assert_eq!(
            history
                .entries
                .iter()
                .map(|entry| entry.revision)
                .collect::<Vec<_>>(),
            expected
        );
        for entry in &history.entries {
            assert_eq!(entry.time_ms, 2000);
            assert_eq!(entry.document.revision(), Some(entry.revision));
            assert_eq!(entry.document.id(), self.query().document_id);
        }
        let lifecycle = history.lifecycle.as_ref().expect("PV15 lifecycle proof");
        assert_eq!(lifecycle.state, DocumentHistoryState::Active);
        assert_eq!(lifecycle.remaining_revisions, 3);
    }
}

#[tokio::test]
async fn should_surface_history_version_status_without_response_decoding_or_fallback() {
    for message in [
        "unsupported version for query: document_history, currently supporting versions 0 to 0 on platform protocol 14, given 1",
        "decoding error: could not decode document history query",
    ] {
        let fixture = Fixture::load(false);
        let sdk = fixture.sdk(Network::Devnet, true);
        let SdkInstance::Mock { dapi, .. } = &sdk.inner else { panic!("mock transport") };
        dapi.lock().await.expect(&fixture.request, &Err(ExecutionError {
            inner: DapiClientError::Transport(TransportError::Grpc(Status::invalid_argument(message))),
            address: None, retries: 0,
        })).unwrap();
        let error = DocumentHistory::fetch(&sdk, fixture.query()).await.unwrap_err();
        assert!(matches!(error, crate::Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(status)))
            if status.code() == Code::InvalidArgument && status.message() == message));
        assert_eq!(dapi.lock().await.request_count(&fixture.request), 1);
        assert_eq!(sdk.protocol_version_number(), 14);
    }
}

struct Provider {
    contract: Arc<DataContract>,
    key: [u8; 48],
}

impl ContextProvider for Provider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok((id == &self.contract.id()).then(|| self.contract.clone()))
    }
    fn get_token_configuration(
        &self,
        _: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }
    fn get_quorum_public_key(
        &self,
        _: u32,
        _: [u8; 32],
        _: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        Ok(self.key)
    }
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}

#[test_case(Network::Mainnet, false; "mainnet revision")]
#[test_case(Network::Mainnet, true; "mainnet cursor")]
#[test_case(Network::Testnet, false; "testnet revision")]
#[test_case(Network::Testnet, true; "testnet cursor")]
#[test_case(Network::Regtest, false; "regtest revision")]
#[test_case(Network::Regtest, true; "regtest cursor")]
#[test_case(Network::Devnet, false; "devnet revision")]
#[test_case(Network::Devnet, true; "devnet cursor")]
#[tokio::test]
async fn should_fetch_history_as_the_first_proved_read_from_an_unpinned_seed(
    network: Network,
    cursor: bool,
) {
    let fixture = Fixture::load(cursor);
    let sdk = fixture.mock_sdk(network).await;
    let (history, metadata, _) =
        DocumentHistory::fetch_with_metadata_and_proof(&sdk, fixture.query(), None)
            .await
            .expect("first history read must reach and verify PV15");
    fixture.assert_history(&history.unwrap());
    assert_eq!(metadata.protocol_version, 15);
    assert_eq!(sdk.protocol_version_number(), 15);
}

#[test_case(Network::Mainnet, false; "mainnet revision")]
#[test_case(Network::Mainnet, true; "mainnet cursor")]
#[test_case(Network::Testnet, false; "testnet revision")]
#[test_case(Network::Testnet, true; "testnet cursor")]
#[test_case(Network::Regtest, false; "regtest revision")]
#[test_case(Network::Regtest, true; "regtest cursor")]
#[test_case(Network::Devnet, false; "devnet revision")]
#[test_case(Network::Devnet, true; "devnet cursor")]
#[tokio::test]
async fn should_fetch_history_proof_info_as_the_first_proved_read_from_an_unpinned_seed(
    network: Network,
    cursor: bool,
) {
    let fixture = Fixture::load(cursor);
    let sdk = fixture.mock_sdk(network).await;
    let info = DocumentHistoryProofInfo::fetch(&sdk, fixture.query())
        .await
        .expect("first proof-info read must reach and verify PV15")
        .unwrap();
    fixture.assert_history(&info.history);
    assert_eq!(
        GetDocumentHistoryResponse::from(info.response.clone()),
        fixture.response
    );
    assert_eq!(sdk.protocol_version_number(), 15);
    assert_eq!(
        info.verify(
            fixture.request.clone(),
            network,
            sdk.version(),
            &fixture.provider()
        )
        .unwrap(),
        Some(info.history)
    );
}

#[test]
fn should_verify_signed_history_directly_from_older_client_versions() {
    for cursor in [false, true] {
        let fixture = Fixture::load(cursor);
        for client in [1, 14] {
            let version = PlatformVersion::get(client).unwrap();
            let history = DocumentHistory::maybe_from_proof(
                fixture.request.clone(),
                fixture.response.clone(),
                Network::Testnet,
                version,
                &fixture.provider(),
            )
            .expect("signed producer layout must verify for an older client")
            .unwrap();
            fixture.assert_history(&history);
            let info = DocumentHistoryProofInfo::maybe_from_proof(
                fixture.request.clone(),
                fixture.response.clone(),
                Network::Testnet,
                version,
                &fixture.provider(),
            )
            .expect("proof information must use the signed producer layout")
            .unwrap();
            fixture.assert_history(&info.history);
        }
    }
}

#[test]
fn should_verify_signed_v0_history_from_an_older_producer() {
    let fixture: Fixture = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/vectors/document_history/pv14_time.json"
    )))
    .expect("signed timestamp-keyed history fixture");
    assert!(matches!(
        fixture.request.version,
        Some(RequestVersion::V0(_))
    ));
    assert!(matches!(
        fixture.response.version,
        Some(ResponseVersion::V0(_))
    ));
    assert_eq!(fixture.response.metadata().unwrap().protocol_version, 14);
    let provider = fixture.provider();
    let history = DocumentHistory::maybe_from_proof(
        fixture.request.clone(),
        fixture.response.clone(),
        Network::Testnet,
        PlatformVersion::get(14).unwrap(),
        &provider,
    )
    .expect("the producer's own timestamp-keyed history rules verify")
    .unwrap();
    assert_eq!(history.lifecycle, None);
    assert_eq!(
        history
            .entries
            .iter()
            .map(|entry| (entry.time_ms, entry.revision))
            .collect::<Vec<_>>(),
        vec![(2000, 2), (3000, 3)]
    );
    let verified = DocumentHistory::maybe_from_proof(
        fixture.request,
        fixture.response,
        Network::Testnet,
        PlatformVersion::latest(),
        &provider,
    )
    .expect("a newer client selects the signed producer's v0 history rules")
    .unwrap();
    assert_eq!(verified, history);
}

#[test_case(false; "revision page")]
#[test_case(true; "cursor page")]
fn should_reject_history_protocol_version_tampering(cursor: bool) {
    let fixture = Fixture::load(cursor);
    let provider = fixture.provider();
    let version = PlatformVersion::latest();
    let history = DocumentHistory::maybe_from_proof(
        fixture.request.clone(),
        fixture.response.clone(),
        Network::Testnet,
        version,
        &provider,
    )
    .expect("the original signed history response verifies")
    .unwrap();
    fixture.assert_history(&history);

    for claimed in [version.protocol_version + 1, 14, 0] {
        let mut forged = fixture.response.clone();
        let Some(ResponseVersion::V1(response)) = &mut forged.version else {
            panic!("versioned history response");
        };
        response.metadata.as_mut().unwrap().protocol_version = claimed;
        for (kind, result) in [
            (
                "history",
                DocumentHistory::maybe_from_proof(
                    fixture.request.clone(),
                    forged.clone(),
                    Network::Testnet,
                    version,
                    &provider,
                )
                .map(|_| ()),
            ),
            (
                "proof information",
                DocumentHistoryProofInfo::maybe_from_proof(
                    fixture.request.clone(),
                    forged.clone(),
                    Network::Testnet,
                    version,
                    &provider,
                )
                .map(|_| ()),
            ),
        ] {
            let error = result.expect_err(&format!(
                "{kind} must reject a history response relabeled as protocol {claimed}"
            ));
            if claimed == 0 {
                assert!(
                    matches!(
                        &error,
                        drive_proof_verifier::Error::ResponseDecodeError { error }
                            if error.contains("unsupported proof query protocol version 0")
                    ),
                    "{kind}: {error}"
                );
            } else if claimed == version.protocol_version + 1 {
                assert!(
                    matches!(error, drive_proof_verifier::Error::InvalidSignature { .. }),
                    "{kind}: {error}"
                );
            }
        }
    }
}

#[test_case(false; "network parser")]
#[test_case(true; "mock parser")]
#[tokio::test]
async fn should_select_signed_history_rules_before_ratchet_and_reject_forged_metadata(mock: bool) {
    let fixture = Fixture::load(true);
    let sdk = fixture.sdk(Network::Devnet, mock);
    let direct = DocumentHistory::maybe_from_proof(
        fixture.request.clone(),
        fixture.response.clone(),
        Network::Devnet,
        sdk.version(),
        &fixture.provider(),
    )
    .expect("direct verification selects the signed history layout")
    .unwrap();
    fixture.assert_history(&direct);
    let mut forged = fixture.response.clone();
    let Some(ResponseVersion::V1(response)) = &mut forged.version else {
        panic!("versioned response");
    };
    response.metadata.as_mut().unwrap().time_ms += 1;
    assert!(sdk
        .parse_proof_with_metadata_and_proof::<GetDocumentHistoryRequest, DocumentHistory>(
            fixture.request.clone(),
            forged,
            "get_document_history"
        )
        .await
        .is_err());
    assert_eq!(sdk.protocol_version_number(), 14);
    let (history, metadata, _) = sdk
        .parse_proof_with_metadata_and_proof::<GetDocumentHistoryRequest, DocumentHistoryProofInfo>(
            fixture.request.clone(),
            fixture.response.clone(),
            "get_document_history",
        )
        .await
        .expect("signed history selects PV15 before the ratchet");
    fixture.assert_history(&history.unwrap().history);
    assert_eq!(metadata.protocol_version, 15);
    assert_eq!(sdk.protocol_version_number(), 15);
}
