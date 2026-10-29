//! Exercise the real SDK -> identity wallet -> FFI error conversion without network I/O.

use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use dash_sdk::dapi_client::mock::MockResult;
use dash_sdk::dapi_client::transport::{TransportError, TransportRequest};
use dash_sdk::dapi_client::{DapiClientError, DumpData, ExecutionError, ExecutionResponse};
use dash_sdk::dapi_grpc::platform::v0::wait_for_state_transition_result_response::{
    wait_for_state_transition_result_response_v0::Result as WaitResult, Version as WaitVersion,
    WaitForStateTransitionResultResponseV0,
};
use dash_sdk::dapi_grpc::platform::v0::{
    BroadcastStateTransitionRequest, BroadcastStateTransitionResponse, Proof, ResponseMetadata,
    StateTransitionBroadcastError, WaitForStateTransitionResultRequest,
    WaitForStateTransitionResultResponse,
};
use dash_sdk::dapi_grpc::tonic::metadata::MetadataValue;
use dash_sdk::dapi_grpc::tonic::Status;
use dash_sdk::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::query_types::{AddressInfo, AddressInfos};
use dash_sdk::{Error as SdkError, RequestSettings, Sdk, SdkBuilder};
use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::consensus::state::address_funds::AddressInvalidNonceError;
use dpp::consensus::ConsensusError;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityV0;
use dpp::platform_value::BinaryData;
use dpp::prelude::{Identifier, Identity};
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::state_transition::identity_topup_from_addresses_transition::methods::IdentityTopUpFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use platform_wallet::error::promote_address_nonce_error;
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::{PlatformWallet, PlatformWalletManager};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const ADDRESS: PlatformAddress = PlatformAddress::P2pkh([7; 20]);
const AMOUNT: u64 = 100_000_000;
const OLD_BALANCE: u64 = 123_000;

#[derive(Debug, Default)]
struct AddressSigner(AtomicUsize);

#[async_trait::async_trait]
impl Signer<PlatformAddress> for AddressSigner {
    async fn sign(&self, _: &PlatformAddress, _: &[u8]) -> Result<BinaryData, ProtocolError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(vec![7; 65].into())
    }

    async fn sign_create_witness(
        &self,
        key: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Ok(AddressWitness::P2pkh {
            signature: self.sign(key, data).await?,
        })
    }

    fn can_sign_with(&self, _: &PlatformAddress) -> bool {
        true
    }
}

struct NoopEvents;
impl EventHandler for NoopEvents {}
impl PlatformEventHandler for NoopEvents {}

fn nonce_error_bytes() -> Vec<u8> {
    ConsensusError::from(AddressInvalidNonceError::new(ADDRESS, 1, 2))
        .serialize_to_bytes_with_platform_version(PlatformVersion::latest())
        .unwrap()
}

fn wait_response(result: WaitResult) -> WaitForStateTransitionResultResponse {
    WaitForStateTransitionResultResponse {
        version: Some(WaitVersion::V0(WaitForStateTransitionResultResponseV0 {
            result: Some(result),
            metadata: Some(ResponseMetadata {
                protocol_version: PlatformVersion::latest().protocol_version,
                ..Default::default()
            }),
        })),
    }
}

fn save<R: TransportRequest>(directory: &tempfile::TempDir, request: &R, response: &MockResult<R>) {
    let dump = DumpData::new(request, response);
    dump.save(&directory.path().join(dump.filename().unwrap()))
        .unwrap();
}

fn response<R>(inner: R) -> ExecutionResponse<R> {
    ExecutionResponse {
        inner,
        retries: 0,
        address: "http://127.0.0.1".parse().unwrap(),
    }
}

fn transport_error(status: Status) -> ExecutionError<DapiClientError> {
    ExecutionError {
        inner: DapiClientError::Transport(TransportError::Grpc(status)),
        address: None,
        retries: 0,
    }
}

struct Fixture {
    wallet: Arc<PlatformWallet>,
    sdk: Sdk,
    identity_id: Identifier,
    broadcast: BroadcastStateTransitionRequest,
    wait: WaitForStateTransitionResultRequest,
    signer: AddressSigner,
}

impl Fixture {
    async fn new(
        broadcast: Result<BroadcastStateTransitionResponse, Status>,
        wait: Result<WaitForStateTransitionResultResponse, Status>,
        funded: bool,
    ) -> Self {
        let mut sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .with_time_tolerance(None)
            .with_settings(RequestSettings {
                retries: Some(0),
                ..Default::default()
            })
            .build()
            .unwrap();
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::from([9; 32]),
            public_keys: Default::default(),
            balance: OLD_BALANCE,
            revision: 0,
        });
        let transition = IdentityTopUpFromAddressesTransition::try_from_inputs_with_signer(
            &identity,
            BTreeMap::from([(ADDRESS, (1, AMOUNT))]),
            &AddressSigner::default(),
            0,
            sdk.version(),
            None,
        )
        .await
        .unwrap();
        let broadcast_request = transition.broadcast_request_for_state_transition().unwrap();
        let wait_request = transition
            .wait_for_state_transition_result_request()
            .unwrap();
        let directory = tempfile::tempdir().unwrap();
        save(
            &directory,
            &broadcast_request,
            &broadcast.map(response).map_err(transport_error),
        );
        save(
            &directory,
            &wait_request,
            &wait.map(response).map_err(transport_error),
        );
        sdk.mock().load_expectations_sync(directory.path()).unwrap();
        sdk.mock()
            .expect_fetch_many::<PlatformAddress, AddressInfo, _, AddressInfos>(
                BTreeSet::from([ADDRESS]),
                Some(AddressInfos::from_iter([(
                    ADDRESS,
                    Some(AddressInfo {
                        address: ADDRESS,
                        nonce: 0,
                        balance: if funded { AMOUNT } else { 0 },
                    }),
                )])),
            )
            .await
            .unwrap();
        let manager = PlatformWalletManager::new(
            Arc::new(sdk.clone()),
            Arc::new(NoPlatformPersistence),
            Arc::new(NoopEvents),
        );
        let wallet = manager
            .create_wallet_from_seed_bytes(
                key_wallet::Network::Testnet,
                &[42; 64],
                WalletAccountCreationOptions::None,
                Some(0),
            )
            .await
            .unwrap();
        wallet
            .wallet_manager()
            .write()
            .await
            .get_wallet_info_mut(&wallet.wallet_id())
            .unwrap()
            .identity_manager
            .add_identity(identity.clone(), 0, wallet.wallet_id(), wallet.persister())
            .unwrap();
        Self {
            wallet,
            sdk,
            identity_id: identity.id(),
            broadcast: broadcast_request,
            wait: wait_request,
            signer: AddressSigner::default(),
        }
    }

    async fn check(
        mut self,
        code: PlatformWalletFFIResultCode,
        broadcasts: usize,
        waits: usize,
    ) -> String {
        let error = self
            .wallet
            .top_up_from_addresses(
                &self.identity_id,
                BTreeMap::from([(ADDRESS, AMOUNT)]),
                &self.signer,
                Some(PutSettings {
                    request_settings: RequestSettings {
                        retries: Some(0),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            )
            .await
            .expect_err("a refused or unverified top-up must never succeed");
        // This is the same blanket conversion used by the production FFI entry point.
        let result = PlatformWalletFFIResult::from(error);
        let message = unsafe { CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(result.code, code, "{message}");
        if code == PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed {
            assert_eq!(
                result.consensus_code, 0,
                "a later refusal is not a safe-retry verdict"
            );
        }
        assert_eq!(
            self.sdk.mock().request_count(&self.broadcast).await,
            broadcasts
        );
        assert_eq!(self.sdk.mock().request_count(&self.wait).await, waits);
        assert_eq!(
            self.signer.0.load(Ordering::SeqCst),
            broadcasts,
            "never rebuild/re-sign after ambiguous submission"
        );
        assert_eq!(
            self.wallet
                .wallet_manager()
                .read()
                .await
                .get_wallet_info(&self.wallet.wallet_id())
                .unwrap()
                .identity_manager
                .identity(&self.identity_id)
                .unwrap()
                .identity
                .balance(),
            OLD_BALANCE
        );
        message
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn should_keep_acknowledged_and_lost_ack_top_ups_unconfirmed_after_invalid_proof() {
    for broadcast in [
        Ok(BroadcastStateTransitionResponse {}),
        Err(Status::unavailable("lost acknowledgement")),
    ] {
        let fixture = Fixture::new(
            broadcast,
            Ok(wait_response(WaitResult::Proof(Proof::default()))),
            true,
        )
        .await;
        let message = fixture
            .check(
                PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
                1,
                1,
            )
            .await;
        assert!(message.contains("unconfirmed"));
        assert!(message.contains("Proof verification error"), "{message}");
        assert!(!message.contains("inputs stay unspendable"));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn should_keep_top_up_unconfirmed_after_wait_transport_failure() {
    for broadcast in [
        Ok(BroadcastStateTransitionResponse {}),
        Err(Status::deadline_exceeded("lost acknowledgement")),
    ] {
        Fixture::new(
            broadcast,
            Err(Status::deadline_exceeded("result wait timed out")),
            true,
        )
        .await
        .check(
            PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
            1,
            1,
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn should_not_promote_later_nonce_rejection_after_possible_acceptance() {
    Fixture::new(
        Err(Status::unavailable("lost acknowledgement")),
        Ok(wait_response(WaitResult::Error(
            StateTransitionBroadcastError {
                code: 40603,
                message: "address nonce mismatch".into(),
                data: nonce_error_bytes(),
            },
        ))),
        true,
    )
    .await
    .check(
        PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
        1,
        1,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn should_preserve_definite_broadcast_refusal_without_waiting() {
    Fixture::new(
        Err(Status::invalid_argument("invalid transition")),
        Ok(WaitForStateTransitionResultResponse::default()),
        true,
    )
    .await
    .check(PlatformWalletFFIResultCode::ErrorUnknown, 1, 0)
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn should_preserve_preflight_failure_without_dispatching() {
    Fixture::new(
        Ok(BroadcastStateTransitionResponse {}),
        Ok(WaitForStateTransitionResultResponse::default()),
        false,
    )
    .await
    .check(PlatformWalletFFIResultCode::ErrorUnknown, 0, 0)
    .await;
}

#[test]
fn should_preserve_definite_nonce_refusal_through_sdk_wallet_and_ffi_conversion() {
    // MockableStatus drops metadata when serializing transport fixtures. Feed
    // the real status directly through the same conversion/promotion functions.
    let mut status = Status::invalid_argument("address nonce mismatch");
    status.metadata_mut().insert_bin(
        "dash-serialized-consensus-error-bin",
        MetadataValue::from_bytes(&nonce_error_bytes()),
    );
    let error = SdkError::from(DapiClientError::Transport(TransportError::Grpc(status)));
    let wallet_error = promote_address_nonce_error(&error)
        .expect("a definite nonce refusal retains its typed wallet error");
    let result = PlatformWalletFFIResult::from(wallet_error);
    assert_eq!(
        result.code,
        PlatformWalletFFIResultCode::ErrorAddressNonceMismatch
    );
    let message = unsafe { CStr::from_ptr(result.message) }.to_string_lossy();
    assert!(message.contains("submitted nonce 1"));
    assert!(message.contains("Platform expected 2"));
}
