//! In-memory transport fixtures for the real public combined put methods.
use super::{SdkBuilder, SdkInstance};
use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::{
    BroadcastStateTransitionRequest, WaitForStateTransitionResultRequest,
    WaitForStateTransitionResultResponse,
};
use dapi_grpc::tonic::Status;
use dpp::address_funds::AddressWitness;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::BinaryData;
use dpp::prelude::{DataContract, Identifier};
use dpp::state_transition::StateTransition;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive_proof_verifier::types::{IdentityContractNonceFetcher, IdentityNonceFetcher};
use rs_dapi_client::transport::TransportError;
use rs_dapi_client::{DapiClientError, ExecutionError, ExecutionResponse, RequestSettings};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Default)]
pub(crate) struct CountingSigner(pub AtomicUsize);
#[async_trait::async_trait]
impl Signer<IdentityPublicKey> for CountingSigner {
    async fn sign(&self, _: &IdentityPublicKey, _: &[u8]) -> Result<BinaryData, ProtocolError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(vec![7; 65].into())
    }
    async fn sign_create_witness(
        &self,
        _: &IdentityPublicKey,
        _: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        panic!("identity-key operation never requests an address witness")
    }
    fn can_sign_with(&self, _: &IdentityPublicKey) -> bool {
        true
    }
}

pub(crate) struct PutFixture {
    pub sdk: Sdk,
    pub document: Document,
    pub document_type: DocumentType,
    pub contract: DataContract,
    pub key: IdentityPublicKey,
    pub signer: CountingSigner,
}

impl PutFixture {
    pub async fn new() -> Self {
        let version = PlatformVersion::latest();
        let mut sdk = SdkBuilder::new_mock()
            .with_version(version)
            .build()
            .unwrap();
        let contract = load_system_data_contract(SystemDataContract::DPNS, version).unwrap();
        let document_type = contract.document_type_cloned_for_name("preorder").unwrap();
        let mut document = document_type.random_document(Some(1), version).unwrap();
        document.set_revision(Some(2));
        let key =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key(1, Some(1), version)
                .unwrap()
                .0;
        for owner in [
            document.owner_id(),
            contract.owner_id(),
            Identifier::new([9; 32]),
        ] {
            sdk.mock()
                .expect_fetch(owner, Some(IdentityNonceFetcher(0)))
                .await
                .unwrap();
            sdk.mock()
                .expect_fetch(
                    (owner, contract.id()),
                    Some(IdentityContractNonceFetcher(0)),
                )
                .await
                .unwrap();
        }
        Self {
            sdk,
            document,
            document_type,
            contract,
            key,
            signer: CountingSigner::default(),
        }
    }

    pub fn settings() -> Option<PutSettings> {
        Some(PutSettings {
            request_settings: RequestSettings {
                retries: Some(0),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    /// Preparation is local; use a fresh nonce cache before invoking the public method so
    /// the independently repeated preparation must produce exactly the same bytes.
    pub async fn expect_submission(
        &mut self,
        transition: &StateTransition,
        status: Status,
    ) -> (
        BroadcastStateTransitionRequest,
        WaitForStateTransitionResultRequest,
    ) {
        self.sdk = Self::new().await.sdk;
        self.signer.0.store(0, Ordering::SeqCst);
        let broadcast = transition.broadcast_request_for_state_transition().unwrap();
        let wait = transition
            .wait_for_state_transition_result_request()
            .unwrap();
        let SdkInstance::Mock { dapi, .. } = &self.sdk.inner else {
            unreachable!()
        };
        let mut dapi = dapi.lock().await;
        dapi.expect(
            &broadcast,
            &Err(ExecutionError {
                inner: DapiClientError::Transport(TransportError::Grpc(status)),
                address: None,
                retries: 0,
            }),
        )
        .unwrap();
        // An empty response is never valid proof. Reaching it must fail closed,
        // while its request hash proves we waited for the original signed bytes.
        dapi.expect(
            &wait,
            &Ok(ExecutionResponse {
                inner: WaitForStateTransitionResultResponse::default(),
                retries: 0,
                address: "http://127.0.0.1".parse().unwrap(),
            }),
        )
        .unwrap();
        (broadcast, wait)
    }

    pub async fn assert_calls(
        &self,
        requests: &(
            BroadcastStateTransitionRequest,
            WaitForStateTransitionResultRequest,
        ),
        expected_waits: usize,
        result: Result<(), Error>,
    ) {
        assert!(
            result.is_err(),
            "an acknowledgement or invalid proof cannot become success"
        );
        let SdkInstance::Mock { dapi, .. } = &self.sdk.inner else {
            unreachable!()
        };
        let dapi = dapi.lock().await;
        assert_eq!(
            dapi.request_count(&requests.0),
            1,
            "submit the originally signed bytes exactly once: {result:?}"
        );
        assert_eq!(
            dapi.request_count(&requests.1),
            expected_waits,
            "poll the original transition hash only when appropriate"
        );
        assert_eq!(
            self.signer.0.load(Ordering::SeqCst),
            1,
            "the public operation must sign once"
        );
        if expected_waits == 1 {
            assert!(
                !matches!(result, Err(Error::AlreadyExists(_))),
                "return the failed proof, not the ambiguous relay acknowledgement"
            );
        }
    }
}
