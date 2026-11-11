//! Top-up of an identity the wallet does not manage, against mock DAPI
//! expectations.
//!
//! What is real and what is modelled:
//! - The identity fetch, the address-nonce lookup, the transition build,
//!   its signing and its broadcast are real, answered by mock expectations.
//!   The broadcast expectation is keyed by the exact bytes of the transition
//!   built from the identity Platform returned.
//! - The mock SDK cannot answer the result wait with a verifiable proof, so
//!   no scenario here reaches the success branch (and so the reconciliation
//!   of the spent addresses, which only runs on success). The submitted
//!   top-up ends `StateTransitionResultUnconfirmed`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use dash_sdk::dapi_client::mock::MockResult;
use dash_sdk::dapi_client::transport::TransportRequest;
use dash_sdk::dapi_client::{DumpData, ExecutionResponse};
use dash_sdk::dapi_grpc::platform::v0::wait_for_state_transition_result_response::{
    wait_for_state_transition_result_response_v0::Result as WaitResult, Version as WaitVersion,
    WaitForStateTransitionResultResponseV0,
};
use dash_sdk::dapi_grpc::platform::v0::{
    BroadcastStateTransitionRequest, BroadcastStateTransitionResponse, Proof, ResponseMetadata,
    WaitForStateTransitionResultRequest, WaitForStateTransitionResultResponse,
};
use dash_sdk::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::query_types::{AddressInfo, AddressInfos};
use dash_sdk::{RequestSettings, Sdk, SdkBuilder};
use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityV0};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::state_transition::identity_topup_from_addresses_transition::methods::IdentityTopUpFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;

use crate::error::PlatformWalletError;
use crate::events::{EventHandler, PlatformEventHandler};
use crate::wallet::persister::NoPlatformPersistence;
use crate::{PlatformWallet, PlatformWalletManager};

const ADDRESS: PlatformAddress = PlatformAddress::P2pkh([7; 20]);
const AMOUNT: u64 = 100_000_000;
const BALANCE: u64 = 123_000;
/// An identity the wallet manages, unrelated to the one topped up.
const OWN_IDENTITY: [u8; 32] = [3; 32];

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

fn identity(id: [u8; 32]) -> Identity {
    Identity::V0(IdentityV0 {
        id: Identifier::from(id),
        public_keys: Default::default(),
        balance: BALANCE,
        revision: 0,
    })
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

/// A wallet managing [`OWN_IDENTITY`], with Platform knowing `on_platform`
/// (or nothing) under `target`, and a funded [`ADDRESS`].
struct Fixture {
    wallet: Arc<PlatformWallet>,
    sdk: Sdk,
    target: Identifier,
    broadcast: BroadcastStateTransitionRequest,
    wait: WaitForStateTransitionResultRequest,
    signer: AddressSigner,
}

impl Fixture {
    async fn new(on_platform: bool) -> Self {
        let mut sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .with_time_tolerance(None)
            .with_settings(RequestSettings {
                retries: Some(0),
                ..Default::default()
            })
            .build()
            .unwrap();
        let target = identity([9; 32]);
        let transition = IdentityTopUpFromAddressesTransition::try_from_inputs_with_signer(
            &target,
            BTreeMap::from([(ADDRESS, (1, AMOUNT))]),
            &AddressSigner::default(),
            0,
            sdk.version(),
            None,
        )
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
            &Ok(response(BroadcastStateTransitionResponse {})),
        );
        // A proof the mock cannot verify: the submitted top-up stays unconfirmed.
        save(
            &directory,
            &wait,
            &Ok(response(WaitForStateTransitionResultResponse {
                version: Some(WaitVersion::V0(WaitForStateTransitionResultResponseV0 {
                    result: Some(WaitResult::Proof(Proof::default())),
                    metadata: Some(ResponseMetadata {
                        protocol_version: PlatformVersion::latest().protocol_version,
                        ..Default::default()
                    }),
                })),
            })),
        );
        sdk.mock().load_expectations_sync(directory.path()).unwrap();
        sdk.mock()
            .expect_fetch(target.id(), on_platform.then(|| target.clone()))
            .await
            .unwrap();
        sdk.mock()
            .expect_fetch_many::<PlatformAddress, AddressInfo, _, AddressInfos>(
                BTreeSet::from([ADDRESS]),
                Some(AddressInfos::from_iter([(
                    ADDRESS,
                    Some(AddressInfo {
                        address: ADDRESS,
                        nonce: 0,
                        balance: AMOUNT,
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
            .add_identity(
                identity(OWN_IDENTITY),
                0,
                wallet.wallet_id(),
                wallet.persister(),
            )
            .unwrap();
        Self {
            wallet,
            sdk,
            target: target.id(),
            broadcast,
            wait,
            signer: AddressSigner::default(),
        }
    }

    async fn top_up(&self) -> Result<u64, PlatformWalletError> {
        self.wallet
            .top_up_external_identity_from_addresses(
                &self.target,
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
    }

    async fn managed_identity_ids(&self) -> Vec<Identifier> {
        self.wallet
            .wallet_manager()
            .read()
            .await
            .get_wallet_info(&self.wallet.wallet_id())
            .unwrap()
            .identity_manager
            .identity_ids()
    }

    async fn submissions(&mut self) -> (usize, usize, usize) {
        let broadcasts = self.sdk.mock().request_count(&self.broadcast).await;
        let waits = self.sdk.mock().request_count(&self.wait).await;
        (broadcasts, waits, self.signer.0.load(Ordering::SeqCst))
    }
}

/// The top-up of an identity the wallet does not manage is built from the
/// identity Platform returned, signed once and submitted, instead of being
/// refused for not being in the identity manager.
#[tokio::test(flavor = "multi_thread")]
async fn should_submit_a_top_up_for_an_identity_the_wallet_does_not_manage() {
    let mut fixture = Fixture::new(true).await;

    let error = fixture
        .top_up()
        .await
        .expect_err("the mock cannot prove the result");

    assert!(
        matches!(
            error,
            PlatformWalletError::Sdk(dash_sdk::Error::StateTransitionResultUnconfirmed(_))
        ),
        "the top-up must be submitted, got {error:?}"
    );
    assert_eq!(fixture.submissions().await, (1, 1, 1));
}

/// Topping up someone else's identity never records it as the wallet's own.
#[tokio::test(flavor = "multi_thread")]
async fn should_leave_the_identity_manager_unchanged() {
    let fixture = Fixture::new(true).await;
    let before = fixture.managed_identity_ids().await;

    let _ = fixture.top_up().await;

    assert_eq!(fixture.managed_identity_ids().await, before);
    assert_eq!(before, vec![Identifier::from(OWN_IDENTITY)]);
}

/// An identity Platform does not know is refused with the typed not-found
/// error before anything is signed or submitted.
#[tokio::test(flavor = "multi_thread")]
async fn should_refuse_an_identity_platform_does_not_know_without_submitting() {
    let mut fixture = Fixture::new(false).await;

    let error = fixture
        .top_up()
        .await
        .expect_err("an unknown identity cannot be topped up");

    assert!(
        matches!(error, PlatformWalletError::IdentityNotFound(id) if id == fixture.target),
        "got {error:?}"
    );
    assert_eq!(fixture.submissions().await, (0, 0, 0));
    assert_eq!(
        fixture.managed_identity_ids().await,
        vec![Identifier::from(OWN_IDENTITY)]
    );
}
