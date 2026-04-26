// Platform Wallet FFI function declarations
// Since these aren't in the C header, we declare them with @_silgen_name

import Foundation

// MARK: - PlatformWalletInfo Functions

@_silgen_name("platform_wallet_info_create_from_seed")
func platform_wallet_info_create_from_seed(
    _ network: NetworkType,
    _ seed: UnsafePointer<UInt8>?,
    _ seed_len: Int,
    _ out_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_info_create_from_mnemonic")
func platform_wallet_info_create_from_mnemonic(
    _ network: NetworkType,
    _ mnemonic: UnsafePointer<CChar>?,
    _ passphrase: UnsafePointer<CChar>?,
    _ out_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_info_get_identity_manager")
func platform_wallet_info_get_identity_manager(
    _ wallet_handle: Handle,
    _ out_manager_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_info_set_identity_manager")
func platform_wallet_info_set_identity_manager(
    _ wallet_handle: Handle,
    _ manager_handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_info_destroy")
func platform_wallet_info_destroy(_ handle: Handle) -> PlatformWalletFFIResult

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
    _ identity_id: UnsafePointer<UInt8>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("identity_manager_get_identity")
func identity_manager_get_identity(
    _ manager_handle: Handle,
    _ identity_id: UnsafePointer<UInt8>,
    _ out_identity_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("identity_manager_get_all_identity_ids")
func identity_manager_get_all_identity_ids(
    _ manager_handle: Handle,
    _ out_array: UnsafeMutablePointer<IdentifierArray>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// Note: `identity_manager_get_primary_identity_id` /
// `identity_manager_set_primary_identity` were removed alongside the
// underlying Rust field. Primary-identity selection moved to the UI
// layer (e.g. `WalletDataModel.selectedIdentityId` on the Swift side).

@_silgen_name("identity_manager_get_identity_count")
func identity_manager_get_identity_count(
    _ manager_handle: Handle,
    _ out_count: UnsafeMutablePointer<Int>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("identity_manager_destroy")
func identity_manager_destroy(_ handle: Handle) -> PlatformWalletFFIResult

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
    _ out_id: UnsafeMutablePointer<UInt8>,
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
    _ block_time: UnsafePointer<FFIBlockTime>,
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
    _ recipient_id: UnsafePointer<UInt8>,
    _ out_request_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_incoming_contact_request")
func managed_identity_get_incoming_contact_request(
    _ identity_handle: Handle,
    _ sender_id: UnsafePointer<UInt8>,
    _ out_request_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_established_contact")
func managed_identity_get_established_contact(
    _ identity_handle: Handle,
    _ contact_id: UnsafePointer<UInt8>,
    _ out_contact_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_is_contact_established")
func managed_identity_is_contact_established(
    _ identity_handle: Handle,
    _ contact_id: UnsafePointer<UInt8>,
    _ out_is_established: UnsafeMutablePointer<Bool>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_send_contact_request")
func managed_identity_send_contact_request(
    _ identity_handle: Handle,
    _ request_handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_accept_contact_request")
func managed_identity_accept_contact_request(
    _ identity_handle: Handle,
    _ request_handle: Handle,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_reject_contact_request")
func managed_identity_reject_contact_request(
    _ identity_handle: Handle,
    _ sender_id: UnsafePointer<UInt8>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_destroy")
func managed_identity_destroy(_ handle: Handle) -> PlatformWalletFFIResult

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
    _ identity_id: UnsafePointer<UInt8>,
    _ out_profile: UnsafeMutablePointer<DashPayProfileFFI>,
    _ out_has_profile: UnsafeMutablePointer<Bool>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("dashpay_profile_ffi_free")
func dashpay_profile_ffi_free(_ profile: UnsafeMutablePointer<DashPayProfileFFI>)

@_silgen_name("platform_wallet_sync_dashpay_profiles")
func platform_wallet_sync_dashpay_profiles(
    _ wallet_handle: Handle,
    _ out_synced_count: UnsafeMutablePointer<UInt32>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_create_dashpay_profile")
func platform_wallet_create_dashpay_profile(
    _ wallet_handle: Handle,
    _ identity_id: UnsafePointer<UInt8>,
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
    _ identity_id: UnsafePointer<UInt8>,
    _ display_name: UnsafePointer<CChar>?,
    _ public_message: UnsafePointer<CChar>?,
    _ avatar_url: UnsafePointer<CChar>?,
    _ avatar_bytes: UnsafePointer<UInt8>?,
    _ avatar_bytes_len: Int,
    _ out_profile: UnsafeMutablePointer<DashPayProfileFFI>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

/// Mirrors `platform_wallet_create_or_update_dashpay_profile_with_signer`
/// from Rust (`packages/rs-platform-wallet-ffi/src/dashpay_profile.rs`).
///
/// `do_create == true` calls
/// `IdentityWallet::create_profile_with_external_signer`; `false`
/// calls `update_profile_with_external_signer`. Both route the
/// document state-transition signature through the supplied
/// `signer_handle` (typically `KeychainSigner.handle`) instead of an
/// internal `IdentitySigner`. Required for watch-only wallets and
/// the architecturally correct path per `swift-sdk/CLAUDE.md`.
@_silgen_name("platform_wallet_create_or_update_dashpay_profile_with_signer")
func platform_wallet_create_or_update_dashpay_profile_with_signer(
    _ wallet_handle: Handle,
    _ identity_id: UnsafePointer<UInt8>,
    _ display_name: UnsafePointer<CChar>?,
    _ public_message: UnsafePointer<CChar>?,
    _ avatar_url: UnsafePointer<CChar>?,
    _ avatar_bytes: UnsafePointer<UInt8>?,
    _ avatar_bytes_len: Int,
    _ do_create: Bool,
    _ signer_handle: OpaquePointer?,
    _ out_profile: UnsafeMutablePointer<DashPayProfileFFI>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - ContactRequest Functions

@_silgen_name("contact_request_create")
func contact_request_create(
    _ sender_id: UnsafePointer<UInt8>,
    _ recipient_id: UnsafePointer<UInt8>,
    _ sender_key_index: UInt32,
    _ recipient_key_index: UInt32,
    _ account_reference: UInt32,
    _ encrypted_public_key: UnsafePointer<UInt8>?,
    _ encrypted_public_key_len: Int,
    _ core_height_created_at: UInt32,
    _ created_at: UInt64,
    _ out_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("contact_request_get_sender_id")
func contact_request_get_sender_id(
    _ request_handle: Handle,
    _ out_id: UnsafeMutablePointer<UInt8>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("contact_request_get_recipient_id")
func contact_request_get_recipient_id(
    _ request_handle: Handle,
    _ out_id: UnsafeMutablePointer<UInt8>,
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
func contact_request_destroy(_ handle: Handle) -> PlatformWalletFFIResult

// MARK: - EstablishedContact Functions

@_silgen_name("established_contact_get_contact_identity_id")
func established_contact_get_contact_identity_id(
    _ contact_handle: Handle,
    _ out_id: UnsafeMutablePointer<UInt8>,
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
func established_contact_destroy(_ handle: Handle) -> PlatformWalletFFIResult

// MARK: - Utility Functions

@_silgen_name("platform_wallet_generate_random_identifier")
func platform_wallet_generate_random_identifier(
    _ out_id: UnsafeMutablePointer<UInt8>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_identifier_array_free")
func platform_wallet_identifier_array_free(_ array: UnsafeMutablePointer<IdentifierArray>)

@_silgen_name("platform_wallet_string_free")
func platform_wallet_string_free(_ string: UnsafeMutablePointer<CChar>)

@_silgen_name("platform_wallet_bytes_free")
func platform_wallet_bytes_free(_ bytes: UnsafeMutablePointer<UInt8>, _ len: Int)

@_silgen_name("platform_wallet_ffi_error_free")
func platform_wallet_ffi_error_free(_ error: UnsafeMutablePointer<PlatformWalletFFIError>)

/// hash160 = RIPEMD160(SHA256(data)). 20-byte output.
///
/// Mirrors `platform_wallet_hash160` from
/// `rs-platform-wallet-ffi/src/utils.rs`. Exposed so the keychain
/// metadata writer can stamp `publicKeyHash` without pulling a
/// RIPEMD-160 implementation into the Swift side (CommonCrypto and
/// CryptoKit don't expose one). Returns 0 on success, -1 on a null /
/// zero-length input.
@_silgen_name("platform_wallet_hash160")
func platform_wallet_hash160(
    _ data: UnsafePointer<UInt8>?,
    _ data_len: Int,
    _ out_hash: UnsafeMutablePointer<UInt8>?
) -> Int32

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
    _ identity_id: UnsafePointer<UInt8>,
    _ name: UnsafePointer<CChar>?,
    _ out_full_domain_name: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

/// Mirrors `platform_wallet_register_dpns_name_with_signer` from Rust
/// (`packages/rs-platform-wallet-ffi/src/dpns.rs`).
///
/// Same as `platform_wallet_register_dpns_name` but signing is routed
/// through the supplied `signer_handle` (typically `KeychainSigner.handle`)
/// instead of through a wallet-derived `IdentitySigner`. Required for
/// watch-only wallets and the path that avoids the inner-lock-deadlock
/// the legacy variant hit when its derivation path tried to
/// `blocking_read` the wallet manager from inside a Tokio worker.
///
/// The wallet handle is still required so Rust can look up the
/// identity from the in-process `IdentityManager` and pick the
/// HIGH/CRITICAL authentication key DPP requires for document state
/// transitions — but no signing happens via the wallet's own seed.
///
/// Caller retains ownership of the signer handle.
@_silgen_name("platform_wallet_register_dpns_name_with_signer")
func platform_wallet_register_dpns_name_with_signer(
    _ wallet_handle: Handle,
    _ identity_id: UnsafePointer<UInt8>,
    _ name: UnsafePointer<CChar>?,
    // Raw `*mut SignerHandle` produced by `dash_sdk_signer_create_with_ctx`
    // (e.g. via `KeychainSigner.handle`). Used as `Signer<IdentityPublicKey>`.
    // Caller retains ownership; this function does NOT destroy it.
    _ signer_handle: OpaquePointer?,
    _ out_full_domain_name: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_resolve_dpns_name")
func platform_wallet_resolve_dpns_name(
    _ wallet_handle: Handle,
    _ name: UnsafePointer<CChar>?,
    _ out_identity_id: UnsafeMutablePointer<UInt8>,
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
    _ identity_id: UnsafePointer<UInt8>,
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
func dpns_name_array_free(_ array: UnsafeMutablePointer<DpnsNameArray>)

@_silgen_name("platform_wallet_sync_contested_dpns_names")
func platform_wallet_sync_contested_dpns_names(
    _ wallet_handle: Handle,
    _ identity_id: UnsafePointer<UInt8>,
    _ out_count: UnsafeMutablePointer<UInt32>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_contested_dpns_names")
func managed_identity_get_contested_dpns_names(
    _ identity_handle: Handle,
    _ out_array: UnsafeMutablePointer<DpnsNameArray>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - Wallet Memory Explorer FFI

/// Mirrors `PlatformWalletMemorySummaryFFI` from
/// `rs-platform-wallet-ffi/src/memory_explorer.rs`.
///
/// Caller-owned struct — Rust populates the slot the caller already
/// allocates, no `_free` is required.
struct PlatformWalletMemorySummaryFFI {
    var identities_count: Int
    var watched_count: Int
    /// One past the wallet's highest already-registered identity
    /// index — the resume position the gap-limit scanner uses next.
    /// `0` when nothing has been registered yet.
    var last_scanned_index: UInt32
    var tracked_asset_locks_count: Int
    var token_balances_count: Int
}

/// All-zero initial value — passed in before the FFI call so that
/// any early-exit path leaves a well-defined struct.
func platformWalletMemorySummaryFFIEmpty() -> PlatformWalletMemorySummaryFFI {
    PlatformWalletMemorySummaryFFI(
        identities_count: 0,
        watched_count: 0,
        last_scanned_index: 0,
        tracked_asset_locks_count: 0,
        token_balances_count: 0
    )
}

@_silgen_name("platform_wallet_list_in_memory_identity_ids")
func platform_wallet_list_in_memory_identity_ids(
    _ wallet_handle: Handle,
    _ out_array: UnsafeMutablePointer<IdentifierArray>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_list_in_memory_watched_identity_ids")
func platform_wallet_list_in_memory_watched_identity_ids(
    _ wallet_handle: Handle,
    _ out_array: UnsafeMutablePointer<IdentifierArray>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_get_in_memory_summary")
func platform_wallet_get_in_memory_summary(
    _ wallet_handle: Handle,
    _ out: UnsafeMutablePointer<PlatformWalletMemorySummaryFFI>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_identity_index")
func managed_identity_get_identity_index(
    _ identity_handle: Handle,
    _ out_has_index: UnsafeMutablePointer<Bool>,
    _ out_index: UnsafeMutablePointer<UInt32>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("managed_identity_get_status")
func managed_identity_get_status(
    _ identity_handle: Handle,
    _ out_status: UnsafeMutablePointer<UInt8>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - Contest vote state FFI

/// Mirrors `ContestContenderFFI` from
/// `rs-platform-wallet-ffi/src/dpns.rs`. Plain scalar struct; no
/// owned allocations (reclaimed wholesale when the parent's
/// contenders buffer is freed via `contest_vote_state_ffi_free`).
struct ContestContenderFFI {
    var identity_id: FFIByteTuple32
    var vote_tally: UInt32
}

/// Mirrors `ContestVoteStateFFI`. Caller owns `label` + the
/// `contenders_ptr` array; release via
/// `contest_vote_state_ffi_free`. Safe to free on a zeroed default.
struct ContestVoteStateFFI {
    var label: UnsafeMutablePointer<CChar>?
    var end_time_ms: UInt64
    var contenders_ptr: UnsafeMutablePointer<ContestContenderFFI>?
    var contenders_count: Int
    var abstain_votes: UInt32
    var lock_votes: UInt32
    /// 0 = None, 1 = WonByIdentity, 2 = Locked.
    /// `winner_identity_id` only valid when `winner_kind == 1`.
    var winner_kind: UInt8
    var winner_identity_id: FFIByteTuple32
}

/// All-zero initial value — the "not found" path leaves this
/// shape, and `contest_vote_state_ffi_free` treats it as a no-op.
func contestVoteStateFFIEmpty() -> ContestVoteStateFFI {
    ContestVoteStateFFI(
        label: nil,
        end_time_ms: 0,
        contenders_ptr: nil,
        contenders_count: 0,
        abstain_votes: 0,
        lock_votes: 0,
        winner_kind: 0,
        winner_identity_id: (
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
        )
    )
}

@_silgen_name("platform_wallet_fetch_contest_vote_state")
func platform_wallet_fetch_contest_vote_state(
    _ wallet_handle: Handle,
    _ identity_id: UnsafePointer<UInt8>,
    _ label: UnsafePointer<CChar>?,
    _ out_state: UnsafeMutablePointer<ContestVoteStateFFI>,
    _ out_found: UnsafeMutablePointer<Bool>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("contest_vote_state_ffi_free")
func contest_vote_state_ffi_free(_ state: UnsafeMutablePointer<ContestVoteStateFFI>)

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
func platform_wallet_contact_request_handle_array_free(
    _ array: UnsafeMutablePointer<ContactRequestHandleArray>
)

@_silgen_name("platform_wallet_get_managed_identity")
func platform_wallet_get_managed_identity(
    _ wallet_handle: Handle,
    _ identity_id: UnsafePointer<UInt8>,
    _ out_managed_identity_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_send_contact_request")
func platform_wallet_send_contact_request(
    _ wallet_handle: Handle,
    _ sender_identity_id: UnsafePointer<UInt8>,
    _ recipient_identity_id: UnsafePointer<UInt8>,
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
    _ our_identity_id: UnsafePointer<UInt8>,
    _ contact_identity_id: UnsafePointer<UInt8>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_fetch_sent_contact_requests")
func platform_wallet_fetch_sent_contact_requests(
    _ wallet_handle: Handle,
    _ identity_id: UnsafePointer<UInt8>,
    _ out_array: UnsafeMutablePointer<ContactRequestHandleArray>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_send_dashpay_payment")
func platform_wallet_send_dashpay_payment(
    _ wallet_handle: Handle,
    _ from_identity_id: UnsafePointer<UInt8>,
    _ to_contact_identity_id: UnsafePointer<UInt8>,
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

/// Mirrors `platform_wallet_send_contact_request_with_signer` from
/// Rust. Same shape as `platform_wallet_send_contact_request` but the
/// document state-transition signature is routed through
/// `signer_handle` (typically `KeychainSigner.handle`) instead of an
/// internal `IdentitySigner`.
@_silgen_name("platform_wallet_send_contact_request_with_signer")
func platform_wallet_send_contact_request_with_signer(
    _ wallet_handle: Handle,
    _ sender_identity_id: UnsafePointer<UInt8>,
    _ recipient_identity_id: UnsafePointer<UInt8>,
    _ account_label: UnsafePointer<CChar>?,
    _ auto_accept_proof: UnsafePointer<UInt8>?,
    _ auto_accept_proof_len: Int,
    _ signer_handle: OpaquePointer?,
    _ out_request_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

/// Mirrors `platform_wallet_accept_contact_request_with_signer` from
/// Rust.
@_silgen_name("platform_wallet_accept_contact_request_with_signer")
func platform_wallet_accept_contact_request_with_signer(
    _ wallet_handle: Handle,
    _ request_handle: Handle,
    _ signer_handle: OpaquePointer?,
    _ out_established_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - Identity transfer / withdraw / update — external-signer FFI

/// Mirrors `PlatformAddressCreditOutputFFI` from
/// `rs-platform-wallet-ffi/src/identity_transfer.rs`. Stripped-down
/// version of `AddressBalanceEntryFFI` — only carries
/// `(address_type, hash, credits)` because the SDK fetches the
/// platform-address nonce at submit time.
@frozen
public struct PlatformAddressCreditOutputFFI {
    public var address_type: UInt8
    public var hash: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
    public var credits: UInt64
}

/// Mirrors `platform_wallet_transfer_credits_with_signer` from Rust
/// (`packages/rs-platform-wallet-ffi/src/identity_transfer.rs`).
///
/// Identity → identity credit transfer routed through the supplied
/// `signer_handle` (typically `KeychainSigner.handle`).
@_silgen_name("platform_wallet_transfer_credits_with_signer")
func platform_wallet_transfer_credits_with_signer(
    _ wallet_handle: Handle,
    _ from_identity_id: UnsafePointer<UInt8>,
    _ to_identity_id: UnsafePointer<UInt8>,
    _ amount: UInt64,
    _ signer_handle: OpaquePointer?,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

/// Mirrors `platform_wallet_transfer_credits_to_addresses_with_signer`
/// from Rust. Identity → 1+ platform addresses transfer.
///
/// `out_new_balance` (when non-null) receives the sender's remaining
/// balance after the transfer.
@_silgen_name("platform_wallet_transfer_credits_to_addresses_with_signer")
func platform_wallet_transfer_credits_to_addresses_with_signer(
    _ wallet_handle: Handle,
    _ from_identity_id: UnsafePointer<UInt8>,
    _ outputs: UnsafePointer<PlatformAddressCreditOutputFFI>?,
    _ outputs_count: Int,
    _ signer_handle: OpaquePointer?,
    _ out_new_balance: UnsafeMutablePointer<UInt64>?,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

/// Mirrors `platform_wallet_withdraw_credits_with_signer` from Rust
/// (`packages/rs-platform-wallet-ffi/src/identity_withdrawal.rs`).
///
/// `to_address` is a NUL-terminated UTF-8 C-string carrying a
/// network-aware Dash P2PKH address (e.g. `"yNPbcFf..."` for
/// testnet).
@_silgen_name("platform_wallet_withdraw_credits_with_signer")
func platform_wallet_withdraw_credits_with_signer(
    _ wallet_handle: Handle,
    _ identity_id: UnsafePointer<UInt8>,
    _ amount: UInt64,
    _ to_address: UnsafePointer<CChar>?,
    _ signer_handle: OpaquePointer?,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

/// Mirrors `platform_wallet_update_identity_with_signer` from Rust
/// (`packages/rs-platform-wallet-ffi/src/identity_update.rs`).
///
/// Add and/or disable identity public keys; signs the
/// `IdentityUpdateTransition` with the identity's MASTER auth key
/// via the supplied `signer_handle`.
///
/// The new keys are passed in as flat `IdentityPubkeyFFI` rows
/// (re-uses the registration-with-signer key-row shape). Caller is
/// responsible for pre-persisting each new key's private material to
/// whatever store the signer reads from (iOS Keychain in the typical
/// case) BEFORE calling this — the signer here only signs the
/// update transition itself with an existing MASTER key.
///
/// Pass `(nil, 0)` for either array to skip the corresponding
/// operation (e.g. disable-only or add-only updates).
@_silgen_name("platform_wallet_update_identity_with_signer")
func platform_wallet_update_identity_with_signer(
    _ wallet_handle: Handle,
    _ identity_id: UnsafePointer<UInt8>,
    _ add_public_keys: UnsafePointer<IdentityPubkeyFFI>?,
    _ add_public_keys_count: Int,
    _ disable_public_key_ids: UnsafePointer<UInt32>?,
    _ disable_public_key_ids_count: Int,
    _ signer_handle: OpaquePointer?,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

/// Mirrors `platform_wallet_register_identity_with_funding_signer`
/// from Rust
/// (`packages/rs-platform-wallet-ffi/src/identity_registration_funded_with_signer.rs`).
///
/// Asset-lock-funded identity registration driven by an external
/// signer. The asset lock proof is built Rust-side from
/// `amount_duffs` (wallet must have spendable Core UTXOs); the
/// IdentityCreate state transition is signed via `signer_handle`.
///
/// Caller pre-derives the new identity's authentication pubkeys via
/// `dash_sdk_derive_identity_keys_from_mnemonic` (works for
/// watch-only wallets, unlike the wallet-handle variant) and ships
/// them in via `identity_pubkeys`. Caller is also responsible for
/// pre-persisting each key's matching private material to the
/// signer's store (iOS Keychain in the typical case) BEFORE calling
/// this so the IdentityCreate signature can complete.
@_silgen_name("platform_wallet_register_identity_with_funding_signer")
func platform_wallet_register_identity_with_funding_signer(
    _ wallet_handle: Handle,
    _ amount_duffs: UInt64,
    _ identity_index: UInt32,
    _ identity_pubkeys: UnsafePointer<IdentityPubkeyFFI>?,
    _ identity_pubkeys_count: Int,
    _ signer_handle: OpaquePointer?,
    _ out_identity_id: UnsafeMutablePointer<(
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )>,
    _ out_identity_handle: UnsafeMutablePointer<Handle>,
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
/// file for the field-by-field semantics. No heap allocations cross
/// the boundary on this struct anymore — every field is a scalar or
/// inline byte tuple.
///
/// `identity_index_is_some` mirrors the new `Option<u32>` shape on
/// `ManagedIdentity.identity_index`: false means the source identity
/// is out-of-wallet (observed) and the `identity_index` field should
/// be ignored.
struct IdentityEntryFFI {
    var identity_id: FFIByteTuple32
    var balance: UInt64
    var revision: UInt64
    var identity_index_is_some: Bool
    var identity_index: UInt32
    var status: UInt8
    var wallet_id_is_some: Bool
    var wallet_id: FFIByteTuple32
}

/// 20-byte C tuple — mirrors a single `[u8; 20]` on the Rust side.
/// Used for RIPEMD160(SHA256) public-key hashes on identity keys.
typealias FFIByteTuple20 = (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8
)

/// Mirrors `IdentityKeyEntryFFI` from
/// `rs-platform-wallet-ffi/src/identity_persistence.rs`.
///
/// No private-key bytes cross this boundary — when
/// `wallet_id_is_some` + `derivation_indices_is_some` are both
/// true, the client should re-derive the 32-byte ECDSA scalar from
/// the named wallet's mnemonic at the DIP-9 identity authentication
/// path `m/9'/coin'/5'/0'/ECDSA'/identity_index'/key_index'` and
/// persist it to the keychain on its own side. Any of those flags
/// false = watch-only.
///
/// `public_key_hash` is the precomputed 20-byte
/// RIPEMD160(SHA256) of the public-key bytes — convenience so
/// clients without a RIPEMD-160 implementation can still attach
/// the hash as metadata on the keychain item.
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
    var public_key_hash: FFIByteTuple20
    var wallet_id_is_some: Bool
    var wallet_id: FFIByteTuple32
    var derivation_indices_is_some: Bool
    var identity_index: UInt32
    var key_index: UInt32
}

/// Expected size of `IdentityKeyEntryFFI` as laid out by Rust's
/// `#[repr(C)]` on 64-bit targets. Mirrors the compile-time
/// assertion at the bottom of `rs-platform-wallet-ffi/src/
/// identity_persistence.rs`. Tested at callback entry via
/// `assertIdentityKeyEntryLayout()`.
let EXPECTED_IDENTITY_KEY_ENTRY_FFI_SIZE: Int = 136

/// Verify the Swift `IdentityKeyEntryFFI` mirror lays out to the
/// same 136-byte shape that Rust's `#[repr(C)]` produces. Called
/// once per process from the persistence-callback hot path so a
/// drift between the two sides surfaces as a clean assertion
/// failure rather than an EXC_BAD_ACCESS in memmove.
func assertIdentityKeyEntryLayout() {
    let actual = MemoryLayout<IdentityKeyEntryFFI>.size
    let actualStride = MemoryLayout<IdentityKeyEntryFFI>.stride
    precondition(
        actual == EXPECTED_IDENTITY_KEY_ENTRY_FFI_SIZE
            && actualStride == EXPECTED_IDENTITY_KEY_ENTRY_FFI_SIZE,
        "IdentityKeyEntryFFI layout mismatch: size=\(actual) stride=\(actualStride), "
            + "expected \(EXPECTED_IDENTITY_KEY_ENTRY_FFI_SIZE). Rust-side "
            + "#[repr(C)] and Swift-side struct have diverged; fix one side."
    )
}

/// Mirrors `IdentityKeyRemovalFFI` from
/// `rs-platform-wallet-ffi/src/identity_persistence.rs` — the
/// `(identity_id, key_id)` composite used by the keys-changeset
/// `removed` surface.
struct IdentityKeyRemovalFFI {
    var identity_id: FFIByteTuple32
    var key_id: UInt32
}

// MARK: - Identity discovery FFI

/// Mirrors `DiscoveredIdentityIdsFFI` from
/// `rs-platform-wallet-ffi/src/identity_discovery.rs`. `ids` points
/// at a contiguous `[[u8; 32]; count]` buffer; reclaim the whole
/// struct by handing it back to
/// `platform_wallet_discover_identities_free`. Safe to free on a
/// zero / null struct (no-op).
struct DiscoveredIdentityIdsFFI {
    var ids: UnsafeMutablePointer<FFIByteTuple32>?
    var count: Int
}

/// Initial all-zero value. Useful as a placeholder before calling
/// the FFI and for the unused-error branches.
func discoveredIdentityIdsFFIEmpty() -> DiscoveredIdentityIdsFFI {
    DiscoveredIdentityIdsFFI(ids: nil, count: 0)
}

/// Gap-limit scan of the wallet's DIP-9 identity auth-key path.
/// `start_index_or_neg1 >= 0` starts at that index; negative means
/// "resume from the wallet's cached last_scanned_index". Pass
/// `gap_limit = 0` to use the Rust default (`IDENTITY_GAP_LIMIT`,
/// currently 5). On success, `out_found` receives a heap-allocated
/// id array the caller frees via
/// `platform_wallet_discover_identities_free`.
@_silgen_name("platform_wallet_discover_identities")
func platform_wallet_discover_identities(
    _ wallet_handle: Handle,
    _ start_index_or_neg1: Int64,
    _ gap_limit: UInt32,
    _ out_found: UnsafeMutablePointer<DiscoveredIdentityIdsFFI>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_discover_identities_free")
func platform_wallet_discover_identities_free(
    _ found: UnsafeMutablePointer<DiscoveredIdentityIdsFFI>
)

// MARK: - Identity-registration-key preview FFI

/// Mirrors `IdentityKeyPreviewFFI` from
/// `rs-platform-wallet-ffi/src/identity_key_preview.rs`.
///
/// All heap allocations (`derivation_path`, `public_key`,
/// `private_key_wif`) are owned by Rust; reclaim each row by
/// handing the enclosing `IdentityKeyPreviewsFFI` back to
/// `platform_wallet_preview_identity_registration_keys_free`. Never
/// free these fields individually.
struct IdentityKeyPreviewFFI {
    var identity_index: UInt32
    var derivation_path: UnsafeMutablePointer<CChar>?
    var public_key: UnsafeMutablePointer<UInt8>?
    var public_key_len: Int
    var private_key_wif: UnsafeMutablePointer<CChar>?
    /// Inline 32-byte ECDSA private-key scalar. Mirror of the
    /// `[u8; 32]` field on the Rust struct. Treat as sensitive — the
    /// Swift caller is expected to copy it straight into the iOS
    /// Keychain (via `KeychainManager.storeIdentityPrivateKey`) and
    /// drop the local reference.
    var private_key_bytes: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
}

/// Mirrors `IdentityKeyPreviewsFFI`. `items` points at a contiguous
/// `[IdentityKeyPreviewFFI; count]` buffer. Release the whole
/// struct (rows + their owned strings + pubkey buffers) via
/// `platform_wallet_preview_identity_registration_keys_free`. Safe
/// to free on a zero / null struct (no-op).
struct IdentityKeyPreviewsFFI {
    var items: UnsafeMutablePointer<IdentityKeyPreviewFFI>?
    var count: Int
}

/// Initial all-zero value — lets us pass a well-defined struct into
/// the FFI call and into `_free` on the cleanup path regardless of
/// whether the call succeeded.
func identityKeyPreviewsFFIEmpty() -> IdentityKeyPreviewsFFI {
    IdentityKeyPreviewsFFI(items: nil, count: 0)
}

/// Derive the first N MASTER identity-authentication keypairs this
/// wallet would probe during a discovery scan. `count_or_neg1< 0`
/// picks the Rust-side default (`IDENTITY_GAP_LIMIT`, currently 5).
/// On success, `out_previews` receives a heap-allocated row array
/// the caller frees via
/// `platform_wallet_preview_identity_registration_keys_free`.
@_silgen_name("platform_wallet_preview_identity_registration_keys")
func platform_wallet_preview_identity_registration_keys(
    _ wallet_handle: Handle,
    _ start_index: UInt32,
    _ count_or_neg1: Int32,
    _ out_previews: UnsafeMutablePointer<IdentityKeyPreviewsFFI>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_preview_identity_registration_keys_free")
func platform_wallet_preview_identity_registration_keys_free(
    _ previews: UnsafeMutablePointer<IdentityKeyPreviewsFFI>
)

// MARK: - Pre-registration key derivation

/// Mirrors `IdentityRegistrationKeyDerivationsFFI` from
/// `rs-platform-wallet-ffi/src/identity_registration_with_signer.rs`.
/// Same row layout as `IdentityKeyPreviewsFFI` (re-uses
/// `IdentityKeyPreviewFFI`), but each row is one
/// `(identity_index, key_id)` pair fixed to a single identity_index.
struct IdentityRegistrationKeyDerivationsFFI {
    var items: UnsafeMutablePointer<IdentityKeyPreviewFFI>?
    var count: Int
}

func identityRegistrationKeyDerivationsFFIEmpty() -> IdentityRegistrationKeyDerivationsFFI {
    IdentityRegistrationKeyDerivationsFFI(items: nil, count: 0)
}

/// Derive every authentication-key pair the upcoming
/// `platform_wallet_register_identity_with_signer` call will need
/// for `identity_index`, returning one row per `key_id` in
/// `0..key_count`.
@_silgen_name("platform_wallet_derive_identity_keys_for_index")
func platform_wallet_derive_identity_keys_for_index(
    _ wallet_handle: Handle,
    _ identity_index: UInt32,
    _ key_count: UInt32,
    _ out_rows: UnsafeMutablePointer<IdentityRegistrationKeyDerivationsFFI>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("platform_wallet_derive_identity_keys_for_index_free")
func platform_wallet_derive_identity_keys_for_index_free(
    _ rows: UnsafeMutablePointer<IdentityRegistrationKeyDerivationsFFI>
)

// MARK: - Mnemonic-driven pre-registration key derivation
//
// Companion entry point to
// `platform_wallet_derive_identity_keys_for_index` that takes the
// BIP-39 mnemonic directly instead of routing through a wallet
// handle. The wallet-handle variant fails for wallets restored from
// SwiftData persistence (the seed lives in iOS Keychain, not in the
// in-process `WalletManager`); this one works for every wallet shape
// because it pulls the seed from the caller per call.
//
// Same row layout as the wallet-handle variant
// (`IdentityRegistrationKeyDerivationsFFI` over `IdentityKeyPreviewFFI`).
// Paired free function frees the matching shape — distinct symbol
// name purely so call sites pair allocator with deallocator 1:1.
@_silgen_name("dash_sdk_derive_identity_keys_from_mnemonic")
func dash_sdk_derive_identity_keys_from_mnemonic(
    _ mnemonic_cstr: UnsafePointer<CChar>,
    _ passphrase_cstr: UnsafePointer<CChar>?,
    _ network: DashSDKNetwork,
    _ identity_index: UInt32,
    _ key_count: UInt32,
    _ out_rows: UnsafeMutablePointer<IdentityRegistrationKeyDerivationsFFI>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

@_silgen_name("dash_sdk_derive_identity_keys_from_mnemonic_free")
func dash_sdk_derive_identity_keys_from_mnemonic_free(
    _ rows: UnsafeMutablePointer<IdentityRegistrationKeyDerivationsFFI>
)

// MARK: - Derive-and-persist callback handles
//
// Used by `dash_sdk_derive_and_persist_identity_keys` (below). The
// Swift side hands the Rust derivation loop two opaque callback
// handles — one for fetching the BIP-39 mnemonic out of Keychain,
// one for writing each derived key back into Keychain — and the
// Rust loop owns the orchestration. Closes the
// "no mnemonic round-tripping" rule that ManagedPlatformWallet's
// `prePersistIdentityKeysForRegistration` previously violated.

/// Opaque Rust-side handle to a Swift-owned mnemonic resolver.
/// Allocated via `dash_sdk_mnemonic_resolver_create`, freed via
/// `dash_sdk_mnemonic_resolver_destroy`. The Rust struct itself
/// is private; Swift only ever holds the pointer.
public struct MnemonicResolverHandle {}

/// Opaque Rust-side handle to a Swift-owned identity-key persister.
public struct IdentityKeyPersisterHandle {}

/// Mirrors the Rust `mnemonic_resolver_result` constants in
/// `derive_and_persist_callbacks.rs`. Used as the return value of
/// the resolver callback so the Rust derivation loop can
/// distinguish "buffer too small" (a programmer error) from
/// "wallet has no mnemonic stored" (a recoverable user-visible
/// error).
enum MnemonicResolverResult: Int32 {
    case success = 0
    case notFound = 1
    case bufferTooSmall = 2
    case other = 3
}

/// Buffer capacity (bytes, excluding trailing NUL) the resolver
/// callback is given to write the mnemonic into. Mirrors the Rust
/// `MNEMONIC_RESOLVER_BUFFER_CAPACITY` constant.
let MNEMONIC_RESOLVER_BUFFER_CAPACITY: Int = 1024

/// Function pointer type for the mnemonic-resolve callback.
/// Returns one of [`MnemonicResolverResult`]'s raw values.
typealias MnemonicResolveCallback = @convention(c) (
    _ ctx: UnsafeRawPointer?,
    _ wallet_id_bytes: UnsafePointer<UInt8>?,
    _ out_mnemonic_utf8: UnsafeMutablePointer<CChar>?,
    _ out_capacity: Int,
    _ out_len: UnsafeMutablePointer<Int>?
) -> Int32

/// Mirrors the Rust `PersistKeyArgs` `#[repr(C)]` struct. Pointer-
/// based per-call payload for the identity-key persister callback.
/// Field order, sizes, and trailing-byte alignment match the Rust
/// definition.
struct PersistKeyArgs {
    var wallet_id_bytes: UnsafePointer<UInt8>?
    var identity_index: UInt32
    var key_id: UInt32
    var key_index: UInt32
    var derivation_path_cstr: UnsafePointer<CChar>?
    var public_key_bytes: UnsafePointer<UInt8>?
    var public_key_len: Int
    var public_key_hash_bytes: UnsafePointer<UInt8>?
    var private_key_bytes: UnsafePointer<UInt8>?
    var key_type: UInt8
    var purpose: UInt8
    var security_level: UInt8
}

/// Expected size of `PersistKeyArgs` as laid out by Rust's
/// `#[repr(C)]` on 64-bit targets. Mirrors the compile-time
/// assertion in `rs-platform-wallet-ffi/src/
/// derive_and_persist_callbacks.rs`. Tested at trampoline entry
/// via `assertPersistKeyArgsLayout()`.
let EXPECTED_PERSIST_KEY_ARGS_SIZE: Int = 72

/// Verify the Swift `PersistKeyArgs` mirror lays out to the same
/// 72-byte shape Rust's `#[repr(C)]` produces. Called once per
/// process from the persister-callback hot path so a drift
/// between the two sides surfaces as a clean assertion failure
/// rather than an EXC_BAD_ACCESS in `assumingMemoryBound`.
func assertPersistKeyArgsLayout() {
    let actual = MemoryLayout<PersistKeyArgs>.size
    let actualStride = MemoryLayout<PersistKeyArgs>.stride
    precondition(
        actual == EXPECTED_PERSIST_KEY_ARGS_SIZE
            && actualStride == EXPECTED_PERSIST_KEY_ARGS_SIZE,
        "PersistKeyArgs layout mismatch: size=\(actual) stride=\(actualStride), "
            + "expected \(EXPECTED_PERSIST_KEY_ARGS_SIZE). Rust-side "
            + "#[repr(C)] and Swift-side struct have diverged; fix one side."
    )
}

/// Function pointer type for the per-key persist callback.
/// Returns [`PERSIST_KEY_SUCCESS`] on a successful Keychain write,
/// [`PERSIST_KEY_FAILURE`] to abort the rest of the Rust derivation
/// loop with an `ErrorWalletOperation`.
///
/// The args parameter is `UnsafeRawPointer?` rather than the more
/// natural `UnsafePointer<PersistKeyArgs>?` because Swift's
/// `@convention(c)` typealiases can only carry types representable
/// in Objective-C, and a pointer to a non-`@objc` Swift struct
/// fails that check. The Rust side ships a `*const PersistKeyArgs`
/// regardless; the Swift trampoline unwraps via
/// `assumingMemoryBound(to: PersistKeyArgs.self)`. Same ABI shape
/// as the Rust `extern "C" fn(_, *const PersistKeyArgs) -> u8`
/// declaration.
typealias PersistKeyCallback = @convention(c) (
    _ ctx: UnsafeRawPointer?,
    _ args: UnsafeRawPointer?
) -> UInt8

/// Persister-callback success/failure tags. Mirror the
/// `PERSIST_KEY_SUCCESS` / `PERSIST_KEY_FAILURE` constants in
/// `derive_and_persist_callbacks.rs`. Trampoline implementations
/// return one of these to keep the wire shape consistent.
let PERSIST_KEY_SUCCESS: UInt8 = 1
let PERSIST_KEY_FAILURE: UInt8 = 0

/// Generic Rust-callable destructor for any Swift-owned `ctx`
/// pointer (typically `Unmanaged.passRetained(self).toOpaque()`).
typealias DeriveAndPersistCtxDestroy = @convention(c) (
    _ ctx: UnsafeMutableRawPointer?
) -> Void

@_silgen_name("dash_sdk_mnemonic_resolver_create")
func dash_sdk_mnemonic_resolver_create(
    _ ctx: UnsafeMutableRawPointer?,
    _ resolve_callback: MnemonicResolveCallback,
    _ destroy_callback: DeriveAndPersistCtxDestroy
) -> UnsafeMutablePointer<MnemonicResolverHandle>?

@_silgen_name("dash_sdk_mnemonic_resolver_destroy")
func dash_sdk_mnemonic_resolver_destroy(
    _ handle: UnsafeMutablePointer<MnemonicResolverHandle>?
)

@_silgen_name("dash_sdk_identity_key_persister_create")
func dash_sdk_identity_key_persister_create(
    _ ctx: UnsafeMutableRawPointer?,
    _ persist_callback: PersistKeyCallback,
    _ destroy_callback: DeriveAndPersistCtxDestroy
) -> UnsafeMutablePointer<IdentityKeyPersisterHandle>?

@_silgen_name("dash_sdk_identity_key_persister_destroy")
func dash_sdk_identity_key_persister_destroy(
    _ handle: UnsafeMutablePointer<IdentityKeyPersisterHandle>?
)

/// Single-FFI identity-key derivation + persistence pipeline.
///
/// Companion to the lower-level
/// `dash_sdk_derive_identity_keys_from_mnemonic` (which leaves
/// orchestration to the caller). This entry point owns the
/// per-key loop on the Rust side and calls back into Swift only
/// for the iOS-only Keychain primitives.
///
/// On success `out_pubkeys` is populated with the derived
/// pubkeys (and their paths) for the caller to build
/// `IdentityPubkey` rows for the subsequent registration call;
/// the 32-byte ECDSA private scalars are NOT included — they
/// were already handed to the persister callback per iteration.
/// Release `out_pubkeys` with
/// `dash_sdk_derive_identity_keys_from_mnemonic_free` (same
/// memory layout, intentionally shares the free function).
@_silgen_name("dash_sdk_derive_and_persist_identity_keys")
func dash_sdk_derive_and_persist_identity_keys(
    _ network: DashSDKNetwork,
    _ wallet_id_bytes: UnsafePointer<UInt8>?,
    _ identity_index: UInt32,
    _ key_count: UInt32,
    _ mnemonic_resolver_handle: UnsafeMutablePointer<MnemonicResolverHandle>?,
    _ persister_handle: UnsafeMutablePointer<IdentityKeyPersisterHandle>?,
    _ out_pubkeys: UnsafeMutablePointer<IdentityRegistrationKeyDerivationsFFI>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - Resolver-driven one-shot sign
//
// Sibling of the lower-level `dash_sdk_sign_with_mnemonic_and_path`
// in rs-sdk-ffi. Routes the mnemonic fetch through a Swift-owned
// `MnemonicResolverHandle` instead of taking it as a raw C-string,
// closing the same swift-sdk/CLAUDE.md "no mnemonic round-tripping"
// rule for the platform-address signing path.

/// Mirrors the Rust `SIGN_WITH_RESOLVER_*` byte tags. Returned via
/// the `out_error` byte parameter on a non-zero rc.
enum SignWithMnemonicResolverError: UInt8 {
    case ok = 0
    case nullPointer = 1
    case invalidUtf8 = 2
    case invalidMnemonic = 3
    case invalidPath = 4
    case derivationFailed = 5
    case signFailed = 6
    case bufferTooSmall = 7
    case unsupportedKeyType = 8
    case resolverNotFound = 9
    case resolverFailed = 10
}

@_silgen_name("dash_sdk_sign_with_mnemonic_resolver_and_path")
func dash_sdk_sign_with_mnemonic_resolver_and_path(
    _ mnemonic_resolver_handle: UnsafeMutablePointer<MnemonicResolverHandle>?,
    _ wallet_id_bytes: UnsafePointer<UInt8>?,
    _ derivation_path_cstr: UnsafePointer<CChar>?,
    _ data: UnsafePointer<UInt8>?,
    _ data_len: Int,
    _ key_type: UInt8,
    _ network: DashSDKNetwork,
    _ out_signature: UnsafeMutablePointer<UInt8>?,
    _ out_signature_capacity: Int,
    _ out_signature_len: UnsafeMutablePointer<Int>?,
    _ out_error: UnsafeMutablePointer<UInt8>?
) -> Int32
