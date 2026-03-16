// ShieldedCryptoFFI.swift
// SwiftDashSDK
//
// @_silgen_name declarations for shielded crypto FFI functions.
// These functions perform Orchard key derivation, bundle building, and note decryption
// entirely within Rust. They do NOT require an SDK handle -- they are pure crypto operations.

import Foundation
import DashSDKFFI

// MARK: - Proving Key Warmup

/// Pre-build and cache the Halo 2 proving key.
/// First call takes ~30 seconds. Subsequent calls are instant.
/// Returns DashSDKResult with no data on success.
@_silgen_name("dash_sdk_shielded_warmup_proving_key")
func dash_sdk_shielded_warmup_proving_key() -> DashSDKResult

// MARK: - Address Derivation

/// Derive an Orchard payment address from a 32-byte spending key.
/// Returns DashSDKResult with a hex-encoded 43-byte address string.
@_silgen_name("dash_sdk_shielded_derive_address")
func dash_sdk_shielded_derive_address(
    _ spending_key_bytes: UnsafePointer<Bytes32Tuple>?,
    _ diversifier_index: UInt32
) -> DashSDKResult

// MARK: - Bundle Building

/// Build an output-only (shield) Orchard bundle.
/// Returns DashSDKResult with a JSON string describing the bundle.
@_silgen_name("dash_sdk_shielded_build_shield_bundle")
func dash_sdk_shielded_build_shield_bundle(
    _ spending_key_bytes: UnsafePointer<Bytes32Tuple>?,
    _ amount: UInt64,
    _ memo: UnsafePointer<Bytes36Tuple>?
) -> DashSDKResult

/// Build a shielded transfer bundle (shielded-to-shielded).
/// Returns DashSDKResult with a JSON string describing the bundle.
@_silgen_name("dash_sdk_shielded_build_transfer_bundle")
func dash_sdk_shielded_build_transfer_bundle(
    _ spending_key_bytes: UnsafePointer<Bytes32Tuple>?,
    _ anchor_bytes: UnsafePointer<Bytes32Tuple>?,
    _ notes_json: UnsafePointer<CChar>?,
    _ recipient_addr_bytes: UnsafePointer<UInt8>?,
    _ recipient_addr_len: Int,
    _ transfer_amount: UInt64,
    _ memo: UnsafePointer<Bytes36Tuple>?
) -> DashSDKResult

/// Build an unshield bundle (shielded pool -> platform address).
/// Returns DashSDKResult with a JSON string describing the bundle.
@_silgen_name("dash_sdk_shielded_build_unshield_bundle")
func dash_sdk_shielded_build_unshield_bundle(
    _ spending_key_bytes: UnsafePointer<Bytes32Tuple>?,
    _ anchor_bytes: UnsafePointer<Bytes32Tuple>?,
    _ notes_json: UnsafePointer<CChar>?,
    _ output_addr_bytes: UnsafePointer<UInt8>?,
    _ output_addr_len: Int,
    _ unshield_amount: UInt64,
    _ memo: UnsafePointer<Bytes36Tuple>?
) -> DashSDKResult

/// Build a withdrawal bundle (shielded pool -> core L1 address).
/// Returns DashSDKResult with a JSON string describing the bundle.
@_silgen_name("dash_sdk_shielded_build_withdrawal_bundle")
func dash_sdk_shielded_build_withdrawal_bundle(
    _ spending_key_bytes: UnsafePointer<Bytes32Tuple>?,
    _ anchor_bytes: UnsafePointer<Bytes32Tuple>?,
    _ notes_json: UnsafePointer<CChar>?,
    _ output_script: UnsafePointer<UInt8>?,
    _ output_script_len: Int,
    _ withdrawal_amount: UInt64,
    _ memo: UnsafePointer<Bytes36Tuple>?,
    _ core_fee_per_byte: UInt32,
    _ pooling: UInt8
) -> DashSDKResult

// MARK: - Note Decryption

/// Decrypt encrypted notes using the spending key's incoming viewing key.
/// Returns DashSDKResult with a JSON array of successfully decrypted notes.
@_silgen_name("dash_sdk_shielded_decrypt_notes")
func dash_sdk_shielded_decrypt_notes(
    _ spending_key_bytes: UnsafePointer<Bytes32Tuple>?,
    _ notes_json: UnsafePointer<CChar>?
) -> DashSDKResult
