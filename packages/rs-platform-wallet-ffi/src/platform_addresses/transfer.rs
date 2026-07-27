//! FFI bindings for platform address transfer operations.

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use super::parse_input_selection;
use crate::runtime::block_on_worker;

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
) -> PlatformWalletFFIResult {
    check_ptr!(out_changeset);
    // Sentinel first: output/input parsing, the wallet lookup, and the async
    // transfer below are all fallible. See
    // `PlatformAddressChangeSetFFI::empty` for the double-free rationale.
    *out_changeset = PlatformAddressChangeSetFFI::empty();
    check_ptr!(signer_address_handle);

    let output_map = unwrap_result_or_return!(parse_outputs(outputs, outputs_count));

    let input_selection = unwrap_result_or_return!(parse_input_selection(
        input_type,
        explicit_inputs,
        explicit_inputs_count,
        nonce_inputs,
        nonce_inputs_count,
    ));

    let fee = parse_fee_strategy(fee_strategy, fee_strategy_count);

    // Clone the wallet out of handle storage so the read lock is released
    // before the long-running transfer, then poll on a worker thread
    // (8 MB stack): the transfer future verifies the execution proof, and
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
            .transfer(
                account_index,
                input_selection,
                output_map,
                fee,
                None, // platform_version -> wallet SDK version
                address_signer,
            )
            .await
    });
    let changeset = unwrap_result_or_return!(result);
    *out_changeset = PlatformAddressChangeSetFFI::from(&changeset);
    PlatformWalletFFIResult::ok()
}
