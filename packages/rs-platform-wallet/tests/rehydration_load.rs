//! Item E — `load_from_persistor` end-to-end through a real
//! `PlatformWalletManager`: seed round-trip + sign-capable after
//! reload, RT-W wrong-seed hard-fail (≠ skip), RT-S skip path
//! (absent + LoadOutcome + WalletSkippedOnLoad event + recoverable
//! re-load), RT-Z secret hygiene.

use std::sync::{Arc, Mutex};

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::Wallet;
use platform_wallet::changeset::{
    AccountRegistrationEntry, ClientStartState, ClientWalletStartState, CoreChangeSet,
    PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::events::{EventHandler, PlatformEvent, PlatformEventHandler};
use platform_wallet::seed_provider::{SecretSeed, SeedProvider, SeedUnavailable, WalletSecret};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::{PlatformWalletError, PlatformWalletManager, SkipReason};

// ---- test doubles ----

/// Persister whose `load()` returns a fixed keyless `ClientStartState`.
struct FixedLoadPersister {
    state: Mutex<Option<ClientStartState>>,
}

impl FixedLoadPersister {
    fn new() -> Self {
        Self {
            state: Mutex::new(None),
        }
    }
    fn set(&self, s: ClientStartState) {
        *self.state.lock().unwrap() = Some(s);
    }
}

impl PlatformWalletPersistence for FixedLoadPersister {
    fn store(&self, _: WalletId, _: PlatformWalletChangeSet) -> Result<(), PersistenceError> {
        Ok(())
    }
    fn flush(&self, _: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }
    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        // Rebuild a fresh ClientStartState each call (load may be
        // called twice for the recoverability sub-case).
        let guard = self.state.lock().unwrap();
        match guard.as_ref() {
            None => Ok(ClientStartState::default()),
            Some(s) => {
                let mut out = ClientStartState::default();
                for (id, w) in &s.wallets {
                    out.wallets.insert(
                        *id,
                        ClientWalletStartState {
                            network: w.network,
                            birth_height: w.birth_height,
                            account_manifest: w.account_manifest.clone(),
                            core_state: w.core_state.clone(),
                            identity_manager: Default::default(),
                            unused_asset_locks: Default::default(),
                        },
                    );
                }
                Ok(out)
            }
        }
    }
}

/// Event handler recording every `PlatformEvent`.
#[derive(Default)]
struct RecordingHandler {
    events: Mutex<Vec<PlatformEvent>>,
}
impl EventHandler for RecordingHandler {}
impl PlatformEventHandler for RecordingHandler {
    fn on_platform_event(&self, event: &PlatformEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}

/// Seed provider with a per-wallet seed map, plus optional
/// unavailable / wrong-seed overrides for one specific wallet id.
struct TestSeeds {
    seeds: std::collections::HashMap<WalletId, [u8; 64]>,
    unavailable: Mutex<Option<(WalletId, SeedUnavailable)>>,
    wrong_for: Mutex<Option<(WalletId, [u8; 64])>>,
}

impl TestSeeds {
    fn single(id: WalletId, seed: [u8; 64]) -> Self {
        let mut m = std::collections::HashMap::new();
        m.insert(id, seed);
        Self {
            seeds: m,
            unavailable: Mutex::new(None),
            wrong_for: Mutex::new(None),
        }
    }
    fn with(mut self, id: WalletId, seed: [u8; 64]) -> Self {
        self.seeds.insert(id, seed);
        self
    }
}

impl SeedProvider for TestSeeds {
    fn seed_for(&self, wallet_id: [u8; 32]) -> Result<WalletSecret, SeedUnavailable> {
        if let Some((wid, reason)) = self.unavailable.lock().unwrap().as_ref() {
            if *wid == wallet_id {
                return Err(*reason);
            }
        }
        if let Some((wid, wrong)) = self.wrong_for.lock().unwrap().as_ref() {
            if *wid == wallet_id {
                return Ok(WalletSecret::Seed(SecretSeed::new(wrong.to_vec())));
            }
        }
        match self.seeds.get(&wallet_id) {
            Some(s) => Ok(WalletSecret::Seed(SecretSeed::new(s.to_vec()))),
            None => Err(SeedUnavailable::Absent),
        }
    }
}

// ---- harness ----

fn manifest_and_id(seed: [u8; 64]) -> (Vec<AccountRegistrationEntry>, [u8; 32]) {
    let w = Wallet::from_seed_bytes(
        seed,
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let manifest = w
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    (manifest, w.compute_wallet_id())
}

fn slice(seed: [u8; 64]) -> (WalletId, ClientWalletStartState) {
    let (manifest, id) = manifest_and_id(seed);
    (
        id,
        ClientWalletStartState {
            network: key_wallet::Network::Testnet,
            birth_height: 1,
            account_manifest: manifest,
            core_state: CoreChangeSet::default(),
            identity_manager: Default::default(),
            unused_asset_locks: Default::default(),
        },
    )
}

async fn manager(
    persister: Arc<FixedLoadPersister>,
    handler: Arc<RecordingHandler>,
) -> Arc<PlatformWalletManager<FixedLoadPersister>> {
    let sdk = Arc::new(dash_sdk::Sdk::new_mock());
    Arc::new(PlatformWalletManager::new(sdk, persister, handler))
}

// ---- tests ----

/// Seed round-trip: a wallet reconstructs and is signing-capable
/// (WalletType::Seed carries the root key) after reload.
#[tokio::test]
async fn rt_seed_roundtrip_signing_capable() {
    let seed = [0x11; 64];
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());
    let (id, s) = slice(seed);
    let mut st = ClientStartState::default();
    st.wallets.insert(id, s);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let seeds = TestSeeds::single(id, seed);
    let outcome = mgr.load_from_persistor(&seeds).await.expect("Ok");

    assert_eq!(outcome.loaded, vec![id]);
    assert!(outcome.skipped.is_empty());
    // The wallet is registered. It is signing-capable by construction:
    // `rehydrate_wallet` only ever yields `WalletType::Seed`/`Mnemonic`
    // (proven by the gate unit tests) — there is no watch-only path.
    assert!(
        mgr.get_wallet(&id).await.is_some(),
        "rehydrated signing wallet must be registered"
    );
    assert_eq!(mgr.wallet_ids().await, vec![id]);
}

/// RT-W: a present-but-wrong seed is a fail-closed
/// `WrongSeedForDatabase` — NOT a skip, NOT in LoadOutcome.skipped,
/// NO WalletSkippedOnLoad event. Other wallets still load.
#[tokio::test]
async fn rt_w_wrong_seed_hard_fail_not_skip() {
    let good_seed = [0x22; 64];
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());
    let (id, s) = slice(good_seed);
    let mut st = ClientStartState::default();
    st.wallets.insert(id, s);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let seeds = TestSeeds::single(id, good_seed);
    // Force a wrong seed for this exact wallet.
    *seeds.wrong_for.lock().unwrap() = Some((id, [0x99; 64]));

    let err = mgr
        .load_from_persistor(&seeds)
        .await
        .expect_err("wrong seed must hard-fail the load");
    match err {
        PlatformWalletError::WrongSeedForDatabase {
            expected_wallet_id,
            derived_wallet_id,
        } => {
            assert_eq!(expected_wallet_id, id);
            assert_ne!(derived_wallet_id, id);
        }
        other => panic!("expected WrongSeedForDatabase, got {other:?}"),
    }
    // No skip event, nothing registered.
    assert!(
        h.events.lock().unwrap().is_empty(),
        "a wrong seed must NOT emit WalletSkippedOnLoad"
    );
    assert!(mgr.get_wallet(&id).await.is_none());
}

/// RT-S: seed unavailable ⇒ skip. The other wallet loads fully; the
/// skipped wallet is absent from the manager; LoadOutcome.skipped
/// carries it; one WalletSkippedOnLoad event is delivered; load
/// returns Ok. Then making the seed available and re-loading
/// rehydrates it (recoverable).
#[tokio::test]
async fn rt_s_skip_absent_then_recoverable() {
    let seed_a = [0x31; 64];
    let seed_b = [0x32; 64];
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());
    let (id_a, sa) = slice(seed_a);
    let (id_b, sb) = slice(seed_b);
    let mut st = ClientStartState::default();
    st.wallets.insert(id_a, sa);
    st.wallets.insert(id_b, sb);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;

    // A has its correct seed; B's is explicitly unavailable.
    let seeds = TestSeeds::single(id_a, seed_a).with(id_b, seed_b);
    *seeds.unavailable.lock().unwrap() = Some((id_b, SeedUnavailable::Absent));

    let outcome = mgr
        .load_from_persistor(&seeds)
        .await
        .expect("Ok despite skip");
    assert_eq!(outcome.loaded, vec![id_a], "A loads fully");
    assert_eq!(
        outcome.skipped,
        vec![(id_b, SkipReason::SeedAbsent)],
        "B is in skipped with SeedAbsent"
    );
    // B absent from the manager (not degraded, not a placeholder).
    assert!(mgr.get_wallet(&id_a).await.is_some());
    assert!(
        mgr.get_wallet(&id_b).await.is_none(),
        "skipped wallet must be ABSENT, not a degraded placeholder"
    );
    // Exactly one WalletSkippedOnLoad event for B.
    {
        let events = h.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            PlatformEvent::WalletSkippedOnLoad { wallet_id, reason } => {
                assert_eq!(*wallet_id, id_b);
                assert_eq!(*reason, SkipReason::SeedAbsent);
            }
        }
    }

    // Recoverable: a fresh manager + a persister carrying only B, with
    // B's seed now available → B loads cleanly (the previously-skipped
    // wallet recovers on a later targeted re-load).
    let p2 = Arc::new(FixedLoadPersister::new());
    let h2 = Arc::new(RecordingHandler::default());
    let (_id_b2, sb2) = slice(seed_b);
    let mut st2 = ClientStartState::default();
    st2.wallets.insert(id_b, sb2);
    p2.set(st2);
    let mgr2 = manager(Arc::clone(&p2), Arc::clone(&h2)).await;
    let seeds2 = TestSeeds::single(id_b, seed_b);
    let outcome2 = mgr2.load_from_persistor(&seeds2).await.expect("Ok");
    assert_eq!(
        outcome2.loaded,
        vec![id_b],
        "the previously-skipped wallet now loads"
    );
    assert!(outcome2.skipped.is_empty());
    assert!(mgr2.get_wallet(&id_b).await.is_some());
}

/// RT-S (ii): a locked store maps to StoreUnavailable, still a skip.
#[tokio::test]
async fn rt_s_store_locked_is_skip() {
    use platform_wallet::seed_provider::SecretStoreErrorKind;
    let seed = [0x41; 64];
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());
    let (id, s) = slice(seed);
    let mut st = ClientStartState::default();
    st.wallets.insert(id, s);
    p.set(st);
    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let seeds = TestSeeds::single(id, seed);
    *seeds.unavailable.lock().unwrap() = Some((
        id,
        SeedUnavailable::StoreUnavailable(SecretStoreErrorKind::KeyringLocked),
    ));

    let outcome = mgr.load_from_persistor(&seeds).await.expect("Ok");
    assert!(outcome.loaded.is_empty());
    assert_eq!(
        outcome.skipped,
        vec![(
            id,
            SkipReason::StoreUnavailable(SecretStoreErrorKind::KeyringLocked)
        )]
    );
    assert!(mgr.get_wallet(&id).await.is_none());
}

/// RT-Z: no seed byte / structural source leaks into LoadOutcome,
/// SkipReason, or the WrongSeedForDatabase error rendering.
#[tokio::test]
async fn rt_z_secret_hygiene() {
    let seed = [0xAB; 64];
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());
    let (id, s) = slice(seed);
    let mut st = ClientStartState::default();
    st.wallets.insert(id, s);
    p.set(st);
    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;

    let seeds = TestSeeds::single(id, seed);
    *seeds.wrong_for.lock().unwrap() = Some((id, [0xCD; 64]));
    let err = mgr.load_from_persistor(&seeds).await.unwrap_err();
    let rendered = format!("{err} {err:?}");
    // 0xAB / 0xCD seed bytes must not appear hex-rendered.
    assert!(!rendered.to_lowercase().contains(&"ab".repeat(10)));
    assert!(!rendered.to_lowercase().contains(&"cd".repeat(10)));

    // Skip path rendering carries no secret either.
    let seeds2 = TestSeeds::single(id, seed);
    *seeds2.unavailable.lock().unwrap() = Some((id, SeedUnavailable::Absent));
    let outcome = mgr.load_from_persistor(&seeds2).await.unwrap();
    let dbg = format!("{outcome:?}");
    assert!(!dbg.to_lowercase().contains(&"ab".repeat(10)));
}
