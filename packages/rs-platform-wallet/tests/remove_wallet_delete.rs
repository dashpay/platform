//! TC-CODE-003 — `PlatformWalletManager::remove_wallet` must call
//! `PlatformWalletPersistence::delete_wallet` so the on-disk cascade
//! actually runs. Without this wiring, in-memory state is gone but
//! the row tree stays — every subsequent reload silently resurrects
//! a "deleted" wallet.
//!
//! Covered:
//!   - TC-CODE-003-1 — happy path: one `delete_wallet` call lands.
//!   - TC-CODE-003-2 — fatal `delete_wallet` error does NOT abort
//!     `remove_wallet`; in-memory cleanup still completes and the
//!     manager surfaces the removed handle.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::changeset::{
    ClientStartState, DeleteWalletReport, PersistenceError, PersistenceErrorKind,
    PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;

/// Persister that records every call it sees. `delete_wallet` can be
/// scripted with a per-call outcome queue; `store` / `flush` always
/// succeed so registration paths land cleanly.
struct RecordingPersister {
    delete_calls: Mutex<Vec<WalletId>>,
    delete_outcomes: Mutex<Vec<DeleteOutcome>>,
    delete_count: AtomicUsize,
}

#[allow(dead_code)]
enum DeleteOutcome {
    Ok,
    Transient,
    Fatal,
}

impl RecordingPersister {
    fn new(outcomes: Vec<DeleteOutcome>) -> Self {
        let mut v = outcomes;
        v.reverse();
        Self {
            delete_calls: Mutex::new(Vec::new()),
            delete_outcomes: Mutex::new(v),
            delete_count: AtomicUsize::new(0),
        }
    }

    fn delete_call_count(&self) -> usize {
        self.delete_count.load(Ordering::SeqCst)
    }

    fn delete_targets(&self) -> Vec<WalletId> {
        self.delete_calls.lock().unwrap().clone()
    }
}

impl PlatformWalletPersistence for RecordingPersister {
    fn store(
        &self,
        _wallet_id: WalletId,
        _changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        Ok(ClientStartState::default())
    }

    fn delete_wallet(&self, wallet_id: WalletId) -> Result<DeleteWalletReport, PersistenceError> {
        self.delete_count.fetch_add(1, Ordering::SeqCst);
        self.delete_calls.lock().unwrap().push(wallet_id);
        let outcome = self
            .delete_outcomes
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(DeleteOutcome::Ok);
        match outcome {
            DeleteOutcome::Ok => Ok(DeleteWalletReport {
                wallet_id,
                backup_path: None,
                rows_removed_per_table: std::collections::BTreeMap::new(),
            }),
            DeleteOutcome::Transient => Err(PersistenceError::backend_with_kind(
                PersistenceErrorKind::Transient,
                StringErr("SQLITE_BUSY"),
            )),
            DeleteOutcome::Fatal => Err(PersistenceError::backend_with_kind(
                PersistenceErrorKind::Fatal,
                StringErr("schema corruption"),
            )),
        }
    }
}

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
    persister: Arc<RecordingPersister>,
) -> Arc<PlatformWalletManager<RecordingPersister>> {
    let sdk = mock_sdk();
    let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
    Arc::new(PlatformWalletManager::new(sdk, persister, handler))
}

fn test_seed_bytes(salt: u8) -> [u8; 64] {
    let mut seed = [0u8; 64];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7).wrapping_add(salt);
    }
    seed
}

/// TC-CODE-003-1 — `remove_wallet` triggers exactly one
/// `persister.delete_wallet` call against the right wallet id.
#[tokio::test]
async fn tc_code_003_1_remove_wallet_calls_persister_delete_wallet() {
    let persister = Arc::new(RecordingPersister::new(vec![DeleteOutcome::Ok]));
    let manager = build_manager(Arc::clone(&persister));

    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            test_seed_bytes(3),
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("wallet registration should succeed under recording persister");

    let wallet_id = wallet.wallet_id();

    let removed = manager
        .remove_wallet(&wallet_id)
        .await
        .expect("remove_wallet should succeed on the happy path");

    assert_eq!(removed.wallet_id(), wallet_id);
    assert_eq!(
        persister.delete_call_count(),
        1,
        "expected exactly one persister.delete_wallet call; saw {}",
        persister.delete_call_count()
    );
    assert_eq!(
        persister.delete_targets(),
        vec![wallet_id],
        "delete_wallet must be called with the removed wallet id"
    );

    // In-memory state really is gone.
    let ids = manager.wallet_ids().await;
    assert!(
        !ids.contains(&wallet_id),
        "wallet must be removed from the manager view"
    );
}

/// TC-CODE-003-2 — fatal `delete_wallet` error must NOT roll back
/// the in-memory cleanup. The user wanted this wallet gone; the disk
/// failure is logged and the call still returns Ok with the handle.
#[tokio::test]
async fn tc_code_003_2_remove_wallet_completes_when_persister_fails() {
    let persister = Arc::new(RecordingPersister::new(vec![DeleteOutcome::Fatal]));
    let manager = build_manager(Arc::clone(&persister));

    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            test_seed_bytes(11),
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("wallet registration should succeed");

    let wallet_id = wallet.wallet_id();

    let removed = manager
        .remove_wallet(&wallet_id)
        .await
        .expect("remove_wallet must succeed even when persister.delete_wallet fails fatally");

    assert_eq!(removed.wallet_id(), wallet_id);
    assert_eq!(
        persister.delete_call_count(),
        1,
        "fatal delete must NOT retry; expected one call, saw {}",
        persister.delete_call_count()
    );

    // In-memory state is gone — we trust the manager, not the
    // persister, for the user-facing view.
    let ids = manager.wallet_ids().await;
    assert!(
        !ids.contains(&wallet_id),
        "in-memory cleanup must run regardless of persister outcome"
    );
}

/// TC-CODE-003-3 — transient `delete_wallet` error triggers exactly
/// one retry (matching the `register_wallet` pattern from CODE-018).
#[tokio::test]
async fn tc_code_003_3_remove_wallet_retries_once_on_transient() {
    // First call: transient. Second call (the retry): Ok.
    let persister = Arc::new(RecordingPersister::new(vec![
        DeleteOutcome::Transient,
        DeleteOutcome::Ok,
    ]));
    let manager = build_manager(Arc::clone(&persister));

    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            test_seed_bytes(23),
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("wallet registration should succeed");

    let wallet_id = wallet.wallet_id();

    manager
        .remove_wallet(&wallet_id)
        .await
        .expect("transient delete must be retried and succeed");

    assert_eq!(
        persister.delete_call_count(),
        2,
        "transient kind must trigger exactly one retry; saw {} call(s)",
        persister.delete_call_count()
    );
}
