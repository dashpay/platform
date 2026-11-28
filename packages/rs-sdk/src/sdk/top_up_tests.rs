use super::{SdkBuilder, SdkInstance};
use crate::platform::transition::asset_lock_submission::{
    as_asset_lock_proof_cl_height_too_low, is_asset_lock_already_consumed,
    submit_with_cl_height_retry,
};
use crate::platform::transition::broadcast::{BroadcastStateTransition, WaitForOutcome};
use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::top_up_identity::{TopUpIdentity, TopUpIdentityWithMetadata};
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::{
    get_identity_balance_and_revision_response::{
        get_identity_balance_and_revision_response_v0::Result as BalanceResult,
        Version as BalanceVersion,
    },
    wait_for_state_transition_result_response::{
        wait_for_state_transition_result_response_v0::Result as WaitResult, Version as WaitVersion,
        WaitForStateTransitionResultResponseV0,
    },
    BroadcastStateTransitionRequest, BroadcastStateTransitionResponse,
    GetIdentityBalanceAndRevisionResponse, WaitForStateTransitionResultResponse,
};
use dapi_grpc::tonic::{metadata::MetadataValue, Status};
use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionOutPointAlreadyConsumedError,
    InvalidAssetLockProofCoreChainHeightError,
};
use dpp::consensus::ConsensusError;
use dpp::dashcore::ecdsa::{EcdsaPublicKey, EcdsaSignature};
use dpp::dashcore::{Network, OutPoint, PrivateKey};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::{Identity, PartialIdentity};
use dpp::key_wallet::bip32::DerivationPath;
use dpp::key_wallet::signer::{Signer, SignerMethod};
use dpp::prelude::{AssetLockProof, Identifier};
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use rs_dapi_client::transport::TransportError;
use rs_dapi_client::{DapiClientError, ExecutionError, ExecutionResponse, RequestSettings};
use std::convert::Infallible;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

struct AssetLockSigner {
    key: PrivateKey,
    calls: AtomicUsize,
}

#[async_trait::async_trait]
impl Signer for AssetLockSigner {
    type Error = Infallible;

    fn supported_methods(&self) -> &[SignerMethod] {
        &[SignerMethod::Digest]
    }

    async fn sign_ecdsa(
        &self,
        _: &DerivationPath,
        digest: [u8; 32],
    ) -> Result<(EcdsaSignature, EcdsaPublicKey), Self::Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok((
            self.key
                .inner
                .sign_recoverable(&digest)
                .signature()
                .unwrap(),
            self.key.inner.public_key(),
        ))
    }

    async fn public_key(&self, _: &DerivationPath) -> Result<EcdsaPublicKey, Self::Error> {
        Ok(self.key.inner.public_key())
    }
}

#[derive(Clone, Copy, Debug)]
enum Api {
    PrivateKey,
    Signer,
    SignerWithMetadata,
}

const VECTOR_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/vectors/test_identity_balance_revision_read"
);

struct Fixture {
    sdk: Sdk,
    identity: Identity,
    signer: AssetLockSigner,
    proof: AssetLockProof,
}

impl Fixture {
    fn new() -> Self {
        let version = PlatformVersion::latest();
        Self {
            sdk: SdkBuilder::new_mock()
                .with_version(version)
                .with_dump_dir(Path::new(VECTOR_DIR))
                .build()
                .unwrap(),
            identity: Identity::create_basic_identity(Identifier::new([1; 32]), version).unwrap(),
            signer: AssetLockSigner {
                key: PrivateKey::from_byte_array(&[7; 32], Network::Regtest).unwrap(),
                calls: AtomicUsize::new(0),
            },
            proof: AssetLockProof::Chain(ChainAssetLockProof {
                core_chain_locked_height: 100,
                out_point: OutPoint::from([9; 36]),
            }),
        }
    }

    fn transition(&self, fee: u16) -> StateTransition {
        IdentityTopUpTransition::try_from_identity_with_private_key(
            &self.identity,
            self.proof.clone(),
            &self.signer.key.inner.to_bytes()[..],
            fee,
            self.sdk.version(),
            None,
        )
        .unwrap()
    }

    async fn submit(&self, api: Api, settings: Option<PutSettings>) -> Result<u64, Error> {
        match api {
            Api::PrivateKey => {
                self.identity
                    .top_up_identity_with_private_key(
                        &self.sdk,
                        self.proof.clone(),
                        &self.signer.key,
                        settings,
                    )
                    .await
            }
            Api::Signer => {
                self.identity
                    .top_up_identity_with_signer(
                        &self.sdk,
                        self.proof.clone(),
                        &DerivationPath::default(),
                        &self.signer,
                        settings,
                    )
                    .await
            }
            Api::SignerWithMetadata => self
                .identity
                .top_up_identity_with_signer_with_metadata(
                    &self.sdk,
                    self.proof.clone(),
                    &DerivationPath::default(),
                    &self.signer,
                    settings,
                )
                .await
                .map(|(balance, _)| balance),
        }
    }

    async fn broadcast_error(
        &self,
        fee: u16,
        status: Status,
        possibly_accepted: bool,
    ) -> BroadcastStateTransitionRequest {
        let request = self
            .transition(fee)
            .broadcast_request_for_state_transition()
            .unwrap();
        let SdkInstance::Mock { dapi, .. } = &self.sdk.inner else {
            unreachable!()
        };
        dapi.lock()
            .await
            .expect(
                &request,
                &Err(ExecutionError {
                    possibly_accepted,
                    inner: DapiClientError::Transport(TransportError::Grpc(status)),
                    address: None,
                    retries: 0,
                }),
            )
            .unwrap();
        request
    }

    async fn snapshot(&self) -> u64 {
        // Reuse a recorded, signed GroveDB balance/revision proof. It proves state,
        // even though this asset lock and top-up have never executed in that state.
        let bytes = include_bytes!("../../tests/vectors/test_identity_balance_revision_read/msg_GetIdentityBalanceAndRevisionRequest_1d1e53ab5e04d9ec5dce4ff9ac048c03122daf7ab2e77108f4bf44af1ad15eae.json");
        let response: Result<Vec<u8>, serde_json::Value> =
            serde_json::from_slice(bytes.split(|byte| *byte == 0).nth(1).unwrap()).unwrap();
        let response: GetIdentityBalanceAndRevisionResponse =
            serde_json::from_slice(&response.unwrap()).unwrap();
        let Some(BalanceVersion::V0(response)) = response.version else {
            panic!("v0 fixture")
        };
        let Some(BalanceResult::Proof(proof)) = response.result else {
            panic!("proof fixture")
        };
        let wait = self
            .transition(0)
            .wait_for_state_transition_result_request()
            .unwrap();
        let SdkInstance::Mock { dapi, .. } = &self.sdk.inner else {
            unreachable!()
        };
        dapi.lock()
            .await
            .expect(
                &wait,
                &Ok(ExecutionResponse {
                    inner: WaitForStateTransitionResultResponse {
                        version: Some(WaitVersion::V0(WaitForStateTransitionResultResponseV0 {
                            result: Some(WaitResult::Proof(proof)),
                            metadata: response.metadata,
                            ..Default::default()
                        })),
                    },
                    address: "http://127.0.0.1".parse().unwrap(),
                    retries: 0,
                }),
            )
            .unwrap();
        let (outcome, _) = self
            .transition(0)
            .wait_for_outcome_with_metadata(&self.sdk, settings())
            .await
            .unwrap();
        assert!(!outcome.is_execution_proved());
        let identity = PartialIdentity::try_from(outcome.into_result()).unwrap();
        assert_eq!(identity.id, self.identity.id());
        identity.balance.unwrap()
    }

    async fn assert_broadcasts(&self, requests: &[BroadcastStateTransitionRequest], api: Api) {
        let SdkInstance::Mock { dapi, .. } = &self.sdk.inner else {
            unreachable!()
        };
        let dapi = dapi.lock().await;
        for request in requests {
            assert_eq!(dapi.request_count(request), 1, "{api:?}");
        }
        if !matches!(api, Api::PrivateKey) {
            assert_eq!(
                self.signer.calls.load(Ordering::SeqCst),
                requests.len(),
                "{api:?}"
            );
        }
    }
}

fn settings() -> Option<PutSettings> {
    Some(PutSettings {
        request_settings: RequestSettings {
            retries: Some(0),
            ..Default::default()
        },
        ..Default::default()
    })
}

fn height_rejection() -> Status {
    consensus_status(InvalidAssetLockProofCoreChainHeightError::new(100, 99).into())
}

fn consensus_status(error: ConsensusError) -> Status {
    let mut status = Status::invalid_argument(error.to_string());
    status.metadata_mut().insert_bin(
        "dash-serialized-consensus-error-bin",
        MetadataValue::from_bytes(
            &error
                .serialize_to_bytes_with_platform_version(PlatformVersion::latest())
                .unwrap(),
        ),
    );
    status
}

// TODO: Cover lost ACK followed by AlreadyExists once the mock supports sequential
// responses for identical requests; it currently stores one fixed response per request.
#[test_case::test_case(Api::PrivateKey; "private_key")]
#[test_case::test_case(Api::Signer; "signer")]
#[test_case::test_case(Api::SignerWithMetadata; "signer_with_metadata")]
#[tokio::test]
async fn should_reject_duplicate_top_up_despite_valid_unchanged_balance_snapshot(api: Api) {
    let fixture = Fixture::new();
    let balance = fixture.snapshot().await;
    let request = fixture
        .broadcast_error(
            0,
            Status::already_exists("tx already exists in cache"),
            true,
        )
        .await;
    // The generic API deliberately still returns the verified snapshot.
    let snapshot: PartialIdentity = fixture
        .transition(0)
        .broadcast_and_wait_for_affected_state(&fixture.sdk, settings())
        .await
        .unwrap();
    assert_eq!(snapshot.balance, Some(balance));
    let result = fixture.submit(api, settings()).await;
    assert!(
        matches!(result, Err(Error::AlreadyExists(_))),
        "{api:?}: {result:?}"
    );
    let SdkInstance::Mock { dapi, .. } = &fixture.sdk.inner else {
        unreachable!()
    };
    assert_eq!(dapi.lock().await.request_count(&request), 2);
}

#[test_case::test_case(Api::PrivateKey; "private_key")]
#[test_case::test_case(Api::Signer; "signer")]
#[test_case::test_case(Api::SignerWithMetadata; "signer_with_metadata")]
#[tokio::test]
async fn should_keep_uncertain_top_up_unconfirmed_despite_valid_balance_snapshot(api: Api) {
    let fixture = Fixture::new();
    fixture.snapshot().await;
    let request = fixture
        .broadcast_error(0, Status::unavailable("response lost"), true)
        .await;
    let result = fixture.submit(api, settings()).await;
    assert!(
        matches!(result, Err(Error::StateTransitionResultUnconfirmed(_))),
        "{api:?}: {result:?}"
    );
    fixture.assert_broadcasts(&[request], api).await;
}

#[test_case::test_case(Api::PrivateKey; "private_key")]
#[test_case::test_case(Api::Signer; "signer")]
#[test_case::test_case(Api::SignerWithMetadata; "signer_with_metadata")]
#[tokio::test(start_paused = true)]
async fn should_retry_top_up_height_lag_then_surface_consumed_outpoint_rejection(api: Api) {
    let fixture = Fixture::new();
    let first = fixture.broadcast_error(0, height_rejection(), false).await;
    let outpoint = OutPoint::from([9; 36]);
    let second = fixture
        .broadcast_error(
            1,
            consensus_status(
                IdentityAssetLockTransactionOutPointAlreadyConsumedError::new(
                    outpoint.txid,
                    outpoint.vout as usize,
                )
                .into(),
            ),
            false,
        )
        .await;
    assert_ne!(first.state_transition, second.state_transition);
    let started = tokio::time::Instant::now();
    let error = fixture.submit(api, settings()).await.unwrap_err();
    assert!(
        is_asset_lock_already_consumed(&error, &outpoint),
        "{api:?}: {error:?}"
    );
    assert_eq!(started.elapsed(), Duration::from_secs(15));
    fixture.assert_broadcasts(&[first, second], api).await;
}

#[test_case::test_case(Api::PrivateKey; "private_key")]
#[test_case::test_case(Api::Signer; "signer")]
#[test_case::test_case(Api::SignerWithMetadata; "signer_with_metadata")]
#[tokio::test(start_paused = true)]
async fn should_bound_direct_and_wallet_owned_top_up_height_retries(api: Api) {
    for wallet_owned in [false, true] {
        let fixture = Fixture::new();
        let mut requests = Vec::new();
        for fee in 0..=14 {
            requests.push(
                fixture
                    .broadcast_error(fee, height_rejection(), false)
                    .await,
            );
        }
        let started = tokio::time::Instant::now();
        let result = if wallet_owned {
            submit_with_cl_height_retry(settings(), |settings| {
                let fixture = &fixture;
                async move {
                    let attempt_started = tokio::time::Instant::now();
                    let result = fixture.submit(api, settings).await;
                    assert_eq!(
                        attempt_started.elapsed(),
                        Duration::ZERO,
                        "the outer helper owns the delay"
                    );
                    result
                }
            })
            .await
        } else {
            fixture.submit(api, settings()).await
        };
        assert!(as_asset_lock_proof_cl_height_too_low(&result.unwrap_err()).is_some());
        assert_eq!(started.elapsed(), Duration::from_secs(210));
        fixture.assert_broadcasts(&requests, api).await;
    }
}

#[test_case::test_case(Api::PrivateKey; "private_key")]
#[test_case::test_case(Api::Signer; "signer")]
#[test_case::test_case(Api::SignerWithMetadata; "signer_with_metadata")]
#[tokio::test(start_paused = true)]
async fn should_not_resign_top_up_after_uncertainty_wrapped_height_rejection(api: Api) {
    let fixture = Fixture::new();
    fixture.snapshot().await;
    let request = fixture.broadcast_error(0, height_rejection(), true).await;
    let started = tokio::time::Instant::now();
    let result = fixture.submit(api, settings()).await;
    assert!(
        matches!(result, Err(Error::StateTransitionResultUnconfirmed(_))),
        "{api:?}: {result:?}"
    );
    assert_eq!(started.elapsed(), Duration::ZERO);
    fixture.assert_broadcasts(&[request], api).await;
}

#[test_case::test_case(Api::PrivateKey; "private_key")]
#[test_case::test_case(Api::Signer; "signer")]
#[test_case::test_case(Api::SignerWithMetadata; "signer_with_metadata")]
#[tokio::test]
async fn should_return_verified_balance_after_acknowledged_top_up(api: Api) {
    let fixture = Fixture::new();
    let balance = fixture.snapshot().await;
    let request = fixture
        .transition(0)
        .broadcast_request_for_state_transition()
        .unwrap();
    fixture
        .sdk
        .mock_dapi()
        .lock()
        .await
        .expect(
            &request,
            &Ok(ExecutionResponse {
                inner: BroadcastStateTransitionResponse::default(),
                address: "http://127.0.0.1".parse().unwrap(),
                retries: 0,
            }),
        )
        .unwrap();
    assert_eq!(fixture.submit(api, settings()).await.unwrap(), balance);
    fixture.assert_broadcasts(&[request], api).await;
}

#[test_case::test_case(Api::PrivateKey; "private_key")]
#[test_case::test_case(Api::Signer; "signer")]
#[test_case::test_case(Api::SignerWithMetadata; "signer_with_metadata")]
#[tokio::test]
async fn should_keep_duplicate_top_up_unconfirmed_when_wait_has_no_valid_proof(api: Api) {
    let fixture = Fixture::new();
    let request = fixture
        .broadcast_error(
            0,
            Status::already_exists("tx already exists in cache"),
            true,
        )
        .await;
    let wait = fixture
        .transition(0)
        .wait_for_state_transition_result_request()
        .unwrap();
    fixture
        .sdk
        .mock_dapi()
        .lock()
        .await
        .expect(
            &wait,
            &Ok(ExecutionResponse {
                inner: WaitForStateTransitionResultResponse::default(),
                address: "http://127.0.0.1".parse().unwrap(),
                retries: 0,
            }),
        )
        .unwrap();
    let result = fixture.submit(api, settings()).await;
    assert!(
        matches!(result, Err(Error::StateTransitionResultUnconfirmed(_))),
        "{api:?}: {result:?}"
    );
    fixture.assert_broadcasts(&[request], api).await;
}
