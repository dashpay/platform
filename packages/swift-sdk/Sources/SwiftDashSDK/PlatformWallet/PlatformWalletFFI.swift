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
