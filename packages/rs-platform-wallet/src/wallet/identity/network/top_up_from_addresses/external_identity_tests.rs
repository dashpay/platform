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
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityV0};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::state_transition::identity_topup_from_addresses_transition::methods::IdentityTopUpFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use key_wallet::account::account_collection::PlatformPaymentAccountKey;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use tokio::sync::{Notify, Semaphore};

use crate::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use crate::error::PlatformWalletError;
use crate::events::{EventHandler, PlatformEventHandler};
use crate::wallet::persister::NoPlatformPersistence;
use crate::wallet::platform_wallet::WalletId;
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

fn settings() -> Option<PutSettings> {
    Some(PutSettings {
        request_settings: RequestSettings {
            retries: Some(0),
            ..Default::default()
        },
        ..Default::default()
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
    manager: Arc<PlatformWalletManager<NoPlatformPersistence>>,
    wallet: Arc<PlatformWallet>,
    sdk: Sdk,
    target: Identifier,
    broadcast: BroadcastStateTransitionRequest,
    wait: WaitForStateTransitionResultRequest,
    signer: AddressSigner,
}

impl Fixture {
    async fn new(on_platform: bool) -> Self {
        Self::build(Some(on_platform), AMOUNT).await
    }

    /// `on_platform` `None`: Platform cannot be reached to look the
    /// identity up, there is no expectation for the request.
    /// `address_balance` is what Platform holds on [`ADDRESS`].
    async fn build(on_platform: Option<bool>, address_balance: u64) -> Self {
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
        if let Some(on_platform) = on_platform {
            sdk.mock()
                .expect_fetch(target.id(), on_platform.then(|| target.clone()))
                .await
                .unwrap();
        }
        sdk.mock()
            .expect_fetch_many::<PlatformAddress, AddressInfo, _, AddressInfos>(
                BTreeSet::from([ADDRESS]),
                Some(AddressInfos::from_iter([(
                    ADDRESS,
                    Some(AddressInfo {
                        address: ADDRESS,
                        nonce: 0,
                        balance: address_balance,
                    }),
                )])),
            )
            .await
            .unwrap();
        let manager = Arc::new(PlatformWalletManager::new(
            Arc::new(sdk.clone()),
            Arc::new(NoPlatformPersistence),
            Arc::new(NoopEvents),
        ));
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
            manager,
            wallet,
            sdk,
            target: target.id(),
            broadcast,
            wait,
            signer: AddressSigner::default(),
        }
    }

    async fn top_up(&self) -> Result<u64, PlatformWalletError> {
        self.top_up_with(vec![(ADDRESS, AMOUNT)]).await
    }

    /// The top-up of any identity, topping up `target`.
    async fn top_up_with(
        &self,
        inputs: Vec<(PlatformAddress, u64)>,
    ) -> Result<u64, PlatformWalletError> {
        self.wallet
            .top_up_identity_from_addresses(&self.target, inputs, &self.signer, settings())
            .await
    }

    /// The deprecated external route, topping up `target`.
    #[allow(deprecated)]
    async fn top_up_external_with(
        &self,
        inputs: Vec<(PlatformAddress, u64)>,
    ) -> Result<u64, PlatformWalletError> {
        self.wallet
            .top_up_external_identity_from_addresses(&self.target, inputs, &self.signer, settings())
            .await
    }

    /// The deprecated managed route, topping up `identity_id`.
    #[allow(deprecated)]
    async fn top_up_managed_with(
        &self,
        identity_id: [u8; 32],
        inputs: Vec<(PlatformAddress, u64)>,
    ) -> Result<u64, PlatformWalletError> {
        self.wallet
            .top_up_from_addresses(
                &Identifier::from(identity_id),
                inputs,
                &self.signer,
                settings(),
            )
            .await
    }

    /// The deprecated managed route, topping up [`OWN_IDENTITY`].
    async fn top_up_own_with(
        &self,
        inputs: Vec<(PlatformAddress, u64)>,
    ) -> Result<u64, PlatformWalletError> {
        self.top_up_managed_with(OWN_IDENTITY, inputs).await
    }

    /// One top-up of [`AMOUNT`] from [`ADDRESS`] on each route, named: the
    /// top-up of any identity and the deprecated external route topping up
    /// `target`, the deprecated managed route [`OWN_IDENTITY`].
    async fn top_up_on_every_route(&self) -> [(&'static str, Result<u64, PlatformWalletError>); 3] {
        let rows = || vec![(ADDRESS, AMOUNT)];
        [
            ("any", self.top_up_with(rows()).await),
            ("external", self.top_up_external_with(rows()).await),
            ("managed", self.top_up_own_with(rows()).await),
        ]
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
/// refused for not being in the identity manager, by the top-up of any
/// identity and by the deprecated external route alike.
#[tokio::test(flavor = "multi_thread")]
async fn should_submit_a_top_up_for_an_identity_the_wallet_does_not_manage() {
    for external in [false, true] {
        let mut fixture = Fixture::new(true).await;

        let error = match external {
            false => fixture.top_up().await,
            true => fixture.top_up_external_with(vec![(ADDRESS, AMOUNT)]).await,
        }
        .expect_err("the mock cannot prove the result");

        assert!(
            matches!(
                error,
                PlatformWalletError::Sdk(dash_sdk::Error::StateTransitionResultUnconfirmed(_))
            ),
            "external route {external}: the top-up must be submitted, got {error:?}"
        );
        assert_eq!(fixture.submissions().await, (1, 1, 1));
    }
}

/// Topping up someone else's identity never records it as the wallet's own,
/// on the top-up of any identity and on the deprecated external route.
#[tokio::test(flavor = "multi_thread")]
async fn should_leave_the_identity_manager_unchanged() {
    for external in [false, true] {
        let fixture = Fixture::new(true).await;
        let before = fixture.managed_identity_ids().await;

        let _ = match external {
            false => fixture.top_up().await,
            true => fixture.top_up_external_with(vec![(ADDRESS, AMOUNT)]).await,
        };

        assert_eq!(fixture.managed_identity_ids().await, before);
        assert_eq!(before, vec![Identifier::from(OWN_IDENTITY)]);
    }
}

/// The deprecated managed route keeps refusing an identity the identity
/// manager does not hold, even one Platform knows, with the typed not-found
/// error and nothing asked, signed or submitted.
#[tokio::test(flavor = "multi_thread")]
async fn should_refuse_an_unmanaged_identity_on_the_deprecated_managed_route() {
    let mut fixture = Fixture::new(true).await;
    let target = fixture.target;

    let refused = fixture
        .top_up_managed_with(target.to_buffer(), vec![(ADDRESS, AMOUNT)])
        .await;

    assert!(
        matches!(&refused, Err(PlatformWalletError::IdentityNotFound(id)) if *id == target),
        "got {refused:?}"
    );
    assert_eq!(fixture.submissions().await, (0, 0, 0));
    assert_eq!(
        fixture.managed_identity_ids().await,
        vec![Identifier::from(OWN_IDENTITY)]
    );
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

/// A recipient lookup that fails (here a transport error) is reported as its
/// own not-sent error, not as a generic SDK error a host must read as "may
/// have gone through", with nothing signed or submitted.
#[tokio::test(flavor = "multi_thread")]
async fn should_report_a_failed_recipient_lookup_as_not_sent() {
    let mut fixture = Fixture::build(None, AMOUNT).await;

    let error = fixture
        .top_up()
        .await
        .expect_err("the identity cannot be looked up");

    assert!(
        matches!(&error, PlatformWalletError::IdentityLookupFailed { identity_id, .. }
            if *identity_id == fixture.target),
        "got {error:?}"
    );
    assert_eq!(fixture.submissions().await, (0, 0, 0));
}

/// An empty funding map is the caller's mistake on every route, refused as
/// `InvalidParameter` before any network I/O, with nothing signed or
/// submitted: whether Platform knows the identity, does not know it, or
/// cannot be asked, the lookup never decides the error.
#[tokio::test(flavor = "multi_thread")]
async fn should_report_empty_inputs_as_an_invalid_parameter_on_every_route() {
    for on_platform in [Some(false), None, Some(true)] {
        let mut fixture = Fixture::build(on_platform, AMOUNT).await;

        let any = fixture.top_up_with(Vec::new()).await;
        let external = fixture.top_up_external_with(Vec::new()).await;
        let managed = fixture.top_up_own_with(Vec::new()).await;
        let managed_unknown = fixture
            .top_up_managed_with(fixture.target.to_buffer(), Vec::new())
            .await;

        for result in [any, external, managed, managed_unknown] {
            assert!(
                matches!(&result, Err(PlatformWalletError::InvalidParameter(reason))
                    if reason == "no funding rows were provided"),
                "{on_platform:?}: got {result:?}"
            );
        }
        assert_eq!(fixture.submissions().await, (0, 0, 0));
    }
}

/// The deprecated managed route reads the identity, and so the balance its
/// growth is measured against, from Platform before signing, as the top-up
/// of any identity does: a read that fails (here no answer to the request)
/// is the not-sent lookup failure, and Platform not knowing the identity is
/// not-found, both with nothing signed or submitted.
#[tokio::test(flavor = "multi_thread")]
async fn should_read_the_managed_identitys_balance_from_platform_before_signing() {
    let own = Identifier::from(OWN_IDENTITY);
    let mut fixture = Fixture::new(true).await;

    let unreachable = fixture.top_up_own_with(vec![(ADDRESS, AMOUNT)]).await;
    assert!(
        matches!(&unreachable, Err(PlatformWalletError::IdentityLookupFailed { identity_id, .. })
            if *identity_id == own),
        "got {unreachable:?}"
    );

    fixture
        .sdk
        .mock()
        .expect_fetch::<Identity, Identifier>(own, None)
        .await
        .unwrap();
    let unknown = fixture.top_up_own_with(vec![(ADDRESS, AMOUNT)]).await;
    assert!(
        matches!(&unknown, Err(PlatformWalletError::IdentityNotFound(id)) if *id == own),
        "got {unknown:?}"
    );
    assert_eq!(fixture.submissions().await, (0, 0, 0));
}

/// A handle to a wallet removed, or re-created under the same id, since it
/// was taken tops up nothing, on the top-up of any identity and on both
/// deprecated routes: refused before the identity is fetched, anything is
/// signed or submitted.
#[tokio::test(flavor = "multi_thread")]
async fn should_refuse_a_top_up_from_a_wallet_no_longer_current_on_every_route() {
    let mut fixture = Fixture::new(true).await;
    let wallet_id = fixture.wallet.wallet_id();
    fixture
        .manager
        .remove_wallet(&wallet_id)
        .await
        .expect("removed");

    for (route, removed) in fixture.top_up_on_every_route().await {
        assert!(
            matches!(&removed, Err(PlatformWalletError::WalletRemovedBeforeSend(id))
                if *id == hex::encode(wallet_id)),
            "{route} route: {removed:?}"
        );
    }

    let recreated = fixture
        .manager
        .create_wallet_from_seed_bytes(
            key_wallet::Network::Testnet,
            &[42; 64],
            WalletAccountCreationOptions::None,
            Some(0),
        )
        .await
        .expect("re-created");
    assert_eq!(recreated.wallet_id(), wallet_id);
    for (route, superseded) in fixture.top_up_on_every_route().await {
        assert!(
            matches!(
                superseded,
                Err(PlatformWalletError::WalletRemovedBeforeSend(_))
            ),
            "{route} route: {superseded:?}"
        );
    }
    assert_eq!(fixture.submissions().await, (0, 0, 0));
}

/// Signs like [`AddressSigner`], but only once released: holds a top-up in
/// its signing step.
#[derive(Debug)]
struct HeldSigner {
    entered: Notify,
    release: Semaphore,
}

impl Default for HeldSigner {
    fn default() -> Self {
        Self {
            entered: Notify::new(),
            release: Semaphore::new(0),
        }
    }
}

#[async_trait::async_trait]
impl Signer<PlatformAddress> for HeldSigner {
    async fn sign(&self, key: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        self.entered.notify_one();
        self.release
            .acquire()
            .await
            .map_err(|e| ProtocolError::Generic(e.to_string()))?
            .forget();
        AddressSigner::default().sign(key, data).await
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

/// A removal waits for a top-up that entered its fenced section,
/// here held while signing, and completes once the top-up returns.
#[tokio::test(flavor = "multi_thread")]
async fn should_hold_a_wallets_removal_until_its_top_up_returns() {
    let fixture = Fixture::new(true).await;
    let wallet_id = fixture.wallet.wallet_id();
    let signer = Arc::new(HeldSigner::default());
    let top_up = {
        let wallet = Arc::clone(&fixture.wallet);
        let signer = Arc::clone(&signer);
        let target = fixture.target;
        tokio::spawn(async move {
            wallet
                .top_up_identity_from_addresses(
                    &target,
                    BTreeMap::from([(ADDRESS, AMOUNT)]),
                    &*signer,
                    settings(),
                )
                .await
        })
    };
    signer.entered.notified().await;

    let removal = {
        let manager = Arc::clone(&fixture.manager);
        tokio::spawn(async move { manager.remove_wallet(&wallet_id).await.map(|_| ()) })
    };
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !removal.is_finished(),
        "the removal waits for the top-up being signed"
    );
    assert!(fixture.wallet.core().is_current_generation().await);

    signer.release.add_permits(1);
    let topped_up = top_up.await.expect("top-up task");
    assert!(
        !matches!(
            topped_up,
            Err(PlatformWalletError::WalletRemovedBeforeSend(_))
        ),
        "the wallet was current when it was checked: {topped_up:?}"
    );
    removal
        .await
        .expect("removal task")
        .expect("the removal completes");
    assert!(!fixture.wallet.core().is_current_generation().await);
}

/// Records every store round.
#[derive(Default)]
struct RecordingPersister(Mutex<Vec<PlatformWalletChangeSet>>);

impl PlatformWalletPersistence for RecordingPersister {
    fn store(
        &self,
        _: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        self.0.lock().unwrap().push(changeset);
        Ok(())
    }

    fn flush(&self, _: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        Ok(ClientStartState::default())
    }
}

const NEW_BALANCE: u64 = 777_000;
const PROOF_HEIGHT: u64 = 4_242;

/// A wallet whose persister records every store round, one of its own
/// platform addresses, and (with `managed`) [`OWN_IDENTITY`] in its identity
/// manager at [`BALANCE`]; the recorded rounds start empty.
async fn confirmed_top_up_wallet(
    managed: bool,
) -> (
    PlatformWalletManager<RecordingPersister>,
    Arc<PlatformWallet>,
    Arc<RecordingPersister>,
    PlatformAddress,
) {
    let persister = Arc::new(RecordingPersister::default());
    let manager = PlatformWalletManager::new(
        Arc::new(dash_sdk::Sdk::new_mock()),
        Arc::clone(&persister),
        Arc::new(NoopEvents),
    );
    let wallet = manager
        .create_wallet_from_seed_bytes(
            key_wallet::Network::Testnet,
            &[42; 64],
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("a wallet");
    let spent = wallet
        .platform()
        .next_unused_receive_address(PlatformPaymentAccountKey {
            account: 0,
            key_class: 0,
        })
        .await
        .expect("a wallet address");
    if managed {
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
    }
    persister.0.lock().unwrap().clear();
    (manager, wallet, persister, spent)
}

/// Runs the composite's seam with a proof-attested result in place of the
/// network: `spent` at nonce 4 and balance 5, the identity at
/// [`NEW_BALANCE`], proved at [`PROOF_HEIGHT`]. Returns the balance the
/// top-up reports.
async fn confirm_top_up(
    wallet: &PlatformWallet,
    identity_id: Identifier,
    spent: PlatformAddress,
) -> u64 {
    let confirmed = AddressInfos::from_iter([(
        spent,
        Some(AddressInfo {
            address: spent,
            nonce: 4,
            balance: 5,
        }),
    )]);
    let metadata = ResponseMetadata {
        height: PROOF_HEIGHT,
        time_ms: 1,
        ..Default::default()
    };
    wallet
        .top_up_identity_with(&identity_id, async {
            Ok((confirmed, NEW_BALANCE, metadata))
        })
        .await
        .expect("a confirmed top-up")
}

/// The spent address is reconciled and persisted pinned at the proof's
/// height.
async fn assert_spent_address_reconciled(
    wallet: &PlatformWallet,
    persister: &RecordingPersister,
    spent: PlatformAddress,
) {
    assert!(wallet
        .platform()
        .addresses_with_balances()
        .await
        .contains(&(spent, 5)));
    let stored = persister.0.lock().unwrap();
    let entry = stored
        .iter()
        .filter_map(|changeset| changeset.platform_addresses.as_ref())
        .flat_map(|addresses| addresses.addresses.iter())
        .find(|entry| PlatformAddress::P2pkh(entry.address.to_bytes()) == spent)
        .expect("the spent address persisted");
    assert_eq!(
        (
            entry.funds.balance,
            entry.funds.nonce,
            entry.funds.as_of_height
        ),
        (5, 4, PROOF_HEIGHT)
    );
}

async fn identity_manager_ids(wallet: &PlatformWallet) -> Vec<Identifier> {
    wallet
        .wallet_manager()
        .read()
        .await
        .get_wallet_info(&wallet.wallet_id())
        .unwrap()
        .identity_manager
        .identity_ids()
}

/// A confirmed top-up of an identity the wallet does not manage, through the
/// composite's seam with a proof-attested result in place of the network:
/// the identity's new balance is returned, the spent wallet address is
/// reconciled and persisted pinned at the proof's height, and the identity
/// manager is still unchanged, nothing about the identity stored.
#[tokio::test(flavor = "multi_thread")]
async fn should_reconcile_the_spent_addresses_of_a_confirmed_top_up() {
    let (_manager, wallet, persister, spent) = confirmed_top_up_wallet(false).await;
    let identities_before = identity_manager_ids(&wallet).await;

    let new_balance = confirm_top_up(&wallet, Identifier::from([9; 32]), spent).await;

    assert_eq!(new_balance, NEW_BALANCE);
    assert_spent_address_reconciled(&wallet, &persister, spent).await;
    assert!(persister
        .0
        .lock()
        .unwrap()
        .iter()
        .all(|changeset| changeset.identities.is_none()));
    assert_eq!(identity_manager_ids(&wallet).await, identities_before);
}

/// A confirmed top-up of one of the wallet's own identities writes the
/// proved balance back to the identity manager, and stores it, so the
/// cached balance is current without waiting for a sync; the spent address
/// is reconciled as for any identity and the manager holds the same
/// identities.
#[tokio::test(flavor = "multi_thread")]
async fn should_write_a_managed_identitys_balance_back_from_a_confirmed_top_up() {
    let (_manager, wallet, persister, spent) = confirmed_top_up_wallet(true).await;
    let own = Identifier::from(OWN_IDENTITY);

    let new_balance = confirm_top_up(&wallet, own, spent).await;

    assert_eq!(new_balance, NEW_BALANCE);
    let cached = wallet
        .wallet_manager()
        .read()
        .await
        .get_wallet_info(&wallet.wallet_id())
        .unwrap()
        .identity_manager
        .identity(&own)
        .expect("still managed")
        .identity
        .balance();
    assert_eq!(cached, NEW_BALANCE);
    assert!(persister
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|changeset| changeset.identities.is_some()));
    assert_spent_address_reconciled(&wallet, &persister, spent).await;
    assert_eq!(identity_manager_ids(&wallet).await, vec![own]);
}

/// Funding rows naming the same address are one input of their sum: the
/// submitted transition is the one built from the single summed input.
#[tokio::test(flavor = "multi_thread")]
async fn should_sum_repeated_funding_rows_into_one_input() {
    let mut fixture = Fixture::new(true).await;

    let error = fixture
        .top_up_with(vec![(ADDRESS, AMOUNT - 1), (ADDRESS, 1)])
        .await
        .expect_err("the mock cannot prove the result");

    assert!(
        matches!(
            error,
            PlatformWalletError::Sdk(dash_sdk::Error::StateTransitionResultUnconfirmed(_))
        ),
        "the summed top-up must be submitted, got {error:?}"
    );
    assert_eq!(fixture.submissions().await, (1, 1, 1));
}

/// Rows whose sum for one address overflows are refused as the caller's
/// mistake on every route, never saturated, with nothing signed or
/// submitted.
#[tokio::test(flavor = "multi_thread")]
async fn should_refuse_an_overflowing_funding_sum_on_every_route() {
    let mut fixture = Fixture::new(true).await;
    let rows = vec![(ADDRESS, u64::MAX), (ADDRESS, 1)];

    let any = fixture.top_up_with(rows.clone()).await;
    let external = fixture.top_up_external_with(rows.clone()).await;
    let managed = fixture.top_up_own_with(rows).await;

    for result in [any, external, managed] {
        assert!(
            matches!(result, Err(PlatformWalletError::InvalidParameter(_))),
            "got {result:?}"
        );
    }
    assert_eq!(fixture.submissions().await, (0, 0, 0));
}

/// An SDK error the top-up does not promote keeps its typed variant, here
/// the SDK's own refusal of an address short of the amount, with nothing
/// signed or submitted.
#[tokio::test(flavor = "multi_thread")]
async fn should_keep_an_unpromoted_sdk_error_typed() {
    let mut fixture = Fixture::build(Some(true), AMOUNT - 1).await;

    let error = fixture.top_up().await.expect_err("the address is short");

    assert!(
        matches!(&error, PlatformWalletError::Sdk(e) if matches!(
            e.consensus_error(),
            Some(ConsensusError::StateError(StateError::AddressNotEnoughFundsError(_)))
        )),
        "got {error:?}"
    );
    assert_eq!(fixture.submissions().await, (0, 0, 0));
}
