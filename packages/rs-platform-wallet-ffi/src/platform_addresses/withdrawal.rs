//! FFI bindings for platform address withdrawal operations.

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};
use dpp::identity::core_script::CoreScript;
use rs_sdk_ffi::{SignerHandle, VTableSigner};
use std::os::raw::c_char;
use std::str::FromStr;

use super::{parse_input_selection, runtime};

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

    let address_signer: &VTableSigner = &*(signer_address_handle as *const VTableSigner);

    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.withdraw(
            account_index,
            input_selection,
            core_script,
            core_fee_per_byte,
            fee,
            None,
            address_signer,
        ))
    });
    let result = unwrap_option_or_return!(option);
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

    // SAFETY: caller guarantees `signer_address_handle` is a valid,
    // non-destroyed handle that outlives this call.
    let address_signer: &VTableSigner = &*(signer_address_handle as *const VTableSigner);

    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| {
        // Network check: reject an address that doesn't belong to the
        // wallet's network before any signing or submission happens.
        // Mirrors the `require_network` precedent used elsewhere in the
        // FFI for Core-address handling. `dashcore::address::Error` has
        // no `From` impl on `PlatformWalletError`, so map it into the
        // typed `AddressOperation` variant the rest of this path uses.
        let checked_address = unchecked_address
            .clone()
            .require_network(wallet.network())
            .map_err(|e| {
                platform_wallet::PlatformWalletError::AddressOperation(format!(
                    "Core address is not valid for the wallet's network ({:?}): {e}",
                    wallet.network()
                ))
            })?;
        let core_script = CoreScript::new(checked_address.script_pubkey());
        runtime().block_on(wallet.withdraw(
            account_index,
            input_selection,
            core_script,
            core_fee_per_byte,
            fee,
            None,
            address_signer,
        ))
    });
    let result = unwrap_option_or_return!(option);
    // `result` is `Result<PlatformAddressChangeSet, PlatformWalletError>`
    // where the address-network failure surfaces as a
    // `dashcore::address::Error` widened into `PlatformWalletError`.
    let changeset = unwrap_result_or_return!(result);
    *out_changeset = PlatformAddressChangeSetFFI::from(&changeset);
    PlatformWalletFFIResult::ok()
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
