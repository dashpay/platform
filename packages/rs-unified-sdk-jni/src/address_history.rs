//! JNI exports for a wallet's Platform address balance change history and for
//! the address-sync queries the query catalog lists.
//!
//! Each export marshals ONE C function:
//!
//! * `balanceHistory` -> `platform_address_wallet_balance_history` (Swift
//!   `ManagedPlatformAddressWallet.balanceHistory(includeCompacted:minProvedHeight:)`),
//!   on the wallet's transient platform-address handle, which it resolves with
//!   `platform_wallet_get_platform` and destroys before returning, as
//!   `walletAddressesWithBalances` does;
//! * `recentBalanceChanges` -> `dash_sdk_address_fetch_recent_balance_changes`;
//! * `compactedBalanceChanges` -> `dash_sdk_address_fetch_compacted_balance_changes`;
//! * `trunkState` -> `dash_sdk_address_fetch_trunk_state`;
//! * `branchState` -> `dash_sdk_address_fetch_branch_state`;
//! * `branchQueryDepth` -> `dash_sdk_address_branch_query_depth` (Swift
//!   `Addresses.branchQueryDepth(estimatedCount:)`).
//!
//! Paging, the wallet-address filter, the height floor and its retries, the
//! reconciliation of the recent and compacted zones, the order, each range's
//! total, each address's bech32m form and the branch depth are Rust's. This
//! module copies the results into `byte[]`s in Rust's order and frees them; it
//! decides nothing.
//!
//! Every BLOB is big-endian. `bytes` is `u32 len` then that many bytes, and
//! `str` is the same with UTF-8 bytes.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.AddressHistoryNative`.

use crate::results::take_error;
use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass};
use jni::sys::{jboolean, jbyteArray, jint, jlong, JNI_FALSE};
use jni::JNIEnv;
use platform_wallet_ffi::error::platform_wallet_ffi_result_free;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::{
    platform_address_wallet_balance_history, platform_address_wallet_destroy,
    platform_address_wallet_free_balance_history, platform_wallet_get_platform,
    AddressBalanceChangeEntryFFI, CompactedAddressBalanceChangeEntryFFI,
};
use rs_sdk_ffi::{
    dash_sdk_address_branch_query_depth, dash_sdk_address_fetch_branch_state,
    dash_sdk_address_fetch_compacted_balance_changes,
    dash_sdk_address_fetch_recent_balance_changes, dash_sdk_address_fetch_trunk_state,
    dash_sdk_branch_state_free, dash_sdk_compacted_balance_changes_free,
    dash_sdk_recent_balance_changes_free, dash_sdk_trunk_state_free, DashSDKBranchState,
    DashSDKCompactedBalanceChanges, DashSDKLeafBoundary, DashSDKRecentBalanceChanges,
    DashSDKResult, DashSDKResultDataType, DashSDKTrunkState, DashSDKTrunkStateElement, SDKHandle,
};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

// ── BLOB helpers ─────────────────────────────────────────────────────────

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn put_bytes(out: &mut Vec<u8>, value: &[u8]) {
    put_u32(out, value.len() as u32);
    out.extend_from_slice(value);
}

/// `len` elements at `data`, or an empty slice for null or 0.
///
/// # Safety
/// `data` must be null or point to `len` initialized elements that outlive
/// the returned slice.
unsafe fn slice<'a, T>(data: *const T, len: usize) -> &'a [T] {
    if data.is_null() || len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }
    }
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

fn new_byte_array(env: &mut JNIEnv, blob: &[u8]) -> jbyteArray {
    env.byte_array_from_slice(blob)
        .map(|a| a.into_raw())
        .unwrap_or(ptr::null_mut())
}

/// The bytes of `value`, or `None` after throwing code 1 when the JNI read
/// fails, so an export never returns null without a pending exception.
fn read_bytes(env: &mut JNIEnv, value: &JByteArray, field: &str) -> Option<Vec<u8>> {
    match env.convert_byte_array(value) {
        Ok(bytes) => Some(bytes),
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} byte[] could not be read"));
            None
        }
    }
}

/// `value` as `u64`, or `None` after throwing code 1 for a negative one.
fn non_negative(env: &mut JNIEnv, value: jlong, field: &str) -> Option<u64> {
    if value < 0 {
        throw_sdk_exception(
            env,
            1,
            &format!("{field} must not be negative, got {value}"),
        );
        None
    } else {
        Some(value as u64)
    }
}

// ── Wallet balance history ───────────────────────────────────────────────

/// The balance change history of the wallet's platform addresses, as one
/// BLOB. Null after throwing: a negative `minProvedHeight` (code 1), or the
/// platform-wallet error of the read shifted by `PWFFI_CODE_OFFSET` (for
/// example 1083, `ErrorBalanceHistoryInconsistentHeights`, which is safe to
/// retry).
///
/// BLOB layout: `u64 provedHeight`, `u32 recentCount`, per recent entry
/// `u64 blockHeight, u8 addressType, u8[20] hash, str address, u8 kind
/// (0 set, 1 added), u64 credits`, then `u32 compactedCount`, per compacted
/// entry `u64 startBlockHeight, u64 endBlockHeight, u8 addressType,
/// u8[20] hash, str address, u8 kind (0 set final, 1 additions), u64 credits,
/// u32 additionCount` and per addition `u64 blockHeight, u64 credits`.
/// `address` is Rust's bech32m form on the wallet's network, empty only if
/// Rust gave none (the decoder refuses that, as Swift's does).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_AddressHistoryNative_balanceHistory(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    include_compacted: jboolean,
    min_proved_height: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(min_proved_height) = non_negative(env, min_proved_height, "minProvedHeight")
        else {
            return ptr::null_mut();
        };
        let mut addr_handle: Handle = 0;
        let result =
            unsafe { platform_wallet_get_platform(wallet_handle as Handle, &mut addr_handle) };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }

        let mut recent: *mut AddressBalanceChangeEntryFFI = ptr::null_mut();
        let mut recent_count: usize = 0;
        let mut compacted: *mut CompactedAddressBalanceChangeEntryFFI = ptr::null_mut();
        let mut compacted_count: usize = 0;
        let mut proved_height: u64 = 0;
        let result = unsafe {
            platform_address_wallet_balance_history(
                addr_handle,
                include_compacted != JNI_FALSE,
                min_proved_height,
                &mut recent,
                &mut recent_count,
                &mut compacted,
                &mut compacted_count,
                &mut proved_height,
            )
        };
        let out = if take_pwffi_error(env, result) {
            ptr::null_mut()
        } else {
            let blob = unsafe {
                encode_balance_history(
                    proved_height,
                    slice(recent, recent_count),
                    slice(compacted, compacted_count),
                )
            };
            new_byte_array(env, &blob)
        };
        // Null with 0 on error, so this is safe on every path.
        unsafe {
            platform_address_wallet_free_balance_history(
                recent,
                recent_count,
                compacted,
                compacted_count,
            )
        };

        let mut destroy_result = unsafe { platform_address_wallet_destroy(addr_handle) };
        if env.exception_check().unwrap_or(false) {
            unsafe { platform_wallet_ffi_result_free(&mut destroy_result) };
        } else {
            let _ = take_pwffi_error(env, destroy_result);
        }
        out
    })
}

/// Copy a balance history into the BLOB [`Java_org_dashfoundation_dashsdk_ffi_AddressHistoryNative_balanceHistory`]
/// documents, in the given order.
///
/// # Safety
/// Every entry's `address_bech32m` must be null or a valid C string, and
/// every compacted entry's `additions` null or `additions_count` elements.
unsafe fn encode_balance_history(
    proved_height: u64,
    recent: &[AddressBalanceChangeEntryFFI],
    compacted: &[CompactedAddressBalanceChangeEntryFFI],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + recent.len() * 96 + compacted.len() * 120);
    put_u64(&mut out, proved_height);
    put_u32(&mut out, recent.len() as u32);
    for entry in recent {
        put_u64(&mut out, entry.block_height);
        out.push(entry.address.address_type);
        out.extend_from_slice(&entry.address.hash);
        put_bytes(&mut out, unsafe { c_bytes(entry.address_bech32m) });
        out.push(entry.kind as u8);
        put_u64(&mut out, entry.credits);
    }
    put_u32(&mut out, compacted.len() as u32);
    for entry in compacted {
        put_u64(&mut out, entry.start_block_height);
        put_u64(&mut out, entry.end_block_height);
        out.push(entry.address.address_type);
        out.extend_from_slice(&entry.address.hash);
        put_bytes(&mut out, unsafe { c_bytes(entry.address_bech32m) });
        out.push(entry.kind as u8);
        put_u64(&mut out, entry.credits);
        let additions = unsafe { slice(entry.additions, entry.additions_count) };
        put_u32(&mut out, additions.len() as u32);
        for addition in additions {
            put_u64(&mut out, addition.block_height);
            put_u64(&mut out, addition.credits);
        }
    }
    out
}

// ── Address-sync queries (network-wide, raw) ─────────────────────────────

/// The success payload of `result` when its data type is `expected`, or null
/// after throwing: the result's own error, or code 4 for an unexpected data
/// type or a null payload (neither of which the FFI returns).
///
/// # Safety
/// `result` must be fresh from an rs-sdk-ffi call.
unsafe fn payload(
    env: &mut JNIEnv,
    result: &DashSDKResult,
    expected: DashSDKResultDataType,
) -> *mut std::ffi::c_void {
    if unsafe { take_error(env, result) } {
        return ptr::null_mut();
    }
    if result.data.is_null() || result.data_type != expected {
        throw_sdk_exception(env, 4, "the address query returned an unexpected result");
        return ptr::null_mut();
    }
    result.data
}

/// Address balance changes of every address since `startHeight`, network
/// wide (one server page). Null after throwing.
///
/// BLOB layout: `u32 blockCount`, per block `u64 blockHeight, u32
/// changeCount`, per change `bytes address, u8 operation (0 set, 1 add),
/// u64 credits`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_AddressHistoryNative_recentBalanceChanges(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    start_height: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(start_height) = non_negative(env, start_height, "startHeight") else {
            return ptr::null_mut();
        };
        let result = unsafe {
            dash_sdk_address_fetch_recent_balance_changes(sdk as *const SDKHandle, start_height)
        };
        let data = unsafe { payload(env, &result, DashSDKResultDataType::RecentBalanceChanges) };
        if data.is_null() {
            return ptr::null_mut();
        }
        let changes = data as *mut DashSDKRecentBalanceChanges;
        let blob = unsafe { encode_recent_changes(&*changes) };
        unsafe { dash_sdk_recent_balance_changes_free(changes) };
        new_byte_array(env, &blob)
    })
}

/// # Safety
/// Every pointer/count pair in `changes` must be valid.
unsafe fn encode_recent_changes(changes: &DashSDKRecentBalanceChanges) -> Vec<u8> {
    let mut out = Vec::new();
    let blocks = unsafe { slice(changes.blocks, changes.blocks_count) };
    put_u32(&mut out, blocks.len() as u32);
    for block in blocks {
        put_u64(&mut out, block.block_height);
        let entries = unsafe { slice(block.changes, block.changes_count) };
        put_u32(&mut out, entries.len() as u32);
        for change in entries {
            put_bytes(&mut out, unsafe {
                slice(change.address, change.address_len)
            });
            out.push(change.operation_type as u8);
            put_u64(&mut out, change.credits);
        }
    }
    out
}

/// Compacted address balance changes since `startBlockHeight`, network wide
/// (at most one server page). Null after throwing.
///
/// BLOB layout: `u32 rangeCount`, per range `u64 startBlockHeight, u64
/// endBlockHeight, u32 changeCount`, per change `bytes address, u8 operation
/// (0 set final, 1 additions), u64 setCreditsValue, u32 entryCount`, per entry
/// `u64 blockHeight, u64 credits`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_AddressHistoryNative_compactedBalanceChanges(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    start_block_height: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(start) = non_negative(env, start_block_height, "startBlockHeight") else {
            return ptr::null_mut();
        };
        let result = unsafe {
            dash_sdk_address_fetch_compacted_balance_changes(sdk as *const SDKHandle, start)
        };
        let data = unsafe { payload(env, &result, DashSDKResultDataType::CompactedBalanceChanges) };
        if data.is_null() {
            return ptr::null_mut();
        }
        let changes = data as *mut DashSDKCompactedBalanceChanges;
        let blob = unsafe { encode_compacted_changes(&*changes) };
        unsafe { dash_sdk_compacted_balance_changes_free(changes) };
        new_byte_array(env, &blob)
    })
}

/// # Safety
/// Every pointer/count pair in `changes` must be valid.
unsafe fn encode_compacted_changes(changes: &DashSDKCompactedBalanceChanges) -> Vec<u8> {
    let mut out = Vec::new();
    let ranges = unsafe { slice(changes.ranges, changes.ranges_count) };
    put_u32(&mut out, ranges.len() as u32);
    for range in ranges {
        put_u64(&mut out, range.start_block_height);
        put_u64(&mut out, range.end_block_height);
        let entries = unsafe { slice(range.changes, range.changes_count) };
        put_u32(&mut out, entries.len() as u32);
        for change in entries {
            put_bytes(&mut out, unsafe {
                slice(change.address, change.address_len)
            });
            out.push(change.operation_type as u8);
            put_u64(&mut out, change.set_credits_value);
            let adds = unsafe { slice(change.add_entries, change.add_entries_count) };
            put_u32(&mut out, adds.len() as u32);
            for add in adds {
                put_u64(&mut out, add.block_height);
                put_u64(&mut out, add.credits);
            }
        }
    }
    out
}

/// # Safety
/// Both pointer/count pairs must be valid.
unsafe fn encode_elements_and_leaves(
    out: &mut Vec<u8>,
    elements: &[DashSDKTrunkStateElement],
    leaves: &[DashSDKLeafBoundary],
) {
    put_u32(out, elements.len() as u32);
    for element in elements {
        put_bytes(out, unsafe { slice(element.key, element.key_len) });
        put_u32(out, element.nonce);
        put_u64(out, element.balance);
    }
    put_u32(out, leaves.len() as u32);
    for leaf in leaves {
        put_bytes(out, unsafe { slice(leaf.key, leaf.key_len) });
        out.extend_from_slice(&leaf.hash);
        put_u64(out, leaf.estimated_count);
    }
}

/// The address tree's trunk state. Null after throwing.
///
/// BLOB layout: `u64 checkpointHeight`, then the elements and leaf
/// boundaries: `u32 elementCount`, per element `bytes key, u32 nonce, u64
/// balance`, `u32 leafCount`, per leaf `bytes key, u8[32] hash, u64
/// estimatedCount`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_AddressHistoryNative_trunkState(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let result = unsafe { dash_sdk_address_fetch_trunk_state(sdk as *const SDKHandle) };
        let data = unsafe { payload(env, &result, DashSDKResultDataType::TrunkState) };
        if data.is_null() {
            return ptr::null_mut();
        }
        let state = data as *mut DashSDKTrunkState;
        let blob = unsafe { encode_trunk_state(&*state) };
        unsafe { dash_sdk_trunk_state_free(state) };
        new_byte_array(env, &blob)
    })
}

/// # Safety
/// Every pointer/count pair in `state` must be valid.
unsafe fn encode_trunk_state(state: &DashSDKTrunkState) -> Vec<u8> {
    let mut out = Vec::new();
    put_u64(&mut out, state.checkpoint_height);
    unsafe {
        encode_elements_and_leaves(
            &mut out,
            slice(state.elements, state.elements_count),
            slice(state.leaf_boundaries, state.leaf_boundaries_count),
        )
    };
    out
}

/// One branch of the address tree under `key` at `depth`, checked against
/// `expectedHash` (32 bytes) at `checkpointHeight`. Whether the depth is
/// allowed is decided by the query (Rust and the server), not here. Null
/// after throwing.
///
/// BLOB layout: the elements and leaf boundaries as in `trunkState`, without
/// the checkpoint height.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_AddressHistoryNative_branchState(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    key: JByteArray,
    depth: jint,
    expected_hash: JByteArray,
    checkpoint_height: jlong,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(checkpoint_height) = non_negative(env, checkpoint_height, "checkpointHeight")
        else {
            return ptr::null_mut();
        };
        if depth < 0 {
            throw_sdk_exception(env, 1, &format!("depth must not be negative, got {depth}"));
            return ptr::null_mut();
        }
        if key.is_null() || expected_hash.is_null() {
            throw_sdk_exception(env, 1, "key and expectedHash must not be null");
            return ptr::null_mut();
        }
        let Some(key) = read_bytes(env, &key, "key") else {
            return ptr::null_mut();
        };
        let Some(hash) = read_bytes(env, &expected_hash, "expectedHash") else {
            return ptr::null_mut();
        };
        if hash.len() != 32 {
            throw_sdk_exception(
                env,
                1,
                &format!("expectedHash must be 32 bytes, got {}", hash.len()),
            );
            return ptr::null_mut();
        }
        let result = unsafe {
            dash_sdk_address_fetch_branch_state(
                sdk as *const SDKHandle,
                key.as_ptr(),
                key.len(),
                depth as u32,
                hash.as_ptr(),
                checkpoint_height,
            )
        };
        let data = unsafe { payload(env, &result, DashSDKResultDataType::BranchState) };
        if data.is_null() {
            return ptr::null_mut();
        }
        let state = data as *mut DashSDKBranchState;
        let blob = unsafe { encode_branch_state(&*state) };
        unsafe { dash_sdk_branch_state_free(state) };
        new_byte_array(env, &blob)
    })
}

/// # Safety
/// Every pointer/count pair in `state` must be valid.
unsafe fn encode_branch_state(state: &DashSDKBranchState) -> Vec<u8> {
    let mut out = Vec::new();
    unsafe {
        encode_elements_and_leaves(
            &mut out,
            slice(state.elements, state.elements_count),
            slice(state.leaf_boundaries, state.leaf_boundaries_count),
        )
    };
    out
}

/// The depth to pass to `branchState` for a leaf boundary of about
/// `estimatedCount` elements (0 when unknown): the depth the address sync
/// picks for such a leaf under the SDK's platform version. 0 for a null SDK
/// handle, and after throwing code 1 for a negative count.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_AddressHistoryNative_branchQueryDepth(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    estimated_count: jlong,
) -> jint {
    guard(&mut env, 0, |env| {
        let Some(estimated_count) = non_negative(env, estimated_count, "estimatedCount") else {
            return 0;
        };
        jint::from(unsafe {
            dash_sdk_address_branch_query_depth(sdk as *const SDKHandle, estimated_count)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_wallet_ffi::{
        AddressBalanceChangeKindFFI, BlockCreditsFFI, CompactedAddressBalanceChangeKindFFI,
        PlatformAddressFFI,
    };
    use rs_sdk_ffi::{
        DashSDKAddressBalanceChange, DashSDKBlockAwareCreditOperationType,
        DashSDKBlockBalanceChanges, DashSDKBlockHeightCreditEntry, DashSDKCompactedAddressChange,
        DashSDKCompactedBlockRange, DashSDKCreditOperationType,
    };
    use std::ffi::CString;

    // Each golden is mirrored by the constant of the same name in
    // `AddressHistoryDecodingTest` in the Kotlin SDK; change both together.

    /// One recent addition, then a compacted range of two additions and a
    /// compacted set on a P2SH address.
    pub(crate) const BALANCE_HISTORY_GOLDEN_HEX: &str = "000000000000000c00000001000000000000000c0007070707070707070707070707070707070707070000000374623101000000000000019000000002000000000000000600000000000000090003030303030303030303030303030303030303030000000374623301000000000000001e00000002000000000000000800000000000000140000000000000006000000000000000a000000000000000100000000000000050101010101010101010101010101010101010101010000000374623200000000000000003200000000";
    /// Block 100 with one add of 5 credits, then block 101 with no changes.
    pub(crate) const RECENT_CHANGES_GOLDEN_HEX: &str = "0000000200000000000000640000000100000015000909090909090909090909090909090909090909010000000000000005000000000000006500000000";
    /// Range 1..10: a set of 77, then additions with one entry (block 3, 33).
    pub(crate) const COMPACTED_CHANGES_GOLDEN_HEX: &str = "000000010000000000000001000000000000000a000000020000001500040404040404040404040404040404040404040400000000000000004d00000000000000150105050505050505050505050505050505050505050100000000000000000000000100000000000000030000000000000021";
    /// Checkpoint 900, one element, one leaf boundary.
    pub(crate) const TRUNK_STATE_GOLDEN_HEX: &str = "00000000000003840000000100000002aabb0000000300000000000003e80000000100000001cc1111111111111111111111111111111111111111111111111111111111111111000000000000002a";

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn address(address_type: u8, byte: u8) -> PlatformAddressFFI {
        PlatformAddressFFI {
            address_type,
            hash: [byte; 20],
        }
    }

    #[test]
    fn should_encode_both_history_sections_in_the_given_order() {
        let tb1 = CString::new("tb1").expect("no NUL");
        let tb3 = CString::new("tb3").expect("no NUL");
        let tb2 = CString::new("tb2").expect("no NUL");
        let mut additions = [
            BlockCreditsFFI {
                block_height: 8,
                credits: 20,
            },
            BlockCreditsFFI {
                block_height: 6,
                credits: 10,
            },
        ];
        let recent = [AddressBalanceChangeEntryFFI {
            block_height: 12,
            address: address(0, 7),
            address_bech32m: tb1.as_ptr().cast_mut(),
            kind: AddressBalanceChangeKindFFI::Added,
            credits: 400,
        }];
        let compacted = [
            CompactedAddressBalanceChangeEntryFFI {
                start_block_height: 6,
                end_block_height: 9,
                address: address(0, 3),
                address_bech32m: tb3.as_ptr().cast_mut(),
                kind: CompactedAddressBalanceChangeKindFFI::Additions,
                credits: 30,
                additions: additions.as_mut_ptr(),
                additions_count: additions.len(),
            },
            CompactedAddressBalanceChangeEntryFFI {
                start_block_height: 1,
                end_block_height: 5,
                address: address(1, 1),
                address_bech32m: tb2.as_ptr().cast_mut(),
                kind: CompactedAddressBalanceChangeKindFFI::SetFinal,
                credits: 50,
                additions: ptr::null_mut(),
                additions_count: 0,
            },
        ];

        let blob = unsafe { encode_balance_history(12, &recent, &compacted) };

        assert_eq!(hex(&blob), BALANCE_HISTORY_GOLDEN_HEX);
    }

    #[test]
    fn should_encode_an_empty_history_as_its_height_and_two_zero_counts() {
        let blob = unsafe { encode_balance_history(7, &[], &[]) };
        assert_eq!(hex(&blob), "00000000000000070000000000000000");
    }

    #[test]
    fn should_encode_recent_changes_block_by_block() {
        let mut addr = [0u8; 21];
        addr[1..].fill(9);
        let mut changes = [DashSDKAddressBalanceChange {
            address: addr.as_mut_ptr(),
            address_len: addr.len(),
            operation_type: DashSDKCreditOperationType::AddToCredits,
            credits: 5,
        }];
        let mut blocks = [
            DashSDKBlockBalanceChanges {
                block_height: 100,
                changes: changes.as_mut_ptr(),
                changes_count: changes.len(),
            },
            DashSDKBlockBalanceChanges {
                block_height: 101,
                changes: ptr::null_mut(),
                changes_count: 0,
            },
        ];
        let recent = DashSDKRecentBalanceChanges {
            blocks: blocks.as_mut_ptr(),
            blocks_count: blocks.len(),
        };

        assert_eq!(
            hex(&unsafe { encode_recent_changes(&recent) }),
            RECENT_CHANGES_GOLDEN_HEX
        );
    }

    #[test]
    fn should_encode_compacted_changes_with_their_add_entries() {
        let mut set_addr = [4u8; 21];
        set_addr[0] = 0;
        let mut add_addr = [5u8; 21];
        add_addr[0] = 1;
        let mut entries = [DashSDKBlockHeightCreditEntry {
            block_height: 3,
            credits: 33,
        }];
        let mut changes = [
            DashSDKCompactedAddressChange {
                address: set_addr.as_mut_ptr(),
                address_len: set_addr.len(),
                operation_type: DashSDKBlockAwareCreditOperationType::BlockAwareSetCredits,
                set_credits_value: 77,
                add_entries: ptr::null_mut(),
                add_entries_count: 0,
            },
            DashSDKCompactedAddressChange {
                address: add_addr.as_mut_ptr(),
                address_len: add_addr.len(),
                operation_type:
                    DashSDKBlockAwareCreditOperationType::BlockAwareAddToCreditsOperations,
                set_credits_value: 0,
                add_entries: entries.as_mut_ptr(),
                add_entries_count: entries.len(),
            },
        ];
        let mut ranges = [DashSDKCompactedBlockRange {
            start_block_height: 1,
            end_block_height: 10,
            changes: changes.as_mut_ptr(),
            changes_count: changes.len(),
        }];
        let compacted = DashSDKCompactedBalanceChanges {
            ranges: ranges.as_mut_ptr(),
            ranges_count: ranges.len(),
        };

        assert_eq!(
            hex(&unsafe { encode_compacted_changes(&compacted) }),
            COMPACTED_CHANGES_GOLDEN_HEX
        );
    }

    #[test]
    fn should_encode_the_trunk_and_a_branch_with_the_same_element_layout() {
        let mut element_key = [0xaa, 0xbb];
        let mut leaf_key = [0xcc];
        let mut elements = [DashSDKTrunkStateElement {
            key: element_key.as_mut_ptr(),
            key_len: element_key.len(),
            nonce: 3,
            balance: 1000,
        }];
        let mut leaves = [DashSDKLeafBoundary {
            key: leaf_key.as_mut_ptr(),
            key_len: leaf_key.len(),
            hash: [0x11; 32],
            estimated_count: 42,
        }];
        let trunk = DashSDKTrunkState {
            elements: elements.as_mut_ptr(),
            elements_count: elements.len(),
            leaf_boundaries: leaves.as_mut_ptr(),
            leaf_boundaries_count: leaves.len(),
            checkpoint_height: 900,
        };
        let branch = DashSDKBranchState {
            elements: elements.as_mut_ptr(),
            elements_count: elements.len(),
            leaf_boundaries: leaves.as_mut_ptr(),
            leaf_boundaries_count: leaves.len(),
        };

        assert_eq!(
            hex(&unsafe { encode_trunk_state(&trunk) }),
            TRUNK_STATE_GOLDEN_HEX
        );
        // A branch is the trunk without its leading checkpoint height.
        assert_eq!(
            hex(&unsafe { encode_branch_state(&branch) }),
            &TRUNK_STATE_GOLDEN_HEX[16..]
        );
    }
}
