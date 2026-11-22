//! FFI bindings for reading where an identity's credit withdrawals to Core
//! stand.
//!
//! [`platform_wallet_identity_withdrawals`] calls
//! `platform_wallet::IdentityWallet::identity_withdrawals`, which reads the
//! identity's `withdrawal` documents from the withdrawals system contract with
//! a proof, most recently updated first. Rust decides everything a client
//! shows: the status, its label, and whether the withdrawal is final. This
//! module only flattens the typed rows into C structs.

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use dash_sdk::platform::documents::page::DocumentsPage;
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
/// array; release the whole array with
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
    /// 2 broadcasted, 3 complete, 4 expired, 5 failed.
    pub status: u8,
    /// The status as Rust names it ("Queued", "Pooled", ...), NUL-terminated
    /// UTF-8. Null only if the label could not be encoded.
    pub status_label: *mut c_char,
    /// Whether the withdrawal reached a status it never leaves (complete or
    /// failed). An expired withdrawal is not final: Platform signs it again.
    pub is_final: bool,
    /// The Core address the withdrawal pays on the wallet's network,
    /// NUL-terminated UTF-8; null for a script that is not an address.
    pub destination: *mut c_char,
    /// The Core output script the withdrawal pays. Null when empty.
    pub output_script: *mut u8,
    /// The length of `output_script` in bytes.
    pub output_script_len: usize,
    /// Platform pooled the withdrawal into an asset unlock transaction.
    pub has_transaction_index: bool,
    /// The index of that asset unlock transaction.
    pub transaction_index: u64,
    /// A quorum signed that transaction.
    pub has_transaction_sign_height: bool,
    /// The Core height at which it was signed.
    pub transaction_sign_height: u32,
    /// When the withdrawal was submitted, block time in milliseconds.
    pub created_at_ms: u64,
    /// When its status last changed, block time in milliseconds.
    pub updated_at_ms: u64,
}

/// Heap-allocate `s` as an owned C string, or null if it holds an interior
/// NUL (which neither a status label nor a Base58 address can).
fn owned_c_string(s: String) -> *mut c_char {
    CString::new(s)
        .map(CString::into_raw)
        .unwrap_or(ptr::null_mut())
}

impl IdentityWithdrawalFFI {
    /// Flatten a typed withdrawal. Allocates the label, the address and the
    /// script; [`Self::release_owned`] frees them.
    fn from_withdrawal(withdrawal: &IdentityWithdrawal) -> Self {
        let script = withdrawal.output_script.as_bytes();
        let (output_script, output_script_len) = if script.is_empty() {
            (ptr::null_mut(), 0)
        } else {
            let boxed: Box<[u8]> = script.to_vec().into_boxed_slice();
            let len = boxed.len();
            (Box::into_raw(boxed) as *mut u8, len)
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
                .unwrap_or(ptr::null_mut()),
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
    /// so a second release is a no-op.
    ///
    /// # Safety
    /// Each pointer must be null or an allocation [`Self::from_withdrawal`]
    /// made, with `output_script_len` its length.
    unsafe fn release_owned(&mut self) {
        for slot in [&mut self.status_label, &mut self.destination] {
            if !slot.is_null() {
                drop(unsafe { CString::from_raw(*slot) });
                *slot = ptr::null_mut();
            }
        }
        if !self.output_script.is_null() {
            drop(unsafe {
                Box::from_raw(ptr::slice_from_raw_parts_mut(
                    self.output_script,
                    self.output_script_len,
                ))
            });
            self.output_script = ptr::null_mut();
            self.output_script_len = 0;
        }
    }
}

/// One page of the withdrawals of `identity_id` to Core, most recently
/// updated first, read with a proof from the withdrawals system contract.
/// Any identity can be asked for, not only the wallet's. Withdrawals from
/// Platform addresses and from the shielded pool are not found: the
/// withdrawals contract stores them under its own owner.
///
/// `limit` is the page size; `0` uses the platform cap (100).
/// `start_after` is null for the first page, or the 32 bytes
/// `out_next_start_after` returned for the previous one.
///
/// On success `*out_rows` / `*out_count` hold the rows (null and 0 when
/// there are none; release with
/// [`platform_wallet_identity_withdrawals_free`]), and `*out_has_next` says
/// whether a next page exists, in which case its cursor is written to the 32
/// bytes at `out_next_start_after` (zeroed otherwise). Every out slot is
/// written before any fallible work, so an error leaves nothing to free.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_withdrawals(
    wallet_handle: Handle,
    identity_id: *const u8,
    limit: u32,
    start_after: *const u8,
    out_rows: *mut *mut IdentityWithdrawalFFI,
    out_count: *mut usize,
    out_has_next: *mut bool,
    out_next_start_after: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_rows);
    check_ptr!(out_count);
    check_ptr!(out_has_next);
    check_ptr!(out_next_start_after);
    unsafe {
        *out_rows = ptr::null_mut();
        *out_count = 0;
        *out_has_next = false;
        ptr::write_bytes(out_next_start_after, 0u8, 32);
    }

    let id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });
    let start_after = if start_after.is_null() {
        None
    } else {
        Some(unwrap_result_or_return!(unsafe {
            read_identifier(start_after)
        }))
    };
    let page = DocumentsPage {
        limit: (limit != 0).then_some(limit),
        start_after,
    };

    // Clone the identity wallet under the registry guard and run the network
    // read outside it, so the read does not hold the registry while it waits.
    let identity_wallet = unwrap_option_or_return!(
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone())
    );
    let result =
        block_on_worker(async move { identity_wallet.identity_withdrawals(&id, page).await });
    let page = unwrap_result_or_return!(result);

    let rows: Vec<IdentityWithdrawalFFI> = page
        .withdrawals
        .iter()
        .map(IdentityWithdrawalFFI::from_withdrawal)
        .collect();
    unsafe {
        publish_array(rows, out_rows, out_count);
        if let Some(next) = page.next.and_then(|next| next.start_after) {
            write_identifier(out_next_start_after, &next);
            *out_has_next = true;
        }
    }
    PlatformWalletFFIResult::ok()
}

/// Release an array [`platform_wallet_identity_withdrawals`] returned: every
/// row's label, address and script, then the array. No-op on null or
/// `count == 0`.
///
/// # Safety
/// `rows` must be null or an array this module returned with exactly
/// `count` elements, not previously freed.
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
    fn should_refuse_null_out_pointers_and_answer_an_unknown_wallet_as_not_found() {
        let identity_id = [1u8; 32];
        let mut rows: *mut IdentityWithdrawalFFI = ptr::null_mut();
        let mut count = 7usize;
        let mut has_next = true;
        let mut next = [9u8; 32];

        let result = unsafe {
            platform_wallet_identity_withdrawals(
                Handle::MAX,
                identity_id.as_ptr(),
                0,
                ptr::null(),
                ptr::null_mut(),
                &mut count,
                &mut has_next,
                next.as_mut_ptr(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        let result = unsafe {
            platform_wallet_identity_withdrawals(
                Handle::MAX,
                identity_id.as_ptr(),
                0,
                ptr::null(),
                &mut rows,
                &mut count,
                &mut has_next,
                next.as_mut_ptr(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
        // The out slots were reset before the lookup failed.
        assert!(rows.is_null());
        assert_eq!(count, 0);
        assert!(!has_next);
        assert_eq!(next, [0u8; 32]);
    }
}
