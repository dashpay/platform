// FFI declarations for AssetLockManager operations.

import Foundation

// MARK: - Types

struct TrackedAssetLockFFI {
    var txid: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
               UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
               UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
               UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)
    var vout: UInt32
    var account_index: UInt32
    var funding_type: UInt32
    var identity_index: UInt32
    var amount: UInt64
    var status: UInt32
    var has_proof: Bool
}

// MARK: - Manager

@_silgen_name("asset_lock_manager_destroy")
func asset_lock_manager_destroy(
    _ handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("asset_lock_manager_list_tracked_locks")
func asset_lock_manager_list_tracked_locks(
    _ handle: Handle,
    _ out_locks: UnsafeMutablePointer<UnsafeMutablePointer<TrackedAssetLockFFI>?>,
    _ out_count: UnsafeMutablePointer<Int>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("asset_lock_manager_free_tracked_locks")
func asset_lock_manager_free_tracked_locks(
    _ locks: UnsafeMutablePointer<TrackedAssetLockFFI>?,
    _ count: Int
)

// MARK: - Build

@_silgen_name("asset_lock_manager_build_transaction")
func asset_lock_manager_build_transaction(
    _ handle: Handle,
    _ amount_duffs: UInt64,
    _ account_index: UInt32,
    _ funding_type: UInt32,
    _ identity_index: UInt32,
    _ out_tx_bytes: UnsafeMutablePointer<UnsafeMutablePointer<UInt8>?>,
    _ out_tx_len: UnsafeMutablePointer<Int>,
    _ out_private_key: UnsafeMutablePointer<(UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                              UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                              UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                              UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("asset_lock_manager_create_funded_proof")
func asset_lock_manager_create_funded_proof(
    _ handle: Handle,
    _ amount_duffs: UInt64,
    _ account_index: UInt32,
    _ funding_type: UInt32,
    _ identity_index: UInt32,
    _ out_proof_bytes: UnsafeMutablePointer<UnsafeMutablePointer<UInt8>?>,
    _ out_proof_len: UnsafeMutablePointer<Int>,
    _ out_private_key: UnsafeMutablePointer<(UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                              UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                              UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                              UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)>,
    _ out_txid: UnsafeMutablePointer<(UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                       UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                       UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                       UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("asset_lock_manager_free_tx_bytes")
func asset_lock_manager_free_tx_bytes(_ bytes: UnsafeMutablePointer<UInt8>?, _ len: Int)

@_silgen_name("asset_lock_manager_free_proof_bytes")
func asset_lock_manager_free_proof_bytes(_ bytes: UnsafeMutablePointer<UInt8>?, _ len: Int)

// MARK: - Sync

@_silgen_name("asset_lock_manager_resume")
func asset_lock_manager_resume(
    _ handle: Handle,
    _ txid: UnsafePointer<(UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)>,
    _ vout: UInt32,
    _ timeout_secs: UInt64,
    _ out_proof_bytes: UnsafeMutablePointer<UnsafeMutablePointer<UInt8>?>,
    _ out_proof_len: UnsafeMutablePointer<Int>,
    _ out_private_key: UnsafeMutablePointer<(UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                              UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                              UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                              UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - Wallet accessor

@_silgen_name("platform_wallet_get_asset_locks")
func platform_wallet_get_asset_locks(
    _ handle: Handle,
    _ out_asset_lock_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult
