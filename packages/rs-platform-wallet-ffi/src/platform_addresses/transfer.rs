//! FFI bindings for platform address transfer operations.

use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use super::{parse_input_selection, runtime};

/// Transfer credits between platform addresses.
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
pub unsafe extern "C" fn platform_address_wallet_transfer(
    handle: Handle,
    account_index: u32,
    input_type: InputSelectionType,
    explicit_inputs: *const ExplicitInputFFI,
    explicit_inputs_count: usize,
    nonce_inputs: *const ExplicitInputWithNonceFFI,
    nonce_inputs_count: usize,
    outputs: *const AddressBalanceEntryFFI,
    outputs_count: usize,
    fee_strategy: *const FeeStrategyStepFFI,
    fee_strategy_count: usize,
    signer_address_handle: *mut SignerHandle,
    out_changeset: *mut PlatformAddressChangeSetFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_changeset.is_null() {
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

    let output_map = match parse_outputs(outputs, outputs_count) {
        Ok(m) => m,
        Err(e) => {
            if !out_error.is_null() {
                *out_error =
                    PlatformWalletFFIError::new(PlatformWalletFFIResult::ErrorInvalidParameter, e);
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };

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
            match runtime().block_on(wallet.transfer(
                account_index,
                input_selection,
                output_map,
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
