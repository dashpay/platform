// Identity registration FFI function declarations
// Mirrors packages/rs-platform-wallet-ffi/src/identity_registration.rs.
//
// The rs-platform-wallet-ffi crate does not emit a public C header (see
// its `lib.rs`: everything is `#[no_mangle] extern "C"`), so the Swift
// side declares the symbols with @_silgen_name the same way the rest
// of the platform-wallet FFI surface is wired up.

import Foundation

// MARK: - FFI Structs

/// Mirrors `IdentityFundingInputFFI` from Rust.
///
/// Nonces are intentionally absent — the SDK fetches each address's
/// on-chain nonce at submit time, so callers only need to name the
/// address + the credit amount to spend.
///
/// - `address_type`: 0 = P2PKH, 1 = P2SH.
/// - `hash`: 20-byte address hash (BE tuple of `UInt8` x 20).
/// - `credits`: credits to spend from this address.
@frozen
public struct IdentityFundingInputFFI {
    public var address_type: UInt8
    public var hash: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
    public var credits: UInt64
}

/// Mirrors `IdentityFundingOutputFFI` from Rust. When `has_output` is
/// false the remaining fields are ignored.
@frozen
public struct IdentityFundingOutputFFI {
    public var has_output: Bool
    public var address_type: UInt8
    public var hash: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )
    public var credits: UInt64
}

/// Mirrors `IdentityPubkeyFFI` from Rust
/// (`packages/rs-platform-wallet-ffi/src/identity_registration_with_signer.rs`).
///
/// One already-derived authentication public key the caller wants the
/// new identity to register. Used by
/// `platform_wallet_register_identity_with_signer` in place of the
/// previous `key_count: UInt32` argument so that pubkey derivation can
/// happen entirely in Swift (via
/// `dash_sdk_derive_identity_keys_from_mnemonic`, which works for
/// watch-only wallets) rather than via the wallet handle on the Rust
/// side.
///
/// Field discriminants match the DPP `repr(u8)` enum layouts:
/// - `key_type`: KeyType discriminant (0 = ECDSA_SECP256K1, etc.)
/// - `purpose`: Purpose discriminant (0 = AUTHENTICATION, etc.)
/// - `security_level`: SecurityLevel discriminant (0 = MASTER, 1 =
///   CRITICAL, 2 = HIGH, 3 = MEDIUM)
///
/// `pubkey_bytes` is borrowed by the FFI for the duration of the call;
/// the caller retains ownership of the buffer.
@frozen
public struct IdentityPubkeyFFI {
    public var key_id: UInt32
    public var key_type: UInt8
    public var purpose: UInt8
    public var security_level: UInt8
    public var pubkey_bytes: UnsafePointer<UInt8>?
    public var pubkey_len: Int
    public var read_only: Bool
    /// Contract-bounds discriminant. `0` = no bounds (Authentication
    /// / Transfer / etc.), `1` = `SingleContract` (Encryption /
    /// Decryption restricted to a contract), `2` =
    /// `SingleContractDocumentType` (further narrowed to a
    /// document type within the contract).
    public var contract_bounds_kind: UInt8
    /// 32-byte contract id when `contract_bounds_kind != 0`. Null
    /// otherwise. Borrowed for the duration of the FFI call.
    public var contract_bounds_id: UnsafePointer<UInt8>?
    /// NUL-terminated UTF-8 document type name when
    /// `contract_bounds_kind == 2`. Null otherwise. Borrowed for
    /// the duration of the FFI call.
    public var contract_bounds_document_type: UnsafePointer<CChar>?
}

// MARK: - FFI Functions

// NOTE: The legacy mnemonic-driven `platform_wallet_register_identity_from_addresses`
// FFI has been deleted; the Rust crate no longer exports it. The
// signer-driven entry point below is the only supported path.

/// Mirrors `platform_wallet_register_identity_with_signer` from Rust.
///
/// Drops the BIP-39 mnemonic + passphrase parameters in favor of TWO
/// opaque `*mut SignerHandle`s — one for the new identity's
/// state-transition keys, one for each input platform address's
/// funding-contribution signature. The new identity's authentication
/// pubkeys are passed in by the caller via `identity_pubkeys` (an
/// array of `IdentityPubkeyFFI` rows) — Swift derives them via
/// `dash_sdk_derive_identity_keys_from_mnemonic` first, which works
/// for watch-only wallets where Rust has no in-process xpriv loaded.
/// Every signature crosses the FFI through the appropriate signer.
///
/// The two-handle split keeps the FFI explicit about both signing
/// roles (callers pass the same `KeychainSigner.handle` for both in
/// the common iOS case; future watch-only / hardware paths can wire
/// distinct signers per role without another ABI change). The
/// underlying `VTableSigner` implements `Signer<IdentityPublicKey>`
/// AND `Signer<PlatformAddress>` and dispatches by `key_type` byte
/// (KeyType discriminants 0–4 → identity-key lookup; `0xFF` →
/// platform-address-hash lookup; see `KeychainSigner.swift`).
@_silgen_name("platform_wallet_register_identity_with_signer")
func platform_wallet_register_identity_with_signer(
    _ wallet_handle: Handle,
    _ identity_index: UInt32,
    // Caller-derived authentication pubkeys for the new identity.
    // Each row carries `(key_id, key_type, purpose, security_level,
    // pubkey_bytes, pubkey_len, read_only)`. The buffer is borrowed —
    // Rust copies what it needs before the call returns.
    _ identity_pubkeys: UnsafePointer<IdentityPubkeyFFI>?,
    _ identity_pubkeys_count: Int,
    // Raw `*mut SignerHandle` produced by `dash_sdk_signer_create_with_ctx`
    // (e.g. via `KeychainSigner.handle`). Used as `Signer<IdentityPublicKey>`.
    // Caller retains ownership; this function does NOT destroy it.
    _ signer_identity_handle: OpaquePointer?,
    // Raw `*mut SignerHandle`. Used as `Signer<PlatformAddress>` —
    // the trampoline receives `key_type == 0xFF` and a 20-byte
    // address hash. Typically the same value as
    // `signer_identity_handle`; pass distinct handles for watch-only
    // wallets that route the two roles to different stores.
    _ signer_address_handle: OpaquePointer?,
    _ inputs: UnsafePointer<IdentityFundingInputFFI>?,
    _ inputs_count: Int,
    _ output: UnsafePointer<IdentityFundingOutputFFI>?,
    _ out_identity_id: UnsafeMutablePointer<(
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )>,
    _ out_identity_handle: UnsafeMutablePointer<Handle>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

/// Mirrors `platform_wallet_top_up_from_addresses_with_signer` from
/// Rust (`packages/rs-platform-wallet-ffi/src/identity_top_up.rs`).
///
/// Top up an existing identity's credit balance from one or more
/// Platform addresses. Reuses the `IdentityFundingInputFFI` row shape
/// the registration FFI exposes — each row names one input address +
/// the credit amount to spend from it.
///
/// Top-up state-transitions are signed with the Platform address
/// inputs' private keys (not the identity's authentication keys), so
/// only one signer handle is required: `signer_address_handle`,
/// dispatching `key_type == 0xFF` through the Swift `KeychainSigner`
/// trampoline.
///
/// On success `out_new_balance` is populated with the post-transition
/// credit balance returned by Platform. The Rust-side
/// `ManagedIdentity` for `identity_id` has its balance updated
/// inside the library call; subsequent reads through the same handle
/// reflect the new value without a round trip.
@_silgen_name("platform_wallet_top_up_from_addresses_with_signer")
func platform_wallet_top_up_from_addresses_with_signer(
    _ wallet_handle: Handle,
    _ identity_id: UnsafePointer<(
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )>,
    _ inputs: UnsafePointer<IdentityFundingInputFFI>?,
    _ inputs_count: Int,
    _ signer_address_handle: OpaquePointer?,
    _ out_new_balance: UnsafeMutablePointer<UInt64>,
    _ out_error: UnsafeMutablePointer<PlatformWalletFFIError>
) -> PlatformWalletFFIResult

// MARK: - Platform-address private-key pre-derivation — REMOVED
//
// The previous design pre-derived platform-address private keys in
// Rust, exported them across the FFI as 32-byte scalars, and had
// Swift persist them in the Keychain keyed by 20-byte address hash.
// That violated the rule that platform-address private keys are
// NEVER persisted — they are pure derivation outputs of
// `(mnemonic, path)` and exist only for the duration of a single
// signing call.
//
// The replacement is `dash_sdk_sign_with_mnemonic_and_path` in
// `KeychainSigner.swift`'s `key_type == 0xFF` branch: pull mnemonic
// from Keychain + derivation path from `PersistentPlatformAddress`,
// hand both to one Rust FFI call that derives, signs, and zeroes
// in-place. No `PlatformAddressPrivateKey(s)FFI`, no Keychain entry
// for the derived bytes, no exporting them across the FFI.
