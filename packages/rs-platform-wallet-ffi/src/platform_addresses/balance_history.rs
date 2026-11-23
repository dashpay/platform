//! FFI bindings for the balance change history of a wallet's platform addresses.

use std::ffi::{CString, NulError};

use platform_wallet::wallet::platform_addresses::AddressBalanceHistory;

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use crate::runtime::block_on_worker;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Balance changes of the wallet's platform addresses: the recent zone, and
/// the compacted zone too when `include_compacted` is set.
///
/// Calls `PlatformAddressWallet::balance_history`, which pages each zone,
/// keeps the changes of every platform payment account's addresses,
/// reconciles the two zones so no block shows in both, orders them newest
/// first and names the wallet's network. Each entry's `address_bech32m` is
/// that address on that network.
///
/// `min_proved_height` seeds the read's height floor: pass 0 for a first
/// read and the `out_proved_height` of the history already shown for every
/// later read, so no page older than what is shown is used. A read that
/// cannot get a page at its floor fails with
/// `ErrorBalanceHistoryInconsistentHeights` (83). 83 means only that every
/// wallet attempt for one page got a successfully fetched, proof-verified
/// reply proved below the floor. Errors from the SDK itself (its own
/// stale-node rejection, transport failures, proof verification failures)
/// are not 83: they keep their existing mapping.
///
/// On success `out_recent` / `out_recent_count` and `out_compacted` /
/// `out_compacted_count` hold heap arrays (null with a count of 0 when
/// empty; the compacted array is always empty when `include_compacted` is
/// false), and `out_proved_height` the floor the read reached. Free both
/// arrays with `platform_address_wallet_free_balance_history`. On error
/// every out pointer is null with a count of 0, and `out_proved_height` is
/// 0.
///
/// A panic while fetching becomes an `ErrorWalletOperation` result where
/// panics unwind (see `catch_panic_to_code`).
///
/// # Safety
/// Every out pointer must be valid and writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_address_wallet_balance_history(
    handle: Handle,
    include_compacted: bool,
    min_proved_height: u64,
    out_recent: *mut *mut AddressBalanceChangeEntryFFI,
    out_recent_count: *mut usize,
    out_compacted: *mut *mut CompactedAddressBalanceChangeEntryFFI,
    out_compacted_count: *mut usize,
    out_proved_height: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_recent);
    check_ptr!(out_recent_count);
    check_ptr!(out_compacted);
    check_ptr!(out_compacted_count);
    check_ptr!(out_proved_height);
    *out_recent = std::ptr::null_mut();
    *out_recent_count = 0;
    *out_compacted = std::ptr::null_mut();
    *out_compacted_count = 0;
    *out_proved_height = 0;

    catch_panic_to_code(
        "platform address balance history",
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        "This read changed nothing; it can be retried.",
        || {
            // Clone the wallet out of handle storage so the storage lock is
            // released before the network round-trips, and poll on a worker
            // (8 MB stack): every page is proof-verified, which recurses past
            // the small stacks of iOS dispatch threads (see runtime.rs).
            let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| wallet.clone());
            let wallet = unwrap_option_or_return!(option);
            let result = block_on_worker(async move {
                wallet
                    .balance_history(include_compacted, min_proved_height)
                    .await
            });
            let history = unwrap_result_or_return!(result);
            let proved_height = history.proved_height;

            let (recent, compacted) = match balance_history_into_ffi(history) {
                Ok(arrays) => arrays,
                Err(error) => {
                    return PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorSerialization,
                        format!("cannot marshal a balance history address: {error}"),
                    )
                }
            };
            (*out_recent, *out_recent_count) = into_raw_array(recent);
            (*out_compacted, *out_compacted_count) = into_raw_array(compacted);
            *out_proved_height = proved_height;
            PlatformWalletFFIResult::ok()
        },
    )
}

/// Free the arrays returned by `platform_address_wallet_balance_history`,
/// including every entry's `address_bech32m` and `additions`.
///
/// # Safety
/// Each pointer / count pair must be exactly what that call returned (or
/// null / 0), freed once.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_free_balance_history(
    recent: *mut AddressBalanceChangeEntryFFI,
    recent_count: usize,
    compacted: *mut CompactedAddressBalanceChangeEntryFFI,
    compacted_count: usize,
) {
    if !recent.is_null() && recent_count > 0 {
        let mut entries = Vec::from_raw_parts(recent, recent_count, recent_count);
        for entry in &mut entries {
            entry.release();
        }
    }
    if !compacted.is_null() && compacted_count > 0 {
        let mut entries = Vec::from_raw_parts(compacted, compacted_count, compacted_count);
        for entry in &mut entries {
            entry.release();
        }
    }
}

/// Marshal both sections of `history`, each address as its bech32m string
/// on `history.network`.
///
/// Every string is built before any entry takes ownership of one, so a
/// failure leaves nothing to free.
fn balance_history_into_ffi(
    history: AddressBalanceHistory,
) -> Result<
    (
        Vec<AddressBalanceChangeEntryFFI>,
        Vec<CompactedAddressBalanceChangeEntryFFI>,
    ),
    NulError,
> {
    let network = history.network;
    let recent = history
        .recent
        .into_iter()
        .map(|entry| {
            let address = CString::new(entry.address.to_bech32m_string(network))?;
            Ok((entry, address))
        })
        .collect::<Result<Vec<_>, NulError>>()?;
    let compacted = history
        .compacted
        .into_iter()
        .map(|entry| {
            let address = CString::new(entry.address.to_bech32m_string(network))?;
            Ok((entry, address))
        })
        .collect::<Result<Vec<_>, NulError>>()?;
    Ok((
        recent
            .into_iter()
            .map(|(entry, address)| AddressBalanceChangeEntryFFI::from_entry(entry, address))
            .collect(),
        compacted
            .into_iter()
            .map(|(entry, address)| {
                CompactedAddressBalanceChangeEntryFFI::from_entry(entry, address)
            })
            .collect(),
    ))
}

/// A heap array and its length for C: null with 0 when `entries` is empty.
fn into_raw_array<T>(entries: Vec<T>) -> (*mut T, usize) {
    let count = entries.len();
    if count == 0 {
        return (std::ptr::null_mut(), 0);
    }
    (Box::into_raw(entries.into_boxed_slice()) as *mut T, count)
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;

    use dpp::address_funds::PlatformAddress;
    use key_wallet::Network;
    use platform_wallet::wallet::platform_addresses::{
        AddressBalanceChange, AddressBalanceChangeEntry, CompactedAddressBalanceChange,
        CompactedAddressBalanceChangeEntry,
    };

    use super::*;
    use crate::runtime::runtime;
    use crate::test_fixtures::{mock_manager_over, noop_persister};
    use dash_sdk::platform::query::{
        RecentAddressBalanceChangesQuery, RecentCompactedAddressBalanceChangesQuery,
    };
    use dash_sdk::query_types::{
        BlockAddressBalanceChanges, RecentAddressBalanceChanges,
        RecentCompactedAddressBalanceChanges,
    };
    use dpp::balances::credits::CreditOperation;
    use key_wallet::account::account_collection::PlatformPaymentAccountKey;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;

    #[test]
    fn should_marshal_and_free_both_sections_with_rust_encoded_addresses() {
        let recent_address = PlatformAddress::P2pkh([7; 20]);
        let history = AddressBalanceHistory {
            network: Network::Testnet,
            recent: vec![AddressBalanceChangeEntry {
                block_height: 12,
                address: recent_address,
                change: AddressBalanceChange::Added(400),
            }],
            compacted: vec![
                CompactedAddressBalanceChangeEntry {
                    start_block_height: 6,
                    end_block_height: 9,
                    address: PlatformAddress::P2pkh([3; 20]),
                    change: CompactedAddressBalanceChange::Additions(vec![(8, 20), (6, 10)]),
                },
                CompactedAddressBalanceChangeEntry {
                    start_block_height: 1,
                    end_block_height: 5,
                    address: PlatformAddress::P2pkh([1; 20]),
                    change: CompactedAddressBalanceChange::SetFinal(50),
                },
            ],
            proved_height: 12,
        };

        let (recent, compacted) =
            balance_history_into_ffi(history).expect("bech32m strings have no NUL");

        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].block_height, 12);
        assert_eq!(recent[0].kind, AddressBalanceChangeKindFFI::Added);
        assert_eq!(recent[0].credits, 400);
        let recent_bech32m = unsafe { CStr::from_ptr(recent[0].address_bech32m) }
            .to_str()
            .expect("ASCII");
        assert_eq!(
            recent_bech32m,
            recent_address.to_bech32m_string(Network::Testnet)
        );
        assert_ne!(
            recent_bech32m,
            recent_address.to_bech32m_string(Network::Mainnet),
            "the address must be encoded on the history's network"
        );

        assert_eq!(
            compacted[0].kind,
            CompactedAddressBalanceChangeKindFFI::Additions
        );
        assert_eq!(compacted[0].credits, 30);
        assert_eq!(compacted[0].additions_count, 2);
        let additions = unsafe {
            std::slice::from_raw_parts(compacted[0].additions, compacted[0].additions_count)
        };
        assert_eq!(additions[0].block_height, 8);
        assert_eq!(additions[0].credits, 20);
        assert_eq!(
            compacted[1].kind,
            CompactedAddressBalanceChangeKindFFI::SetFinal
        );
        assert_eq!(compacted[1].credits, 50);
        assert!(compacted[1].additions.is_null());
        assert_eq!(compacted[1].additions_count, 0);
        assert!(!compacted[1].address_bech32m.is_null());

        let (recent_ptr, recent_count) = into_raw_array(recent);
        let (compacted_ptr, compacted_count) = into_raw_array(compacted);
        unsafe {
            platform_address_wallet_free_balance_history(
                recent_ptr,
                recent_count,
                compacted_ptr,
                compacted_count,
            )
        };
    }

    #[test]
    fn should_return_null_arrays_for_an_empty_history() {
        let history = AddressBalanceHistory {
            network: Network::Testnet,
            recent: Vec::new(),
            compacted: Vec::new(),
            proved_height: 0,
        };

        let (recent, compacted) = balance_history_into_ffi(history).expect("nothing to encode");
        let (recent_ptr, recent_count) = into_raw_array(recent);
        let (compacted_ptr, compacted_count) = into_raw_array(compacted);

        assert!(recent_ptr.is_null());
        assert_eq!(recent_count, 0);
        assert!(compacted_ptr.is_null());
        assert_eq!(compacted_count, 0);
        unsafe {
            platform_address_wallet_free_balance_history(
                recent_ptr,
                recent_count,
                compacted_ptr,
                compacted_count,
            )
        };
    }

    #[test]
    fn should_report_an_invalid_handle_without_touching_the_out_arrays() {
        let mut recent: *mut AddressBalanceChangeEntryFFI = std::ptr::null_mut();
        let mut recent_count = 0usize;
        let mut compacted: *mut CompactedAddressBalanceChangeEntryFFI = std::ptr::null_mut();
        let mut compacted_count = 0usize;
        let mut proved_height = 7u64;

        let mut result = unsafe {
            platform_address_wallet_balance_history(
                Handle::MAX,
                true,
                12,
                &mut recent,
                &mut recent_count,
                &mut compacted,
                &mut compacted_count,
                &mut proved_height,
            )
        };

        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(recent.is_null());
        assert_eq!(recent_count, 0);
        assert!(compacted.is_null());
        assert_eq!(compacted_count, 0);
        assert_eq!(proved_height, 0);
        unsafe { platform_wallet_ffi_result_free(&mut result) };
    }

    /// Calls the export on `handle` with `min_proved_height`, returning the
    /// result, both arrays (freed) as `(block_height, credits)` recent rows
    /// and a compacted count, and the proved height it wrote.
    fn call_balance_history(
        handle: Handle,
        min_proved_height: u64,
    ) -> (PlatformWalletFFIResult, Vec<(u64, u64)>, usize, u64) {
        let mut recent: *mut AddressBalanceChangeEntryFFI = std::ptr::null_mut();
        let mut recent_count = 0usize;
        let mut compacted: *mut CompactedAddressBalanceChangeEntryFFI = std::ptr::null_mut();
        let mut compacted_count = 0usize;
        let mut proved_height = u64::MAX;
        let result = unsafe {
            platform_address_wallet_balance_history(
                handle,
                true,
                min_proved_height,
                &mut recent,
                &mut recent_count,
                &mut compacted,
                &mut compacted_count,
                &mut proved_height,
            )
        };
        let rows = if recent.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(recent, recent_count) }
                .iter()
                .map(|entry| (entry.block_height, entry.credits))
                .collect()
        };
        unsafe {
            platform_address_wallet_free_balance_history(
                recent,
                recent_count,
                compacted,
                compacted_count,
            )
        };
        (result, rows, compacted_count, proved_height)
    }

    /// The export holds the floor it is given and reports the height it
    /// reached, through a real handle over a wallet whose SDK is a mock that
    /// answers both zones at height 20: a floor of 25 refuses every reply
    /// with code 83, and a floor of 15 returns the wallet's row with 20.
    #[test]
    fn should_hold_the_given_floor_and_report_the_reached_height_through_the_export() {
        const REPLY_HEIGHT: u64 = 20;
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_network(dashcore::Network::Testnet)
            .build()
            .expect("mock sdk");
        // The manager keeps a clone of `sdk`; the clones share the mock.
        let manager = mock_manager_over(sdk.clone(), noop_persister());
        let wallet = runtime()
            .block_on(manager.create_wallet_from_mnemonic(
                "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
                dashcore::Network::Testnet,
                WalletAccountCreationOptions::Default,
                None,
            ))
            .expect("the wallet is created");
        let address =
            runtime()
                .block_on(wallet.platform().next_unused_receive_address(
                    PlatformPaymentAccountKey {
                        account: 0,
                        key_class: 0,
                    },
                ))
                .expect("an address of platform payment account 0");
        runtime().block_on(async {
            sdk.mock()
                .expect_fetch(
                    RecentAddressBalanceChangesQuery::new(0),
                    Some(RecentAddressBalanceChanges(vec![
                        BlockAddressBalanceChanges {
                            block_height: 18,
                            changes: [(address, CreditOperation::AddToCredits(500))]
                                .into_iter()
                                .collect(),
                        },
                    ])),
                )
                .await
                .expect("register the recent page");
            sdk.mock()
                .expect_fetch(
                    RecentCompactedAddressBalanceChangesQuery::new(0),
                    Some(RecentCompactedAddressBalanceChanges(Vec::new())),
                )
                .await
                .expect("register the compacted page");
        });
        sdk.mock().set_response_height(REPLY_HEIGHT);
        // What `platform_wallet_get_platform` registers for a host.
        let handle = PLATFORM_ADDRESS_WALLET_STORAGE.insert(wallet.platform().clone());

        let (mut refused, rows, compacted_count, proved_height) = call_balance_history(handle, 25);
        assert_eq!(
            refused.code,
            PlatformWalletFFIResultCode::ErrorBalanceHistoryInconsistentHeights
        );
        assert!(rows.is_empty());
        assert_eq!(compacted_count, 0);
        assert_eq!(proved_height, 0);
        unsafe { platform_wallet_ffi_result_free(&mut refused) };

        let (mut read, rows, compacted_count, proved_height) = call_balance_history(handle, 15);
        assert_eq!(read.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(rows, vec![(18, 500)]);
        assert_eq!(compacted_count, 0);
        assert_eq!(proved_height, REPLY_HEIGHT);
        unsafe { platform_wallet_ffi_result_free(&mut read) };

        PLATFORM_ADDRESS_WALLET_STORAGE.remove(handle);
    }
}
