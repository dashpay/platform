//! Address-funded identity registration driven by external `SignerHandle`s.
//!
//! The single entry point in this module — `platform_wallet_register_identity_with_signer`
//! — replaces the legacy mnemonic-driven FFI. Instead of taking a
//! BIP-39 mnemonic across the FFI, the caller supplies:
//!
//! - the wallet handle (used for network lookup, Platform-address
//!   state, and the SDK call — *not* for identity-key derivation),
//! - the already-derived identity authentication public keys
//!   ([`IdentityPubkeyFFI`] rows). Earlier revisions derived these
//!   on the Rust side via the wallet handle, but that path fails on
//!   watch-only wallets restored from Swift-side persisted state
//!   (the seed lives in iOS Keychain, not in the in-process
//!   `WalletManager`). Swift now derives the pubkeys via
//!   [`crate::dash_sdk_derive_identity_keys_from_mnemonic`] and
//!   threads them through this FFI, which works for every wallet
//!   shape regardless of how it was loaded into the process.
//! - **two** [`SignerHandle`]s (typically two views of the same
//!   Swift-side `KeychainSigner`):
//!     - `signer_identity_handle` — used as `Signer<IdentityPublicKey>`
//!       for the new identity's state-transition signatures.
//!     - `signer_address_handle` — used as `Signer<PlatformAddress>`
//!       for each input platform address's funding-contribution
//!       signature.
//!
//! The two-signer split is what unlocks **watch-only** wallets — the
//! wallet's own seed never needs to participate in signing on this
//! path. For wallets where the same backing store fulfils both roles
//! (the common iOS case) the caller passes the same
//! `KeychainSigner.handle` for both arguments and the trampoline
//! dispatches by `key_type` (KeyType discriminant 0–4 → identity-key
//! lookup; `0xFF` → platform-address-hash lookup).
//!
//! The Swift caller is responsible for:
//! 1. Calling [`crate::platform_wallet_preview_identity_registration_keys`]
//!    (or deriving via the existing `key_wallet_*` FFI) to obtain the
//!    `(pubkey, derivation_path)` pairs for the new identity's keys, AND
//! 2. Persisting those pairs to SwiftData / Keychain so the
//!    identity `KeychainSigner` can later look up the matching private
//!    keys.
//!
//! Platform-address private keys are NOT pre-persisted. The address
//! signer trampoline derives them on demand per signing call from
//! `(mnemonic-in-Keychain, derivation-path-in-SwiftData)` via
//! `dash_sdk_sign_with_mnemonic_and_path` and zeroes the buffer
//! immediately. See `KeychainSigner.swift`.
//!
//! After this call completes, every state-transition signature
//! (identity AND platform-address) crosses the FFI boundary via the
//! supplied `SignerHandle`s rather than via a Rust-derived seed.
//!
//! See `swift-sdk/CLAUDE.md` for the architectural reasoning behind
//! pushing the seed off the FFI boundary.

use dashcore::PrivateKey as DashPrivateKey;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use platform_wallet::derive_identity_auth_keypair;
use rs_sdk_ffi::{SignerHandle, VTableSigner};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::ptr;
use std::slice;
use zeroize::Zeroize;

use crate::error::*;
use crate::handle::*;
use crate::identity_key_preview::IdentityKeyPreviewFFI;
use crate::identity_registration::{IdentityFundingInputFFI, IdentityFundingOutputFFI};
use crate::runtime::block_on_worker;

/// One identity authentication public key the caller has already
/// derived (via [`crate::dash_sdk_derive_identity_keys_from_mnemonic`]
/// or equivalent) and now wants the placeholder identity to be built
/// from. Mirrored on the Swift side as `IdentityPubkeyFFI`.
///
/// Field discriminants match the DPP enum repr(u8) layout exactly:
/// - `key_type`: [`KeyType`] discriminant (0 = ECDSA_SECP256K1, etc.).
/// - `purpose`: [`Purpose`] discriminant (0 = AUTHENTICATION, etc.).
/// - `security_level`: [`SecurityLevel`] discriminant (0 = MASTER,
///   1 = CRITICAL, 2 = HIGH, 3 = MEDIUM).
///
/// `pubkey_bytes` is borrowed by the FFI for the duration of the call;
/// the caller retains ownership. Compressed secp256k1 pubkeys are
/// always 33 bytes (`pubkey_len == 33`); BLS would be 48; etc.
///
/// **Contract bounds** — Encryption / Decryption keys carry a
/// reference to the contract (and optionally a document type)
/// they're allowed to operate within. Encoded inline as:
///   - `contract_bounds_kind == 0` → no bounds.
///   - `contract_bounds_kind == 1` → `SingleContract`. The first
///     32 bytes at `contract_bounds_id` are the contract id; the
///     `contract_bounds_document_type` pointer is ignored.
///   - `contract_bounds_kind == 2` → `SingleContractDocumentType`.
///     `contract_bounds_id` is the 32-byte contract id;
///     `contract_bounds_document_type` is a NUL-terminated UTF-8
///     document type name. Both must be non-null.
///
/// All pointers are borrowed for the call duration only — the
/// FFI does not retain or free them.
#[repr(C)]
pub struct IdentityPubkeyFFI {
    pub key_id: u32,
    pub key_type: u8,
    pub purpose: u8,
    pub security_level: u8,
    pub pubkey_bytes: *const u8,
    pub pubkey_len: usize,
    pub read_only: bool,
    /// Discriminant for the contract-bounds union. See struct doc.
    pub contract_bounds_kind: u8,
    /// 32-byte contract id when `contract_bounds_kind != 0`.
    pub contract_bounds_id: *const u8,
    /// NUL-terminated UTF-8 document type name when
    /// `contract_bounds_kind == 2`. Null otherwise.
    pub contract_bounds_document_type: *const std::os::raw::c_char,
}

/// Decode the optional `contract_bounds_*` payload off an
/// [`IdentityPubkeyFFI`] row.
///
/// Shared by the registration and update FFI paths so
/// Encryption / Decryption keys can carry their bounds through
/// either entry point. `kind == 0` is "no bounds"; `1` is
/// `SingleContract { id }`; `2` is
/// `SingleContractDocumentType { id, document_type_name }`.
///
/// **Encryption / Decryption purposes require contract bounds.**
/// Drive scopes those purposes to a single contract (and optionally
/// a document type), so registering or updating an identity with an
/// unbounded encryption / decryption key produces a key that cannot
/// be used. We reject `kind == 0` for those purposes here so the
/// failure surfaces as a clean FFI error rather than a key Drive
/// silently can't use.
///
/// `purpose` is the parsed `Purpose` discriminant for the row, used
/// only for the encryption / decryption guard above.
///
/// `row_index` and `field_label` only flavour error messages
/// (different callers want different prefixes — `add_public_keys[i]`
/// for update, `identity_pubkeys[i]` for registration). On any
/// error, populates `out_error` (when non-null) and returns
/// `Err(result_code)` so the caller can early-return with the same
/// FFI status it was already returning.
///
/// # Safety
/// Each non-null pointer in the row must remain valid for the
/// duration of the call. `contract_bounds_id` (when not null) must
/// point at >=32 bytes; `contract_bounds_document_type` (when not
/// null) must be a NUL-terminated UTF-8 C string.
pub(crate) unsafe fn decode_contract_bounds(
    row: &IdentityPubkeyFFI,
    purpose: Purpose,
    row_index: usize,
    field_label: &str,
    out_error: *mut PlatformWalletFFIError,
) -> Result<Option<ContractBounds>, PlatformWalletFFIResult> {
    match row.contract_bounds_kind {
        0 => {
            if matches!(purpose, Purpose::ENCRYPTION | Purpose::DECRYPTION) {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidParameter,
                        format!(
                            "{}[{}].contract_bounds_kind = 0 (no bounds) but purpose = {:?} \
                             requires bounds — Drive scopes Encryption / Decryption keys to a \
                             specific contract (use kind 1 or 2)",
                            field_label, row_index, purpose
                        ),
                    );
                }
                return Err(PlatformWalletFFIResult::ErrorInvalidParameter);
            }
            Ok(None)
        }
        1 => {
            if row.contract_bounds_id.is_null() {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorNullPointer,
                        format!(
                            "{}[{}].contract_bounds_id is null but kind == 1 \
                             (SingleContract)",
                            field_label, row_index
                        ),
                    );
                }
                return Err(PlatformWalletFFIResult::ErrorNullPointer);
            }
            let id_bytes: [u8; 32] =
                match <[u8; 32]>::try_from(slice::from_raw_parts(row.contract_bounds_id, 32)) {
                    Ok(b) => b,
                    Err(_) => {
                        unreachable!("from_raw_parts(_, 32) always yields exactly 32 bytes")
                    }
                };
            Ok(Some(ContractBounds::SingleContract {
                id: Identifier::from(id_bytes),
            }))
        }
        2 => {
            if row.contract_bounds_id.is_null() || row.contract_bounds_document_type.is_null() {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorNullPointer,
                        format!(
                            "{}[{}].contract_bounds_id or .contract_bounds_document_type \
                             is null but kind == 2 (SingleContractDocumentType)",
                            field_label, row_index
                        ),
                    );
                }
                return Err(PlatformWalletFFIResult::ErrorNullPointer);
            }
            let id_bytes: [u8; 32] =
                match <[u8; 32]>::try_from(slice::from_raw_parts(row.contract_bounds_id, 32)) {
                    Ok(b) => b,
                    Err(_) => {
                        unreachable!("from_raw_parts(_, 32) always yields exactly 32 bytes")
                    }
                };
            let doc_type = match CStr::from_ptr(row.contract_bounds_document_type).to_str() {
                Ok(s) => s.to_string(),
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorUtf8Conversion,
                            format!(
                                "{}[{}].contract_bounds_document_type is not valid UTF-8: {}",
                                field_label, row_index, e
                            ),
                        );
                    }
                    return Err(PlatformWalletFFIResult::ErrorUtf8Conversion);
                }
            };
            Ok(Some(ContractBounds::SingleContractDocumentType {
                id: Identifier::from(id_bytes),
                document_type_name: doc_type,
            }))
        }
        other => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!(
                        "{}[{}].contract_bounds_kind = {} is not a valid discriminant \
                         (0=none, 1=SingleContract, 2=SingleContractDocumentType)",
                        field_label, row_index, other
                    ),
                );
            }
            Err(PlatformWalletFFIResult::ErrorInvalidParameter)
        }
    }
}

/// Register a new identity funded by Platform-address balances, using
/// **two** external [`SignerHandle`]s — one for the new identity's
/// state-transition keys, one for the input platform addresses.
///
/// Replaces the deleted mnemonic-driven `platform_wallet_register_identity_from_addresses`:
/// - No mnemonic / passphrase parameters.
/// - The new identity's authentication pubkeys are now passed in by
///   the caller as a [`IdentityPubkeyFFI`] array (previously this
///   function derived them via the wallet handle, which fails for
///   watch-only wallets where the seed lives in iOS Keychain rather
///   than the in-process `WalletManager`). Swift derives them via
///   [`crate::dash_sdk_derive_identity_keys_from_mnemonic`] and
///   threads them through here.
/// - Every state-transition signature crosses the FFI through one of
///   the supplied signer handles.
/// - `signer_identity_handle` and `signer_address_handle` are
///   `*mut SignerHandle`s produced by `dash_sdk_signer_create_with_ctx`
///   (typically two views of the same Swift `KeychainSigner`). The
///   caller retains ownership of both; this function does NOT
///   destroy them.
///
/// The two-signer split keeps the FFI explicit about both signing
/// roles even when most callers pass the same handle twice — it
/// unblocks watch-only wallets (where the identity signer is a
/// hardware HSM and the address signer reaches into the Keychain)
/// and Keychain-backed platform-address keys without another ABI
/// change later.
///
/// On success both `out_identity_id` (32 bytes) and
/// `out_identity_handle` are populated. The returned handle points at
/// a freshly-inserted `ManagedIdentity` in `MANAGED_IDENTITY_STORAGE`
/// wrapping the new identity together with `identity_index`.
///
/// # Safety
/// - All pointer parameters follow the same null / lifetime rules as
///   the mnemonic-based variant.
/// - `identity_pubkeys` must point at a valid `[IdentityPubkeyFFI;
///   identity_pubkeys_count]` array, and each row's `pubkey_bytes`
///   must be a valid `[u8; pubkey_len]` buffer for the duration of
///   the call. The caller retains ownership of every buffer.
/// - `signer_identity_handle` and `signer_address_handle` must each be
///   a valid, non-destroyed handle and must outlive this call. Passing
///   the same pointer for both is supported and expected — the
///   underlying `VTableSigner` impls `Signer<IdentityPublicKey>` AND
///   `Signer<PlatformAddress>` and dispatches by `key_type` byte.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_register_identity_with_signer(
    wallet_handle: Handle,
    identity_index: u32,
    identity_pubkeys: *const IdentityPubkeyFFI,
    identity_pubkeys_count: usize,
    signer_identity_handle: *mut SignerHandle,
    signer_address_handle: *mut SignerHandle,
    inputs: *const IdentityFundingInputFFI,
    inputs_count: usize,
    output: *const IdentityFundingOutputFFI,
    out_identity_id: *mut [u8; 32],
    out_identity_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let invariant_violation: Option<&'static str> = if inputs.is_null() {
        Some("`inputs` pointer is null")
    } else if inputs_count == 0 {
        Some("`inputs_count` is zero")
    } else if identity_pubkeys.is_null() {
        Some("`identity_pubkeys` pointer is null")
    } else if identity_pubkeys_count == 0 {
        Some("`identity_pubkeys_count` must be >= 1")
    } else if signer_identity_handle.is_null() {
        Some("`signer_identity_handle` pointer is null")
    } else if signer_address_handle.is_null() {
        Some("`signer_address_handle` pointer is null")
    } else if out_identity_id.is_null() {
        Some("`out_identity_id` pointer is null")
    } else if out_identity_handle.is_null() {
        Some("`out_identity_handle` pointer is null")
    } else {
        None
    };
    if let Some(detail) = invariant_violation {
        if !out_error.is_null() {
            *out_error =
                PlatformWalletFFIError::new(PlatformWalletFFIResult::ErrorNullPointer, detail);
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    // Parse the inputs and optional output exactly the same way the
    // mnemonic variant does. Keeping the FFI shape identical lets
    // Swift call this function by swapping the symbol name and
    // dropping the mnemonic + passphrase arguments.
    let entries = slice::from_raw_parts(inputs, inputs_count);
    let mut input_map: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    for entry in entries {
        let address = match entry.address_type {
            0 => PlatformAddress::P2pkh(entry.hash),
            1 => PlatformAddress::P2sh(entry.hash),
            _ => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidParameter,
                        "invalid address_type (expected 0 or 1)",
                    );
                }
                return PlatformWalletFFIResult::ErrorInvalidParameter;
            }
        };
        // Sum duplicate rows for the same address rather than
        // overwriting — see the matching comment in
        // `identity_top_up.rs`. Caller may legitimately split one
        // address across rows; `insert` alone would silently
        // under-fund the new identity by the prior contribution.
        input_map
            .entry(address)
            .and_modify(|existing| *existing = existing.saturating_add(entry.credits))
            .or_insert(entry.credits);
    }

    let output_map = if output.is_null() {
        None
    } else {
        let output_ref = &*output;
        if output_ref.has_output {
            let address = match output_ref.address_type {
                0 => PlatformAddress::P2pkh(output_ref.hash),
                1 => PlatformAddress::P2sh(output_ref.hash),
                _ => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorInvalidParameter,
                            "invalid output address_type (expected 0 or 1)",
                        );
                    }
                    return PlatformWalletFFIResult::ErrorInvalidParameter;
                }
            };
            Some((address, output_ref.credits))
        } else {
            None
        }
    };

    // Re-acquire each `VTableSigner` behind its handle as a borrowed
    // reference inside the future. Round-tripping the pointers through
    // `usize` gives the spawned future a `Send + 'static` capture (the
    // raw pointer is `!Send`, but `usize` is). The actual signer state
    // — `Inner::Callback { ctx, vtable }` — is `Send + Sync` (see the
    // unsafe impls in `rs-sdk-ffi/src/signer.rs`).
    //
    // The two pointers may legitimately alias when the caller is
    // sharing one `KeychainSigner` for both roles; the `Signer<K>`
    // trait is generic over `K`, so the same `VTableSigner` value is
    // viewed as `Signer<IdentityPublicKey>` *or* `Signer<PlatformAddress>`
    // at the call site below depending on which generic parameter
    // `register_from_addresses` instantiates.
    let signer_identity_addr = signer_identity_handle as usize;
    let signer_address_addr = signer_address_handle as usize;

    // Materialize the caller-supplied pubkey rows into a BTreeMap of
    // `IdentityPublicKey` once, *before* entering the wallet-storage
    // closure. This lookup is independent of the wallet handle (we no
    // longer derive from the seed here — Swift derived these via the
    // mnemonic-driven FFI which works for watch-only wallets too) and
    // a parse failure should not depend on whether the wallet handle
    // happens to be valid. Validation errors propagate out the same
    // way as before via `out_error`.
    let pubkey_rows: &[IdentityPubkeyFFI] =
        slice::from_raw_parts(identity_pubkeys, identity_pubkeys_count);
    let mut keys_map: BTreeMap<u32, IdentityPublicKey> = BTreeMap::new();
    for (i, row) in pubkey_rows.iter().enumerate() {
        let key_type = match KeyType::try_from(row.key_type) {
            Ok(kt) => kt,
            Err(_) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidParameter,
                        format!(
                            "identity_pubkeys[{}].key_type = {} is not a valid KeyType discriminant",
                            i, row.key_type
                        ),
                    );
                }
                return PlatformWalletFFIResult::ErrorInvalidParameter;
            }
        };
        let purpose = match Purpose::try_from(row.purpose) {
            Ok(p) => p,
            Err(_) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidParameter,
                        format!(
                            "identity_pubkeys[{}].purpose = {} is not a valid Purpose discriminant",
                            i, row.purpose
                        ),
                    );
                }
                return PlatformWalletFFIResult::ErrorInvalidParameter;
            }
        };
        let security_level = match SecurityLevel::try_from(row.security_level) {
            Ok(sl) => sl,
            Err(_) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidParameter,
                        format!(
                            "identity_pubkeys[{}].security_level = {} is not a valid SecurityLevel discriminant",
                            i, row.security_level
                        ),
                    );
                }
                return PlatformWalletFFIResult::ErrorInvalidParameter;
            }
        };
        if row.pubkey_bytes.is_null() || row.pubkey_len == 0 {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    format!("identity_pubkeys[{}].pubkey_bytes is null or empty", i),
                );
            }
            return PlatformWalletFFIResult::ErrorNullPointer;
        }
        let pubkey_bytes: Vec<u8> =
            slice::from_raw_parts(row.pubkey_bytes, row.pubkey_len).to_vec();
        // Decode the optional contract-bounds payload through the
        // shared helper. Earlier revisions silently dropped these
        // fields here, so Encryption / Decryption keys registered
        // via this entry point ended up unbounded on Platform —
        // matching the update path's parser closes the gap. The
        // helper also rejects unscoped Encryption / Decryption
        // keys (Drive requires a contract scope for those).
        let contract_bounds =
            match decode_contract_bounds(row, purpose, i, "identity_pubkeys", out_error) {
                Ok(b) => b,
                Err(code) => return code,
            };
        keys_map.insert(
            row.key_id,
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: row.key_id,
                purpose,
                security_level,
                contract_bounds,
                key_type,
                read_only: row.read_only,
                data: BinaryData::new(pubkey_bytes),
                disabled_at: None,
            }),
        );
    }

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity_wallet = wallet.identity().clone();

            // Pubkey derivation moved up above the wallet-storage
            // closure: the caller supplies the pubkeys directly via
            // `identity_pubkeys`, so we no longer need to consult the
            // wallet manager here. The wallet handle is still
            // required for `wallet.identity()` and for the SDK call
            // below — those uses are unchanged.
            let placeholder = Identity::V0(IdentityV0 {
                id: Identifier::default(),
                public_keys: keys_map,
                balance: 0,
                revision: 0,
            });

            // SAFETY: the caller guaranteed both signer handles are
            // valid and outlive this call. `signer_*_addr` are the
            // same pointers reinterpreted as `usize` so they can
            // travel into the `'static + Send` future below.
            let result = block_on_worker(async move {
                let identity_signer: &VTableSigner =
                    unsafe { &*(signer_identity_addr as *const VTableSigner) };
                let address_signer: &VTableSigner =
                    unsafe { &*(signer_address_addr as *const VTableSigner) };

                identity_wallet
                    .register_from_addresses(
                        &placeholder,
                        input_map,
                        output_map,
                        identity_index,
                        identity_signer,
                        address_signer,
                        None,
                    )
                    .await
            });

            match result {
                Ok(identity) => {
                    let id_bytes: [u8; 32] = identity.id().to_buffer();
                    *out_identity_id = id_bytes;
                    let managed = platform_wallet::ManagedIdentity::new(identity, identity_index);
                    let handle = MANAGED_IDENTITY_STORAGE.insert(managed);
                    *out_identity_handle = handle;
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("register_from_addresses failed: {}", e),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidHandle,
                    "Invalid platform-wallet handle",
                );
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

// ---------------------------------------------------------------------------
// Pre-registration key derivation
// ---------------------------------------------------------------------------

/// Heap-allocated array of [`IdentityKeyPreviewFFI`] rows handed back
/// by [`platform_wallet_derive_identity_keys_for_index`]. Same memory
/// layout (and same free function) as [`crate::identity_key_preview::IdentityKeyPreviewsFFI`]
/// so the existing release machinery can reclaim it.
///
/// We deliberately re-use `IdentityKeyPreviewFFI` rather than inventing
/// a new row type so the Swift side can drop the result through the
/// existing `previewIdentityRegistrationKeys`-style marshalling code
/// (just iterated over a different `(identity_index, key_index)` set).
#[repr(C)]
pub struct IdentityRegistrationKeyDerivationsFFI {
    pub items: *mut IdentityKeyPreviewFFI,
    pub count: usize,
}

impl IdentityRegistrationKeyDerivationsFFI {
    fn empty() -> Self {
        Self {
            items: ptr::null_mut(),
            count: 0,
        }
    }
}

/// Derive every authentication-key pair the upcoming
/// [`platform_wallet_register_identity_with_signer`] call will need
/// for `identity_index`, returning one row per key id in `0..key_count`.
///
/// Sister function to [`crate::platform_wallet_preview_identity_registration_keys`]:
/// the preview only walks the MASTER slot at key_id 0 across many
/// `identity_index` values; this function fixes the `identity_index`
/// and walks `key_count` consecutive `key_id`s. Used at registration
/// time so the Swift `KeychainSigner` can pre-stash every key the
/// signing pass will ask for.
///
/// On success the array is owned by Rust and must be released via
/// [`platform_wallet_derive_identity_keys_for_index_free`]. On error
/// the struct is left at its zero state.
///
/// # Superseded — prefer [`crate::dash_sdk_derive_identity_keys_from_mnemonic`]
///
/// This entry point fails with `"Cannot derive private keys from
/// watch-only wallet"` for wallets restored from Swift-side persisted
/// state (where the seed lives in iOS Keychain rather than in the
/// in-process `WalletManager`). Most call sites should prefer
/// [`crate::dash_sdk_derive_identity_keys_from_mnemonic`], which
/// takes the mnemonic directly and works on every wallet shape
/// regardless of how it was loaded into the process.
///
/// Kept around for any out-of-tree consumer that still binds to the
/// old symbol; new code should not call it.
///
/// # Safety
/// `wallet_handle` must come from the platform-wallet handle registry.
/// `out_rows` must be a valid, writable pointer. `out_error` may be
/// null.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_derive_identity_keys_for_index(
    wallet_handle: Handle,
    identity_index: u32,
    key_count: u32,
    out_rows: *mut IdentityRegistrationKeyDerivationsFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_rows.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "out_rows is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    *out_rows = IdentityRegistrationKeyDerivationsFFI::empty();
    if key_count == 0 {
        return PlatformWalletFFIResult::Success;
    }

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let wm = wallet.wallet_manager().blocking_read();
            let wallet_id = wallet.wallet_id();
            let key_wallet = match wm.get_wallet(&wallet_id) {
                Some(w) => w,
                None => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorInvalidHandle,
                            "Wallet not found in wallet manager",
                        );
                    }
                    return PlatformWalletFFIResult::ErrorInvalidHandle;
                }
            };
            let network = key_wallet.network;

            let mut rows: Vec<IdentityKeyPreviewFFI> = Vec::with_capacity(key_count as usize);

            // Hand-roll cleanup on failure: each successfully-pushed
            // row owns CString / Vec allocations that won't be freed
            // by `Vec::drop` (we declare them as raw pointers).
            let cleanup = |rows: Vec<IdentityKeyPreviewFFI>| {
                for row in rows {
                    if !row.derivation_path.is_null() {
                        let _ = CString::from_raw(row.derivation_path);
                    }
                    if !row.public_key.is_null() {
                        let _ = Vec::from_raw_parts(
                            row.public_key,
                            row.public_key_len,
                            row.public_key_len,
                        );
                    }
                    if !row.private_key_wif.is_null() {
                        let _ = CString::from_raw(row.private_key_wif);
                    }
                }
            };

            for key_id in 0..key_count {
                let (path, ext_priv, public_key) =
                    match derive_identity_auth_keypair(key_wallet, network, identity_index, key_id)
                    {
                        Ok(t) => t,
                        Err(e) => {
                            cleanup(rows);
                            if !out_error.is_null() {
                                *out_error = PlatformWalletFFIError::new(
                                    PlatformWalletFFIResult::ErrorWalletOperation,
                                    format!(
                                        "derive_identity_keys_for_index: derivation failed at \
                                         (identity={}, key={}): {}",
                                        identity_index, key_id, e
                                    ),
                                );
                            }
                            return PlatformWalletFFIResult::ErrorWalletOperation;
                        }
                    };

                let path_cstring = match CString::new(path.to_string()) {
                    Ok(s) => s,
                    Err(e) => {
                        cleanup(rows);
                        if !out_error.is_null() {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorUtf8Conversion,
                                format!("derivation path contained NUL byte: {}", e),
                            );
                        }
                        return PlatformWalletFFIResult::ErrorUtf8Conversion;
                    }
                };

                let pub_bytes: [u8; 33] = public_key.serialize();
                let mut pub_box: Box<[u8]> = pub_bytes.to_vec().into_boxed_slice();
                let pub_ptr = pub_box.as_mut_ptr();
                let pub_len = pub_box.len();
                std::mem::forget(pub_box);

                let dash_private = DashPrivateKey {
                    compressed: true,
                    network,
                    inner: ext_priv.private_key,
                };
                let wif_cstring = match CString::new(dash_private.to_wif()) {
                    Ok(s) => s,
                    Err(e) => {
                        unsafe {
                            drop(Vec::from_raw_parts(pub_ptr, pub_len, pub_len));
                        }
                        drop(path_cstring);
                        cleanup(rows);
                        if !out_error.is_null() {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorUtf8Conversion,
                                format!("WIF string contained NUL byte: {}", e),
                            );
                        }
                        return PlatformWalletFFIResult::ErrorUtf8Conversion;
                    }
                };

                rows.push(IdentityKeyPreviewFFI {
                    identity_index,
                    derivation_path: path_cstring.into_raw(),
                    public_key: pub_ptr,
                    public_key_len: pub_len,
                    private_key_wif: wif_cstring.into_raw(),
                    private_key_bytes: ext_priv.private_key.secret_bytes(),
                });
            }

            let mut boxed = rows.into_boxed_slice();
            let items_ptr = boxed.as_mut_ptr();
            let items_count = boxed.len();
            std::mem::forget(boxed);

            *out_rows = IdentityRegistrationKeyDerivationsFFI {
                items: items_ptr,
                count: items_count,
            };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidHandle,
                    "Invalid platform-wallet handle",
                );
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Release a [`IdentityRegistrationKeyDerivationsFFI`] previously
/// populated by [`platform_wallet_derive_identity_keys_for_index`].
///
/// Safe to call on a zero / null struct or null outer pointer (no-op).
/// Each row's owned strings (`derivation_path`, `private_key_wif`)
/// and pubkey buffer are reclaimed.
///
/// # Safety
/// `rows.items` must have been handed out by
/// [`platform_wallet_derive_identity_keys_for_index`] and must not be
/// freed twice.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_derive_identity_keys_for_index_free(
    rows: *mut IdentityRegistrationKeyDerivationsFFI,
) {
    if rows.is_null() {
        return;
    }
    let owned = std::mem::replace(&mut *rows, IdentityRegistrationKeyDerivationsFFI::empty());
    if owned.items.is_null() || owned.count == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts_mut(owned.items, owned.count);
    for row in slice.iter_mut() {
        if !row.derivation_path.is_null() {
            let _ = CString::from_raw(row.derivation_path);
        }
        if !row.public_key.is_null() {
            let _ = Vec::from_raw_parts(row.public_key, row.public_key_len, row.public_key_len);
        }
        if !row.private_key_wif.is_null() {
            // The WIF string encodes the same 32-byte secret as
            // `private_key_bytes`; scrub the buffer in place before
            // dropping so the heap allocation isn't released with
            // recoverable key material.
            let mut wif = CString::from_raw(row.private_key_wif).into_bytes_with_nul();
            wif.zeroize();
            row.private_key_wif = ptr::null_mut();
        }
        // Final inline secret scalar — wipe before the row slab is
        // returned to the allocator.
        row.private_key_bytes.zeroize();
    }
    // Reclaim the row array itself.
    let _ = Box::from_raw(slice as *mut [IdentityKeyPreviewFFI]);
}

// ---------------------------------------------------------------------------
// Pre-registration platform-address private-key derivation — DELETED
// ---------------------------------------------------------------------------
//
// The previous design pre-derived platform-address private keys in Rust,
// shipped them across the FFI as 32-byte scalars, and had Swift persist
// them in the Keychain keyed by 20-byte address hash. That violated the
// "platform-address private keys are NEVER persisted" rule: those keys
// are pure derivation outputs of `(mnemonic, path)` and exist only for
// the duration of one signing call.
//
// The replacement path lives in `rs-sdk-ffi` as
// `dash_sdk_sign_with_mnemonic_and_path`: the Swift `KeychainSigner`
// trampoline pulls the mnemonic from Keychain on the 0xFF branch,
// looks up the derivation path on the matching `PersistentPlatformAddress`
// SwiftData row, and calls that one-shot FFI to produce a signature.
// The derived key never leaves Rust, never crosses the FFI as bytes,
// and never lands in the Keychain.
