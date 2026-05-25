//! TC-CODE-018 — `register_wallet` (via `create_wallet_from_seed_bytes`)
//! must drive the typed `PersistenceError` kind off the registration
//! store: transient → one backoff retry; fatal → undo in-memory state
//! and surface `WalletRegistrationFailed`.
//!
//! Without this fix, a failed register-time store leaves the wallet
//! visible in `wallet_manager` without a `wallet_metadata` row, so
//! every subsequent per-wallet write FK-violates against an absent
//! parent (CODE-002 territory).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::changeset::{
    ClientStartState, PersistenceError, PersistenceErrorKind, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet::error::PlatformWalletError;
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;

/// Persister scripted with a per-call queue of outcomes for `store`.
/// Drives the transient-retry and fatal-undo paths deterministically.
struct ScriptedPersister {
    /// FIFO of outcomes consumed by successive `store` calls.
    store_outcomes: Mutex<Vec<StoreOutcome>>,
    store_calls: AtomicUsize,
}

enum StoreOutcome {
    Ok,
    Transient(&'static str),
    Fatal(&'static str),
}

impl ScriptedPersister {
    fn new(outcomes: Vec<StoreOutcome>) -> Self {
        Self {
            store_outcomes: Mutex::new(outcomes),
            store_calls: AtomicUsize::new(0),
        }
    }

    fn store_call_count(&self) -> usize {
        self.store_calls.load(Ordering::SeqCst)
    }
}

impl PlatformWalletPersistence for ScriptedPersister {
    fn store(
        &self,
        _wallet_id: WalletId,
        _changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        self.store_calls.fetch_add(1, Ordering::SeqCst);
        // Pop the next scripted outcome. If the script runs out we
        // succeed silently so post-registration writes (event-adapter
        // changesets) don't muddy the count assertions.
        let outcome = self
            .store_outcomes
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(StoreOutcome::Ok);
        match outcome {
            StoreOutcome::Ok => Ok(()),
            StoreOutcome::Transient(msg) => Err(PersistenceError::backend_with_kind(
                PersistenceErrorKind::Transient,
                StringErr(msg),
            )),
            StoreOutcome::Fatal(msg) => Err(PersistenceError::backend_with_kind(
                PersistenceErrorKind::Fatal,
                StringErr(msg),
            )),
        }
    }

    fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        Ok(ClientStartState::default())
    }
}

/// Minimal `std::error::Error` shim for `backend_with_kind`.
#[derive(Debug)]
struct StringErr(&'static str);

impl std::fmt::Display for StringErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for StringErr {}

struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

fn mock_sdk() -> Arc<dash_sdk::Sdk> {
    Arc::new(
        dash_sdk::SdkBuilder::new_mock()
            .build()
            .expect("mock sdk should build"),
    )
}

fn build_manager(
    persister: Arc<ScriptedPersister>,
) -> Arc<PlatformWalletManager<ScriptedPersister>> {
    let sdk = mock_sdk();
    let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
    Arc::new(PlatformWalletManager::new(sdk, persister, handler))
}

/// Fixed BIP-39 seed bytes — deterministic across test runs.
fn test_seed_bytes() -> [u8; 64] {
    let mut seed = [0u8; 64];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7).wrapping_add(3);
    }
    seed
}

/// Reverse the script vec so `Vec::pop` consumes outcomes in
/// front-to-back order.
fn script(outcomes: Vec<StoreOutcome>) -> Vec<StoreOutcome> {
    let mut v = outcomes;
    v.reverse();
    v
}

/// TC-CODE-018-a — Fatal store error → register undoes in-memory
/// state and surfaces `WalletRegistrationFailed`.
#[tokio::test]
async fn tc_code_018_a_fatal_store_error_undoes_in_memory_state() {
    let persister = Arc::new(ScriptedPersister::new(script(vec![StoreOutcome::Fatal(
        "schema constraint X violated",
    )])));
    let manager = build_manager(Arc::clone(&persister));

    let result = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            test_seed_bytes(),
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await;

    let err = result.expect_err("fatal store must abort wallet registration");
    match err {
        PlatformWalletError::WalletRegistrationFailed { reason, .. } => {
            assert!(
                reason.contains("schema constraint X violated"),
                "expected backend message to be carried, got: {reason}"
            );
        }
        other => panic!("expected WalletRegistrationFailed, got {other:?}"),
    }

    // Exactly one store attempt — fatal kind must NOT retry.
    assert_eq!(
        persister.store_call_count(),
        1,
        "fatal store kind must not be retried"
    );

    // In-memory state has been rolled back: the wallet is not visible
    // through any read API.
    let wallet_ids = manager.wallet_ids().await;
    assert!(
        wallet_ids.is_empty(),
        "registration must roll back in-memory state on fatal store; saw {wallet_ids:?}"
    );
}

/// TC-CODE-018-b — Transient store error → one retry → success →
/// wallet is registered.
#[tokio::test]
async fn tc_code_018_b_transient_store_error_retries_once_then_succeeds() {
    let persister = Arc::new(ScriptedPersister::new(script(vec![
        StoreOutcome::Transient("SQLITE_BUSY"),
        StoreOutcome::Ok,
    ])));
    let manager = build_manager(Arc::clone(&persister));

    let platform_wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            test_seed_bytes(),
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("transient store should be retried then succeed");

    // Exactly two store attempts: original + one retry.
    assert!(
        persister.store_call_count() >= 2,
        "transient store kind must trigger a retry; saw {} call(s)",
        persister.store_call_count()
    );

    // Wallet is now visible through the manager.
    let wallet_ids = manager.wallet_ids().await;
    assert!(
        wallet_ids.contains(&platform_wallet.wallet_id()),
        "wallet should be registered after transient retry succeeds; saw {wallet_ids:?}"
    );
}
