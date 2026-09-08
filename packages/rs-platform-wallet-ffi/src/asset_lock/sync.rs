//! FFI bindings for asset lock sync/resume operations.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use platform_wallet::PlatformWalletError;
use std::ffi::CString;
use std::os::raw::c_char;
use std::time::Duration;

/// Largest bound a caller may request, in seconds (one year).
///
/// The wait paths downstream build their deadline as
/// `Instant::now() + timeout`, which **panics** as soon as the resulting
/// instant is not representable — and a panic raised inside an
/// `extern "C"` frame aborts the host process rather than returning a
/// [`PlatformWalletFFIResult`]. `timeout_secs` arrives as an unrestricted
/// `u64`, so a caller passing `UInt64.max` (or any large sentinel) would
/// take the host down. Clamping is preferable to rejecting: every value
/// past this point already means "effectively forever" to a mobile host
/// that will not survive the wait anyway.
const MAX_TIMEOUT_SECS: u64 = 365 * 24 * 60 * 60;

/// Convert an FFI `timeout_secs` into the `Option<Duration>` the resume
/// path takes.
///
/// `0` declines to specify a bound — `resume_asset_lock` reads the
/// resulting `None` as "apply the recovery policy's own default" (see
/// the `# Timeouts` section on [`asset_lock_manager_resume`]). Anything
/// larger than [`MAX_TIMEOUT_SECS`] is clamped so the deadline
/// arithmetic downstream stays representable.
fn resume_timeout(timeout_secs: u64) -> Option<Duration> {
    (timeout_secs != 0).then(|| Duration::from_secs(timeout_secs.min(MAX_TIMEOUT_SECS)))
}

/// Records the bounds handed to `resume_asset_lock` on this thread.
///
/// The bound is otherwise unobservable from outside: it is consumed deep
/// inside the async resume, behind a manager handle, on a path that by
/// definition does not return until it expires. Recording it is what lets
/// the clamp be pinned on the **exported** paths rather than on the private
/// conversion helper — an entry point that converts `timeout_secs` some
/// other way, or drops the converted bound on its way to the manager,
/// records something different and fails the pins below instead of quietly
/// restoring the host abort an unrepresentable deadline causes in an
/// `extern "C"` frame.
///
/// Thread-local, so each test owns the record of the calls it made.
#[cfg(test)]
mod timeout_probe {
    use std::cell::RefCell;
    use std::time::Duration;

    thread_local! {
        static RECORDED: RefCell<Vec<Option<Duration>>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn record(timeout: Option<Duration>) {
        RECORDED.with(|recorded| recorded.borrow_mut().push(timeout));
    }

    /// Everything recorded on this thread since the last take, in call order.
    pub(super) fn take() -> Vec<Option<Duration>> {
        RECORDED.with(|recorded| recorded.borrow_mut().drain(..).collect())
    }
}

/// Pass-through that records the bound under `cfg(test)`.
///
/// It stands **in the argument position** of the `resume_asset_lock` calls
/// below rather than beside them, and that placement is the whole point: a
/// recording statement next to the call still records the right value when
/// the call itself is handed a different one, so the pin passes while the
/// clamp is gone. Being the argument, what is recorded is what is consumed
/// — there is no value in between to diverge.
#[cfg(test)]
fn forwarded(timeout: Option<Duration>) -> Option<Duration> {
    timeout_probe::record(timeout);
    timeout
}

/// Production twin of the `cfg(test)` recorder above: the identity, inlined
/// away. No ABI, no state, no branch.
#[cfg(not(test))]
#[inline(always)]
fn forwarded(timeout: Option<Duration>) -> Option<Duration> {
    timeout
}

/// Build an `OutPoint` from a 32-byte raw txid pointer and a vout.
///
/// **FFI invariant:** the `txid` parameter is typed `*const [u8; 32]`,
/// not `*const u8`. The `.expect("txid is 32 bytes")` below is sound
/// *only* because the type system pins the length. If a future caller
/// weakens the signature to `*const u8 + usize_len`, this panic
/// becomes reachable across the FFI boundary and must be replaced
/// with a fallible return. Don't relax the signature without
/// hardening the body.
fn parse_outpoint(txid: *const [u8; 32], vout: u32) -> dashcore::OutPoint {
    use dashcore::hashes::Hash;
    let txid_bytes = unsafe { *txid };
    dashcore::OutPoint {
        txid: dashcore::Txid::from_slice(&txid_bytes).expect("txid is 32 bytes"),
        vout,
    }
}

/// Resume a tracked asset lock from whatever stage it's at.
///
/// On success:
/// - `out_proof_bytes`/`out_proof_len`: bincode-encoded AssetLockProof
/// - `out_derivation_path`: NUL-terminated C string with the
///   credit-output derivation path (free with
///   `platform_wallet_string_free`).
///
/// Free proof bytes with `asset_lock_manager_free_proof_bytes`.
///
/// Unlike `asset_lock_manager_create_funded_proof`, this entry point
/// does **not** take a core signer handle — the resume path only
/// re-derives the proof and the credit-output derivation path from
/// the already-tracked lock state; signing the consume transition is
/// the next stage's responsibility (e.g.
/// [`crate::platform_wallet_register_identity_with_funding_signer`]).
///
/// # Timeouts
///
/// `timeout_secs` bounds only the stages that still have to WAIT for a
/// proof: a `Built` / `Broadcast` row whose local record does not
/// already hold finality, plus the defensive proof-less
/// `RecoveredFromChain` fallback. Every other resume — a row carrying
/// an `InstantSendLocked` / `ChainLocked` proof, and a `Built` /
/// `Broadcast` row the local finality probe settles before any
/// transport work — returns without ever consulting it.
///
/// `timeout_secs == 0` does **not** request an unbounded wait — it
/// declines to specify one, and `resume_asset_lock` then applies the
/// recovery policy's own default: the 180s
/// `UNCONFIRMED_BROADCAST_PROOF_TIMEOUT` (sized to comfortably cover a
/// ~2.5min ChainLock) on every arm that waits for a proof — a `Built`
/// re-broadcast whatever the broadcaster answered, a `Broadcast` row,
/// and the defensive proof-less `RecoveredFromChain` fallback alike.
///
/// A resume cannot gather evidence that rules out a wait which never
/// ends. Even a positively ACCEPTED (`Ok`) re-broadcast only
/// establishes that the transaction reached the network: a sibling
/// spending the same outpoint may confirm the instant afterwards, and
/// from then on no proof for this transaction can arrive. The wait
/// cannot see that happen — it wakes on lock events and re-reads the
/// tracked funding transaction only — so an unbounded wait is a
/// `Notify` loop with no terminating event, which under the
/// `runtime().block_on(...)` below pins the calling host thread
/// permanently rather than merely delaying an answer.
///
/// Expiry is non-destructive: the tracked row keeps its status, so a
/// proof arriving afterwards is returned by the very next resume
/// straight from the record, without waiting at all. On the `Built` /
/// `Broadcast` arms the expiry surfaces as
/// `TransactionBroadcastUnconfirmed`.
///
/// A `timeout_secs` in `1..=31_536_000` (one year) keeps its exact
/// semantics, `FinalityTimeout` included — the substitution above is
/// gated on the caller having declined to choose. Anything larger is
/// silently CLAMPED to one year rather than honoured or rejected: the
/// wait paths downstream build their deadline as `Instant::now() +
/// timeout` and panic on an unrepresentable instant, which in an
/// `extern "C"` frame aborts the host process. A caller passing
/// `UInt64.max` as an "effectively forever" sentinel therefore gets one
/// year, not forever.
#[no_mangle]
pub unsafe extern "C" fn asset_lock_manager_resume(
    handle: Handle,
    txid: *const [u8; 32],
    vout: u32,
    timeout_secs: u64,
    out_proof_bytes: *mut *mut u8,
    out_proof_len: *mut usize,
    out_derivation_path: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(txid);
    check_ptr!(out_proof_bytes);
    check_ptr!(out_proof_len);
    check_ptr!(out_derivation_path);

    let out_point = parse_outpoint(txid, vout);
    let timeout = resume_timeout(timeout_secs);

    let option = ASSET_LOCK_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.resume_asset_lock(&out_point, forwarded(timeout)))
    });
    let result = unwrap_option_or_return!(option);
    let (proof, path) = unwrap_result_or_return!(result);
    let bytes = unwrap_result_or_return!(dpp::bincode::encode_to_vec(
        &proof,
        dpp::bincode::config::standard()
    ));
    let len = bytes.len();
    let boxed = bytes.into_boxed_slice();
    *out_proof_bytes = Box::into_raw(boxed) as *mut u8;
    *out_proof_len = len;
    let path_c = unwrap_result_or_return!(CString::new(path.to_string()));
    *out_derivation_path = path_c.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Fire-and-forget variant of [`asset_lock_manager_resume`] for the
/// app-launch / app-foreground catch-up flow.
///
/// Calls `resume_asset_lock` internally and drops the returned
/// `(proof, derivation_path)` tuple — the chain-lock cascade that
/// `wait_for_proof` drives also queues an `AssetLockChangeSet` via
/// `advance_asset_lock_status` that writes `statusRaw = 3 +
/// proofBytes` back to SwiftData, which is the only payload the UI
/// needs. There's no proof or path to plumb out, so the FFI surface
/// stays free of out-params (and the matching free callbacks).
///
/// Returns `ok` on a successful proof resolution, an error on
/// timeout / wait failure. The Swift caller is expected to schedule
/// this on a background queue — `runtime().block_on(...)` parks the
/// calling thread for the duration of the wait.
///
/// # Timeouts
///
/// Identical contract to [`asset_lock_manager_resume`], which this
/// delegates to: `timeout_secs == 0` selects the recovery policy's
/// default rather than an unbounded wait. That default is the 180s
/// `UNCONFIRMED_BROADCAST_PROOF_TIMEOUT`, and it applies to every arm
/// that waits for a proof — a `Built` re-broadcast whatever the
/// broadcaster answered, a `Broadcast` row, the defensive proof-less
/// `RecoveredFromChain` fallback. Pass a `timeout_secs` in
/// `1..=31_536_000` (one year) for a different upper bound; larger
/// values are clamped to one year, because the deadline arithmetic
/// downstream panics on an unrepresentable instant and that panic
/// aborts the host process from an `extern "C"` frame.
///
/// That policy is what makes this entry point safe to fan out at
/// launch. The catch-up sweep starts one call per stuck lock; when
/// zero meant "wait forever", a device that was offline (or an SPV
/// session that never connected), and equally a lock whose outpoint a
/// sibling transaction had already taken, turned each of those into a
/// permanently parked worker thread. Every arm is bounded now: expiry
/// simply ends the pass, leaving the row tracked and resumable, and
/// the next sweep picks up a proof that landed in between straight
/// from the record.
#[no_mangle]
pub unsafe extern "C" fn asset_lock_manager_catch_up_blocking(
    handle: Handle,
    txid: *const [u8; 32],
    vout: u32,
    timeout_secs: u64,
) -> PlatformWalletFFIResult {
    check_ptr!(txid);

    let out_point = parse_outpoint(txid, vout);
    let timeout = resume_timeout(timeout_secs);

    tracing::info!(
        outpoint = %out_point,
        timeout_secs,
        "asset_lock_manager_catch_up_blocking: entered"
    );

    let option = ASSET_LOCK_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.resume_asset_lock(&out_point, forwarded(timeout)))
    });
    let result = match option {
        Some(r) => r,
        None => {
            tracing::warn!(
                outpoint = %out_point,
                "asset_lock_manager_catch_up_blocking: invalid manager handle"
            );
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidHandle,
                "Manager handle invalid".to_string(),
            );
        }
    };
    match result {
        Ok(_) => {
            tracing::info!(
                outpoint = %out_point,
                "asset_lock_manager_catch_up_blocking: resume_asset_lock succeeded"
            );
            PlatformWalletFFIResult::ok()
        }
        Err(e) => {
            tracing::warn!(
                outpoint = %out_point,
                error = %e,
                "asset_lock_manager_catch_up_blocking: resume_asset_lock failed"
            );
            match e {
                // Double-spend verdicts route through the typed conversion
                // so the host receives the real code. In practice that is
                // always the provisional ErrorAssetLockInputContested
                // (48), which bounds the wait but keeps the lock for a
                // later retry: the resume never raises the terminal
                // ErrorAssetLockInputConflict (47), which stays reserved
                // for a finalized-ancestry proof the wallet cannot make.
                // 47 is matched anyway so the reserved code would cross
                // intact rather than flattening the day it ships.
                // Flattening either to ErrorWalletOperation would leave
                // the host with a spinner it can never resolve.
                conflict @ (PlatformWalletError::AssetLockInputConflict { .. }
                | PlatformWalletError::AssetLockInputContested { .. }) => conflict.into(),
                other => PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorWalletOperation,
                    format!("{}", other),
                ),
            }
        }
    }
}

/// Recover a tracked asset lock from a serialized transaction.
///
/// Re-tracks the asset lock in memory so it can be resumed later.
/// The transaction must be a valid asset lock transaction with a
/// special transaction payload.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn asset_lock_manager_recover(
    handle: Handle,
    tx_bytes: *const u8,
    tx_bytes_len: usize,
    amount_duffs: u64,
    account_index: u32,
    funding_type: u32,
    identity_index: u32,
    txid: *const [u8; 32],
    vout: u32,
    proof_bytes: *const u8,
    proof_len: usize,
) -> PlatformWalletFFIResult {
    check_ptr!(tx_bytes);
    check_ptr!(txid);

    // Parse transaction
    let tx_data = std::slice::from_raw_parts(tx_bytes, tx_bytes_len);
    let tx: dashcore::Transaction =
        unwrap_result_or_return!(dashcore::consensus::deserialize(tx_data));

    let funding = unwrap_option_or_return!(super::build::parse_funding_type(funding_type));

    let out_point = parse_outpoint(txid, vout);

    // Parse optional proof
    let proof = if !proof_bytes.is_null() && proof_len > 0 {
        let data = std::slice::from_raw_parts(proof_bytes, proof_len);
        let (p, _) = unwrap_result_or_return!(dpp::bincode::decode_from_slice(
            data,
            dpp::bincode::config::standard()
        ));
        Some(p)
    } else {
        None
    };

    let option = ASSET_LOCK_MANAGER_STORAGE.with_item(handle, |manager| {
        manager.recover_asset_lock_blocking(
            tx,
            amount_duffs,
            account_index,
            funding,
            identity_index,
            out_point,
            proof,
        );
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::{
        asset_lock_manager_catch_up_blocking, asset_lock_manager_resume, resume_timeout,
        timeout_probe, MAX_TIMEOUT_SECS,
    };
    use crate::error::PlatformWalletFFIResultCode;
    use crate::handle::{Handle, ASSET_LOCK_MANAGER_STORAGE};
    use crate::runtime::runtime;
    use platform_wallet::test_support::{test_platform_wallet_manager, NoopTestPersister};
    use platform_wallet::PlatformWalletManager;
    use std::os::raw::c_char;
    use std::sync::Arc;
    use std::time::Duration;

    /// The clamp both exported paths must apply.
    fn one_year() -> Option<Duration> {
        Some(Duration::from_secs(MAX_TIMEOUT_SECS))
    }

    /// A handle over a real, wallet-backed `AssetLockManager`.
    ///
    /// A live manager is what makes these pins bite. `HandleStorage::with_item`
    /// runs its closure only for a handle it finds, so an absent handle returns
    /// before the entry point ever reaches `resume_asset_lock` — and a pin that
    /// observes the bound short of that call cannot tell a forwarded bound from
    /// a dropped one. The wallet tracks no asset locks, so the resume looks the
    /// outpoint up, fails `AssetLockNotTracked` and returns without waiting on
    /// anything.
    ///
    /// The returned manager owns the registered wallet and must be kept alive
    /// for the duration of the call.
    fn live_manager_handle() -> (Arc<PlatformWalletManager<NoopTestPersister>>, Handle) {
        runtime().block_on(async {
            let (manager, wallet_id) = test_platform_wallet_manager().await;
            let wallet = manager
                .get_wallet(&wallet_id)
                .await
                .expect("the manager just registered this wallet");
            let handle = ASSET_LOCK_MANAGER_STORAGE.insert(Arc::clone(wallet.asset_locks()));
            (manager, handle)
        })
    }

    /// Drive [`asset_lock_manager_resume`] against a live manager and report
    /// the bound it forwarded alongside its result code.
    ///
    /// Called from the plain test thread: the entry point does its own
    /// `runtime().block_on(...)`, exactly as the host thread does, and nesting
    /// that inside an outer `block_on` would abort.
    fn resume_extern_with(
        handle: Handle,
        timeout_secs: u64,
    ) -> (PlatformWalletFFIResultCode, Vec<Option<Duration>>) {
        let _ = timeout_probe::take();
        let txid = [7u8; 32];
        let mut proof_bytes: *mut u8 = std::ptr::null_mut();
        let mut proof_len: usize = 0;
        let mut derivation_path: *mut c_char = std::ptr::null_mut();

        let result = unsafe {
            asset_lock_manager_resume(
                handle,
                &txid,
                0,
                timeout_secs,
                &mut proof_bytes,
                &mut proof_len,
                &mut derivation_path,
            )
        };

        (result.code, timeout_probe::take())
    }

    /// Drive [`asset_lock_manager_catch_up_blocking`], as above.
    fn catch_up_extern_with(
        handle: Handle,
        timeout_secs: u64,
    ) -> (PlatformWalletFFIResultCode, Vec<Option<Duration>>) {
        let _ = timeout_probe::take();
        let txid = [7u8; 32];

        let result =
            unsafe { asset_lock_manager_catch_up_blocking(handle, &txid, 0, timeout_secs) };

        (result.code, timeout_probe::take())
    }

    /// The clamp has to hold on the ABI itself, not just in the helper.
    ///
    /// `asset_lock_manager_resume` is one of the two symbols a host can
    /// actually reach, and the conversion helper is private: an entry point
    /// that stops routing `timeout_secs` through it — or that grows a third
    /// conversion of its own, or hands the manager something else entirely —
    /// restores the host abort while every helper test stays green. So
    /// exercise the exported symbol with the sentinel hosts really pass
    /// (`UInt64.max`), against a manager that actually consumes the bound, and
    /// pin what arrives there.
    #[test]
    fn the_exported_resume_clamps_an_extreme_timeout() {
        let (_manager, handle) = live_manager_handle();

        let (code, recorded) = resume_extern_with(handle, u64::MAX);

        assert_eq!(
            code,
            PlatformWalletFFIResultCode::ErrorAssetLockNotTracked,
            "the resume must have run — this code comes from inside \
             `resume_asset_lock`, past the handle lookup, so the bound below \
             is one the manager was really called with"
        );
        assert_eq!(
            recorded,
            vec![one_year()],
            "the exported resume must hand the manager `UInt64.max` clamped to \
             one year"
        );
        let bound = recorded[0].expect("a non-zero request is bounded");
        // The arithmetic downstream. Aborts the host, from an `extern "C"`
        // frame, on an unrepresentable instant.
        let _deadline = std::time::Instant::now() + bound;

        ASSET_LOCK_MANAGER_STORAGE.remove(handle);
    }

    /// Same contract on the launch catch-up symbol, which is the one the
    /// hosts fan out at app start — and therefore the one that takes the
    /// process down if its bound is unrepresentable.
    #[test]
    fn the_exported_catch_up_clamps_an_extreme_timeout() {
        let (_manager, handle) = live_manager_handle();

        let (code, recorded) = catch_up_extern_with(handle, u64::MAX);

        assert_eq!(
            code,
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "the catch-up maps every resume failure but a double-spend verdict \
             to this one code, so this proves the resume ran rather than the \
             handle lookup missing"
        );
        assert_eq!(
            recorded,
            vec![one_year()],
            "the exported catch-up must hand the manager `UInt64.max` clamped \
             to one year"
        );
        let bound = recorded[0].expect("a non-zero request is bounded");
        let _deadline = std::time::Instant::now() + bound;

        ASSET_LOCK_MANAGER_STORAGE.remove(handle);
    }

    /// Neither exported path may reshape the values a host legitimately
    /// passes: `0` still declines to specify a bound (and draws the recovery
    /// policy's own default downstream), and the 300s the launch catch-up
    /// asks for arrives intact.
    #[test]
    fn the_exported_paths_leave_ordinary_timeouts_alone() {
        let (_manager, handle) = live_manager_handle();

        assert_eq!(resume_extern_with(handle, 0).1, vec![None]);
        assert_eq!(catch_up_extern_with(handle, 0).1, vec![None]);
        assert_eq!(
            resume_extern_with(handle, 300).1,
            vec![Some(Duration::from_secs(300))]
        );
        assert_eq!(
            catch_up_extern_with(handle, 300).1,
            vec![Some(Duration::from_secs(300))]
        );

        ASSET_LOCK_MANAGER_STORAGE.remove(handle);
    }

    /// `timeout_secs` is an unrestricted `u64` on both public resume entry
    /// points. The wait paths it feeds build their deadline as
    /// `Instant::now() + timeout`, which panics once that instant is not
    /// representable — and a panic in an `extern "C"` frame aborts the host
    /// process instead of returning a result code. A caller passing
    /// `UInt64.max` must therefore get a long wait, not a crashed app.
    #[test]
    fn an_extreme_timeout_stays_a_representable_deadline() {
        let timeout = resume_timeout(u64::MAX).expect("a non-zero request is bounded");

        // The arithmetic the resume path performs on this value. Panics —
        // and so aborts the host — on an unrepresentable instant.
        let _deadline = std::time::Instant::now() + timeout;
        assert_eq!(timeout, Duration::from_secs(MAX_TIMEOUT_SECS));
    }

    /// Zero keeps its meaning: it declines to specify a bound and lets the
    /// recovery policy pick its own default. It is NOT a
    /// request for a zero-length wait, which would expire every proof wait
    /// instantly.
    #[test]
    fn zero_declines_to_specify_a_bound() {
        assert_eq!(resume_timeout(0), None);
    }

    /// An ordinary request passes through untouched — the clamp must not
    /// quietly reshape the 300s ceiling the launch catch-up asks for.
    #[test]
    fn an_ordinary_timeout_passes_through_unchanged() {
        assert_eq!(resume_timeout(300), Some(Duration::from_secs(300)));
    }
}
