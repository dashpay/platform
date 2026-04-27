//! FFI bindings for platform address withdrawal operations.

use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use dpp::identity::core_script::CoreScript;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use super::{parse_input_selection, runtime};

/// Withdraw platform credits to a Core L1 address.
///
/// `signer_address_handle` is a `*mut SignerHandle` produced by
/// `dash_sdk_signer_create_with_ctx` (e.g. via `KeychainSigner.handle`)
/// and is consumed as `Signer<PlatformAddress>` for each input
/// address. The caller retains ownership of the handle; this function
/// does NOT destroy it.
///
/// Free result with `platform_address_wallet_free_changeset`.
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
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_changeset.is_null() || output_script.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    if signer_address_handle.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "signer_address_handle is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let script_bytes = std::slice::from_raw_parts(output_script, output_script_len);
    let core_script = CoreScript::from_bytes(script_bytes.to_vec());

    let input_selection = match parse_input_selection(
        input_type,
        explicit_inputs,
        explicit_inputs_count,
        nonce_inputs,
        nonce_inputs_count,
        out_error,
    ) {
        Ok(s) => s,
        Err(code) => return code,
    };

    let fee = parse_fee_strategy(fee_strategy, fee_strategy_count);

    // SAFETY: caller guarantees `signer_address_handle` is a valid,
    // non-destroyed handle that outlives this call.
    let address_signer: &VTableSigner = &*(signer_address_handle as *const VTableSigner);

    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime().block_on(wallet.withdraw(
                account_index,
                input_selection,
                core_script,
                core_fee_per_byte,
                fee,
                None, // platform_version = latest
                address_signer,
            )) {
                Ok(changeset) => {
                    *out_changeset = PlatformAddressChangeSetFFI::from(&changeset);
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            e.to_string(),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}
