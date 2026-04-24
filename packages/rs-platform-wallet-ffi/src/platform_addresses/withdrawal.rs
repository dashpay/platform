//! FFI bindings for platform address withdrawal operations.

use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use dpp::identity::core_script::CoreScript;

use super::{parse_input_selection, runtime};

/// Withdraw platform credits to a Core L1 address.
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
    out_changeset: *mut PlatformAddressChangeSetFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_changeset.is_null() || output_script.is_null() {
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

    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime().block_on(wallet.withdraw(
                account_index,
                input_selection,
                core_script,
                core_fee_per_byte,
                fee,
                None, // platform_version = latest
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
