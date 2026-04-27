// FFI declarations for CoreWallet operations.

import Foundation

// MARK: - CoreWallet FFI

@_silgen_name("core_wallet_destroy")
func core_wallet_destroy(
    _ handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("core_wallet_get_balance")
func core_wallet_get_balance(
    _ handle: Handle,
    _ out_confirmed: UnsafeMutablePointer<UInt64>?,
    _ out_unconfirmed: UnsafeMutablePointer<UInt64>?,
    _ out_immature: UnsafeMutablePointer<UInt64>?,
    _ out_locked: UnsafeMutablePointer<UInt64>?,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("core_wallet_get_network")
func core_wallet_get_network(
    _ handle: Handle,
    _ out_network: UnsafeMutablePointer<UInt32>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("core_wallet_next_receive_address")
func core_wallet_next_receive_address(
    _ handle: Handle,
    _ account_index: UInt32,
    _ out_address: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("core_wallet_next_change_address")
func core_wallet_next_change_address(
    _ handle: Handle,
    _ account_index: UInt32,
    _ out_address: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("core_wallet_free_address")
func core_wallet_free_address(_ address: UnsafeMutablePointer<CChar>?)

@_silgen_name("core_wallet_broadcast_transaction")
func core_wallet_broadcast_transaction(
    _ handle: Handle,
    _ tx_bytes: UnsafePointer<UInt8>?,
    _ tx_bytes_len: Int,
    _ out_txid: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("core_wallet_send_to_addresses")
func core_wallet_send_to_addresses(
    _ handle: Handle,
    _ account_type: UInt32,
    _ account_index: UInt32,
    _ addresses: UnsafePointer<UnsafePointer<CChar>?>?,
    _ amounts: UnsafePointer<UInt64>?,
    _ count: Int,
    _ out_tx_bytes: UnsafeMutablePointer<UnsafeMutablePointer<UInt8>?>,
    _ out_tx_len: UnsafeMutablePointer<Int>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("core_wallet_free_tx_bytes")
func core_wallet_free_tx_bytes(_ bytes: UnsafeMutablePointer<UInt8>?, _ len: Int)
