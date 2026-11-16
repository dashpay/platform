use super::*;
use crate::platform_wallet_ffi_result_free;
use std::sync::Arc;

use dash_sdk::dapi_client::mock::MockResult;
use dash_sdk::dapi_client::transport::{TransportError, TransportRequest};
use dash_sdk::dapi_client::{DapiClientError, DumpData, ExecutionError, ExecutionResponse};
use dash_sdk::dapi_grpc::platform::v0::BroadcastStateTransitionResponse;
use dash_sdk::dapi_grpc::tonic::Status;
use dash_sdk::platform::documents::document_history_query::{
    DocumentHistoryFilter, DocumentHistoryQuery,
};
use dash_sdk::platform::documents::transitions::DocumentEraseTransitionBuilder;
use dash_sdk::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use dash_sdk::query_types::{
    DocumentHistory, DocumentHistoryLifecycle, DocumentHistoryState, IdentityContractNonceFetcher,
};
use dash_sdk::{RequestSettings, SdkBuilder};
use dpp::address_funds::AddressWitness;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::factory::DataContractFactory;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, IdentityV0, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{platform_value, BinaryData};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::PlatformWalletManager;
use tokio::task::block_in_place;

#[derive(Debug)]
struct Signing(bool);
#[async_trait::async_trait]
impl Signer<IdentityPublicKey> for Signing {
    async fn sign(&self, _: &IdentityPublicKey, _: &[u8]) -> Result<BinaryData, ProtocolError> {
        Ok(vec![7; 65].into())
    }
    async fn sign_create_witness(
        &self,
        _: &IdentityPublicKey,
        _: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        unreachable!()
    }
    fn can_sign_with(&self, _: &IdentityPublicKey) -> bool {
        self.0
    }
}
struct Events;
impl EventHandler for Events {}
impl PlatformEventHandler for Events {}

fn save<R: TransportRequest>(directory: &tempfile::TempDir, request: &R, response: &MockResult<R>) {
    let dump = DumpData::new(request, response);
    dump.save(&directory.path().join(dump.filename().unwrap()))
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn should_preserve_signer_unavailable_and_unknown_submission_through_the_erase_export() {
    let version = PlatformVersion::latest();
    let contract = Arc::new(
        DataContractFactory::new(version.protocol_version)
            .unwrap()
            .create_with_value_config(
                Identifier::from([1; 32]),
                1,
                platform_value!({
                    "note": { "type": "object", "documentsKeepHistory": true, "canBeErased": true,
                        "properties": {"message": {"type": "string", "maxLength": 64, "position": 0}}, "additionalProperties": false }
                }),
                None,
                None,
            )
            .unwrap()
            .data_contract_owned(),
    );
    let owner = Identifier::from([3; 32]);
    let document = Identifier::from([2; 32]);
    let key: IdentityPublicKey = IdentityPublicKeyV0 {
        id: 1,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::HIGH,
        key_type: KeyType::ECDSA_SECP256K1,
        data: vec![7; 33].into(),
        ..Default::default()
    }
    .into();
    for available in [false, true] {
        let build_sdk = || {
            SdkBuilder::new_mock()
                .with_version(version)
                .with_time_tolerance(None)
                .with_settings(RequestSettings {
                    retries: Some(0),
                    ..Default::default()
                })
                .build()
                .unwrap()
        };
        let mut sdk = build_sdk();
        sdk.mock()
            .expect_fetch(contract.id(), Some(contract.as_ref().clone()))
            .await
            .unwrap();
        sdk.mock()
            .expect_fetch(
                DocumentHistoryQuery {
                    data_contract_id: contract.id(),
                    document_type_name: "note".into(),
                    document_id: document,
                    filter: DocumentHistoryFilter::StartAtTime(0),
                    limit: Some(1),
                },
                Some(DocumentHistory {
                    entries: vec![],
                    lifecycle: Some(DocumentHistoryLifecycle {
                        state: DocumentHistoryState::Erasing,
                        remaining_revisions: 3,
                        times: Default::default(),
                    }),
                }),
            )
            .await
            .unwrap();
        sdk.mock()
            .expect_fetch::<IdentityContractNonceFetcher, _>((owner, contract.id()), None)
            .await
            .unwrap();
        if available {
            let mut signing_sdk = build_sdk();
            signing_sdk
                .mock()
                .expect_fetch::<IdentityContractNonceFetcher, _>((owner, contract.id()), None)
                .await
                .unwrap();
            let transition = DocumentEraseTransitionBuilder::new(
                contract.clone(),
                "note".into(),
                document,
                owner,
            )
            .sign(&signing_sdk, &key, &Signing(true), version)
            .await
            .unwrap();
            let broadcast = transition.broadcast_request_for_state_transition().unwrap();
            let wait = transition
                .wait_for_state_transition_result_request()
                .unwrap();
            let directory = tempfile::tempdir().unwrap();
            save(
                &directory,
                &broadcast,
                &Ok(ExecutionResponse {
                    inner: BroadcastStateTransitionResponse {},
                    retries: 0,
                    address: "http://127.0.0.1".parse().unwrap(),
                }),
            );
            let error: MockResult<
                dash_sdk::dapi_grpc::platform::v0::WaitForStateTransitionResultRequest,
            > = Err(ExecutionError {
                inner: DapiClientError::Transport(TransportError::Grpc(Status::unavailable(
                    "closing stream unavailable",
                ))),
                address: None,
                retries: 0,
                possibly_accepted: false,
            });
            save(&directory, &wait, &error);
            sdk.mock().load_expectations_sync(directory.path()).unwrap();
        }
        let manager = PlatformWalletManager::new(
            Arc::new(sdk),
            Arc::new(NoPlatformPersistence),
            Arc::new(Events),
        );
        let wallet = manager
            .create_wallet_from_seed_bytes(
                key_wallet::Network::Testnet,
                &[7; 64],
                WalletAccountCreationOptions::None,
                Some(0),
            )
            .await
            .unwrap();
        {
            let mut state = wallet.state_mut().await.unwrap();
            state
                .identity_manager
                .add_identity(
                    IdentityV0 {
                        id: owner,
                        public_keys: [(1, key.clone())].into(),
                        balance: 100_000_000,
                        revision: 0,
                    }
                    .into(),
                    0,
                    wallet.wallet_id(),
                    wallet.persister(),
                )
                .unwrap();
        }
        let handle = PLATFORM_WALLET_STORAGE.insert(wallet);
        let signer = Box::new(VTableSigner::from_native(Arc::new(Signing(available))));
        let name = CString::new("note").unwrap();
        let mut out = DocumentEraseOutcomeFFI::default();
        let mut result = block_in_place(|| unsafe {
            platform_wallet_document_erase_and_observe(
                handle,
                owner.as_bytes().as_ptr(),
                contract.id().as_bytes().as_ptr(),
                name.as_ptr(),
                document.as_bytes().as_ptr(),
                Box::as_ref(&signer) as *const VTableSigner as *mut SignerHandle,
                &mut out,
            )
        });
        assert_eq!(
            result.code as u32,
            if available { 20 } else { 31 },
            "{}",
            if result.message.is_null() {
                "No error message".into()
            } else {
                unsafe { CStr::from_ptr(result.message) }.to_string_lossy()
            }
        );
        assert_eq!(out.kind, DocumentEraseOutcomeKindFFI::Unset);
        unsafe {
            platform_wallet_ffi_result_free(&mut result);
        }
        PLATFORM_WALLET_STORAGE.remove(handle);
    }
}
