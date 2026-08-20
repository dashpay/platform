//! FFI bindings for platform address withdrawal operations.

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};
use dash_sdk::dapi_client::CanRetry;
use dpp::identity::core_script::CoreScript;
use platform_wallet::PlatformWalletError;
use rs_sdk_ffi::{SignerHandle, VTableSigner};
use std::os::raw::c_char;
use std::str::FromStr;

use super::parse_input_selection;
use crate::runtime::block_on_worker;

/// Withdraw platform credits to a Core L1 address.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_address_wallet_withdraw(
    handle: Handle,
    account_index: u32,
    input_type: InputSelectionType,
    explicit_inputs: *const ExplicitInputFFI,
    explicit_inputs_count: usize,
    nonce_inputs: *const ExplicitInputWithNonceFFI,
    nonce_inputs_count: usize,
    output_script: *const u8,
    output_script_len: usize,
    core_fee_per_byte: u32,
    fee_strategy: *const FeeStrategyStepFFI,
    fee_strategy_count: usize,
    signer_address_handle: *mut SignerHandle,
    out_changeset: *mut PlatformAddressChangeSetFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_changeset);
    // Sentinel first: input parsing, the wallet lookup, and the async
    // withdraw below are all fallible. See
    // `PlatformAddressChangeSetFFI::empty` for the double-free rationale.
    *out_changeset = PlatformAddressChangeSetFFI::empty();
    check_ptr!(output_script);
    check_ptr!(signer_address_handle);

    let script_bytes = std::slice::from_raw_parts(output_script, output_script_len);
    let core_script = CoreScript::from_bytes(script_bytes.to_vec());

    let input_selection = unwrap_result_or_return!(parse_input_selection(
        input_type,
        explicit_inputs,
        explicit_inputs_count,
        nonce_inputs,
        nonce_inputs_count,
    ));

    let fee = parse_fee_strategy(fee_strategy, fee_strategy_count);

    // Clone the wallet out of handle storage so the read lock is released
    // before the long-running withdraw, then poll on a worker thread
    // (8 MB stack): the withdraw future verifies the execution proof, and
    // GroveDB proof verification recurses past the ~512 KB stacks of iOS
    // dispatch / Swift-concurrency threads (see runtime.rs) — polling it
    // on the calling thread crashes with EXC_BAD_ACCESS after the funds
    // already moved on-chain. Round-trip the signer pointer through
    // `usize` so the future's capture is `Send + 'static`; the caller
    // guarantees the handle outlives this synchronously-awaited call.
    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| wallet.clone());
    let wallet = unwrap_option_or_return!(option);
    let signer_addr = signer_address_handle as usize;
    let result = block_on_worker(async move {
        let address_signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
        wallet
            .withdraw(
                account_index,
                input_selection,
                core_script,
                core_fee_per_byte,
                fee,
                None,
                address_signer,
            )
            .await
    });
    let changeset = unwrap_result_or_return!(result);
    *out_changeset = PlatformAddressChangeSetFFI::from(&changeset);
    PlatformWalletFFIResult::ok()
}

/// Withdraw platform credits to a Core L1 address given as a base58
/// string (e.g. `yXV…` on testnet / `X…` on mainnet).
///
/// Sibling of [`platform_address_wallet_withdraw`] that accepts a
/// human-facing Core address instead of a pre-built `output_script`
/// byte buffer. The address is parsed and **network-checked against
/// the wallet's own network** entirely on the Rust side — a
/// testnet-shaped address can never be withdrawn to on a mainnet
/// wallet (and vice versa). The resulting P2PKH/P2SH `script_pubkey`
/// is then handed to the same `wallet.withdraw(...)` entry point, so
/// input selection, fee strategy, and signing are identical to the
/// raw-script path.
///
/// `signer_address_handle` is a `*mut SignerHandle` produced by
/// `dash_sdk_signer_create_with_ctx` (e.g. via `KeychainSigner.handle`)
/// and is consumed as `Signer<PlatformAddress>` for each input
/// address. The caller retains ownership of the handle; this function
/// does NOT destroy it.
///
/// Free result with `platform_address_wallet_free_changeset`.
///
/// # Safety
/// - `core_address` must be a valid, non-null, NUL-terminated C string.
/// - `signer_address_handle` must be a valid, non-destroyed
///   `*mut SignerHandle` that outlives this call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_address_wallet_withdraw_to_address(
    handle: Handle,
    account_index: u32,
    input_type: InputSelectionType,
    explicit_inputs: *const ExplicitInputFFI,
    explicit_inputs_count: usize,
    nonce_inputs: *const ExplicitInputWithNonceFFI,
    nonce_inputs_count: usize,
    core_address: *const c_char,
    core_fee_per_byte: u32,
    fee_strategy: *const FeeStrategyStepFFI,
    fee_strategy_count: usize,
    signer_address_handle: *mut SignerHandle,
    out_changeset: *mut PlatformAddressChangeSetFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_changeset);
    // Sentinel first — address parsing, the `ErrorInvalidNetwork` path, and
    // the async withdraw are all reachable from a caller-controlled address
    // string, so publish the empty changeset before them (and before the
    // `core_address` / signer null checks below) to keep every early return
    // FFI-safe. See `PlatformAddressChangeSetFFI::empty` for the double-free
    // rationale.
    *out_changeset = PlatformAddressChangeSetFFI::empty();
    check_ptr!(core_address);
    check_ptr!(signer_address_handle);

    let address_str = unwrap_result_or_return!(std::ffi::CStr::from_ptr(core_address).to_str());
    // Parse the address as network-unchecked first; the network is
    // pulled from the wallet (not threaded as a parameter, which would
    // be ambiguous if the two disagreed) and enforced below.
    let unchecked_address = unwrap_result_or_return!(dashcore::Address::from_str(address_str));

    let input_selection = unwrap_result_or_return!(parse_input_selection(
        input_type,
        explicit_inputs,
        explicit_inputs_count,
        nonce_inputs,
        nonce_inputs_count,
    ));

    let fee = parse_fee_strategy(fee_strategy, fee_strategy_count);

    // Clone the wallet out of handle storage so the read lock is released
    // before the network check + long-running withdraw.
    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| wallet.clone());
    let wallet = unwrap_option_or_return!(option);

    // Network check: reject an address that doesn't belong to the
    // wallet's network before any signing or submission happens.
    // Mirrors the `require_network` precedent used elsewhere in the
    // FFI for Core-address handling. `require_network` consumes the
    // unchecked address, which isn't reused afterwards. Surfaced as the
    // dedicated `ErrorInvalidNetwork` code instead of flattening to
    // `ErrorUnknown` via the blanket `From<PlatformWalletError>` impl.
    let checked_address = match unchecked_address.require_network(wallet.network()) {
        Ok(a) => a,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidNetwork,
                format!(
                    "Core address is not valid for the wallet's network ({:?}): {e}",
                    wallet.network()
                ),
            );
        }
    };
    let core_script = CoreScript::new(checked_address.script_pubkey());

    // Poll on a worker thread (8 MB stack): the withdraw future verifies
    // the execution proof, and GroveDB proof verification recurses past
    // the ~512 KB stacks of iOS dispatch / Swift-concurrency threads (see
    // runtime.rs) — polling it on the calling thread crashes with
    // EXC_BAD_ACCESS after the funds already moved on-chain. Round-trip
    // the signer pointer through `usize` so the future's capture is
    // `Send + 'static`; the caller guarantees the handle outlives this
    // synchronously-awaited call.
    let signer_addr = signer_address_handle as usize;
    let result = block_on_worker(async move {
        let address_signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
        wallet
            .withdraw(
                account_index,
                input_selection,
                core_script,
                core_fee_per_byte,
                fee,
                None,
                address_signer,
            )
            .await
    });
    let changeset = unwrap_result_or_return!(result);
    *out_changeset = PlatformAddressChangeSetFFI::from(&changeset);
    PlatformWalletFFIResult::ok()
}

/// How `platform_address_wallet_preflight_withdrawal` should present a
/// `PlatformWalletError` returned by the planner.
///
/// Extracted from the FFI entry point so the exact error→outcome mapping is
/// unit-testable without a wallet handle or a mock SDK (the classification is
/// the part that carries the retryability policy; see
/// [`classify_preflight_error`]).
#[derive(Debug, PartialEq, Eq)]
enum PreflightOutcome {
    /// Not an FFI error: write `can_withdraw = false` (zeroed figures) and
    /// return a Success-coded result whose message is the error's `Display`.
    /// Covers both the permanent "can't fund" states and transient, RETRYABLE
    /// "can't confirm right now" failures.
    DisabledWithReason,
    /// A genuine fault: leave `out` untouched and return the mapped FFI error
    /// code (via the blanket `From<PlatformWalletError>`), so callers can
    /// surface/monitor it through `throwIfError()`.
    Structural,
}

/// Classify a planner error into the FFI outcome the preflight entry point
/// should present.
///
/// * `OnlyDustInputs` / `AddressOperation` — the planner's typed "can't fund"
///   states (dust-only, fee/per-input/minimum headroom, too-many-inputs,
///   above-maximum, no funded addresses) → [`PreflightOutcome::DisabledWithReason`].
/// * `Sdk(e)` where `e.can_retry()` — the balance query
///   (`AddressInfo::fetch_many`) failed transiently (stale node, timeout, or
///   proof-verification error, per [`CanRetry::can_retry`]); a retry could
///   clear it → [`PreflightOutcome::DisabledWithReason`] ("can't confirm right
///   now").
/// * Everything else — a bad/missing handle-or-account (`WalletNotFound` /
///   `AddressSync`) AND NON-retryable `Sdk` errors (misconfiguration, invariant
///   violation, protocol/consensus error, …) → [`PreflightOutcome::Structural`].
///   A non-retryable SDK fault must NOT masquerade as a retryable "can't fund";
///   it is a real error a caller should see.
fn classify_preflight_error(err: &PlatformWalletError) -> PreflightOutcome {
    match err {
        PlatformWalletError::OnlyDustInputs { .. } | PlatformWalletError::AddressOperation(_) => {
            PreflightOutcome::DisabledWithReason
        }
        PlatformWalletError::Sdk(e) if e.can_retry() => PreflightOutcome::DisabledWithReason,
        _ => PreflightOutcome::Structural,
    }
}

/// Preflight an AUTO withdrawal of a platform-payment account WITHOUT signing,
/// broadcasting, or consuming a Core receive address.
///
/// Runs the same Rust planning phase the real withdraw path executes
/// (`PlatformAddressWallet::preflight_withdrawal` →
/// `plan_withdrawal`/`reserve_withdrawal_fee_on_largest_input`): it reads the
/// account's authoritative on-chain balances (one `AddressInfo::fetch_many`
/// proof query — the SAME balances the spend re-fetches and hard-checks), drops
/// sub-`min_input_amount` dust, estimates the transition fee from the selected
/// input count (NOT from any destination script — no Core address is needed or
/// touched), reserves that fee on the largest-balance input, and verifies the
/// net clears `system_limits.min_withdrawal_amount`. Because the plan is sized
/// from on-chain balances rather than the wallet cache, gating a UI submit
/// button on the result keeps it in lockstep with what the spend path accepts
/// even when the cached balance is stale or doubled.
///
/// On success `out` is written with `can_withdraw = true` and the net /
/// estimated-fee figures, and the call returns [`PlatformWalletFFIResult::ok`].
///
/// A genuine **"can't fund"** outcome — the account is all dust
/// (`OnlyDustInputs`), the largest input can't keep the per-input minimum after
/// the fee, the net falls below the minimum withdrawal amount, more funded
/// addresses than the protocol's `max_address_inputs` clear the minimum, the
/// net exceeds `max_withdrawal_amount`, or there are no funded addresses
/// (`AddressOperation`) — is NOT an FFI error: `out` is written with
/// `can_withdraw = false` (and zeroed figures) and the call still returns a
/// **Success-coded** [`PlatformWalletFFIResult`] whose `message` carries the
/// planner's typed `Display` reason (so a caller that wants a human-readable
/// explanation can read it without mirroring protocol constants in Swift). The
/// authoritative signal is `can_withdraw`; the message is advisory.
///
/// A **transient, RETRYABLE** network / proof-verification failure — the
/// planner's `AddressInfo::fetch_many` balance query couldn't reach a node or
/// its proof failed to verify — is ALSO reported as `can_withdraw = false` with
/// a Success code and the SDK error's message, NOT as a structural FFI error.
/// "Retryable" is decided authoritatively by
/// [`dash_sdk::Error::can_retry()`](dash_sdk::dapi_client::CanRetry::can_retry)
/// (true for stale-node, timeout, and proof-verification errors), so only SDK
/// errors that a retry could actually clear take this arm. Rationale: from the
/// UI's perspective a retryable failure is "can't confirm you can withdraw
/// right now" (retry when connectivity returns), not "this handle/account is
/// broken." Surfacing it as a normal disabled-with-reason result lets the
/// caller show a retryable explanation instead of a silent structural throw
/// indistinguishable from a bad handle.
///
/// A **structural** failure is reported as an FFI error code with `out` left
/// untouched: a bad/destroyed handle, a missing account at `account_index`
/// (`WalletNotFound` / `AddressSync`), OR a NON-retryable SDK error —
/// misconfiguration, an invariant violation, a protocol/consensus error, etc.
/// (`PlatformWalletError::Sdk(e)` where `!e.can_retry()`). Those are genuine
/// faults a caller should surface/monitor via `throwIfError()`, not silently
/// present as a retryable "can't fund."
///
/// # Safety
/// - `out` must be a valid, non-null, writable `*mut WithdrawalPreflightFFI`.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_preflight_withdrawal(
    handle: Handle,
    account_index: u32,
    _core_fee_per_byte: u32,
    out: *mut WithdrawalPreflightFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out);

    // Clone the wallet out of handle storage so the read lock is released
    // before the fetch, then poll on a worker thread (8 MB stack). The
    // preflight now issues an `AddressInfo::fetch_many` proof query to read
    // authoritative on-chain balances (so the gate can't approve what the
    // spend rejects), and GroveDB proof verification recurses past the
    // ~512 KB stacks of iOS dispatch / Swift-concurrency threads (see
    // runtime.rs) — polling it on the calling thread would crash with
    // EXC_BAD_ACCESS, the same reason the withdraw path uses the worker.
    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| wallet.clone());
    let wallet = unwrap_option_or_return!(option);
    let result = block_on_worker(async move { wallet.preflight_withdrawal(account_index).await });
    // Peel the FFI-local outer failure off first: `classify_preflight_error`
    // reasons about typed domain errors and would otherwise be handed a panic
    // to classify as retryable / not-fundable.
    let result = unwrap_result_or_return!(crate::panic_guard::peel_boundary(result));

    match result {
        Ok(plan) => {
            *out = WithdrawalPreflightFFI {
                can_withdraw: true,
                net_withdrawable: plan.net_withdrawable,
                estimated_fee: plan.estimated_fee,
            };
            PlatformWalletFFIResult::ok()
        }
        // Route the error through the shared classifier so the retryability
        // policy lives in one unit-testable place (`classify_preflight_error`).
        Err(err) => match classify_preflight_error(&err) {
            // "Can't fund" (permanent) and "can't confirm right now" (transient
            // but RETRYABLE) both present as a NORMAL result, not an FFI error:
            // `can_withdraw = false` with zeroed figures so the UI can disable
            // submit and explain why. Carry the typed reason as a Success-coded
            // message so callers that want a human-readable explanation can read
            // it; the `can_withdraw` flag is the authoritative signal and the
            // Success code keeps this off the error path (`.check()` on the Swift
            // side only inspects the code).
            PreflightOutcome::DisabledWithReason => {
                *out = WithdrawalPreflightFFI {
                    can_withdraw: false,
                    net_withdrawable: 0,
                    estimated_fee: 0,
                };
                PlatformWalletFFIResult::success_with_message(err.to_string())
            }
            // Structural failures (bad handle / missing wallet / missing account,
            // and NON-retryable SDK errors) stay FFI errors with `out` untouched,
            // mapped via the blanket `From<PlatformWalletError>`.
            PreflightOutcome::Structural => err.into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::Network;

    /// Pins the exact network-validation mechanism
    /// `platform_address_wallet_withdraw_to_address` relies on: a
    /// testnet-prefixed Core address must pass `require_network` on a
    /// testnet wallet and fail on a mainnet wallet, and the resulting
    /// script must be a P2PKH that builds a `CoreScript`.
    ///
    /// We exercise the helper logic directly (parse → require_network →
    /// script_pubkey → CoreScript) rather than the FFI entry point,
    /// which would need a live wallet handle.
    #[test]
    fn withdraw_address_network_check_rejects_wrong_network() {
        // A valid testnet-prefixed (0x8C, "y…") P2PKH address.
        let addr = "yMqShkrgjTRuReBGFpQr7FozEF1QcNBBYA";
        let unchecked = dashcore::Address::from_str(addr).expect("valid base58 address");

        // Mainnet wallet must reject a testnet address.
        assert!(
            unchecked.clone().require_network(Network::Mainnet).is_err(),
            "testnet address must fail require_network(Mainnet)"
        );

        // Testnet wallet must accept it, and the script must be P2PKH.
        let checked = unchecked
            .require_network(Network::Testnet)
            .expect("testnet address must pass require_network(Testnet)");
        let script = checked.script_pubkey();
        let core_script = CoreScript::new(script);
        assert!(
            core_script.is_p2pkh(),
            "a P2PKH address must produce a P2PKH CoreScript"
        );
    }

    // ---- Preflight error classification -------------------------------------
    //
    // Pin the exact `PlatformWalletError` → outcome mapping the preflight FFI
    // relies on, so the retryability policy can't silently regress. The
    // classifier is the load-bearing part: a NON-retryable SDK error
    // (misconfig / invariant / consensus) must stay STRUCTURAL — presenting it
    // as a Success-coded retryable "can't fund" would hide a genuine fault from
    // `throwIfError()` monitoring. Retryable SDK errors and the planner's typed
    // can't-fund states must be disabled-with-reason.

    /// A RETRYABLE SDK error (per `dash_sdk::Error::can_retry()` — here a
    /// timeout) must be `DisabledWithReason`: the balance query can be retried,
    /// so the UI shows "can't confirm right now," not a structural throw.
    #[test]
    fn retryable_sdk_error_is_disabled_with_reason() {
        let err = PlatformWalletError::Sdk(dash_sdk::Error::TimeoutReached(
            std::time::Duration::from_secs(1),
            "balance query timed out".to_string(),
        ));
        // Guard the premise: this variant really is retryable.
        assert!(
            matches!(&err, PlatformWalletError::Sdk(e) if e.can_retry()),
            "test premise: TimeoutReached must be retryable"
        );
        assert_eq!(
            classify_preflight_error(&err),
            PreflightOutcome::DisabledWithReason
        );
    }

    /// A NON-retryable SDK error (here a misconfiguration) must be
    /// `Structural`: a retry can't clear it, so it stays a real FFI error that
    /// `throwIfError()` surfaces — NOT a Success-coded retryable "can't fund".
    /// This is the exact over-broad-catch-all case the second review flagged.
    #[test]
    fn non_retryable_sdk_error_is_structural() {
        let err =
            PlatformWalletError::Sdk(dash_sdk::Error::Config("bad SDK configuration".to_string()));
        // Guard the premise: this variant is NOT retryable.
        assert!(
            matches!(&err, PlatformWalletError::Sdk(e) if !e.can_retry()),
            "test premise: Config must NOT be retryable"
        );
        assert_eq!(classify_preflight_error(&err), PreflightOutcome::Structural);
    }

    /// A missing wallet handle / account is structural.
    #[test]
    fn wallet_not_found_is_structural() {
        let err = PlatformWalletError::WalletNotFound("0xdeadbeef".to_string());
        assert_eq!(classify_preflight_error(&err), PreflightOutcome::Structural);
    }

    /// The planner's typed can't-fund states are disabled-with-reason (the UI
    /// shows the reason and keeps submit disabled — no throw).
    #[test]
    fn cant_fund_states_are_disabled_with_reason() {
        let dust = PlatformWalletError::OnlyDustInputs {
            sub_min_count: 3,
            sub_min_aggregate: 500,
            min_input_amount: 100_000,
        };
        assert_eq!(
            classify_preflight_error(&dust),
            PreflightOutcome::DisabledWithReason
        );

        let op = PlatformWalletError::AddressOperation("net below minimum withdrawal".to_string());
        assert_eq!(
            classify_preflight_error(&op),
            PreflightOutcome::DisabledWithReason
        );
    }
}
