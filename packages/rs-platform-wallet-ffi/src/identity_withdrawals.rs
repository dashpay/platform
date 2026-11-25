//! FFI bindings for reading where an identity's credit withdrawals to Core
//! stand.
//!
//! [`platform_wallet_identity_withdrawals`] calls
//! `platform_wallet::IdentityWallet::identity_withdrawals`, which reads the
//! identity's `withdrawal` documents from the withdrawals system contract with
//! proofs, every page at one block height, and returns the whole list, each
//! withdrawal once, most recently updated first. Rust decides everything a client
//! shows: the status, its label, and whether the withdrawal is final. This
//! module only flattens the typed rows into C structs.

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use dash_sdk::platform::identity_withdrawals::IdentityWithdrawal;

use crate::check_ptr;
use crate::dpns_marketplace::publish_array;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// One withdrawal of an identity to Core, as
/// [`platform_wallet_identity_withdrawals`] reports it.
///
/// `status_label`, `destination` and `output_script` are owned by the
/// array and are read-only for the caller: do not modify a row, its strings
/// or its script, and release the whole array, unchanged, with
/// [`platform_wallet_identity_withdrawals_free`]. Each optional value is a
/// `has_*` flag and the value, which is meaningful only when the flag is
/// set; `destination` is null when the output script is not an address.
#[repr(C)]
pub struct IdentityWithdrawalFFI {
    /// The id of the withdrawal document.
    pub id: [u8; 32],
    /// The identity the credits were withdrawn from.
    pub owner_id: [u8; 32],
    /// The credits withdrawn, before the Core transaction fee.
    pub amount: u64,
    /// The Core fee rate the withdrawal offered, in duffs per byte.
    pub core_fee_per_byte: u32,
    /// The withdrawals contract status code: 0 queued, 1 pooled,
    /// 2 broadcasted (Platform built the asset unlock transaction and queued
    /// it for a quorum to sign and for broadcast to Core; this status does not
    /// show whether either has happened), 3 complete (Platform saw the transaction in a chain-locked Core
    /// block), 4 expired, 5 failed.
    pub status: u8,
    /// The status as Rust names it ("Queued", "Pooled", ...), NUL-terminated
    /// UTF-8. Null only if the label could not be encoded.
    pub status_label: *const c_char,
    /// Whether the withdrawal reached a status it never leaves (complete or
    /// failed). An expired withdrawal is not final: Platform signs it again.
    pub is_final: bool,
    /// The Core address the withdrawal pays on the wallet's network,
    /// NUL-terminated UTF-8; null for a script that is not an address.
    pub destination: *const c_char,
    /// The Core output script the withdrawal pays. Null when empty.
    pub output_script: *const u8,
    /// The length of `output_script` in bytes.
    pub output_script_len: usize,
    /// Platform pooled the withdrawal into an asset unlock transaction.
    pub has_transaction_index: bool,
    /// The index of that asset unlock transaction.
    pub transaction_index: u64,
    /// Platform built the asset unlock transaction.
    pub has_transaction_sign_height: bool,
    /// The Core chain-locked height Platform built the asset unlock
    /// transaction at (its request height). Not proof that a quorum signed
    /// it or that Core accepted it.
    pub transaction_sign_height: u32,
    /// When the withdrawal was submitted, block time in milliseconds.
    pub created_at_ms: u64,
    /// When its status last changed, block time in milliseconds.
    pub updated_at_ms: u64,
}

/// Heap-allocate `s` as an owned C string, or null if it holds an interior
/// NUL (which neither a status label nor a Base58 address can).
pub(crate) fn owned_c_string(s: String) -> *const c_char {
    CString::new(s)
        .map(|c| CString::into_raw(c).cast_const())
        .unwrap_or(ptr::null())
}

impl IdentityWithdrawalFFI {
    /// Flatten a typed withdrawal. Allocates the label, the address and the
    /// script; [`Self::release_owned`] frees them.
    fn from_withdrawal(withdrawal: &IdentityWithdrawal) -> Self {
        let script = withdrawal.output_script.as_bytes();
        let (output_script, output_script_len) = if script.is_empty() {
            (ptr::null(), 0)
        } else {
            let boxed: Box<[u8]> = script.to_vec().into_boxed_slice();
            let len = boxed.len();
            (Box::into_raw(boxed).cast::<u8>().cast_const(), len)
        };
        Self {
            id: withdrawal.id.to_buffer(),
            owner_id: withdrawal.owner_id.to_buffer(),
            amount: withdrawal.amount,
            core_fee_per_byte: withdrawal.core_fee_per_byte,
            status: u8::from(withdrawal.status),
            status_label: owned_c_string(withdrawal.status.to_string()),
            is_final: withdrawal.is_final(),
            destination: withdrawal
                .destination
                .as_ref()
                .map(|address| owned_c_string(address.to_string()))
                .unwrap_or(ptr::null()),
            output_script,
            output_script_len,
            has_transaction_index: withdrawal.transaction_index.is_some(),
            transaction_index: withdrawal.transaction_index.unwrap_or_default(),
            has_transaction_sign_height: withdrawal.transaction_sign_height.is_some(),
            transaction_sign_height: withdrawal.transaction_sign_height.unwrap_or_default(),
            created_at_ms: withdrawal.created_at,
            updated_at_ms: withdrawal.updated_at,
        }
    }

    /// Free the strings and the script this row owns and null their slots,
    /// so a second release is a no-op. The pointers are read-only for the
    /// caller; only here are they turned back into the owned allocations.
    ///
    /// # Safety
    /// Each pointer must be null or an allocation [`Self::from_withdrawal`]
    /// made, with `output_script_len` its length.
    unsafe fn release_owned(&mut self) {
        for slot in [&mut self.status_label, &mut self.destination] {
            if !slot.is_null() {
                drop(unsafe { CString::from_raw(slot.cast_mut()) });
                *slot = ptr::null();
            }
        }
        if !self.output_script.is_null() {
            drop(unsafe {
                Box::from_raw(ptr::slice_from_raw_parts_mut(
                    self.output_script.cast_mut(),
                    self.output_script_len,
                ))
            });
            self.output_script = ptr::null();
            self.output_script_len = 0;
        }
    }
}

/// Every withdrawal of `identity_id` to Core, each listed once, most
/// recently updated first, read with proofs from the withdrawals system
/// contract (protocol version 14 and later). Rust reads every page at one
/// block height; when no walk stays at one height in a few attempts the call
/// fails with an error the caller can retry by reading again. Any identity
/// can be asked for, not only the wallet's. Withdrawals from Platform
/// addresses and from the shielded pool are not found: the withdrawals
/// contract stores them under its own owner.
///
/// On success `*out_rows` / `*out_count` hold the rows (null and 0 when
/// there are none); release them with
/// [`platform_wallet_identity_withdrawals_free`]. Every supplied out slot is
/// reset before anything is checked, so an error, including a refused null
/// slot or a caught panic, leaves nothing to free.
///
/// # Safety
/// - `out_rows` and `out_count`, when not null, must be valid and writable.
///   A null one is refused with `ErrorNullPointer`.
/// - `identity_id`, when not null, must point to 32 readable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_withdrawals(
    wallet_handle: Handle,
    identity_id: *const u8,
    out_rows: *mut *mut IdentityWithdrawalFFI,
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
        "platform_wallet_identity_withdrawals",
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        None,
        "Nothing was changed; read the withdrawals again.",
        || unsafe { identity_withdrawals_inner(wallet_handle, identity_id, out_rows, out_count) },
    )
}

/// The body of [`platform_wallet_identity_withdrawals`], run inside its panic
/// guard after the out slots were reset.
///
/// # Safety
/// As [`platform_wallet_identity_withdrawals`].
unsafe fn identity_withdrawals_inner(
    wallet_handle: Handle,
    identity_id: *const u8,
    out_rows: *mut *mut IdentityWithdrawalFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_rows);
    check_ptr!(out_count);

    let id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });

    // Clone the identity wallet under the registry guard and run the network
    // read outside it, so the read does not hold the registry while it waits.
    let identity_wallet = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result = block_on_worker(async move { identity_wallet.identity_withdrawals(&id).await });
    let withdrawals = unwrap_result_or_return!(result);

    let rows: Vec<IdentityWithdrawalFFI> = withdrawals
        .iter()
        .map(IdentityWithdrawalFFI::from_withdrawal)
        .collect();
    unsafe { publish_array(rows, out_rows, out_count) };
    PlatformWalletFFIResult::ok()
}

/// Release an array [`platform_wallet_identity_withdrawals`] returned: every
/// row's label, address and script, then the array. No-op on null or
/// `count == 0`.
///
/// # Safety
/// `rows` must be null or an array this module returned with exactly
/// `count` elements, not previously freed, whose rows, strings and scripts
/// the caller did not modify.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_withdrawals_free(
    rows: *mut IdentityWithdrawalFFI,
    count: usize,
) {
    if rows.is_null() || count == 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(rows, count) };
    for row in slice.iter_mut() {
        unsafe { row.release_owned() };
    }
    drop(unsafe { Box::from_raw(slice as *mut [IdentityWithdrawalFFI]) });
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::dashcore::{Address, Network, PubkeyHash, ScriptBuf};
    use dpp::prelude::Identifier;
    use dpp::system_data_contracts::withdrawals_contract::WithdrawalStatus;
    use std::ffi::CStr;

    fn withdrawal(status: WithdrawalStatus, output_script: ScriptBuf) -> IdentityWithdrawal {
        let destination = Address::from_script(&output_script, Network::Testnet).ok();
        IdentityWithdrawal {
            id: Identifier::from([1; 32]),
            owner_id: Identifier::from([2; 32]),
            amount: 150_000,
            core_fee_per_byte: 1,
            output_script,
            destination,
            status,
            transaction_index: None,
            transaction_sign_height: None,
            created_at: 1_700_000_000_000,
            updated_at: 1_700_000_600_000,
        }
    }

    fn p2pkh_script() -> ScriptBuf {
        ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([7; 20]))
    }

    fn read_c_string(ptr: *const c_char) -> Option<String> {
        (!ptr.is_null()).then(|| {
            unsafe { CStr::from_ptr(ptr) }
                .to_str()
                .expect("UTF-8")
                .to_owned()
        })
    }

    /// Publish `rows` the way the entry point does and release them through
    /// the public destructor, so the tests use the real allocation shape.
    fn publish_and_free(rows: Vec<IdentityWithdrawalFFI>) {
        let mut out: *mut IdentityWithdrawalFFI = ptr::null_mut();
        let mut count = 0usize;
        unsafe {
            publish_array(rows, &mut out, &mut count);
            platform_wallet_identity_withdrawals_free(out, count);
        }
    }

    #[test]
    fn should_accept_null_and_empty_arrays_in_the_free_function() {
        unsafe {
            platform_wallet_identity_withdrawals_free(ptr::null_mut(), 0);
            platform_wallet_identity_withdrawals_free(ptr::null_mut(), 3);
        }
        publish_and_free(Vec::new());
    }

    #[test]
    fn should_flatten_a_queued_withdrawal_with_its_destination_and_no_core_transaction() {
        let script = p2pkh_script();
        let typed = withdrawal(WithdrawalStatus::QUEUED, script.clone());
        let row = IdentityWithdrawalFFI::from_withdrawal(&typed);

        assert_eq!(row.id, [1; 32]);
        assert_eq!(row.owner_id, [2; 32]);
        assert_eq!(row.amount, 150_000);
        assert_eq!(row.core_fee_per_byte, 1);
        assert_eq!(row.status, 0);
        assert_eq!(read_c_string(row.status_label).as_deref(), Some("Queued"));
        assert!(!row.is_final);
        assert_eq!(
            read_c_string(row.destination),
            typed.destination.as_ref().map(ToString::to_string)
        );
        let bytes = unsafe { std::slice::from_raw_parts(row.output_script, row.output_script_len) };
        assert_eq!(bytes, script.as_bytes());
        assert!(!row.has_transaction_index);
        assert_eq!(row.transaction_index, 0);
        assert!(!row.has_transaction_sign_height);
        assert_eq!(row.transaction_sign_height, 0);
        assert_eq!(row.created_at_ms, 1_700_000_000_000);
        assert_eq!(row.updated_at_ms, 1_700_000_600_000);

        publish_and_free(vec![row]);
    }

    #[test]
    fn should_carry_the_core_transaction_and_rust_finality_for_each_status() {
        let cases = [
            (WithdrawalStatus::POOLED, 1u8, "Pooled", false),
            (WithdrawalStatus::BROADCASTED, 2, "Broadcasted", false),
            (WithdrawalStatus::COMPLETE, 3, "Complete", true),
            (WithdrawalStatus::EXPIRED, 4, "Expired", false),
            (WithdrawalStatus::FAILED, 5, "Failed", true),
        ];
        let rows = cases
            .iter()
            .map(|&(status, code, label, is_final)| {
                let mut typed = withdrawal(status, p2pkh_script());
                typed.transaction_index = Some(42);
                typed.transaction_sign_height = Some(1_234_567);
                let row = IdentityWithdrawalFFI::from_withdrawal(&typed);
                assert_eq!(row.status, code);
                assert_eq!(read_c_string(row.status_label).as_deref(), Some(label));
                assert_eq!(row.is_final, is_final, "{label}");
                assert!(row.has_transaction_index);
                assert_eq!(row.transaction_index, 42);
                assert!(row.has_transaction_sign_height);
                assert_eq!(row.transaction_sign_height, 1_234_567);
                row
            })
            .collect();

        publish_and_free(rows);
    }

    #[test]
    fn should_leave_the_destination_null_for_a_script_that_is_not_an_address() {
        let row = IdentityWithdrawalFFI::from_withdrawal(&withdrawal(
            WithdrawalStatus::QUEUED,
            ScriptBuf::from_bytes(vec![0x6a; 23]),
        ));
        assert!(row.destination.is_null());
        assert_eq!(row.output_script_len, 23);

        publish_and_free(vec![row]);
    }

    #[test]
    fn should_reset_the_supplied_count_when_the_rows_slot_is_null() {
        let identity_id = [1u8; 32];
        let mut count = 7usize;

        let result = unsafe {
            platform_wallet_identity_withdrawals(
                Handle::MAX,
                identity_id.as_ptr(),
                ptr::null_mut(),
                &mut count,
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_eq!(count, 0);
    }

    #[test]
    fn should_reset_the_supplied_rows_when_the_count_slot_is_null() {
        let identity_id = [1u8; 32];
        // A non-null value the call must overwrite; never dereferenced.
        let mut rows: *mut IdentityWithdrawalFFI = ptr::NonNull::dangling().as_ptr();

        let result = unsafe {
            platform_wallet_identity_withdrawals(
                Handle::MAX,
                identity_id.as_ptr(),
                &mut rows,
                ptr::null_mut(),
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert!(rows.is_null());
    }

    #[test]
    fn should_answer_an_unknown_wallet_as_not_found_with_empty_outputs() {
        let identity_id = [1u8; 32];
        let mut rows: *mut IdentityWithdrawalFFI = ptr::NonNull::dangling().as_ptr();
        let mut count = 7usize;

        let result = unsafe {
            platform_wallet_identity_withdrawals(
                Handle::MAX,
                identity_id.as_ptr(),
                &mut rows,
                &mut count,
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
        assert!(rows.is_null());
        assert_eq!(count, 0);
    }
}
