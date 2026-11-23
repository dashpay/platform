//! FFI bindings for a wallet's tokens in token shielded pools: which pools it
//! watches and what the local scan found there
//! (`platform_wallet::wallet::shielded::token_pool`).
//!
//! The pools a wallet watches are scanned by the ordinary shielded sync pass
//! (`platform_wallet_manager_shielded_sync_sync_now` and the background loop),
//! so there is no sync export here. A type 28 purchase watches the bought
//! token's pool by itself. Rust decides everything: which accounts get a row,
//! the balance, the note counts and the scan state. These exports only
//! resolve the handles, call the shielded coordinator and flatten its rows.

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use platform_wallet::wallet::shielded::ShieldedTokenPoolBalance;
use platform_wallet::PlatformWalletError;

use crate::check_ptr;
use crate::dpns_marketplace::publish_array;
use crate::error::*;
use crate::handle::*;
use crate::identity_withdrawals::owned_c_string;
use crate::runtime::block_on_worker;
use crate::shielded_send::resolve_wallet_and_coordinator;
use crate::shielded_sync::map_shielded_error;
use crate::types::{read_identifier, write_identifier};
use crate::unwrap_result_or_return;

/// One account's tokens in one watched token shielded pool, as
/// [`platform_wallet_manager_shielded_token_pool_balances`] reports it.
///
/// `last_error` is owned by the array and is read-only for the caller:
/// release the whole array, unchanged, with
/// [`platform_wallet_manager_shielded_token_pool_balances_free`]. Each
/// optional value is a `has_*` flag and the value, which is meaningful only
/// when the flag is set.
#[repr(C)]
pub struct ShieldedTokenPoolBalanceFFI {
    /// The watched token's id.
    pub token_id: [u8; 32],
    /// The contract that defines the token.
    pub data_contract_id: [u8; 32],
    /// The token's position in its contract.
    pub token_contract_position: u16,
    /// The Orchard account the notes belong to.
    pub account_index: u32,
    /// The value of the account's notes whose nullifiers the scan has not
    /// seen and that no spend in flight holds, in the token's smallest unit:
    /// what a token pool spend can use now.
    pub balance: u64,
    /// How many notes make up `balance`.
    pub unspent_note_count: u32,
    /// The value of the account's unspent notes a spend in flight holds, in
    /// the token's smallest unit: they leave when a scan sees them spent, or
    /// return to `balance` if the spend can no longer execute.
    pub reserved_balance: u64,
    /// The scan has checked some of the pool's notes for this account.
    pub has_scanned_note_count: bool,
    /// How many of the pool's notes, from the first without a gap, were
    /// checked for this account. The balance covers exactly these.
    pub scanned_note_count: u64,
    /// The scan has read a chunk of the pool.
    pub has_pool_note_count: bool,
    /// The pool's note count at the last chunk read.
    pub pool_note_count: u64,
    /// The scan has read a chunk of the pool.
    pub has_scanned_height: bool,
    /// The height the last chunk read was proven at.
    pub scanned_height: u64,
    /// Why the last scan of the pool stopped early, NUL-terminated UTF-8;
    /// null when it did not (or when the reason could not be encoded).
    pub last_error: *const c_char,
}

impl ShieldedTokenPoolBalanceFFI {
    /// Flatten a typed row. Allocates `last_error`; [`Self::release_owned`]
    /// frees it.
    fn from_balance(balance: ShieldedTokenPoolBalance) -> Self {
        Self {
            token_id: balance.watch.token_id.to_buffer(),
            data_contract_id: balance.watch.data_contract_id.to_buffer(),
            token_contract_position: balance.watch.token_contract_position,
            account_index: balance.account_index,
            balance: balance.balance,
            unspent_note_count: balance.unspent_note_count,
            reserved_balance: balance.reserved_balance,
            has_scanned_note_count: balance.scanned_note_count.is_some(),
            scanned_note_count: balance.scanned_note_count.unwrap_or_default(),
            has_pool_note_count: balance.pool_note_count.is_some(),
            pool_note_count: balance.pool_note_count.unwrap_or_default(),
            has_scanned_height: balance.scanned_height.is_some(),
            scanned_height: balance.scanned_height.unwrap_or_default(),
            last_error: balance
                .last_error
                .map(owned_c_string)
                .unwrap_or(ptr::null()),
        }
    }

    /// Free the string this row owns and null its slot, so a second release
    /// is a no-op.
    ///
    /// # Safety
    /// `last_error` must be null or the allocation [`Self::from_balance`]
    /// made.
    unsafe fn release_owned(&mut self) {
        if !self.last_error.is_null() {
            drop(unsafe { CString::from_raw(self.last_error.cast_mut()) });
            self.last_error = ptr::null();
        }
    }
}

/// Watch the shielded pool of the token at `token_contract_position` of
/// `data_contract_id` for `wallet_id_bytes`, so every later shielded sync
/// scans it for the wallet's notes
/// (`NetworkShieldedCoordinator::watch_token_pool`). Reads the token's
/// configuration from Platform with proofs first: a token without a shielded
/// pool, or whose configuration cannot be read (a later call may succeed), is
/// refused with `ErrorWalletOperation` and a message saying which. Watching a
/// watched pool again changes nothing. On success the token's id is written
/// to `out_token_id`.
///
/// Call it off the host's UI thread: it reads from the network.
///
/// # Safety
/// - `wallet_id_bytes` and `data_contract_id` must each point to 32 readable
///   bytes.
/// - `out_token_id` must point to 32 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_token_pool_watch(
    handle: Handle,
    wallet_id_bytes: *const u8,
    data_contract_id: *const u8,
    token_contract_position: u16,
    out_token_id: *mut u8,
) -> PlatformWalletFFIResult {
    // A panic in the read or the store write must not unwind across this
    // `extern "C"` frame. Watching is idempotent, so the host may ask again.
    catch_panic_to_code(
        "platform_wallet_manager_shielded_token_pool_watch",
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        "Watching a token pool again is safe; ask again.",
        || unsafe {
            shielded_token_pool_watch_inner(
                handle,
                wallet_id_bytes,
                data_contract_id,
                token_contract_position,
                out_token_id,
            )
        },
    )
}

/// Body of [`platform_wallet_manager_shielded_token_pool_watch`], run inside
/// its panic guard.
///
/// # Safety
/// As [`platform_wallet_manager_shielded_token_pool_watch`].
unsafe fn shielded_token_pool_watch_inner(
    handle: Handle,
    wallet_id_bytes: *const u8,
    data_contract_id: *const u8,
    token_contract_position: u16,
    out_token_id: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(data_contract_id);
    check_ptr!(out_token_id);
    let wallet_id = unsafe { read_wallet_id(wallet_id_bytes) };
    let data_contract_id = unwrap_result_or_return!(unsafe { read_identifier(data_contract_id) });
    let (_wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(value) => value,
        Err(error) => return error,
    };
    let result = block_on_worker(async move {
        coordinator
            .watch_token_pool(wallet_id, data_contract_id, token_contract_position)
            .await
    });
    match result {
        Ok(watch) => {
            unsafe { write_identifier(out_token_id, &watch.token_id) };
            PlatformWalletFFIResult::ok()
        }
        Err(error) => map_shielded_error(error, "shielded token pool watch"),
    }
}

/// Stop watching `token_id`'s shielded pool for `wallet_id_bytes`,
/// forgetting the wallet's notes and scan state there
/// (`NetworkShieldedCoordinator::unwatch_token_pool`). Unwatching a pool not
/// watched changes nothing. Waits for a shielded sync pass that holds the
/// store, so call it off the host's UI thread.
///
/// # Safety
/// `wallet_id_bytes` and `token_id` must each point to 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_token_pool_unwatch(
    handle: Handle,
    wallet_id_bytes: *const u8,
    token_id: *const u8,
) -> PlatformWalletFFIResult {
    catch_panic_to_code(
        "platform_wallet_manager_shielded_token_pool_unwatch",
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        "Unwatching a token pool again is safe; ask again.",
        || unsafe { shielded_token_pool_unwatch_inner(handle, wallet_id_bytes, token_id) },
    )
}

/// Body of [`platform_wallet_manager_shielded_token_pool_unwatch`], run
/// inside its panic guard.
///
/// # Safety
/// As [`platform_wallet_manager_shielded_token_pool_unwatch`].
unsafe fn shielded_token_pool_unwatch_inner(
    handle: Handle,
    wallet_id_bytes: *const u8,
    token_id: *const u8,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(token_id);
    let wallet_id = unsafe { read_wallet_id(wallet_id_bytes) };
    let token_id = unwrap_result_or_return!(unsafe { read_identifier(token_id) });
    let (_wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(value) => value,
        Err(error) => return error,
    };
    let result =
        block_on_worker(async move { coordinator.unwatch_token_pool(wallet_id, token_id).await });
    match result {
        Ok(()) => PlatformWalletFFIResult::ok(),
        Err(error) => map_shielded_error(error, "shielded token pool unwatch"),
    }
}

/// `wallet_id_bytes`'s tokens in every shielded pool it watches, one row per
/// watched pool and registered account, from the local scan alone
/// (`NetworkShieldedCoordinator::token_pool_balances`). No network read; it
/// waits for a shielded sync pass that holds the store, so call it off the
/// host's UI thread.
///
/// On success `*out_rows` / `*out_count` hold the rows (null and 0 when the
/// wallet watches no pool); release them with
/// [`platform_wallet_manager_shielded_token_pool_balances_free`]. Every
/// supplied out slot is reset before anything is checked, so an error,
/// including a refused null slot or a caught panic, leaves nothing to free.
///
/// # Safety
/// - `out_rows` and `out_count`, when not null, must be valid and writable.
///   A null one is refused with `ErrorNullPointer`.
/// - `wallet_id_bytes`, when not null, must point to 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_token_pool_balances(
    handle: Handle,
    wallet_id_bytes: *const u8,
    out_rows: *mut *mut ShieldedTokenPoolBalanceFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    // Reset the supplied slots before the guard, so even a refused call or a
    // caught panic leaves them empty.
    unsafe {
        if !out_rows.is_null() {
            *out_rows = ptr::null_mut();
        }
        if !out_count.is_null() {
            *out_count = 0;
        }
    }
    catch_panic_to_code(
        "platform_wallet_manager_shielded_token_pool_balances",
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        "Nothing was changed; read the token pool balances again.",
        || unsafe {
            shielded_token_pool_balances_inner(handle, wallet_id_bytes, out_rows, out_count)
        },
    )
}

/// Body of [`platform_wallet_manager_shielded_token_pool_balances`], run
/// inside its panic guard after the out slots were reset.
///
/// # Safety
/// As [`platform_wallet_manager_shielded_token_pool_balances`].
unsafe fn shielded_token_pool_balances_inner(
    handle: Handle,
    wallet_id_bytes: *const u8,
    out_rows: *mut *mut ShieldedTokenPoolBalanceFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_rows);
    check_ptr!(out_count);
    check_ptr!(wallet_id_bytes);
    let wallet_id = unsafe { read_wallet_id(wallet_id_bytes) };
    let (_wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(value) => value,
        Err(error) => return error,
    };
    let result: Result<Vec<ShieldedTokenPoolBalance>, PlatformWalletError> =
        block_on_worker(async move { coordinator.token_pool_balances(wallet_id).await });
    let balances = match result {
        Ok(balances) => balances,
        Err(error) => return map_shielded_error(error, "shielded token pool balances"),
    };
    let rows: Vec<ShieldedTokenPoolBalanceFFI> = balances
        .into_iter()
        .map(ShieldedTokenPoolBalanceFFI::from_balance)
        .collect();
    unsafe { publish_array(rows, out_rows, out_count) };
    PlatformWalletFFIResult::ok()
}

/// Release an array [`platform_wallet_manager_shielded_token_pool_balances`]
/// returned: every row's `last_error`, then the array. No-op on null or
/// `count == 0`.
///
/// # Safety
/// `rows` must be null or an array this module returned with exactly `count`
/// elements, not previously freed, whose rows and strings the caller did not
/// modify.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_token_pool_balances_free(
    rows: *mut ShieldedTokenPoolBalanceFFI,
    count: usize,
) {
    if rows.is_null() || count == 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(rows, count) };
    for row in slice.iter_mut() {
        unsafe { row.release_owned() };
    }
    drop(unsafe { Box::from_raw(slice as *mut [ShieldedTokenPoolBalanceFFI]) });
}

/// Copy the 32-byte wallet id at `ptr`.
///
/// # Safety
/// `ptr` must point to 32 readable bytes.
unsafe fn read_wallet_id(ptr: *const u8) -> [u8; 32] {
    let mut wallet_id = [0u8; 32];
    unsafe { ptr::copy_nonoverlapping(ptr, wallet_id.as_mut_ptr(), wallet_id.len()) };
    wallet_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::prelude::Identifier;
    use platform_wallet::wallet::shielded::ShieldedTokenPoolWatch;
    use std::ffi::CStr;

    fn balance(last_error: Option<&str>, scanned: bool) -> ShieldedTokenPoolBalance {
        ShieldedTokenPoolBalance {
            watch: ShieldedTokenPoolWatch {
                token_id: Identifier::from([3; 32]),
                data_contract_id: Identifier::from([4; 32]),
                token_contract_position: 2,
            },
            account_index: 1,
            balance: 1_500,
            unspent_note_count: 3,
            reserved_balance: 250,
            scanned_note_count: scanned.then_some(2_048),
            pool_note_count: scanned.then_some(4_096),
            scanned_height: scanned.then_some(123_456),
            last_error: last_error.map(str::to_owned),
        }
    }

    /// Publish `rows` the way the entry point does and release them through
    /// the public destructor, so the tests use the real allocation shape.
    fn publish_and_free(rows: Vec<ShieldedTokenPoolBalanceFFI>) {
        let mut out: *mut ShieldedTokenPoolBalanceFFI = ptr::null_mut();
        let mut count = 0usize;
        unsafe {
            publish_array(rows, &mut out, &mut count);
            platform_wallet_manager_shielded_token_pool_balances_free(out, count);
        }
    }

    #[test]
    fn should_accept_null_and_empty_arrays_in_the_free_function() {
        unsafe {
            platform_wallet_manager_shielded_token_pool_balances_free(ptr::null_mut(), 0);
            platform_wallet_manager_shielded_token_pool_balances_free(ptr::null_mut(), 3);
        }
        publish_and_free(Vec::new());
    }

    #[test]
    fn should_flatten_a_scanned_row_with_its_last_error() {
        let row =
            ShieldedTokenPoolBalanceFFI::from_balance(balance(Some("chunk read failed"), true));

        assert_eq!(row.token_id, [3; 32]);
        assert_eq!(row.data_contract_id, [4; 32]);
        assert_eq!(row.token_contract_position, 2);
        assert_eq!(row.account_index, 1);
        assert_eq!(row.balance, 1_500);
        assert_eq!(row.unspent_note_count, 3);
        assert_eq!(row.reserved_balance, 250);
        assert!(row.has_scanned_note_count);
        assert_eq!(row.scanned_note_count, 2_048);
        assert!(row.has_pool_note_count);
        assert_eq!(row.pool_note_count, 4_096);
        assert!(row.has_scanned_height);
        assert_eq!(row.scanned_height, 123_456);
        assert!(!row.last_error.is_null());
        assert_eq!(
            unsafe { CStr::from_ptr(row.last_error) }.to_str(),
            Ok("chunk read failed")
        );

        publish_and_free(vec![row]);
    }

    #[test]
    fn should_clear_every_flag_and_leave_the_error_null_before_the_first_scan() {
        let row = ShieldedTokenPoolBalanceFFI::from_balance(balance(None, false));

        assert!(!row.has_scanned_note_count);
        assert_eq!(row.scanned_note_count, 0);
        assert!(!row.has_pool_note_count);
        assert_eq!(row.pool_note_count, 0);
        assert!(!row.has_scanned_height);
        assert_eq!(row.scanned_height, 0);
        assert!(row.last_error.is_null());

        publish_and_free(vec![row]);
    }

    #[test]
    fn should_free_several_rows_with_and_without_errors() {
        publish_and_free(vec![
            ShieldedTokenPoolBalanceFFI::from_balance(balance(Some("first"), true)),
            ShieldedTokenPoolBalanceFFI::from_balance(balance(None, false)),
            ShieldedTokenPoolBalanceFFI::from_balance(balance(Some("third"), false)),
        ]);
    }

    #[test]
    fn should_null_the_error_slot_so_a_second_release_is_a_no_op() {
        let mut row = ShieldedTokenPoolBalanceFFI::from_balance(balance(Some("once"), true));
        unsafe {
            row.release_owned();
            assert!(row.last_error.is_null());
            row.release_owned();
        }
    }

    #[test]
    fn should_reset_the_supplied_count_when_the_rows_slot_is_null() {
        let wallet_id = [1u8; 32];
        let mut count = 7usize;

        let result = unsafe {
            platform_wallet_manager_shielded_token_pool_balances(
                Handle::MAX,
                wallet_id.as_ptr(),
                ptr::null_mut(),
                &mut count,
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_eq!(count, 0);
    }

    #[test]
    fn should_reset_the_supplied_rows_when_the_count_slot_is_null() {
        let wallet_id = [1u8; 32];
        // A non-null value the call must overwrite; never dereferenced.
        let mut rows: *mut ShieldedTokenPoolBalanceFFI = ptr::NonNull::dangling().as_ptr();

        let result = unsafe {
            platform_wallet_manager_shielded_token_pool_balances(
                Handle::MAX,
                wallet_id.as_ptr(),
                &mut rows,
                ptr::null_mut(),
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert!(rows.is_null());
    }

    #[test]
    fn should_refuse_a_null_wallet_id_in_the_balances_with_empty_outputs() {
        let mut rows: *mut ShieldedTokenPoolBalanceFFI = ptr::NonNull::dangling().as_ptr();
        let mut count = 7usize;

        let result = unsafe {
            platform_wallet_manager_shielded_token_pool_balances(
                Handle::MAX,
                ptr::null(),
                &mut rows,
                &mut count,
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert!(rows.is_null());
        assert_eq!(count, 0);
    }

    #[test]
    fn should_answer_an_unknown_manager_in_the_balances_with_empty_outputs() {
        let wallet_id = [1u8; 32];
        let mut rows: *mut ShieldedTokenPoolBalanceFFI = ptr::NonNull::dangling().as_ptr();
        let mut count = 7usize;

        let result = unsafe {
            platform_wallet_manager_shielded_token_pool_balances(
                Handle::MAX,
                wallet_id.as_ptr(),
                &mut rows,
                &mut count,
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert!(rows.is_null());
        assert_eq!(count, 0);
    }

    #[test]
    fn should_refuse_null_pointers_in_the_watch_and_write_no_token_id() {
        let id = [1u8; 32];
        let mut out = [0xFFu8; 32];

        for (wallet, contract, out_ptr) in [
            (ptr::null(), id.as_ptr(), out.as_mut_ptr()),
            (id.as_ptr(), ptr::null(), out.as_mut_ptr()),
            (id.as_ptr(), id.as_ptr(), ptr::null_mut()),
        ] {
            let result = unsafe {
                platform_wallet_manager_shielded_token_pool_watch(
                    Handle::MAX,
                    wallet,
                    contract,
                    0,
                    out_ptr,
                )
            };
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }
        assert_eq!(out, [0xFF; 32], "nothing is written");
    }

    #[test]
    fn should_answer_an_unknown_manager_in_the_watch_and_write_no_token_id() {
        let id = [1u8; 32];
        let mut out = [0xFFu8; 32];

        let result = unsafe {
            platform_wallet_manager_shielded_token_pool_watch(
                Handle::MAX,
                id.as_ptr(),
                id.as_ptr(),
                0,
                out.as_mut_ptr(),
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert_eq!(out, [0xFF; 32], "nothing is written");
    }

    /// A refused watch reaches the host as a wallet operation error whose
    /// message carries Rust's reason, so the host can show which refusal it
    /// was without a code of its own.
    #[test]
    fn should_report_a_refused_watch_with_rust_reason_in_the_message() {
        let token_id = Identifier::from([3; 32]);
        for (error, expected) in [
            (
                PlatformWalletError::TokenShieldedPoolDisabled { token_id },
                "has no shielded pool",
            ),
            (
                PlatformWalletError::TokenConfigurationUnavailable {
                    token_id,
                    reason: "contract not found".to_string(),
                },
                "contract not found",
            ),
            (PlatformWalletError::ShieldedNotBound, "not bound"),
        ] {
            let result = map_shielded_error(error, "shielded token pool watch");
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorWalletOperation
            );
            let message = unsafe { CStr::from_ptr(result.message) }
                .to_str()
                .expect("UTF-8");
            assert!(
                message.contains("shielded token pool watch failed"),
                "{message}"
            );
            assert!(message.contains(expected), "{message}");
        }
    }

    #[test]
    fn should_refuse_null_pointers_and_an_unknown_manager_in_the_unwatch() {
        let id = [1u8; 32];

        for (wallet, token) in [(ptr::null(), id.as_ptr()), (id.as_ptr(), ptr::null())] {
            let result = unsafe {
                platform_wallet_manager_shielded_token_pool_unwatch(Handle::MAX, wallet, token)
            };
            assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }

        let result = unsafe {
            platform_wallet_manager_shielded_token_pool_unwatch(
                Handle::MAX,
                id.as_ptr(),
                id.as_ptr(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
    }
}
