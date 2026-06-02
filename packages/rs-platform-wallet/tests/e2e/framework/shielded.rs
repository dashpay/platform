//! Wave H — shielded (Orchard) e2e harness.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` §4 "Wave H — Shielded (Orchard)
//! harness extensions" and §3 "### Shielded (SH)".
//!
//! Everything here is gated behind `#[cfg(feature = "shielded")]`; the
//! SH cases compile only under `--features shielded` (the `e2e` feature
//! pulls `shielded` in). The cost center is the Halo-2 prover — see
//! [`shielded_prover`].
//!
//! # Per-test isolation model
//!
//! The production `PlatformWalletManager` holds ONE coordinator per
//! network and `configure_shielded` refuses to repoint, so the harness
//! does NOT route through it. Instead [`bind_shielded`] routes every
//! case through ONE process-shared [`NetworkShieldedCoordinator`] over a
//! single persisted SQLite tree (see [`shared_coordinator`]). The
//! commitment tree is chain-wide — identical for every wallet on the
//! network — so sharing it is sharing a cache of public chain data, not
//! wallet state. Per-case isolation is preserved by `SubwalletId =
//! (wallet_id, account_index)` scoping: each case mints a fresh seed, so
//! its notes / spent-marks / watermarks never bleed into another case's.
//!
//! The first case pays one full ~1M-note Orchard scan into the shared
//! tree; [`bind_shielded`] then seeds each freshly-bound account's
//! watermark to the shared `tree_size`, so cases 2..N start their fetch
//! at the tip-aligned chunk and pull only the handful of notes since —
//! turning a per-case full re-scan into a per-case tip-delta scan
//! (~25-30x on the Orchard-scan portion of the suite).
//!
//! [`new_file_backed_coordinator`] still mints a private per-call tree
//! for the two cases that need a controlled, isolated tree (SH-007's
//! bind-ordering hook and SH-013's empty-accounts error path); they do
//! not benefit from the shared tree but keep the rest of the suite's win.
//!
//! # Adversarial injection hooks (SH-020..SH-035 — follow-up wave)
//!
//! The functional cases (SH-001..SH-019) call the guarded
//! `PlatformWallet::shielded_*` methods. The adversarial cases bypass
//! those guards to reach Drive's validation directly; the seams they
//! need ([`build_raw_shielded_transition`], [`broadcast_raw`],
//! [`mutate_serialized_bundle`], [`TamperingProver`], …) live here and
//! are gated behind [`adversarial_enabled`] so a stray malformed
//! broadcast can't pollute a normal functional run.

#![cfg(feature = "shielded")]

use std::sync::Arc;
use std::time::Duration;

use dpp::shielded::builder::OrchardProver;
use grovedb_commitment_tree::ProvingKey;
use platform_wallet::wallet::shielded::{
    CachedOrchardProver, FileBackedShieldedStore, InMemoryShieldedStore,
    NetworkShieldedCoordinator, ShieldedStore, SubwalletId,
};

use super::wallet_factory::TestWallet;
use super::{FrameworkError, FrameworkResult};

/// Env gate for the adversarial / abuse cases (SH-020..SH-035). The
/// hooks below that broadcast malformed transitions are no-ops unless
/// this is set, so the functional tier never accidentally hammers Drive
/// with garbage. Mirrors the `PLATFORM_WALLET_E2E_BANK_CORE_GATE`
/// convention.
pub const ADVERSARIAL_GATE_ENV: &str = "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL";

/// Whether the adversarial abuse pass is enabled this run. Accepts the
/// same truthy aliases the rest of the harness uses (`1`/`true`/`yes`/`on`,
/// case-insensitive).
pub fn adversarial_enabled() -> bool {
    matches!(
        std::env::var(ADVERSARIAL_GATE_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Process-wide warmed Orchard prover.
///
/// [`CachedOrchardProver`] is zero-sized — the expensive Halo-2
/// [`ProvingKey`] lives in a `OnceLock` inside the prover module, so a
/// single [`CachedOrchardProver::warm_up`] builds it once for the whole
/// process and every SH case borrows `&CachedOrchardProver` cheaply.
///
/// First call blocks ~30 s building the key; subsequent calls are
/// instant. Returns a `'static` handle so callers can pass
/// `&shielded_prover()` straight to the `shielded_*` methods (the
/// `OrchardProver` impl is on `&CachedOrchardProver`).
pub fn shielded_prover() -> &'static CachedOrchardProver {
    static PROVER: CachedOrchardProver = CachedOrchardProver;
    PROVER.warm_up();
    &PROVER
}

/// Handle returned by [`bind_shielded`]: the per-test coordinator plus
/// the bound account list, so the test can drive `sync(true)` and read
/// balances without re-deriving anything.
pub struct ShieldedHandle {
    /// Coordinator backing this case. For [`bind_shielded`] this is the
    /// process-shared coordinator (one persisted tree across the suite);
    /// SH-007 / SH-013 build a private one via
    /// [`new_file_backed_coordinator`] and wrap it themselves.
    pub coordinator: Arc<NetworkShieldedCoordinator>,
    /// ZIP-32 account indices bound on the wallet, ascending.
    pub accounts: Vec<u32>,
}

impl ShieldedHandle {
    /// Force a sync pass so on-chain notes are scanned into the store.
    /// `force=true` bypasses the coordinator's caught-up cooldown.
    pub async fn sync(&self) {
        let _ = self.coordinator.sync(true).await;
    }

    /// This wallet's per-account unspent shielded balances.
    pub async fn balances(
        &self,
        wallet: &TestWallet,
    ) -> FrameworkResult<std::collections::BTreeMap<u32, u64>> {
        wallet
            .platform_wallet()
            .shielded_balances(&self.coordinator)
            .await
            .map_err(|e| FrameworkError::Wallet(format!("shielded_balances: {e}")))
    }
}

/// Bind `accounts` on the wallet's shielded sub-wallet, routed through
/// the process-shared coordinator.
///
/// All cases (except SH-007 / SH-013, which use
/// [`new_file_backed_coordinator`] for a controlled private tree) share
/// ONE [`NetworkShieldedCoordinator`] over ONE persisted SQLite tree
/// (see [`shared_coordinator`]). The first case pays the full ~1M-note
/// Orchard scan into that tree; this function then seeds each
/// freshly-bound account's watermark to the shared `tree_size`, so
/// cases 2..N start their fetch at the tip-aligned chunk and pull only
/// the notes since (including the one this case is about to shield,
/// which lands at a position `>= tree_size` and so is past the seed).
///
/// Without the watermark seed a fresh-seed wallet binds at watermark 0,
/// collapsing the sync's `MIN`-watermark fetch start back to position 0
/// — a shared tree alone would save only the local append, not the
/// dominant network re-fetch. Seeding is what converts the shared tree
/// into a fetch speedup.
///
/// Per-case isolation holds because notes / spent-marks / watermarks are
/// `SubwalletId`-scoped and each case uses a distinct (fresh-seed)
/// `wallet_id`; `shielded_balances` reads per-subwallet notes, never the
/// shared tree, so a case's pre-sync "balance is 0" assertion stays
/// genuine.
///
/// Errors: [`FrameworkError::Wallet`] for coordinator, `bind_shielded`,
/// or watermark-seed failures.
pub async fn bind_shielded(
    wallet: &TestWallet,
    accounts: &[u32],
    workdir: &std::path::Path,
) -> FrameworkResult<ShieldedHandle> {
    let coordinator = shared_coordinator(wallet, workdir).await?;
    let seed = wallet.seed_bytes();
    wallet
        .platform_wallet()
        .bind_shielded(&seed, accounts, &coordinator)
        .await
        .map_err(|e| FrameworkError::Wallet(format!("bind_shielded: {e}")))?;

    // Seed this wallet's per-account watermark to the shared tree's
    // current leaf count so the next sync fetches only the tip delta.
    // `bind_shielded` registers the wallet on the coordinator and (for a
    // fresh seed) leaves every account at watermark 0; overwrite that
    // with `tree_size` under one write guard. A note at exactly
    // `tree_size` still passes the sync's strict `position < watermark`
    // save gate, so nothing this case owns is skipped.
    {
        let mut store = coordinator.store().write().await;
        let tree_size = store
            .tree_size()
            .map_err(|e| FrameworkError::Wallet(format!("bind_shielded: tree_size: {e}")))?;
        for &account in accounts {
            let id = SubwalletId::new(wallet.id(), account);
            store
                .set_last_synced_note_index(id, tree_size)
                .map_err(|e| {
                    FrameworkError::Wallet(format!("bind_shielded: seed watermark: {e}"))
                })?;
        }
    }

    Ok(ShieldedHandle {
        coordinator,
        accounts: accounts.to_vec(),
    })
}

/// Process-shared coordinator over ONE persisted commitment-tree SQLite
/// file for the whole suite — the speedup's single most important seam.
///
/// Built lazily on the first [`bind_shielded`] from the shared SDK and a
/// single deterministic path `<workdir>/shielded/shared_tree_<network>.sqlite`.
/// Every case routes through the SAME `Arc<RwLock<FileBackedShieldedStore>>`
/// the coordinator owns, so there is exactly one SQLite handle (no
/// cross-handle WAL contention) and one persisted tree whose `tree_size`
/// carries the full ~1M-leaf scan forward across cases.
async fn shared_coordinator(
    wallet: &TestWallet,
    workdir: &std::path::Path,
) -> FrameworkResult<Arc<NetworkShieldedCoordinator>> {
    static SHARED: tokio::sync::OnceCell<Arc<NetworkShieldedCoordinator>> =
        tokio::sync::OnceCell::const_new();
    let pw = wallet.platform_wallet();
    let network = pw.sdk().network;
    let dir = workdir.join("shielded");
    let sdk = pw.sdk_arc();
    SHARED
        .get_or_try_init(|| async {
            std::fs::create_dir_all(&dir).map_err(|e| {
                FrameworkError::Io(format!("create shielded dir {}: {e}", dir.display()))
            })?;
            let db_path = dir.join(format!("shared_tree_{network}.sqlite"));
            let store = FileBackedShieldedStore::open_path(&db_path, 100)
                .map_err(|e| FrameworkError::Wallet(format!("open shared shielded store: {e}")))?;
            Ok(Arc::new(NetworkShieldedCoordinator::new(
                sdk, network, db_path, store,
            )))
        })
        .await
        .cloned()
}

/// Construct a PRIVATE per-call FileBacked coordinator over a fresh
/// SQLite path WITHOUT binding — the controlled-tree path for the two
/// cases that need an isolated tree: SH-007's bind-ordering hook (the
/// coordinator's tree is advanced via `sync(true)` before the second
/// wallet binds, so it must start empty) and SH-013's empty-accounts
/// error path (which errors before any sync). These two skip the shared
/// tree of [`bind_shielded`]; the rest of the suite keeps the speedup.
pub async fn new_file_backed_coordinator(
    wallet: &TestWallet,
    workdir: &std::path::Path,
) -> FrameworkResult<Arc<NetworkShieldedCoordinator>> {
    let dir = workdir.join("shielded");
    std::fs::create_dir_all(&dir)
        .map_err(|e| FrameworkError::Io(format!("create shielded dir {}: {e}", dir.display())))?;
    let unique = format!(
        "{}-{}.sqlite",
        hex::encode(&wallet.id()[..6]),
        next_db_seq(),
    );
    let db_path = dir.join(unique);
    let store = FileBackedShieldedStore::open_path(&db_path, 100)
        .map_err(|e| FrameworkError::Wallet(format!("open shielded store: {e}")))?;
    let pw = wallet.platform_wallet();
    Ok(Arc::new(NetworkShieldedCoordinator::new(
        pw.sdk_arc(),
        pw.sdk().network,
        db_path,
        store,
    )))
}

/// Monotonic per-process counter so each coordinator gets a distinct
/// SQLite file even when two binds in one test share a wallet id prefix.
fn next_db_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// In-memory store for SH-005's witness-availability split. The
/// coordinator only accepts a FileBacked store, so the in-memory arm
/// drives the `operations::*` free functions directly with this store.
/// Its `witness()` is a hard `Err` (Found-027), which is exactly what
/// SH-005 pins.
pub fn in_memory_store() -> Arc<tokio::sync::RwLock<InMemoryShieldedStore>> {
    Arc::new(tokio::sync::RwLock::new(InMemoryShieldedStore::default()))
}

/// Poll `shielded_balances` after a forced sync until `account`'s
/// balance reaches `expected`, or `timeout` elapses.
///
/// Drives a `coordinator.sync(true)` each poll (the caught-up cooldown
/// is bypassed by `force=true`), mirroring the
/// [`super::tokens::wait_for_token_balance`] event-driven +
/// chain-confirmed shape. Returns the observed balance on success.
///
/// Errors: [`FrameworkError::Cleanup`] on timeout (carries account +
/// expected for triage), [`FrameworkError::Wallet`] never — fetch
/// failures are logged and retried.
pub async fn wait_for_shielded_balance(
    wallet: &TestWallet,
    handle: &ShieldedHandle,
    account: u32,
    expected: u64,
    timeout: Duration,
) -> FrameworkResult<u64> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        handle.sync().await;
        match handle.balances(wallet).await {
            Ok(balances) => {
                let current = balances.get(&account).copied().unwrap_or(0);
                if current >= expected {
                    return Ok(current);
                }
                tracing::debug!(
                    target: "platform_wallet::e2e::shielded",
                    account,
                    current,
                    expected,
                    "shielded balance below target"
                );
            }
            Err(err) => tracing::debug!(
                target: "platform_wallet::e2e::shielded",
                account,
                error = %err,
                "shielded_balances fetch failed; retrying"
            ),
        }

        if std::time::Instant::now() >= deadline {
            return Err(FrameworkError::Cleanup(format!(
                "wait_for_shielded_balance timed out after {timeout:?} \
                 (account={account} expected={expected})"
            )));
        }
        tokio::time::sleep(super::wait::DEFAULT_POLL_INTERVAL).await;
    }
}

/// Thin wrapper over `shielded_default_address` returning the raw 43
/// bytes (SH-003 transfer-recipient plumbing). Errors if `account`
/// isn't bound.
pub async fn shielded_default_address_43(
    wallet: &TestWallet,
    account: u32,
) -> FrameworkResult<[u8; 43]> {
    wallet
        .platform_wallet()
        .shielded_default_address(account)
        .await
        .ok_or_else(|| {
            FrameworkError::Wallet(format!("shielded account {account} has no default address"))
        })
}

/// Best-effort teardown sweep: unshield any residual shielded balance on
/// every bound account back to the bank's primary transparent platform
/// address, preventing a bank-fund leak across a long suite.
///
/// **MUST NOT fail teardown.** Every error is swallowed and logged at
/// `warn` — the RED-by-design cases (SH-005 in-memory arm, any
/// intentionally-broken `witness()` path) WILL fail the sweep, and that
/// failure must never propagate. Mirrors `cancel_pending` and the PA
/// identity-sweep floor (best-effort, below-floor balances left for the
/// next-run orphan sweep).
///
/// Finally unregisters the wallet from its coordinator so the shared
/// coordinator's registry stays bounded across the suite. This purges
/// only the case's per-subwallet state (notes, spent marks, watermarks);
/// the chain-wide commitment tree is left intact for the next case.
pub async fn teardown_sweep_shielded(
    wallet: &TestWallet,
    handle: &ShieldedHandle,
    bank_addr_bech32m: &str,
) {
    let prover = shielded_prover();
    for &account in &handle.accounts {
        // Re-scan so the residual is current before we attempt to drain.
        handle.sync().await;
        let balance = match handle.balances(wallet).await {
            Ok(b) => b.get(&account).copied().unwrap_or(0),
            Err(err) => {
                tracing::warn!(
                    target: "platform_wallet::e2e::shielded",
                    account,
                    error = %err,
                    "teardown sweep: balance read failed; skipping account"
                );
                continue;
            }
        };
        if balance == 0 {
            continue;
        }
        // The unshield itself pays a shielded fee, so we can't drain the
        // full balance — the spend's note-selection folds the fee into
        // the requirement. Leave a conservative fee headroom; if it's
        // still short the unshield errors and we swallow it.
        const FEE_HEADROOM: u64 = 5_000_000;
        let sweep_amount = balance.saturating_sub(FEE_HEADROOM);
        if sweep_amount == 0 {
            continue;
        }
        match wallet
            .platform_wallet()
            .shielded_unshield_to(
                &handle.coordinator,
                account,
                bank_addr_bech32m,
                sweep_amount,
                prover,
            )
            .await
        {
            Ok(()) => tracing::info!(
                target: "platform_wallet::e2e::shielded",
                account,
                sweep_amount,
                "teardown sweep: unshielded residual to bank"
            ),
            Err(err) => tracing::warn!(
                target: "platform_wallet::e2e::shielded",
                account,
                sweep_amount,
                error = %err,
                "teardown sweep: unshield failed (best-effort, swallowed)"
            ),
        }
    }

    // Bound the shared coordinator's registry: drop this case's
    // registration and per-subwallet store state. The chain-wide tree
    // (the speedup's carried-forward cache) is left intact.
    handle.coordinator.unregister_wallet(wallet.id()).await;
}

// ---------------------------------------------------------------------------
// Adversarial injection hooks (SH-020..SH-035)
//
// These reach Drive with transitions the guarded `PlatformWallet::shielded_*`
// methods would never assemble: built via the production build/broadcast
// split (`operations::build_*_st`) + the `test-utils` spend-assembly seams,
// then byte-tampered and broadcast directly. All live broadcasts are gated
// behind `adversarial_enabled()`.
// ---------------------------------------------------------------------------

/// A `SerializedBundle` field selector for [`mutate_serialized_bundle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleField {
    /// Halo-2 proof bytes (SH-025).
    Proof,
    /// 64-byte binding signature (SH-034).
    BindingSignature,
    /// 32-byte Sinsemilla anchor (SH-026).
    Anchor,
    /// Net value balance (SH-022 / SH-024).
    ValueBalance,
}

/// How to mutate the selected byte field.
#[derive(Debug, Clone)]
pub enum BundleMutation {
    /// Overwrite the whole field with these bytes (length-flexible —
    /// truncation / overrun is itself part of the abuse surface).
    Overwrite(Vec<u8>),
    /// Zero every byte of the field.
    Zero,
    /// XOR-flip the byte at this index.
    FlipByte(usize),
}

/// An `OrchardProver` that emits a structurally-valid-looking but
/// circuit-invalid proof, for the proof-substitution arm of SH-025.
///
/// The trait is just `proving_key()`, so a tampering prover hands back a
/// real key and the abuse case corrupts the resulting proof bytes
/// post-hoc via [`mutate_serialized_bundle`]. Holding the inner cached
/// prover keeps the key build shared.
pub struct TamperingProver;

impl OrchardProver for &TamperingProver {
    fn proving_key(&self) -> &ProvingKey {
        // Borrow the shared, warmed key; the abuse case tampers with the
        // emitted proof bytes afterwards rather than corrupting the key.
        // The cached prover handle is itself `'static`, so the
        // double-reference we hand the inner impl lives long enough.
        static PROVER_REF: std::sync::OnceLock<&'static CachedOrchardProver> =
            std::sync::OnceLock::new();
        let prover: &'static &'static CachedOrchardProver = PROVER_REF.get_or_init(shielded_prover);
        OrchardProver::proving_key(prover)
    }
}

/// Broadcast a built [`StateTransition`] directly, returning the typed
/// backend error so an abuse case can assert the exact rejection variant.
/// Bypasses the guarded `shielded_*` methods.
///
/// Gated: refuses unless [`adversarial_enabled`], so a stray malformed
/// broadcast can't pollute a normal functional run. Same broadcast path
/// PA-006 replays through.
pub async fn broadcast_raw(
    sdk: &Arc<dash_sdk::Sdk>,
    state_transition: &dpp::state_transition::StateTransition,
) -> FrameworkResult<()> {
    use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;

    if !adversarial_enabled() {
        return Err(FrameworkError::Config(format!(
            "broadcast_raw refused: set {ADVERSARIAL_GATE_ENV} to run the abuse pass"
        )));
    }
    state_transition
        .broadcast(sdk.as_ref(), None)
        .await
        .map_err(|e| FrameworkError::Sdk(format!("broadcast_raw: {e}")))
}

/// Mutate one `SerializedBundle` field of a built shielded
/// [`StateTransition`] in place, before broadcast (SH-022/024/025/026/034).
///
/// The shielded transition V0 structs expose `actions` / `value_balance`
/// (or `unshielding_amount`) / `anchor` / `proof` / `binding_signature`
/// as public fields, so the tamper is a direct field write — no byte
/// offsets. The Orchard proof + binding signature are bound to these
/// public inputs, so any mutation yields a transition the BACKEND must
/// reject. Returns an error if `field` doesn't apply to the transition's
/// type (e.g. `ValueBalance` on an unshield, which carries
/// `unshielding_amount` instead — use [`BundleField::ValueBalance`] for
/// both; this maps it onto whichever field the variant has).
pub fn mutate_serialized_bundle(
    st: &mut dpp::state_transition::StateTransition,
    field: BundleField,
    mutation: &BundleMutation,
) -> FrameworkResult<()> {
    use dpp::state_transition::StateTransition;

    /// Apply `mutation` to a `Vec<u8>` field (proof).
    fn mutate_vec(buf: &mut Vec<u8>, m: &BundleMutation) {
        match m {
            BundleMutation::Overwrite(bytes) => *buf = bytes.clone(),
            BundleMutation::Zero => buf.iter_mut().for_each(|b| *b = 0),
            BundleMutation::FlipByte(i) => {
                if let Some(b) = buf.get_mut(*i) {
                    *b ^= 0xFF;
                }
            }
        }
    }
    /// Apply `mutation` to a fixed-size byte array field (anchor / sig).
    fn mutate_arr(buf: &mut [u8], m: &BundleMutation) {
        match m {
            BundleMutation::Overwrite(bytes) => {
                for (dst, src) in buf.iter_mut().zip(bytes.iter()) {
                    *dst = *src;
                }
            }
            BundleMutation::Zero => buf.iter_mut().for_each(|b| *b = 0),
            BundleMutation::FlipByte(i) => {
                if let Some(b) = buf.get_mut(*i) {
                    *b ^= 0xFF;
                }
            }
        }
    }

    macro_rules! tamper_v0 {
        ($v0:expr, $has_value_balance:tt) => {{
            match field {
                BundleField::Proof => mutate_vec(&mut $v0.proof, mutation),
                BundleField::BindingSignature => mutate_arr(&mut $v0.binding_signature, mutation),
                BundleField::Anchor => mutate_arr(&mut $v0.anchor, mutation),
                BundleField::ValueBalance => tamper_v0!(@value $v0, $has_value_balance),
            }
        }};
        (@value $v0:expr, value_balance) => {{
            // value_balance is u64; the overwrite's first 8 LE bytes set it.
            if let BundleMutation::Overwrite(bytes) = mutation {
                let mut le = [0u8; 8];
                for (d, s) in le.iter_mut().zip(bytes.iter()) {
                    *d = *s;
                }
                $v0.value_balance = u64::from_le_bytes(le);
            } else if matches!(mutation, BundleMutation::Zero) {
                $v0.value_balance = 0;
            } else {
                $v0.value_balance = $v0.value_balance.wrapping_add(1);
            }
        }};
        (@value $v0:expr, unshielding_amount) => {{
            if let BundleMutation::Overwrite(bytes) = mutation {
                let mut le = [0u8; 8];
                for (d, s) in le.iter_mut().zip(bytes.iter()) {
                    *d = *s;
                }
                $v0.unshielding_amount = u64::from_le_bytes(le);
            } else if matches!(mutation, BundleMutation::Zero) {
                $v0.unshielding_amount = 0;
            } else {
                $v0.unshielding_amount = $v0.unshielding_amount.wrapping_add(1);
            }
        }};
    }

    use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
    use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
    use dpp::state_transition::unshield_transition::UnshieldTransition;

    match st {
        StateTransition::Unshield(UnshieldTransition::V0(v0)) => {
            tamper_v0!(v0, unshielding_amount)
        }
        StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(v0)) => {
            tamper_v0!(v0, value_balance)
        }
        StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(v0)) => {
            tamper_v0!(v0, unshielding_amount)
        }
        other => {
            return Err(FrameworkError::Wallet(format!(
                "mutate_serialized_bundle: unsupported transition variant for tampering: {:?}",
                std::mem::discriminant(other)
            )));
        }
    }
    Ok(())
}

/// Build a real, valid Type-17 unshield [`StateTransition`] for `account`
/// against the wallet's synced notes WITHOUT broadcasting it — the shared
/// capture seam for the byte-tamper abuse cases (SH-022/024/025/026/034).
///
/// Reserves and selects notes via the production reservation path
/// (`test-utils` seam), then calls `operations::build_unshield_st`. The
/// reservation is intentionally NOT released: the abuse case discards the
/// transition after tampering, and the per-test coordinator is torn down,
/// so the in-memory pending mark is irrelevant.
pub async fn capture_unshield_st(
    wallet: &TestWallet,
    handle: &ShieldedHandle,
    account: u32,
    to_platform_addr: &dpp::address_funds::PlatformAddress,
    amount: u64,
) -> FrameworkResult<dpp::state_transition::StateTransition> {
    use platform_wallet::wallet::shielded::{operations, OrchardKeySet, SubwalletId};

    let pw = wallet.platform_wallet();
    let id = SubwalletId::new(pw.wallet_id(), account);
    let keyset = OrchardKeySet::from_seed(&wallet.seed_bytes(), pw.sdk().network, account)
        .map_err(|e| FrameworkError::Wallet(format!("capture_unshield_st: keyset: {e}")))?;

    let (selected, _total, exact_fee) = operations::test_utils::reserve_unspent_notes_for_test(
        &pw.sdk_arc(),
        handle.coordinator.store(),
        id,
        amount,
        1,
    )
    .await
    .map_err(|e| FrameworkError::Wallet(format!("capture_unshield_st: reserve: {e}")))?;

    operations::build_unshield_st(
        &pw.sdk_arc(),
        handle.coordinator.store(),
        &keyset,
        to_platform_addr,
        amount,
        exact_fee,
        &selected,
        &shielded_prover(),
    )
    .await
    .map_err(|e| FrameworkError::Wallet(format!("capture_unshield_st: build: {e}")))
}

/// All unspent notes for `account`, so an abuse case can capture a note
/// to build a second (double-spend / replay) or duplicated (intra-bundle)
/// transition against. Reads via the `test-utils` seam.
pub async fn unspent_notes(
    wallet: &TestWallet,
    handle: &ShieldedHandle,
    account: u32,
) -> FrameworkResult<Vec<platform_wallet::wallet::shielded::ShieldedNote>> {
    use platform_wallet::wallet::shielded::{operations, SubwalletId};
    let pw = wallet.platform_wallet();
    let id = SubwalletId::new(pw.wallet_id(), account);
    operations::test_utils::unspent_notes_for_test(handle.coordinator.store(), id)
        .await
        .map_err(|e| FrameworkError::Wallet(format!("unspent_notes: {e}")))
}

/// Build a Type-17 unshield [`StateTransition`] against a CHOSEN note set,
/// SKIPPING the reservation guard — the build-against-note seam for the
/// double-spend (SH-020), replay (SH-021), and intra-bundle-duplicate
/// (SH-033) abuse cases. The caller computes the fee
/// (`compute_minimum_shielded_fee`) since reservation (which derives it)
/// is bypassed.
pub async fn build_unshield_st_against_notes(
    wallet: &TestWallet,
    handle: &ShieldedHandle,
    account: u32,
    to_platform_addr: &dpp::address_funds::PlatformAddress,
    amount: u64,
    exact_fee: u64,
    notes: &[platform_wallet::wallet::shielded::ShieldedNote],
) -> FrameworkResult<dpp::state_transition::StateTransition> {
    use platform_wallet::wallet::shielded::{operations, OrchardKeySet};
    let pw = wallet.platform_wallet();
    let keyset = OrchardKeySet::from_seed(&wallet.seed_bytes(), pw.sdk().network, account)
        .map_err(|e| FrameworkError::Wallet(format!("build_unshield_st_against_notes: {e}")))?;
    operations::build_unshield_st(
        &pw.sdk_arc(),
        handle.coordinator.store(),
        &keyset,
        to_platform_addr,
        amount,
        exact_fee,
        notes,
        &shielded_prover(),
    )
    .await
    .map_err(|e| FrameworkError::Wallet(format!("build_unshield_st_against_notes: {e}")))
}

/// Inject a `ShieldedNote` with caller-controlled `note_data` / `cmx` /
/// `nullifier` into a store, for the serde-abuse SH-027. A malformed
/// `note_data` (≠115 bytes) must surface a typed error — never a panic —
/// when the spend path's `deserialize_note` reads it.
///
/// This seam IS achievable through the public `ShieldedStore` trait
/// (`save_note` + `append_commitment`), so it is wired live. Builds a
/// note that note-selection will pick (`value > 0`, unspent) but whose
/// `note_data` the caller controls.
pub async fn seed_malformed_note<S>(
    store: &Arc<tokio::sync::RwLock<S>>,
    id: platform_wallet::wallet::shielded::SubwalletId,
    value: u64,
    note_data: Vec<u8>,
    cmx: [u8; 32],
    nullifier: [u8; 32],
) -> FrameworkResult<()>
where
    S: platform_wallet::wallet::shielded::ShieldedStore,
{
    use platform_wallet::wallet::shielded::{ShieldedNote, ShieldedStore};
    let note = ShieldedNote {
        position: 0,
        cmx,
        nullifier,
        block_height: 0,
        is_spent: false,
        value,
        note_data,
    };
    let mut guard = store.write().await;
    guard
        .save_note(id, &note)
        .map_err(|e| FrameworkError::Wallet(format!("seed_malformed_note: save_note: {e}")))?;
    guard
        .append_commitment(&cmx, true)
        .map_err(|e| FrameworkError::Wallet(format!("seed_malformed_note: append: {e}")))?;
    Ok(())
}
