// FFI declarations for PlatformWalletManager, PlatformWallet, and PlatformAddressWallet
// These link against the platform-wallet-ffi symbols in the unified static library.

import Foundation

// MARK: - Persistence Callbacks

struct PersistenceCallbacks {
    var context: UnsafeMutableRawPointer? = nil
    var on_store_fn: (@convention(c) (UnsafeMutableRawPointer?, UnsafePointer<UInt8>?) -> Int32)? = nil
    var on_flush_fn: (@convention(c) (UnsafeMutableRawPointer?, UnsafePointer<UInt8>?) -> Int32)? = nil
    var on_persist_address_balances_fn: (@convention(c) (
        UnsafeMutableRawPointer?,
        UnsafePointer<UInt8>?,
        UnsafeRawPointer?,
        Int
    ) -> Int32)? = nil
    var on_persist_wallet_changeset_fn: (@convention(c) (
        UnsafeMutableRawPointer?,
        UnsafePointer<UInt8>?,
        UnsafeRawPointer?
    ) -> Int32)? = nil
    var on_persist_sync_state_fn: (@convention(c) (
        UnsafeMutableRawPointer?,
        UnsafePointer<UInt8>?,
        UInt64,
        UInt64,
        UInt64
    ) -> Int32)? = nil
    var on_persist_account_fn: (@convention(c) (
        UnsafeMutableRawPointer?,
        UnsafePointer<UInt8>?,
        UnsafeRawPointer?
    ) -> Int32)? = nil
    var on_load_wallet_list_fn: (@convention(c) (
        UnsafeMutableRawPointer?,
        UnsafeMutablePointer<UnsafeRawPointer?>?,
        UnsafeMutablePointer<Int>?
    ) -> Int32)? = nil
    var on_load_wallet_list_free_fn: (@convention(c) (
        UnsafeMutableRawPointer?,
        UnsafeRawPointer?,
        Int
    ) -> Void)? = nil
    var on_persist_wallet_metadata_fn: (@convention(c) (
        UnsafeMutableRawPointer?,
        UnsafePointer<UInt8>?,
        UInt8,
        UInt32
    ) -> Int32)? = nil
    var on_persist_account_addresses_fn: (@convention(c) (
        UnsafeMutableRawPointer?,
        UnsafePointer<UInt8>?,
        UnsafeRawPointer?,
        UnsafeRawPointer?,
        Int
    ) -> Int32)? = nil
}

// MARK: - Core Address Entry
//
// Mirrors `rs-platform-wallet-ffi/src/core_address_types.rs`. Field
// order and types must match bit-for-bit — Rust sends an array of
// these via `on_persist_account_addresses_fn`. String pointers are
// Rust-owned and valid only for the duration of the callback.

struct CoreAddressEntryFFI {
    // 33-byte compressed pubkey (zero when `has_public_key == false`).
    var public_key: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    ) = (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
    var has_public_key: Bool = false
    var pool_type_tag: UInt8 = 0
    var address_index: UInt32 = 0
    var is_used: Bool = false
    var balance: UInt64 = 0
    var address_base58: UnsafePointer<CChar>? = nil
    var derivation_path: UnsafePointer<CChar>? = nil
}

// MARK: - Watch-only Restore Types
//
// Mirrors `rs-platform-wallet-ffi/src/wallet_restore_types.rs`. Field
// order and types must match bit-for-bit — Rust reads the array
// elements as `#[repr(C)]` structs via `slice::from_raw_parts`. All
// pointer fields are Swift-owned during the load-callback window and
// released by `on_load_wallet_list_free_fn`.

struct AccountSpecFFI {
    var type_tag: UInt8 = 0
    var standard_tag: UInt8 = 0
    var index: UInt32 = 0
    var registration_index: UInt32 = 0
    var key_class: UInt32 = 0
    // Fixed 32-byte buffers. Swift imports a fixed-size C array as a
    // tuple; `withUnsafeMutableBytes` / `withUnsafeBytes` to fill and
    // read.
    var user_identity_id: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    ) = (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
    var friend_identity_id: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    ) = (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
    var account_xpub_bytes: UnsafePointer<UInt8>? = nil
    var account_xpub_bytes_len: Int = 0
}

struct WalletRestoreEntryFFI {
    var wallet_id: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    ) = (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
    var network: UInt8 = 0
    var accounts: UnsafePointer<AccountSpecFFI>? = nil
    var accounts_count: Int = 0
}

// MARK: - Event Handler Callbacks

struct EventHandlerCallbacks {
    var context: UnsafeMutableRawPointer?
    var on_wallet_event_fn: (@convention(c) (UnsafeMutableRawPointer?, UnsafePointer<UInt8>?, Int) -> Void)?
    var on_error_fn: (@convention(c) (UnsafeMutableRawPointer?, UnsafePointer<CChar>?) -> Void)?
}

// MARK: - Platform Address Types

struct PlatformAddressFFI {
    var address_type: UInt8
    var hash: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
               UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
               UInt8, UInt8, UInt8, UInt8)
}

struct AddressBalanceEntryFFI {
    var address: PlatformAddressFFI
    var balance: UInt64
}

struct AddressSyncConfigFFI {
    var min_privacy_count: UInt64
    var max_concurrent_requests: UInt32
    var max_iterations: UInt32
    var full_rescan_after_time_s: UInt64
}

struct PlatformAddressChangeSetFFI {
    var updated: UnsafeMutablePointer<AddressBalanceEntryFFI>?
    var updated_count: Int
}

struct FoundAddressEntryFFI {
    var index: UInt32
    var address: PlatformAddressFFI
    var nonce: UInt32
    var balance: UInt64
}

struct AbsentAddressEntryFFI {
    var index: UInt32
    var address: PlatformAddressFFI
}

struct AddressSyncMetricsFFI {
    var trunk_queries: UInt32
    var branch_queries: UInt32
    var total_elements_seen: UInt32
    var total_proof_bytes: UInt32
    var iterations: UInt32
    var compacted_queries: UInt32
    var recent_queries: UInt32
    var recent_entries_returned: UInt32
    var compacted_entries_returned: UInt32
}

struct AddressSyncResultFFI {
    var found: UnsafeMutablePointer<FoundAddressEntryFFI>?
    var found_count: Int
    var absent: UnsafeMutablePointer<AbsentAddressEntryFFI>?
    var absent_count: Int
    var checkpoint_height: UInt64
    var new_sync_height: UInt64
    var new_sync_timestamp: UInt64
    var last_known_recent_block: UInt64
    var metrics: AddressSyncMetricsFFI
}

// MARK: - SPV Sync Progress

struct FFISpvSyncProgress {
    var overall_state: UInt32
    var overall_percentage: Double

    var has_headers: Bool
    var headers_state: UInt32
    var headers_current: UInt32
    var headers_target: UInt32
    var headers_percentage: Double

    var has_filter_headers: Bool
    var filter_headers_state: UInt32
    var filter_headers_current: UInt32
    var filter_headers_target: UInt32
    var filter_headers_percentage: Double

    var has_filters: Bool
    var filters_state: UInt32
    var filters_current: UInt32
    var filters_target: UInt32
    var filters_percentage: Double

    var has_masternodes: Bool
    var masternodes_state: UInt32
    var masternodes_current: UInt32
    var masternodes_target: UInt32
    var masternodes_percentage: Double
}

// MARK: - SDK Inner Pointer

@_silgen_name("dash_sdk_get_inner_sdk_ptr")
func dash_sdk_get_inner_sdk_ptr(
    _ handle: UnsafeMutablePointer<SDKHandle>?
) -> UnsafeRawPointer?

// MARK: - PlatformWalletManager FFI

@_silgen_name("platform_wallet_manager_create")
func platform_wallet_manager_create(
    _ sdk_ptr: UnsafeRawPointer?,
    _ persistence: UnsafePointer<PersistenceCallbacks>?,
    _ event_handler: UnsafePointer<EventHandlerCallbacks>?,
    _ out_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_manager_create_wallet_from_mnemonic")
func platform_wallet_manager_create_wallet_from_mnemonic(
    _ manager_handle: Handle,
    _ mnemonic: UnsafePointer<CChar>?,
    _ network: UInt32,
    _ account_options: UInt32,
    _ out_wallet_handle: UnsafeMutablePointer<Handle>,
    _ out_wallet_id: UnsafeMutablePointer<(UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                           UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                           UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                           UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_manager_create_wallet_from_seed")
func platform_wallet_manager_create_wallet_from_seed(
    _ manager_handle: Handle,
    _ network: UInt32,
    _ seed_bytes: UnsafePointer<UInt8>?,
    _ seed_len: Int,
    _ account_options: UInt32,
    _ out_wallet_handle: UnsafeMutablePointer<Handle>,
    _ out_wallet_id: UnsafeMutablePointer<(UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                           UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                           UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                           UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - PlatformWalletManager SPV FFI

@_silgen_name("platform_wallet_manager_sync_progress")
func platform_wallet_manager_sync_progress(
    _ handle: Handle,
    _ out_progress: UnsafeMutablePointer<FFISpvSyncProgress>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_manager_spv_is_running")
func platform_wallet_manager_spv_is_running(
    _ handle: Handle,
    _ out_running: UnsafeMutablePointer<Bool>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_manager_spv_start")
func platform_wallet_manager_spv_start(
    _ handle: Handle,
    _ data_dir: UnsafePointer<CChar>?,
    _ network: UInt32,
    _ user_agent: UnsafePointer<CChar>?,
    _ peers: UnsafePointer<UnsafePointer<CChar>?>?,
    _ peer_count: Int,
    _ restrict_to_configured_peers: Bool,
    _ start_from_height: UInt32,
    _ masternode_sync_enabled: Bool,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_manager_spv_stop")
func platform_wallet_manager_spv_stop(
    _ handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_manager_spv_clear_storage")
func platform_wallet_manager_spv_clear_storage(
    _ handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_manager_load_from_persistor")
func platform_wallet_manager_load_from_persistor(
    _ manager_handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_manager_get_wallet")
func platform_wallet_manager_get_wallet(
    _ manager_handle: Handle,
    _ wallet_id: UnsafePointer<UInt8>?,
    _ out_wallet_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_manager_destroy")
func platform_wallet_manager_destroy(
    _ handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - Xpub Rendering

@_silgen_name("platform_wallet_account_xpub_to_string")
func platform_wallet_account_xpub_to_string(
    _ bytes: UnsafePointer<UInt8>?,
    _ bytes_len: Int,
    _ out_string: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_free_string")
func platform_wallet_free_string(_ s: UnsafeMutablePointer<CChar>?)

// MARK: - PlatformWallet FFI

@_silgen_name("platform_wallet_get_core")
func platform_wallet_get_core(
    _ handle: Handle,
    _ out_core_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_get_id")
func platform_wallet_get_id(
    _ handle: Handle,
    _ out_wallet_id: UnsafeMutablePointer<(UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                           UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                           UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                                           UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_get_balance")
func platform_wallet_get_balance(
    _ handle: Handle,
    _ out_spendable: UnsafeMutablePointer<UInt64>?,
    _ out_unconfirmed: UnsafeMutablePointer<UInt64>?,
    _ out_immature: UnsafeMutablePointer<UInt64>?,
    _ out_locked: UnsafeMutablePointer<UInt64>?,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_get_platform")
func platform_wallet_get_platform(
    _ handle: Handle,
    _ out_platform_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_flush_persist")
func platform_wallet_flush_persist(
    _ handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_load_and_apply_persisted")
func platform_wallet_load_and_apply_persisted(
    _ handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_destroy")
func platform_wallet_destroy(
    _ handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - PlatformAddressWallet FFI

@_silgen_name("platform_address_wallet_total_credits")
func platform_address_wallet_total_credits(
    _ handle: Handle,
    _ out_credits: UnsafeMutablePointer<UInt64>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_address_wallet_addresses_with_balances")
func platform_address_wallet_addresses_with_balances(
    _ handle: Handle,
    _ out_entries: UnsafeMutablePointer<UnsafeMutablePointer<AddressBalanceEntryFFI>?>,
    _ out_count: UnsafeMutablePointer<Int>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_address_wallet_sync_balances")
func platform_address_wallet_sync_balances(
    _ handle: Handle,
    _ has_config: Bool,
    _ config: UnsafePointer<AddressSyncConfigFFI>?,
    _ out_result: UnsafeMutablePointer<AddressSyncResultFFI>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_address_wallet_restore_sync_state")
func platform_address_wallet_restore_sync_state(
    _ handle: Handle,
    _ sync_height: UInt64,
    _ sync_timestamp: UInt64,
    _ last_known_recent_block: UInt64,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_address_wallet_destroy")
func platform_address_wallet_destroy(
    _ handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - Memory Free Functions

@_silgen_name("platform_address_wallet_free_address_balances")
func platform_address_wallet_free_address_balances(
    _ entries: UnsafeMutablePointer<AddressBalanceEntryFFI>?,
    _ count: Int
)

@_silgen_name("platform_address_wallet_free_changeset")
func platform_address_wallet_free_changeset(_ changeset: UnsafePointer<PlatformAddressChangeSetFFI>?)

@_silgen_name("platform_address_wallet_free_sync_result")
func platform_address_wallet_free_sync_result(_ result: UnsafePointer<AddressSyncResultFFI>?)

