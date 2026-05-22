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
//! does NOT route through it. Instead [`bind_shielded`] builds a
//! per-test [`NetworkShieldedCoordinator`] directly over a fresh SQLite
//! file under the workdir slot. The commitment tree is network-shared
//! on-chain, but each test scans it into its own DB so two parallel
//! tests never share store state.
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
    CachedOrchardProver, FileBackedShieldedStore, InMemoryShieldedStore, NetworkShieldedCoordinator,
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
    /// Per-test FileBacked coordinator (one SQLite handle).
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

/// Build a per-test FileBacked coordinator and bind `accounts` on the
/// wallet's shielded sub-wallet.
///
/// Constructs a fresh SQLite tree under `<workdir>/shielded/<wallet>-<n>.sqlite`
/// — a unique path per call so parallel tests never share store state
/// (the on-chain tree is network-shared, but each test scans it into its
/// own DB). FileBacked is mandatory: the in-memory store's `witness()`
/// is a hard `Err` (Found-027), so spends against it cannot build a
/// proof (see SH-005).
///
/// Errors: [`FrameworkError::Wallet`] for store-open, coordinator, or
/// `bind_shielded` failures.
pub async fn bind_shielded(
    wallet: &TestWallet,
    accounts: &[u32],
    workdir: &std::path::Path,
) -> FrameworkResult<ShieldedHandle> {
    let coordinator = new_file_backed_coordinator(wallet, workdir).await?;
    let seed = wallet.seed_bytes();
    wallet
        .platform_wallet()
        .bind_shielded(&seed, accounts, &coordinator)
        .await
        .map_err(|e| FrameworkError::Wallet(format!("bind_shielded: {e}")))?;
    Ok(ShieldedHandle {
        coordinator,
        accounts: accounts.to_vec(),
    })
}

/// Construct a per-test FileBacked coordinator over a fresh SQLite path
/// WITHOUT binding — used by SH-007's controlled bind-ordering hook (the
/// coordinator's tree is advanced via `sync(true)` before the second
/// wallet binds).
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
}

// ---------------------------------------------------------------------------
// Adversarial injection hooks (SH-020..SH-035 — follow-up wave)
//
// These build now so the abuse pass can wire against them. They expose
// the protocol-boundary seam (raw build → byte-mutate → broadcast) that
// bypasses the guarded `PlatformWallet::shielded_*` methods. Live
// broadcasts are gated behind `adversarial_enabled()`.
// ---------------------------------------------------------------------------

/// Which shielded transition the raw builder should produce. The
/// follow-up wave maps each arm onto the matching
/// `dpp::shielded::builder::build_*_transition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawShieldedKind {
    /// Type 16 — shielded → shielded transfer.
    Transfer,
    /// Type 17 — unshield to a transparent address.
    Unshield,
    /// Type 19 — withdraw to a Core L1 address.
    Withdraw,
}

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

/// Production-gap marker for adversarial hooks that CANNOT reach Drive
/// with a properly-formed-but-tampered shielded transition because the
/// wallet exposes no seam to capture a built `SerializedBundle` / raw
/// spend bytes (see the module-level gap notes and the SH-020/022/024/
/// 025/026/033/034 case docs). A case hitting this is RED-by-gap: the
/// finding is the MISSING seam, not a weakened assertion.
pub const ADVERSARIAL_SEAM_MISSING: &str =
    "no public seam to capture a built shielded SerializedBundle / raw spend ST bytes — \
     shielded operations::* build AND broadcast internally (contrast transparent \
     transfer_capturing_st_bytes), and extract_spends_and_anchor / reserve_unspent_notes / \
     build_spend_bundle are private. Add a build-only shielded capture seam (returning the \
     serialized StateTransition before broadcast) to wire this abuse case to the backend.";

/// Build a raw shielded state transition from caller-supplied,
/// possibly-out-of-range inputs that the guarded wallet wrapper would
/// reject (output > input for SH-022, under-floor fee for SH-023,
/// `u64`/`i64` boundary for SH-024, duplicate spend for SH-033, stale
/// anchor for SH-026).
///
/// **Blocked by [`ADVERSARIAL_SEAM_MISSING`].** Constructing a valid-
/// except-for-the-tamper transition requires real `SpendableNote`s + an
/// `Anchor` from the wallet's private `extract_spends_and_anchor`, and
/// the public dpp `build_*_transition` enforce the value/fee/overflow
/// guards internally — so neither path can emit the out-of-range bundle
/// these cases need. The signature pins the inputs the abuse cases want.
#[allow(clippy::too_many_arguments)]
pub fn build_raw_shielded_transition(
    _kind: RawShieldedKind,
    _anchor: [u8; 32],
    _value_balance: i64,
    _fee: Option<u64>,
    _proof_override: Option<Vec<u8>>,
) -> FrameworkResult<Vec<u8>> {
    Err(FrameworkError::NotImplemented(
        "build_raw_shielded_transition: see ADVERSARIAL_SEAM_MISSING",
    ))
}

/// Broadcast arbitrary serialized [`StateTransition`] bytes directly,
/// returning the typed backend error so an abuse case can assert the
/// exact rejection variant. Bypasses the guarded `shielded_*` methods.
///
/// Gated: refuses unless [`adversarial_enabled`], so a stray malformed
/// broadcast can't pollute a normal functional run. The seam itself is
/// real — `StateTransition::deserialize_from_bytes` + `broadcast`
/// (the same path PA-006 replays through).
pub async fn broadcast_raw(
    sdk: &Arc<dash_sdk::Sdk>,
    state_transition_bytes: &[u8],
) -> FrameworkResult<()> {
    use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
    use dpp::serialization::PlatformDeserializable;
    use dpp::state_transition::StateTransition;

    if !adversarial_enabled() {
        return Err(FrameworkError::Config(format!(
            "broadcast_raw refused: set {ADVERSARIAL_GATE_ENV} to run the abuse pass"
        )));
    }
    let st = StateTransition::deserialize_from_bytes(state_transition_bytes)
        .map_err(|e| FrameworkError::Wallet(format!("broadcast_raw: deserialize ST: {e}")))?;
    st.broadcast(sdk.as_ref(), None)
        .await
        .map_err(|e| FrameworkError::Sdk(format!("broadcast_raw: {e}")))
}

/// Flip / truncate / zero bytes in a built transition's serialized
/// `SerializedBundle` field before broadcast (SH-022/024/025/026/034).
///
/// **Blocked by [`ADVERSARIAL_SEAM_MISSING`].** Operates on a captured
/// valid-build's bytes, which the wallet does not expose.
pub fn mutate_serialized_bundle(
    _bytes: &mut [u8],
    _field: BundleField,
    _mutation: BundleMutation,
) -> FrameworkResult<()> {
    Err(FrameworkError::NotImplemented(
        "mutate_serialized_bundle: see ADVERSARIAL_SEAM_MISSING",
    ))
}

/// Build a spend directly against a chosen note WITHOUT going through
/// `reserve_unspent_notes`, for the double-spend (SH-020) and replay
/// (SH-021) arms.
///
/// **Blocked by [`ADVERSARIAL_SEAM_MISSING`].** Requires the private
/// `extract_spends_and_anchor` + `build_spend_bundle`.
pub fn build_against_note() -> FrameworkResult<Vec<u8>> {
    Err(FrameworkError::NotImplemented(
        "build_against_note: see ADVERSARIAL_SEAM_MISSING",
    ))
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

/// Resubmit a captured single-use asset-lock proof, for SH-035
/// (Core-L1 gated).
///
/// **Blocked by [`ADVERSARIAL_SEAM_MISSING`]** plus the SH-018 Core-L1
/// asset-lock private-key gap (no test seam returns the one-time key).
pub fn reuse_asset_lock_proof() -> FrameworkResult<()> {
    Err(FrameworkError::NotImplemented(
        "reuse_asset_lock_proof: see ADVERSARIAL_SEAM_MISSING + SH-018 Core-L1 key gap",
    ))
}

/// A scriptable mock sync source for SH-028 (interrupt mid-chunk) and
/// SH-029 (reorg / out-of-order / rescan-from-0). Holds scripted note
/// chunks plus a cancellation flag the test flips to interrupt a pass.
///
/// Seam reserved for the follow-up wave; the type exists now so the
/// abuse cases can be authored against a stable handle.
#[derive(Default)]
pub struct MockSyncSource {
    /// Scripted chunks the source will yield, in order. Each inner Vec
    /// is one chunk's worth of opaque note bytes.
    pub chunks: Vec<Vec<Vec<u8>>>,
    /// Set by the test to interrupt the next chunk (SH-028).
    pub cancel_after_chunk: Option<usize>,
}

impl MockSyncSource {
    /// Trip the cancellation flag so the next pass stops after
    /// `chunk_index` (SH-028's mid-chunk interrupt).
    pub fn cancel_after(&mut self, chunk_index: usize) {
        self.cancel_after_chunk = Some(chunk_index);
    }
}
