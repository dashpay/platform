// ShieldedPoolClientFFI.swift
// SwiftDashSDK
//
// @_silgen_name declarations for the shielded pool client FFI functions.
// These functions manage a local commitment tree (SQLite-backed), note tracking,
// and Orchard bundle construction. The ShieldedPoolClient handle is an opaque
// Rust type not exposed in the C header, so we use OpaquePointer.

import Foundation
import DashSDKFFI

// MARK: - Lifecycle

/// Create a new ShieldedPoolClient backed by a SQLite database.
/// Returns DashSDKResult with data = *mut ShieldedPoolClient (opaque handle).
@_silgen_name("dash_sdk_shielded_pool_client_create")
func dash_sdk_shielded_pool_client_create(
    _ db_path: UnsafePointer<CChar>?,
    _ spending_key_bytes: UnsafePointer<Bytes32Tuple>?
) -> DashSDKResult

/// Destroy a ShieldedPoolClient handle. Safe to call with nil.
@_silgen_name("dash_sdk_shielded_pool_client_destroy")
func dash_sdk_shielded_pool_client_destroy(
    _ handle: OpaquePointer?
)

/// Get the default Orchard payment address for this client.
/// Returns DashSDKResult with data = hex-encoded address string.
@_silgen_name("dash_sdk_shielded_pool_client_get_address")
func dash_sdk_shielded_pool_client_get_address(
    _ handle: OpaquePointer?
) -> DashSDKResult

/// Get the current shielded balance (sum of unspent note values).
/// Returns DashSDKResult with data = decimal string of the balance (u64).
@_silgen_name("dash_sdk_shielded_pool_client_get_balance")
func dash_sdk_shielded_pool_client_get_balance(
    _ handle: OpaquePointer?
) -> DashSDKResult

// MARK: - Sync (requires SDK handle for network access)

/// Sync notes from the shielded pool. Fetches, decrypts, and appends to tree.
/// Returns DashSDKResult with data = JSON string {"newNotes":N,"balance":N}.
@_silgen_name("dash_sdk_shielded_pool_client_sync_notes")
func dash_sdk_shielded_pool_client_sync_notes(
    _ handle: OpaquePointer?,
    _ sdk_handle: UnsafePointer<SDKHandle>?
) -> DashSDKResult

/// Check which notes have been spent on-chain.
/// Returns DashSDKResult with data = JSON string {"spentCount":N,"balance":N}.
@_silgen_name("dash_sdk_shielded_pool_client_sync_nullifiers")
func dash_sdk_shielded_pool_client_sync_nullifiers(
    _ handle: OpaquePointer?,
    _ sdk_handle: UnsafePointer<SDKHandle>?
) -> DashSDKResult

// MARK: - Bundle Building (returns *mut DashSDKOrchardBundleParams)

/// Build an output-only Orchard bundle for shielding (transparent -> shielded).
/// Returns DashSDKResult with data = *mut DashSDKOrchardBundleParams.
/// Free with dash_sdk_shielded_bundle_params_free.
@_silgen_name("dash_sdk_shielded_pool_client_build_shield_bundle")
func dash_sdk_shielded_pool_client_build_shield_bundle(
    _ handle: OpaquePointer?,
    _ amount: UInt64,
    _ memo: UnsafePointer<Bytes36Tuple>?
) -> DashSDKResult

/// Build a transfer bundle (shielded -> shielded).
/// Selects notes and gets merkle paths automatically.
/// Returns DashSDKResult with data = *mut DashSDKOrchardBundleParams.
/// Free with dash_sdk_shielded_bundle_params_free.
@_silgen_name("dash_sdk_shielded_pool_client_build_transfer_bundle")
func dash_sdk_shielded_pool_client_build_transfer_bundle(
    _ handle: OpaquePointer?,
    _ recipient_address: UnsafePointer<UInt8>?,
    _ recipient_address_len: Int,
    _ amount: UInt64,
    _ memo: UnsafePointer<Bytes36Tuple>?
) -> DashSDKResult

/// Build an unshield bundle (shielded -> platform address).
/// Returns DashSDKResult with data = *mut DashSDKOrchardBundleParams.
/// Free with dash_sdk_shielded_bundle_params_free.
@_silgen_name("dash_sdk_shielded_pool_client_build_unshield_bundle")
func dash_sdk_shielded_pool_client_build_unshield_bundle(
    _ handle: OpaquePointer?,
    _ output_address: UnsafePointer<UInt8>?,
    _ output_address_len: Int,
    _ amount: UInt64,
    _ memo: UnsafePointer<Bytes36Tuple>?
) -> DashSDKResult

/// Build a withdrawal bundle (shielded -> L1 Core address).
/// Returns DashSDKResult with data = *mut DashSDKOrchardBundleParams.
/// Free with dash_sdk_shielded_bundle_params_free.
@_silgen_name("dash_sdk_shielded_pool_client_build_withdrawal_bundle")
func dash_sdk_shielded_pool_client_build_withdrawal_bundle(
    _ handle: OpaquePointer?,
    _ output_script: UnsafePointer<UInt8>?,
    _ output_script_len: Int,
    _ amount: UInt64,
    _ memo: UnsafePointer<Bytes36Tuple>?,
    _ core_fee_per_byte: UInt32,
    _ pooling: UInt8
) -> DashSDKResult
