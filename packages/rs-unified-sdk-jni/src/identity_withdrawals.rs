//! JNI export for reading where an identity's credit withdrawals to Core stand.
//!
//! `identityWithdrawals` marshals ONE C function of `platform-wallet-ffi`,
//! `platform_wallet_identity_withdrawals` (Swift
//! `ManagedPlatformWallet.identityWithdrawals(identityId:)` in
//! `IdentityWithdrawals.swift`). Rust reads the identity's `withdrawal`
//! documents with proofs, every page at one block height, orders them most
//! recently updated first and decides each one's status, label and whether it
//! is final. This module copies the rows into a `byte[]` in Rust's order and
//! frees them; it decides nothing.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.IdentityWithdrawalsNative`,
//! decoded by `org.dashfoundation.dashsdk.identity.decodeIdentityWithdrawals`.

use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass};
use jni::sys::{jbyteArray, jlong};
use jni::JNIEnv;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::{
    platform_wallet_identity_withdrawals, platform_wallet_identity_withdrawals_free,
    IdentityWithdrawalFFI,
};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

/// Every withdrawal of `identityId` (32 bytes) to Core, as a BLOB in Rust's
/// order (most recently updated first). Null after throwing: an identity id
/// that is not 32 bytes (code 1), or the platform-wallet error of the read,
/// shifted by `PWFFI_CODE_OFFSET` (for example the retryable error when no
/// walk of the listing stayed at one block height).
///
/// BLOB layout, big-endian: `u32 rowCount`, then per row
/// `u8[32] id, u8[32] ownerId, u64 amount, u32 coreFeePerByte, u8 status,
/// str statusLabel, u8 isFinal, u8 hasDestination [str destination],
/// u32 scriptLen u8[scriptLen] outputScript, u8 hasTransactionIndex
/// [u64 transactionIndex], u8 hasTransactionSignHeight
/// [u32 transactionSignHeight], u64 createdAtMs, u64 updatedAtMs`, where
/// `str` is `u32 byteLen` then that many UTF-8 bytes. A bracketed value is
/// present only when its flag is 1. A row without a status label (Rust could
/// not encode it) is written with `statusLabel` empty, which the decoder
/// refuses, as Swift's does.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityWithdrawalsNative_identityWithdrawals(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(id) = read_identity_id(env, &identity_id) else {
            return ptr::null_mut();
        };
        let mut rows: *mut IdentityWithdrawalFFI = ptr::null_mut();
        let mut count: usize = 0;
        let result = unsafe {
            platform_wallet_identity_withdrawals(
                wallet_handle as Handle,
                id.as_ptr(),
                &mut rows,
                &mut count,
            )
        };
        // The FFI resets both out slots before any fallible work, so freeing
        // is safe on every path, including an error.
        let blob = if take_pwffi_error(env, result) {
            None
        } else if rows.is_null() || count == 0 {
            Some(encode_rows(&[]))
        } else {
            Some(encode_rows(unsafe {
                std::slice::from_raw_parts(rows, count)
            }))
        };
        unsafe { platform_wallet_identity_withdrawals_free(rows, count) };
        match blob {
            Some(blob) => env
                .byte_array_from_slice(&blob)
                .map(|a| a.into_raw())
                .unwrap_or(ptr::null_mut()),
            None => ptr::null_mut(),
        }
    })
}

/// The 32-byte identity id, or `None` after throwing code 1.
fn read_identity_id(env: &mut JNIEnv, value: &JByteArray) -> Option<[u8; 32]> {
    if value.is_null() {
        throw_sdk_exception(env, 1, "identityId must not be null");
        return None;
    }
    // A failed read is thrown as code 1 too, so the export never returns null
    // without a pending exception.
    let bytes = match env.convert_byte_array(value) {
        Ok(bytes) => bytes,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "identityId byte[] could not be read");
            return None;
        }
    };
    match <[u8; 32]>::try_from(bytes.as_slice()) {
        Ok(id) => Some(id),
        Err(_) => {
            throw_sdk_exception(
                env,
                1,
                &format!("identityId must be 32 bytes, got {}", bytes.len()),
            );
            None
        }
    }
}

/// Append `value` as `u32 byteLen` and its bytes.
fn put_bytes(out: &mut Vec<u8>, value: &[u8]) {
    out.extend_from_slice(&(value.len() as u32).to_be_bytes());
    out.extend_from_slice(value);
}

/// The bytes of a NUL-terminated C string, empty for null.
///
/// # Safety
/// `value` must be null or a valid NUL-terminated string.
unsafe fn c_bytes<'a>(value: *const c_char) -> &'a [u8] {
    if value.is_null() {
        &[]
    } else {
        unsafe { CStr::from_ptr(value) }.to_bytes()
    }
}

/// Copy `rows` into the BLOB [`Java_org_dashfoundation_dashsdk_ffi_IdentityWithdrawalsNative_identityWithdrawals`]
/// documents, in the given order.
fn encode_rows(rows: &[IdentityWithdrawalFFI]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + rows.len() * 160);
    out.extend_from_slice(&(rows.len() as u32).to_be_bytes());
    for row in rows {
        out.extend_from_slice(&row.id);
        out.extend_from_slice(&row.owner_id);
        out.extend_from_slice(&row.amount.to_be_bytes());
        out.extend_from_slice(&row.core_fee_per_byte.to_be_bytes());
        out.push(row.status);
        put_bytes(&mut out, unsafe { c_bytes(row.status_label) });
        out.push(u8::from(row.is_final));
        if row.destination.is_null() {
            out.push(0);
        } else {
            out.push(1);
            put_bytes(&mut out, unsafe { c_bytes(row.destination) });
        }
        let script = if row.output_script.is_null() || row.output_script_len == 0 {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(row.output_script, row.output_script_len) }
        };
        put_bytes(&mut out, script);
        out.push(u8::from(row.has_transaction_index));
        if row.has_transaction_index {
            out.extend_from_slice(&row.transaction_index.to_be_bytes());
        }
        out.push(u8::from(row.has_transaction_sign_height));
        if row.has_transaction_sign_height {
            out.extend_from_slice(&row.transaction_sign_height.to_be_bytes());
        }
        out.extend_from_slice(&row.created_at_ms.to_be_bytes());
        out.extend_from_slice(&row.updated_at_ms.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    /// The two rows of the golden BLOB: a pooled, not final withdrawal to an
    /// address, then a complete one paying a bare script with both Core
    /// transaction fields set.
    ///
    /// Mirrored by `IdentityWithdrawalsDecodingTest.GOLDEN_HEX` in the Kotlin
    /// SDK; change both together.
    pub(crate) const GOLDEN_HEX: &str = concat!(
        "00000002",
        // row 0
        "0101010101010101010101010101010101010101010101010101010101010101",
        "0202020202020202020202020202020202020202020202020202020202020202",
        "00000000000249f0",
        "00000001",
        "01",
        "00000006",
        "506f6f6c6564",
        "00",
        "01",
        "00000004",
        "79416263",
        "00000003",
        "76a914",
        "01",
        "000000000000002a",
        "00",
        "0000018bcfe56800",
        "0000018bcfee8fc0",
        // row 1
        "0303030303030303030303030303030303030303030303030303030303030303",
        "0202020202020202020202020202020202020202020202020202020202020202",
        "0000000000000064",
        "00000002",
        "03",
        "00000008",
        "436f6d706c657465",
        "01",
        "00",
        "00000002",
        "6a00",
        "01",
        "0000000000000007",
        "01",
        "0012d687",
        "0000000000000001",
        "0000000000000002",
    );

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn row(
        id: u8,
        amount: u64,
        fee: u32,
        status: u8,
        label: &CString,
        is_final: bool,
        destination: Option<&CString>,
        script: &[u8],
        transaction_index: Option<u64>,
        sign_height: Option<u32>,
        created: u64,
        updated: u64,
    ) -> IdentityWithdrawalFFI {
        IdentityWithdrawalFFI {
            id: [id; 32],
            owner_id: [2; 32],
            amount,
            core_fee_per_byte: fee,
            status,
            status_label: label.as_ptr(),
            is_final,
            destination: destination.map_or(ptr::null(), |d| d.as_ptr()),
            output_script: if script.is_empty() {
                ptr::null()
            } else {
                script.as_ptr()
            },
            output_script_len: script.len(),
            has_transaction_index: transaction_index.is_some(),
            transaction_index: transaction_index.unwrap_or_default(),
            has_transaction_sign_height: sign_height.is_some(),
            transaction_sign_height: sign_height.unwrap_or_default(),
            created_at_ms: created,
            updated_at_ms: updated,
        }
    }

    #[test]
    fn should_encode_an_empty_list_as_a_zero_count() {
        assert_eq!(encode_rows(&[]), vec![0, 0, 0, 0]);
    }

    #[test]
    fn should_encode_rows_in_the_given_order_with_every_optional_gated_by_its_flag() {
        let pooled = CString::new("Pooled").expect("no NUL");
        let complete = CString::new("Complete").expect("no NUL");
        let destination = CString::new("yAbc").expect("no NUL");
        let p2pkh_prefix = [0x76, 0xa9, 0x14];
        let op_return = [0x6a, 0x00];
        let rows = [
            row(
                1,
                150_000,
                1,
                1,
                &pooled,
                false,
                Some(&destination),
                &p2pkh_prefix,
                Some(42),
                None,
                1_700_000_000_000,
                1_700_000_600_000,
            ),
            row(
                3,
                100,
                2,
                3,
                &complete,
                true,
                None,
                &op_return,
                Some(7),
                Some(1_234_567),
                1,
                2,
            ),
        ];

        assert_eq!(hex(&encode_rows(&rows)), GOLDEN_HEX);
    }

    #[test]
    fn should_encode_a_missing_label_as_empty_and_an_empty_script_as_zero_length() {
        let rows = [IdentityWithdrawalFFI {
            status_label: ptr::null(),
            output_script: ptr::null(),
            output_script_len: 0,
            ..row(
                9,
                0,
                0,
                0,
                &CString::new("x").expect("no NUL"),
                false,
                None,
                &[],
                None,
                None,
                0,
                0,
            )
        }];
        let blob = encode_rows(&rows);
        // count, id, owner, amount, fee, status
        let label_at = 4 + 32 + 32 + 8 + 4 + 1;
        assert_eq!(&blob[label_at..label_at + 4], &[0, 0, 0, 0]);
        // label (empty), isFinal, hasDestination, then the script length.
        let script_at = label_at + 4 + 1 + 1;
        assert_eq!(&blob[script_at..script_at + 4], &[0, 0, 0, 0]);
    }
}
