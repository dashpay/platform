// ShieldedFFI.swift
// SwiftDashSDK
//
// @_silgen_name declarations for all shielded pool and nullifier sync FFI functions.
// These functions are compiled into the static library but not yet in the C header,
// so we declare them directly by symbol name, following the pattern in PlatformWalletFFI.swift.

import Foundation
import DashSDKFFI

// MARK: - Shielded Query Functions

/// Fetch all anchors from the shielded pool.
/// Returns JSON array of hex-encoded 32-byte anchor strings.
@_silgen_name("dash_sdk_shielded_get_anchors")
func dash_sdk_shielded_get_anchors(
    _ sdk_handle: UnsafePointer<SDKHandle>?
) -> DashSDKResult

/// Fetch the most recent anchor from the shielded pool.
/// Returns hex-encoded string of the 32-byte anchor hash.
@_silgen_name("dash_sdk_shielded_get_most_recent_anchor")
func dash_sdk_shielded_get_most_recent_anchor(
    _ sdk_handle: UnsafePointer<SDKHandle>?
) -> DashSDKResult

/// Fetch nullifier statuses from the shielded pool.
/// Returns JSON array of objects with "nullifier" (hex) and "isSpent" (bool).
@_silgen_name("dash_sdk_shielded_get_nullifiers")
func dash_sdk_shielded_get_nullifiers(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ nullifiers_ptr: UnsafePointer<UInt8>?,
    _ nullifiers_count: UInt32
) -> DashSDKResult

/// Fetch encrypted notes from the shielded pool, paginated.
/// Returns JSON array of encrypted note objects.
@_silgen_name("dash_sdk_shielded_get_encrypted_notes")
func dash_sdk_shielded_get_encrypted_notes(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ start_index: UInt64,
    _ count: UInt32
) -> DashSDKResult

/// Fetch the total shielded pool balance.
/// Returns a decimal string of the balance (u64).
@_silgen_name("dash_sdk_shielded_get_pool_state")
func dash_sdk_shielded_get_pool_state(
    _ sdk_handle: UnsafePointer<SDKHandle>?
) -> DashSDKResult

// MARK: - Shielded Transition Functions (Build + Broadcast Combined)

/// Shield funds from platform addresses into the shielded pool.
@_silgen_name("dash_sdk_shielded_shield_funds")
func dash_sdk_shielded_shield_funds(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ inputs: UnsafePointer<FFIShieldInput>?,
    _ inputs_count: UInt32,
    _ bundle: UnsafePointer<FFIOrchardBundleParams>?,
    _ amount: UInt64,
    _ fee_from_input_index: UInt16
) -> DashSDKResult

/// Transfer funds within the shielded pool (shielded-to-shielded).
@_silgen_name("dash_sdk_shielded_transfer")
func dash_sdk_shielded_transfer(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ bundle: UnsafePointer<FFIOrchardBundleParams>?,
    _ value_balance: UInt64
) -> DashSDKResult

/// Unshield funds from the shielded pool to a platform address.
@_silgen_name("dash_sdk_shielded_unshield_funds")
func dash_sdk_shielded_unshield_funds(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ output_address: UnsafePointer<UInt8>?,
    _ output_address_len: Int,
    _ unshielding_amount: UInt64,
    _ bundle: UnsafePointer<FFIOrchardBundleParams>?
) -> DashSDKResult

/// Shield funds from an L1 instant asset lock into the shielded pool.
@_silgen_name("dash_sdk_shielded_shield_from_instant_lock")
func dash_sdk_shielded_shield_from_instant_lock(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ instant_lock_bytes: UnsafePointer<UInt8>?,
    _ instant_lock_len: Int,
    _ transaction_bytes: UnsafePointer<UInt8>?,
    _ transaction_len: Int,
    _ output_index: UInt32,
    _ private_key: UnsafePointer<UInt8>?,
    _ bundle: UnsafePointer<FFIOrchardBundleParams>?,
    _ value_balance: UInt64
) -> DashSDKResult

/// Shield funds from an L1 chain asset lock into the shielded pool.
@_silgen_name("dash_sdk_shielded_shield_from_chain_lock")
func dash_sdk_shielded_shield_from_chain_lock(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ core_chain_locked_height: UInt32,
    _ out_point_bytes: UnsafePointer<Bytes36Tuple>?,
    _ private_key: UnsafePointer<UInt8>?,
    _ bundle: UnsafePointer<FFIOrchardBundleParams>?,
    _ value_balance: UInt64
) -> DashSDKResult

/// Withdraw funds from the shielded pool to a Core (L1) address.
@_silgen_name("dash_sdk_shielded_withdraw")
func dash_sdk_shielded_withdraw(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ unshielding_amount: UInt64,
    _ bundle: UnsafePointer<FFIOrchardBundleParams>?,
    _ core_fee_per_byte: UInt32,
    _ pooling: UInt8,
    _ output_script: UnsafePointer<UInt8>?,
    _ output_script_len: Int
) -> DashSDKResult

// MARK: - Shielded Builder Functions (Build Only, Returns Serialized Hex)

/// Build a shielded transfer without broadcasting. Returns hex-encoded serialized bytes.
@_silgen_name("dash_sdk_shielded_build_transfer")
func dash_sdk_shielded_build_transfer(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ bundle: UnsafePointer<FFIOrchardBundleParams>?,
    _ value_balance: UInt64
) -> DashSDKResult

/// Build an unshield transition without broadcasting. Returns hex-encoded serialized bytes.
@_silgen_name("dash_sdk_shielded_build_unshield")
func dash_sdk_shielded_build_unshield(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ output_address: UnsafePointer<UInt8>?,
    _ output_address_len: Int,
    _ unshielding_amount: UInt64,
    _ bundle: UnsafePointer<FFIOrchardBundleParams>?
) -> DashSDKResult

/// Build a shield transition without broadcasting. Returns hex-encoded serialized bytes.
@_silgen_name("dash_sdk_shielded_build_shield")
func dash_sdk_shielded_build_shield(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ inputs: UnsafePointer<FFIShieldInput>?,
    _ inputs_count: UInt32,
    _ bundle: UnsafePointer<FFIOrchardBundleParams>?,
    _ amount: UInt64,
    _ fee_from_input_index: UInt16
) -> DashSDKResult

/// Build a shield-from-instant-lock transition without broadcasting.
@_silgen_name("dash_sdk_shielded_build_shield_from_instant_lock")
func dash_sdk_shielded_build_shield_from_instant_lock(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ instant_lock_bytes: UnsafePointer<UInt8>?,
    _ instant_lock_len: Int,
    _ transaction_bytes: UnsafePointer<UInt8>?,
    _ transaction_len: Int,
    _ output_index: UInt32,
    _ private_key: UnsafePointer<UInt8>?,
    _ bundle: UnsafePointer<FFIOrchardBundleParams>?,
    _ value_balance: UInt64
) -> DashSDKResult

/// Build a shield-from-chain-lock transition without broadcasting.
@_silgen_name("dash_sdk_shielded_build_shield_from_chain_lock")
func dash_sdk_shielded_build_shield_from_chain_lock(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ core_chain_locked_height: UInt32,
    _ out_point_bytes: UnsafePointer<Bytes36Tuple>?,
    _ private_key: UnsafePointer<UInt8>?,
    _ bundle: UnsafePointer<FFIOrchardBundleParams>?,
    _ value_balance: UInt64
) -> DashSDKResult

/// Build a shielded withdrawal without broadcasting.
@_silgen_name("dash_sdk_shielded_build_withdrawal")
func dash_sdk_shielded_build_withdrawal(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ unshielding_amount: UInt64,
    _ bundle: UnsafePointer<FFIOrchardBundleParams>?,
    _ core_fee_per_byte: UInt32,
    _ pooling: UInt8,
    _ output_script: UnsafePointer<UInt8>?,
    _ output_script_len: Int
) -> DashSDKResult

// MARK: - Broadcast Function

/// Broadcast a pre-built, serialized state transition.
@_silgen_name("dash_sdk_shielded_broadcast")
func dash_sdk_shielded_broadcast(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ st_bytes: UnsafePointer<UInt8>?,
    _ st_len: Int
) -> DashSDKResult

// MARK: - Nullifier Sync Functions

/// Synchronize nullifier statuses using privacy-preserving BLAST sync.
/// Returns a pointer to FFINullifierSyncResult, or nil on error.
/// Free the result with dash_sdk_nullifier_sync_result_free.
@_silgen_name("dash_sdk_sync_nullifiers")
func dash_sdk_sync_nullifiers(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ nullifiers_ptr: UnsafePointer<UInt8>?,
    _ nullifiers_count: UInt32,
    _ config: UnsafePointer<FFINullifierSyncConfig>?,
    _ last_sync_height: UInt64,
    _ last_sync_timestamp: UInt64
) -> UnsafeMutablePointer<FFINullifierSyncResult>?

/// Synchronize nullifiers, returning result via DashSDKResult for better error handling.
@_silgen_name("dash_sdk_sync_nullifiers_with_result")
func dash_sdk_sync_nullifiers_with_result(
    _ sdk_handle: UnsafePointer<SDKHandle>?,
    _ nullifiers_ptr: UnsafePointer<UInt8>?,
    _ nullifiers_count: UInt32,
    _ config: UnsafePointer<FFINullifierSyncConfig>?,
    _ last_sync_height: UInt64,
    _ last_sync_timestamp: UInt64
) -> DashSDKResult

/// Free a nullifier sync result. Safe to call with nil.
@_silgen_name("dash_sdk_nullifier_sync_result_free")
func dash_sdk_nullifier_sync_result_free(
    _ result: UnsafeMutablePointer<FFINullifierSyncResult>?
)
