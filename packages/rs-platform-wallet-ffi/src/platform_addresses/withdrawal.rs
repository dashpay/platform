//! FFI bindings for platform address withdrawal operations.

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};
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
/// A **transient** network / proof-verification failure — the planner's
/// `AddressInfo::fetch_many` balance query couldn't reach a node or its proof
/// failed to verify (`PlatformWalletError::Sdk`) — is ALSO reported as
/// `can_withdraw = false` with a Success code and the SDK error's message,
/// NOT as a structural FFI error. Rationale: from the UI's perspective this is
/// "can't confirm you can withdraw right now" (retry when connectivity
/// returns), not "this handle/account is broken." Surfacing it as a normal
/// disabled-with-reason result lets the caller show a retryable explanation
/// instead of a silent structural throw indistinguishable from a bad handle.
///
/// Only a **structural** failure — a bad/destroyed handle, or a missing
/// account at `account_index` (`WalletNotFound` / `AddressSync`) — is reported
/// as an FFI error code with `out` left untouched.
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

    match result {
        Ok(plan) => {
            *out = WithdrawalPreflightFFI {
                can_withdraw: true,
                net_withdrawable: plan.net_withdrawable,
                estimated_fee: plan.estimated_fee,
            };
            PlatformWalletFFIResult::ok()
        }
        // "Can't fund" (and "can't confirm right now") is a NORMAL result, not
        // an FFI error: report it as `can_withdraw = false` with zeroed figures
        // so the UI can disable submit and explain why, without treating it as
        // a failure.
        //
        // `OnlyDustInputs` (every funded address below `min_input_amount`) and
        // `AddressOperation` (the fee / per-input / min-withdrawal headroom
        // failures, the too-many-inputs and above-max-withdrawal gates inside
        // `reserve_withdrawal_fee_on_largest_input`, plus the "no funded
        // addresses" case in `select_withdrawable_inputs`) are all genuine
        // can't-fund states.
        //
        // `Sdk(_)` is the planner's `AddressInfo::fetch_many` balance query
        // failing transiently (node unreachable / proof didn't verify). That is
        // "can't confirm you can withdraw right now" — a retryable, non-
        // structural condition — so it belongs here rather than on the error
        // path where it would be indistinguishable from a bad handle and leave
        // the UI with a silently-disabled button and no reason. The SDK error's
        // `Display` is surfaced verbatim as the reason.
        Err(
            e @ (PlatformWalletError::OnlyDustInputs { .. }
            | PlatformWalletError::AddressOperation(_)
            | PlatformWalletError::Sdk(_)),
        ) => {
            *out = WithdrawalPreflightFFI {
                can_withdraw: false,
                net_withdrawable: 0,
                estimated_fee: 0,
            };
            // Carry the typed reason as a Success-coded message so callers that
            // want a human-readable explanation can read it; the `can_withdraw`
            // flag is the authoritative signal and the Success code keeps this
            // off the error path (`.check()` on the Swift side only inspects
            // the code).
            PlatformWalletFFIResult::success_with_message(e.to_string())
        }
        // Structural failures (bad handle / missing wallet / missing account)
        // stay FFI errors with `out` untouched, mapped via the blanket
        // `From<PlatformWalletError>`.
        Err(other) => other.into(),
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
}
