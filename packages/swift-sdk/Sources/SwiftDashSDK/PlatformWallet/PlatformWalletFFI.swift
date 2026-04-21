// Platform Wallet FFI function declarations
// Since these aren't in the C header, we declare them with @_silgen_name

import Foundation

// MARK: - PlatformWalletInfo Functions

@_silgen_name("platform_wallet_info_create_from_seed")
func platform_wallet_info_create_from_seed(
    _ seed: UnsafePointer<UInt8>?,
    _ seed_len: Int,
    _ out_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_info_create_from_mnemonic")
func platform_wallet_info_create_from_mnemonic(
    _ mnemonic: UnsafePointer<CChar>?,
    _ passphrase: UnsafePointer<CChar>?,
    _ out_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_info_get_identity_manager")
func platform_wallet_info_get_identity_manager(
    _ wallet_handle: Handle,
    _ network: NetworkType,
    _ out_manager_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_info_set_identity_manager")
func platform_wallet_info_set_identity_manager(
    _ wallet_handle: Handle,
    _ network: NetworkType,
    _ manager_handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_info_destroy")
func platform_wallet_info_destroy(_ handle: Handle)

// MARK: - IdentityManager Functions

@_silgen_name("identity_manager_create")
func identity_manager_create(
    _ out_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("identity_manager_add_identity")
func identity_manager_add_identity(
    _ manager_handle: Handle,
    _ identity_handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("identity_manager_remove_identity")
func identity_manager_remove_identity(
    _ manager_handle: Handle,
    _ identity_id: IdentifierBytes,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("identity_manager_get_identity")
func identity_manager_get_identity(
    _ manager_handle: Handle,
    _ identity_id: IdentifierBytes,
    _ out_identity_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("identity_manager_get_all_identity_ids")
func identity_manager_get_all_identity_ids(
    _ manager_handle: Handle,
    _ out_array: UnsafeMutablePointer<IdentifierArray>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("identity_manager_get_primary_identity_id")
func identity_manager_get_primary_identity_id(
    _ manager_handle: Handle,
    _ out_id: UnsafeMutablePointer<IdentifierBytes>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("identity_manager_set_primary_identity")
func identity_manager_set_primary_identity(
    _ manager_handle: Handle,
    _ identity_id: IdentifierBytes,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("identity_manager_get_identity_count")
func identity_manager_get_identity_count(
    _ manager_handle: Handle,
    _ out_count: UnsafeMutablePointer<Int>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("identity_manager_destroy")
func identity_manager_destroy(_ handle: Handle)

// MARK: - ManagedIdentity Functions

@_silgen_name("managed_identity_create_from_identity_bytes")
func managed_identity_create_from_identity_bytes(
    _ bytes: UnsafePointer<UInt8>?,
    _ bytes_len: Int,
    _ out_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_id")
func managed_identity_get_id(
    _ identity_handle: Handle,
    _ out_id: UnsafeMutablePointer<IdentifierBytes>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_balance")
func managed_identity_get_balance(
    _ identity_handle: Handle,
    _ out_balance: UnsafeMutablePointer<UInt64>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_label")
func managed_identity_get_label(
    _ identity_handle: Handle,
    _ out_label: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_set_label")
func managed_identity_set_label(
    _ identity_handle: Handle,
    _ label: UnsafePointer<CChar>?,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_last_updated_balance_block_time")
func managed_identity_get_last_updated_balance_block_time(
    _ identity_handle: Handle,
    _ out_block_time: UnsafeMutablePointer<FFIBlockTime>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_set_last_updated_balance_block_time")
func managed_identity_set_last_updated_balance_block_time(
    _ identity_handle: Handle,
    _ block_time: FFIBlockTime,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_last_synced_keys_block_time")
func managed_identity_get_last_synced_keys_block_time(
    _ identity_handle: Handle,
    _ out_block_time: UnsafeMutablePointer<FFIBlockTime>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - Revision / public keys

@_silgen_name("managed_identity_get_revision")
func managed_identity_get_revision(
    _ identity_handle: Handle,
    _ out_revision: UnsafeMutablePointer<UInt64>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

/// Mirrors `IdentityPublicKeyFFI` from
/// `rs-platform-wallet-ffi/src/managed_identity.rs`. See there for
/// the field semantics and ownership rules.
struct IdentityPublicKeyFFI {
    var key_id: UInt32
    var purpose: UInt8
    var security_level: UInt8
    var key_type: UInt8
    var read_only: Bool
    var disabled_at_is_some: Bool
    var disabled_at: UInt64
    var data_ptr: UnsafeMutablePointer<UInt8>?
    var data_len: Int
}

@_silgen_name("managed_identity_get_public_keys")
func managed_identity_get_public_keys(
    _ identity_handle: Handle,
    _ out_keys: UnsafeMutablePointer<UnsafeMutablePointer<IdentityPublicKeyFFI>?>,
    _ out_count: UnsafeMutablePointer<Int>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_free_public_keys")
func managed_identity_free_public_keys(
    _ keys: UnsafeMutablePointer<IdentityPublicKeyFFI>?,
    _ count: Int
)

@_silgen_name("managed_identity_get_sent_contact_request_ids")
func managed_identity_get_sent_contact_request_ids(
    _ identity_handle: Handle,
    _ out_array: UnsafeMutablePointer<IdentifierArray>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_incoming_contact_request_ids")
func managed_identity_get_incoming_contact_request_ids(
    _ identity_handle: Handle,
    _ out_array: UnsafeMutablePointer<IdentifierArray>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_established_contact_ids")
func managed_identity_get_established_contact_ids(
    _ identity_handle: Handle,
    _ out_array: UnsafeMutablePointer<IdentifierArray>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_sent_contact_request")
func managed_identity_get_sent_contact_request(
    _ identity_handle: Handle,
    _ recipient_id: IdentifierBytes,
    _ out_request_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_incoming_contact_request")
func managed_identity_get_incoming_contact_request(
    _ identity_handle: Handle,
    _ sender_id: IdentifierBytes,
    _ out_request_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_established_contact")
func managed_identity_get_established_contact(
    _ identity_handle: Handle,
    _ contact_id: IdentifierBytes,
    _ out_contact_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_is_contact_established")
func managed_identity_is_contact_established(
    _ identity_handle: Handle,
    _ contact_id: IdentifierBytes,
    _ out_is_established: UnsafeMutablePointer<Bool>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_send_contact_request")
func managed_identity_send_contact_request(
    _ identity_handle: Handle,
    _ recipient_id: IdentifierBytes,
    _ sender_key_index: UInt32,
    _ recipient_key_index: UInt32,
    _ account_reference: UInt32,
    _ encrypted_public_key: UnsafePointer<UInt8>?,
    _ encrypted_public_key_len: Int,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_accept_contact_request")
func managed_identity_accept_contact_request(
    _ identity_handle: Handle,
    _ sender_id: IdentifierBytes,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_reject_contact_request")
func managed_identity_reject_contact_request(
    _ identity_handle: Handle,
    _ sender_id: IdentifierBytes,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_destroy")
func managed_identity_destroy(_ handle: Handle)

// MARK: - DashPay Profile

/// Mirrors `DashPayProfileFFI` from
/// `rs-platform-wallet-ffi/src/dashpay_profile.rs`. See there for
/// field semantics and ownership rules.
///
/// `display_name`, `public_message`, `avatar_url` are heap-allocated
/// C strings (nullable); the caller releases them with
/// `dashpay_profile_ffi_free`. `avatar_hash` / `avatar_fingerprint`
/// are inline arrays — read them only when the corresponding
/// `_is_some` flag is true.
struct DashPayProfileFFI {
    var display_name: UnsafeMutablePointer<CChar>?
    var public_message: UnsafeMutablePointer<CChar>?
    var avatar_url: UnsafeMutablePointer<CChar>?
    var avatar_hash_is_some: Bool
    var avatar_hash: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
    var avatar_fingerprint_is_some: Bool
    var avatar_fingerprint: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
}

/// All-zero `DashPayProfileFFI` — used as the out-param initial
/// value. Writing an empty instance before the FFI call guarantees
/// any early-exit paths still leave a well-defined struct.
func dashPayProfileFFIEmpty() -> DashPayProfileFFI {
    DashPayProfileFFI(
        display_name: nil,
        public_message: nil,
        avatar_url: nil,
        avatar_hash_is_some: false,
        avatar_hash: (
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0
        ),
        avatar_fingerprint_is_some: false,
        avatar_fingerprint: (0, 0, 0, 0, 0, 0, 0, 0)
    )
}

@_silgen_name("managed_identity_get_dashpay_profile")
func managed_identity_get_dashpay_profile(
    _ identity_handle: Handle,
    _ out_profile: UnsafeMutablePointer<DashPayProfileFFI>,
    _ out_has_profile: UnsafeMutablePointer<Bool>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_get_dashpay_profile")
func platform_wallet_get_dashpay_profile(
    _ wallet_handle: Handle,
    _ identity_id: IdentifierBytes,
    _ out_profile: UnsafeMutablePointer<DashPayProfileFFI>,
    _ out_has_profile: UnsafeMutablePointer<Bool>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("dashpay_profile_ffi_free")
func dashpay_profile_ffi_free(_ profile: DashPayProfileFFI)

@_silgen_name("platform_wallet_sync_dashpay_profiles")
func platform_wallet_sync_dashpay_profiles(
    _ wallet_handle: Handle,
    _ out_synced_count: UnsafeMutablePointer<UInt32>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_create_dashpay_profile")
func platform_wallet_create_dashpay_profile(
    _ wallet_handle: Handle,
    _ identity_id: IdentifierBytes,
    _ display_name: UnsafePointer<CChar>?,
    _ public_message: UnsafePointer<CChar>?,
    _ avatar_url: UnsafePointer<CChar>?,
    _ avatar_bytes: UnsafePointer<UInt8>?,
    _ avatar_bytes_len: Int,
    _ out_profile: UnsafeMutablePointer<DashPayProfileFFI>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_update_dashpay_profile")
func platform_wallet_update_dashpay_profile(
    _ wallet_handle: Handle,
    _ identity_id: IdentifierBytes,
    _ display_name: UnsafePointer<CChar>?,
    _ public_message: UnsafePointer<CChar>?,
    _ avatar_url: UnsafePointer<CChar>?,
    _ avatar_bytes: UnsafePointer<UInt8>?,
    _ avatar_bytes_len: Int,
    _ out_profile: UnsafeMutablePointer<DashPayProfileFFI>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - ContactRequest Functions

@_silgen_name("contact_request_create")
func contact_request_create(
    _ sender_id: IdentifierBytes,
    _ recipient_id: IdentifierBytes,
    _ sender_key_index: UInt32,
    _ recipient_key_index: UInt32,
    _ account_reference: UInt32,
    _ encrypted_public_key: UnsafePointer<UInt8>?,
    _ encrypted_public_key_len: Int,
    _ created_at: UInt64,
    _ out_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("contact_request_get_sender_id")
func contact_request_get_sender_id(
    _ request_handle: Handle,
    _ out_id: UnsafeMutablePointer<IdentifierBytes>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("contact_request_get_recipient_id")
func contact_request_get_recipient_id(
    _ request_handle: Handle,
    _ out_id: UnsafeMutablePointer<IdentifierBytes>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("contact_request_get_sender_key_index")
func contact_request_get_sender_key_index(
    _ request_handle: Handle,
    _ out_index: UnsafeMutablePointer<UInt32>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("contact_request_get_recipient_key_index")
func contact_request_get_recipient_key_index(
    _ request_handle: Handle,
    _ out_index: UnsafeMutablePointer<UInt32>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("contact_request_get_account_reference")
func contact_request_get_account_reference(
    _ request_handle: Handle,
    _ out_reference: UnsafeMutablePointer<UInt32>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("contact_request_get_encrypted_public_key")
func contact_request_get_encrypted_public_key(
    _ request_handle: Handle,
    _ out_bytes: UnsafeMutablePointer<UnsafeMutablePointer<UInt8>?>,
    _ out_len: UnsafeMutablePointer<Int>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("contact_request_get_created_at")
func contact_request_get_created_at(
    _ request_handle: Handle,
    _ out_timestamp: UnsafeMutablePointer<UInt64>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("contact_request_destroy")
func contact_request_destroy(_ handle: Handle)

// MARK: - EstablishedContact Functions

@_silgen_name("established_contact_get_contact_identity_id")
func established_contact_get_contact_identity_id(
    _ contact_handle: Handle,
    _ out_id: UnsafeMutablePointer<IdentifierBytes>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("established_contact_get_alias")
func established_contact_get_alias(
    _ contact_handle: Handle,
    _ out_alias: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("established_contact_set_alias")
func established_contact_set_alias(
    _ contact_handle: Handle,
    _ alias: UnsafePointer<CChar>?,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("established_contact_clear_alias")
func established_contact_clear_alias(
    _ contact_handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("established_contact_get_note")
func established_contact_get_note(
    _ contact_handle: Handle,
    _ out_note: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("established_contact_set_note")
func established_contact_set_note(
    _ contact_handle: Handle,
    _ note: UnsafePointer<CChar>?,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("established_contact_clear_note")
func established_contact_clear_note(
    _ contact_handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("established_contact_is_hidden")
func established_contact_is_hidden(
    _ contact_handle: Handle,
    _ out_is_hidden: UnsafeMutablePointer<Bool>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("established_contact_hide")
func established_contact_hide(
    _ contact_handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("established_contact_unhide")
func established_contact_unhide(
    _ contact_handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("established_contact_destroy")
func established_contact_destroy(_ handle: Handle)

// MARK: - Utility Functions

@_silgen_name("platform_wallet_generate_random_identifier")
func platform_wallet_generate_random_identifier(
    _ out_id: UnsafeMutablePointer<IdentifierBytes>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_identifier_array_free")
func platform_wallet_identifier_array_free(_ array: IdentifierArray)

@_silgen_name("platform_wallet_string_free")
func platform_wallet_string_free(_ string: UnsafeMutablePointer<CChar>)

@_silgen_name("platform_wallet_bytes_free")
func platform_wallet_bytes_free(_ bytes: UnsafeMutablePointer<UInt8>)

@_silgen_name("platform_wallet_ffi_error_free")
func platform_wallet_ffi_error_free(_ error: PlatformWalletFFIError)

// MARK: - DPNS name FFI

/// Mirrors `DpnsSearchResultFFI` from
/// `rs-platform-wallet-ffi/src/dpns.rs`. Caller owns each entry's
/// `label` C-string; release the whole array (labels included) via
/// `dpns_search_results_free`.
struct DpnsSearchResultFFI {
    var identity_id: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
    var label: UnsafeMutablePointer<CChar>?
}

@_silgen_name("platform_wallet_register_dpns_name")
func platform_wallet_register_dpns_name(
    _ wallet_handle: Handle,
    _ identity_id: IdentifierBytes,
    _ name: UnsafePointer<CChar>?,
    _ out_full_domain_name: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_resolve_dpns_name")
func platform_wallet_resolve_dpns_name(
    _ wallet_handle: Handle,
    _ name: UnsafePointer<CChar>?,
    _ out_identity_id: UnsafeMutablePointer<IdentifierBytes>,
    _ out_found: UnsafeMutablePointer<Bool>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_search_dpns_names")
func platform_wallet_search_dpns_names(
    _ wallet_handle: Handle,
    _ prefix: UnsafePointer<CChar>?,
    _ limit: UInt32,
    _ out_results: UnsafeMutablePointer<UnsafeMutablePointer<DpnsSearchResultFFI>?>,
    _ out_count: UnsafeMutablePointer<Int>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("dpns_search_results_free")
func dpns_search_results_free(
    _ results: UnsafeMutablePointer<DpnsSearchResultFFI>?,
    _ count: Int
)

/// Mirrors `DpnsNameArray` from `rs-platform-wallet-ffi/src/dpns.rs`.
/// Each `labels[i]` is an owned NUL-terminated UTF-8 C-string;
/// release the whole array (labels included) via
/// `dpns_name_array_free`.
struct DpnsNameArray {
    var labels: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
    var count: Int
}

@_silgen_name("platform_wallet_sync_dpns_names")
func platform_wallet_sync_dpns_names(
    _ wallet_handle: Handle,
    _ identity_id: IdentifierBytes,
    _ out_added: UnsafeMutablePointer<UInt32>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_dpns_names")
func managed_identity_get_dpns_names(
    _ identity_handle: Handle,
    _ out_array: UnsafeMutablePointer<DpnsNameArray>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("dpns_name_array_free")
func dpns_name_array_free(_ array: DpnsNameArray)

@_silgen_name("platform_wallet_sync_contested_dpns_names")
func platform_wallet_sync_contested_dpns_names(
    _ wallet_handle: Handle,
    _ identity_id: IdentifierBytes,
    _ out_count: UnsafeMutablePointer<UInt32>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_contested_dpns_names")
func managed_identity_get_contested_dpns_names(
    _ identity_handle: Handle,
    _ out_array: UnsafeMutablePointer<DpnsNameArray>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - DashPay contact requests + payments FFI

/// Mirrors `ContactRequestHandleArray` from
/// `rs-platform-wallet-ffi/src/dashpay.rs`. Caller owns both the
/// array and every handle inside it; release via
/// `platform_wallet_contact_request_handle_array_free` (array) and
/// `contact_request_destroy` (each handle).
struct ContactRequestHandleArray {
    var handles: UnsafeMutablePointer<Handle>?
    var count: Int
}

@_silgen_name("platform_wallet_contact_request_handle_array_free")
func platform_wallet_contact_request_handle_array_free(_ array: ContactRequestHandleArray)

@_silgen_name("platform_wallet_get_managed_identity")
func platform_wallet_get_managed_identity(
    _ wallet_handle: Handle,
    _ identity_id: IdentifierBytes,
    _ out_managed_identity_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_send_contact_request")
func platform_wallet_send_contact_request(
    _ wallet_handle: Handle,
    _ sender_identity_id: IdentifierBytes,
    _ recipient_identity_id: IdentifierBytes,
    _ account_label: UnsafePointer<CChar>?,
    _ auto_accept_proof: UnsafePointer<UInt8>?,
    _ auto_accept_proof_len: Int,
    _ out_request_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_sync_contact_requests")
func platform_wallet_sync_contact_requests(
    _ wallet_handle: Handle,
    _ out_array: UnsafeMutablePointer<ContactRequestHandleArray>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_accept_contact_request")
func platform_wallet_accept_contact_request(
    _ wallet_handle: Handle,
    _ request_handle: Handle,
    _ out_established_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_reject_contact_request")
func platform_wallet_reject_contact_request(
    _ wallet_handle: Handle,
    _ our_identity_id: IdentifierBytes,
    _ contact_identity_id: IdentifierBytes,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_fetch_sent_contact_requests")
func platform_wallet_fetch_sent_contact_requests(
    _ wallet_handle: Handle,
    _ identity_id: IdentifierBytes,
    _ out_array: UnsafeMutablePointer<ContactRequestHandleArray>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_send_dashpay_payment")
func platform_wallet_send_dashpay_payment(
    _ wallet_handle: Handle,
    _ from_identity_id: IdentifierBytes,
    _ to_contact_identity_id: IdentifierBytes,
    _ amount_duffs: UInt64,
    _ memo: UnsafePointer<CChar>?,
    _ out_txid: UnsafeMutablePointer<(
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - Identity persistence FFI

/// 32-byte C tuple — mirrors a single `[u8; 32]` on the Rust side.
/// Swift imports fixed-size byte arrays this way; every persister-
/// callback struct that carries an `identity_id` / `wallet_id` uses
/// this shape.
typealias FFIByteTuple32 = (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
)

/// Mirrors `IdentityEntryFFI` from
/// `rs-platform-wallet-ffi/src/identity_persistence.rs`. See that
/// file for the field-by-field semantics. Heap allocations (the
/// `label` C-string) are released by the Rust side after the
/// callback returns — Swift must finish reading any strings before
/// yielding.
struct IdentityEntryFFI {
    var identity_id: FFIByteTuple32
    var balance: UInt64
    var revision: UInt64
    var identity_index: UInt32
    var label: UnsafeMutablePointer<CChar>?
    var status: UInt8
    var wallet_id_is_some: Bool
    var wallet_id: FFIByteTuple32
}

/// Mirrors `IdentityKeyEntryFFI` from
/// `rs-platform-wallet-ffi/src/identity_persistence.rs`.
///
/// `private_key_kind` decodes as:
///   0 → None (ignore the `private_key_*` columns)
///   1 → Clear (`private_key_bytes` holds raw 32-byte key material)
///   2 → AtWalletDerivationPath (`private_key_wallet_id` +
///       `private_key_derivation_path` identify the seed-derived
///       key; no Keychain write needed because the seed itself is
///       already stored at the wallet level).
struct IdentityKeyEntryFFI {
    var identity_id: FFIByteTuple32
    var key_id: UInt32
    var purpose: UInt8
    var security_level: UInt8
    var key_type: UInt8
    var read_only: Bool
    var disabled_at_is_some: Bool
    var disabled_at: UInt64
    var public_key_data_ptr: UnsafeMutablePointer<UInt8>?
    var public_key_data_len: Int
    var private_key_kind: UInt8
    var private_key_bytes: FFIByteTuple32
    var private_key_wallet_id: FFIByteTuple32
    var private_key_derivation_path: UnsafeMutablePointer<CChar>?
}

/// Mirrors `IdentityKeyRemovalFFI` from
/// `rs-platform-wallet-ffi/src/identity_persistence.rs` — the
/// `(identity_id, key_id)` composite used by the keys-changeset
/// `removed` surface.
struct IdentityKeyRemovalFFI {
    var identity_id: FFIByteTuple32
    var key_id: UInt32
}
