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

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::identity_key_preview::IdentityKeyPreviewFFI;
use crate::identity_keys_from_mnemonic::zeroize_and_free_row;
use crate::identity_registration::{IdentityFundingInputFFI, IdentityFundingOutputFFI};
use crate::runtime::block_on_worker;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

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
/// only for the encryption / decryption guard above. `row_index` and
/// `field_label` only flavour error messages (different callers want
/// different prefixes — `add_public_keys[i]` for update,
/// `identity_pubkeys[i]` for registration).
///
/// Returns `Err(PlatformWalletFFIResult)` carrying the FFI error the
/// caller should bubble up (the result already holds the message);
/// caller does `unwrap_result_or_return!(decode_contract_bounds(...))`.
pub(crate) unsafe fn decode_contract_bounds(
    row: &IdentityPubkeyFFI,
    purpose: Purpose,
    row_index: usize,
    field_label: &str,
) -> Result<Option<ContractBounds>, PlatformWalletFFIResult> {
    match row.contract_bounds_kind {
        0 => {
            if matches!(purpose, Purpose::ENCRYPTION | Purpose::DECRYPTION) {
                return Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorInvalidParameter,
                    format!(
                        "{field_label}[{row_index}].contract_bounds_kind = 0 (no bounds) but \
                         purpose = {purpose:?} requires bounds — Drive scopes Encryption / \
                         Decryption keys to a specific contract (use kind 1 or 2)"
                    ),
                ));
            }
            Ok(None)
        }
        1 => {
            if row.contract_bounds_id.is_null() {
                return Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorNullPointer,
                    format!(
                        "{field_label}[{row_index}].contract_bounds_id is null but kind == 1 \
                         (SingleContract)"
                    ),
                ));
            }
            let id_bytes: [u8; 32] =
                match <[u8; 32]>::try_from(slice::from_raw_parts(row.contract_bounds_id, 32)) {
                    Ok(b) => b,
                    Err(_) => unreachable!("from_raw_parts(_, 32) always yields exactly 32 bytes"),
                };
            Ok(Some(ContractBounds::SingleContract {
                id: Identifier::from(id_bytes),
            }))
        }
        2 => {
            if row.contract_bounds_id.is_null() || row.contract_bounds_document_type.is_null() {
                return Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorNullPointer,
                    format!(
                        "{field_label}[{row_index}].contract_bounds_id or \
                         .contract_bounds_document_type is null but kind == 2 \
                         (SingleContractDocumentType)"
                    ),
                ));
            }
            let id_bytes: [u8; 32] =
                match <[u8; 32]>::try_from(slice::from_raw_parts(row.contract_bounds_id, 32)) {
                    Ok(b) => b,
                    Err(_) => unreachable!("from_raw_parts(_, 32) always yields exactly 32 bytes"),
                };
            let doc_type = match CStr::from_ptr(row.contract_bounds_document_type).to_str() {
                Ok(s) => s.to_string(),
                Err(e) => {
                    return Err(PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                        format!(
                            "{field_label}[{row_index}].contract_bounds_document_type is not \
                             valid UTF-8: {e}"
                        ),
                    ));
                }
            };
            Ok(Some(ContractBounds::SingleContractDocumentType {
                id: Identifier::from(id_bytes),
                document_type_name: doc_type,
            }))
        }
        other => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!(
                "{field_label}[{row_index}].contract_bounds_kind = {other} is not a valid \
                 discriminant (0=none, 1=SingleContract, 2=SingleContractDocumentType)"
            ),
        )),
    }
}

/// Decode a C-side slice of [`IdentityPubkeyFFI`] rows into the
/// `BTreeMap<u32, IdentityPublicKey>` shape every identity-creation
/// path consumes.
///
/// Shared between the address-funded registration entry point
/// ([`platform_wallet_register_identity_from_addresses_signer`] in
/// this file) and the asset-lock-funded entry points
/// ([`platform_wallet_register_identity_with_funding_signer`] /
/// [`platform_wallet_resume_identity_with_existing_asset_lock_signer`]
/// in `identity_registration_funded_with_signer.rs`). The two paths
/// previously each owned their own decoder; one diverged in March
/// 2026 by dropping `contract_bounds` from Swift pubkey rows
/// (silently registering keys with the wrong semantics, see PR
/// review thread `r3247674469`). Centralising the decoder closes
/// that drift surface — a future field on `IdentityPubkeyFFI`
/// can't land in only one path.
///
/// Per-row validation:
/// - `key_type` / `purpose` / `security_level` round-trip through
///   `TryFrom` so an out-of-range byte from Swift surfaces as
///   `ErrorInvalidParameter` instead of silently coercing.
/// - `pubkey_bytes` must be non-null and non-empty.
/// - `contract_bounds` decoded via [`decode_contract_bounds`], which
///   enforces that Encryption / Decryption keys carry bounds
///   (Drive rejects unbounded ones).
///
/// Returns `Err(PlatformWalletFFIResult)` carrying the FFI error the
/// caller should bubble up directly via
/// [`crate::unwrap_result_or_return`].
///
/// # Safety
/// - `identity_pubkeys` must point to `identity_pubkeys_count`
///   contiguous `IdentityPubkeyFFI` rows that outlive this call.
/// - Each row's `pubkey_bytes` / `contract_bounds_id` /
///   `contract_bounds_document_type` pointer must satisfy the
///   contract documented on [`IdentityPubkeyFFI`].
pub(crate) unsafe fn decode_identity_pubkeys(
    identity_pubkeys: *const IdentityPubkeyFFI,
    identity_pubkeys_count: usize,
) -> Result<BTreeMap<u32, IdentityPublicKey>, PlatformWalletFFIResult> {
    let pubkey_rows: &[IdentityPubkeyFFI] =
        slice::from_raw_parts(identity_pubkeys, identity_pubkeys_count);
    let mut keys_map: BTreeMap<u32, IdentityPublicKey> = BTreeMap::new();
    for (i, row) in pubkey_rows.iter().enumerate() {
        let key_type = KeyType::try_from(row.key_type).map_err(|e| {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("identity_pubkeys[{i}].key_type invalid: {e}"),
            )
        })?;
        let purpose = Purpose::try_from(row.purpose).map_err(|e| {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("identity_pubkeys[{i}].purpose invalid: {e}"),
            )
        })?;
        let security_level = SecurityLevel::try_from(row.security_level).map_err(|e| {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("identity_pubkeys[{i}].security_level invalid: {e}"),
            )
        })?;
        if row.pubkey_bytes.is_null() || row.pubkey_len == 0 {
            return Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorNullPointer,
                format!("identity_pubkeys[{i}].pubkey_bytes is null or empty"),
            ));
        }
        let pubkey_bytes: Vec<u8> =
            slice::from_raw_parts(row.pubkey_bytes, row.pubkey_len).to_vec();
        let contract_bounds = decode_contract_bounds(row, purpose, i, "identity_pubkeys")?;
        // Reject duplicate key IDs explicitly. A plain `insert` into the
        // `BTreeMap` would silently last-wins, dropping the earlier row — the
        // surviving key is still structurally validated, but the caller asked
        // to register two keys and only got one, which is a caller bug worth
        // surfacing rather than hiding. All callers (the four Android
        // registration paths and the invitation-claim path) build a distinct
        // key set, so a collision is always a mistake.
        if keys_map.contains_key(&row.key_id) {
            return Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!(
                    "identity_pubkeys[{i}] has a duplicate key id {}",
                    row.key_id
                ),
            ));
        }
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
    Ok(keys_map)
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
/// change later. Passing the same pointer for both is supported and
/// expected — the underlying `VTableSigner` impls
/// `Signer<IdentityPublicKey>` AND `Signer<PlatformAddress>` and
/// dispatches by `key_type` byte.
///
/// On success both `out_identity_id` (32 bytes) and
/// `out_identity_handle` are populated. The returned handle points at
/// a freshly-inserted `ManagedIdentity` in `MANAGED_IDENTITY_STORAGE`
/// wrapping the new identity together with `identity_index`.
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
) -> PlatformWalletFFIResult {
    check_ptr!(inputs);
    check_ptr!(identity_pubkeys);
    check_ptr!(signer_identity_handle);
    check_ptr!(signer_address_handle);
    check_ptr!(out_identity_id);
    check_ptr!(out_identity_handle);
    if inputs_count == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "`inputs_count` is zero",
        );
    }
    if identity_pubkeys_count == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "`identity_pubkeys_count` must be >= 1",
        );
    }

    let entries = slice::from_raw_parts(inputs, inputs_count);
    let mut input_map: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    for entry in entries {
        let address = match entry.address_type {
            0 => PlatformAddress::P2pkh(entry.hash),
            1 => PlatformAddress::P2sh(entry.hash),
            _ => {
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorInvalidParameter,
                    "invalid address_type (expected 0 or 1)",
                );
            }
        };
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
                    return PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorInvalidParameter,
                        "invalid output address_type (expected 0 or 1)",
                    );
                }
            };
            Some((address, output_ref.credits))
        } else {
            None
        }
    };

    let signer_identity_addr = signer_identity_handle as usize;
    let signer_address_addr = signer_address_handle as usize;

    let keys_map = unwrap_result_or_return!(decode_identity_pubkeys(
        identity_pubkeys,
        identity_pubkeys_count,
    ));

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wallet = wallet.clone();

        let placeholder = Identity::V0(IdentityV0 {
            id: Identifier::default(),
            public_keys: keys_map,
            balance: 0,
            revision: 0,
        });

        block_on_worker(async move {
            let identity_signer: &VTableSigner =
                unsafe { &*(signer_identity_addr as *const VTableSigner) };
            let address_signer: &VTableSigner =
                unsafe { &*(signer_address_addr as *const VTableSigner) };

            // The composite registers the identity AND reconciles the
            // spent funding addresses' platform-address balances from
            // the proof.
            wallet
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
        })
    });
    let result = unwrap_option_or_return!(option);
    let identity = unwrap_result_or_return!(result);
    let id_bytes: [u8; 32] = identity.id().to_buffer();
    *out_identity_id = id_bytes;
    let managed = platform_wallet::ManagedIdentity::new(identity, identity_index);
    let handle = MANAGED_IDENTITY_STORAGE.insert(managed);
    *out_identity_handle = handle;
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Pre-registration key derivation
// ---------------------------------------------------------------------------

/// Heap-allocated array of [`IdentityKeyPreviewFFI`] rows handed back
/// by [`platform_wallet_derive_identity_keys_for_index`]. Same memory
/// layout (and same free function) as
/// [`crate::identity_key_preview::IdentityKeyPreviewsFFI`] so the
/// existing release machinery can reclaim it.
///
/// We deliberately re-use `IdentityKeyPreviewFFI` rather than
/// inventing a new row type so the Swift side can drop the result
/// through the existing `previewIdentityRegistrationKeys`-style
/// marshalling code (just iterated over a different
/// `(identity_index, key_index)` set).
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
/// Sister function to
/// [`crate::platform_wallet_preview_identity_registration_keys`]: the
/// preview only walks the MASTER slot at key_id 0 across many
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
/// regardless of how it was loaded into the process. Kept around for
/// any out-of-tree consumer that still binds to the old symbol; new
/// code should not call it.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_derive_identity_keys_for_index(
    wallet_handle: Handle,
    identity_index: u32,
    key_count: u32,
    out_rows: *mut IdentityRegistrationKeyDerivationsFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_rows);
    *out_rows = IdentityRegistrationKeyDerivationsFFI::empty();
    if key_count == 0 {
        return PlatformWalletFFIResult::ok();
    }

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wm = wallet.wallet_manager().blocking_read();
        let wallet_id = wallet.wallet_id();
        let key_wallet = match wm.get_wallet(&wallet_id) {
            Some(w) => w,
            None => {
                return Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorInvalidHandle,
                    "Wallet not found in wallet manager",
                ));
            }
        };
        let network = key_wallet.network;

        let mut rows: Vec<IdentityKeyPreviewFFI> = Vec::with_capacity(key_count as usize);

        let cleanup = |mut rows: Vec<IdentityKeyPreviewFFI>| {
            for row in &mut rows {
                // Route through the shared helper so this error-path
                // cleanup scrubs the WIF backing bytes + raw scalar
                // exactly like the public
                // `platform_wallet_derive_identity_keys_for_index_free`
                // entry point does — keys for rows 0..N already built
                // when a later derivation fails must not be left in
                // freed heap. See the public `_free` below.
                unsafe { zeroize_and_free_row(row) };
            }
        };

        for key_id in 0..key_count {
            let (path, ext_priv, public_key) =
                match derive_identity_auth_keypair(key_wallet, network, identity_index, key_id) {
                    Ok(t) => t,
                    Err(e) => {
                        cleanup(rows);
                        return Err(PlatformWalletFFIResult::err(
                            PlatformWalletFFIResultCode::ErrorWalletOperation,
                            format!(
                                "derive_identity_keys_for_index: derivation failed at \
                                 (identity={identity_index}, key={key_id}): {e}"
                            ),
                        ));
                    }
                };

            let path_cstring = match CString::new(path.to_string()) {
                Ok(s) => s,
                Err(e) => {
                    cleanup(rows);
                    return Err(PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                        format!("derivation path contained NUL byte: {e}"),
                    ));
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
                    return Err(PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                        format!("WIF string contained NUL byte: {e}"),
                    ));
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
        Ok(rows)
    });
    let inner = unwrap_option_or_return!(option);
    let rows = unwrap_result_or_return!(inner);

    let mut boxed = rows.into_boxed_slice();
    let items_ptr = boxed.as_mut_ptr();
    let items_count = boxed.len();
    std::mem::forget(boxed);

    *out_rows = IdentityRegistrationKeyDerivationsFFI {
        items: items_ptr,
        count: items_count,
    };
    PlatformWalletFFIResult::ok()
}

/// Release a [`IdentityRegistrationKeyDerivationsFFI`] previously
/// populated by [`platform_wallet_derive_identity_keys_for_index`].
///
/// Safe to call on a zero / null struct or null outer pointer (no-op).
/// Each row's owned strings (`derivation_path`, `private_key_wif`)
/// and pubkey buffer are reclaimed. `rows.items` must have been
/// handed out by [`platform_wallet_derive_identity_keys_for_index`]
/// and must not be freed twice.
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
        // Same shared helper the mid-loop cleanup closure uses, so the
        // success-path release and the error-path release can't drift.
        zeroize_and_free_row(row);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a heap-detached `IdentityKeyPreviewFFI` exactly the way
    /// the `platform_wallet_derive_identity_keys_for_index` build loop
    /// does (CString::into_raw for path + WIF, a leaked
    /// `Box<[u8]>` pubkey, a real secret scalar inline) so the
    /// cleanup / free paths are exercised on genuinely-owned
    /// allocations rather than borrowed stack data.
    fn make_owned_row(secret: [u8; 32]) -> IdentityKeyPreviewFFI {
        let path = CString::new("m/9'/1'/5'/0'/0'/0'/0'").unwrap();
        let wif = CString::new("cQ_fake_wif_for_test_only_not_a_real_key").unwrap();
        let mut pub_box: Box<[u8]> = vec![0x02u8; 33].into_boxed_slice();
        let pub_ptr = pub_box.as_mut_ptr();
        let pub_len = pub_box.len();
        std::mem::forget(pub_box);

        IdentityKeyPreviewFFI {
            identity_index: 3,
            derivation_path: path.into_raw(),
            public_key: pub_ptr,
            public_key_len: pub_len,
            private_key_wif: wif.into_raw(),
            private_key_bytes: secret,
        }
    }

    /// The mid-loop error-path `cleanup` closure now routes every
    /// partially-built row through `zeroize_and_free_row`. This
    /// asserts that path scrubs the inline 32-byte scalar in place and
    /// nulls every owned pointer, leaving each row safe to release a
    /// second time (double-free idempotency) — the regression the
    /// adversarial review caught (rows 0..N's WIF + scalar were
    /// previously freed without scrubbing when a later derivation
    /// failed).
    #[test]
    fn cleanup_path_zeroizes_secret_and_is_idempotent() {
        let secret = [0xABu8; 32];
        // Two rows, mirroring `key_count > 1` where a later index
        // fails after earlier rows were already built and pushed.
        let mut rows = vec![make_owned_row(secret), make_owned_row(secret)];

        for row in &rows {
            assert_eq!(row.private_key_bytes, secret);
            assert!(!row.derivation_path.is_null());
            assert!(!row.private_key_wif.is_null());
            assert!(!row.public_key.is_null());
        }

        // Exactly what the `cleanup` closure body does.
        for row in &mut rows {
            // SAFETY: rows own freshly-detached allocations and have
            // not crossed the FFI boundary, so this is the sole
            // release.
            unsafe { zeroize_and_free_row(row) };
        }

        for row in &rows {
            assert_eq!(
                row.private_key_bytes, [0u8; 32],
                "private_key_bytes must be zeroized by the cleanup path"
            );
            assert!(row.derivation_path.is_null());
            assert!(row.private_key_wif.is_null());
            assert!(row.public_key.is_null());
            assert_eq!(row.public_key_len, 0);
        }

        // Second release must not double-free or panic.
        for row in &mut rows {
            unsafe { zeroize_and_free_row(row) };
            assert_eq!(row.private_key_bytes, [0u8; 32]);
        }
    }

    /// The public `platform_wallet_derive_identity_keys_for_index_free`
    /// round-trip wipes secrets and resets the outer struct so a
    /// second free is a no-op. We build the rows directly (the public
    /// derive path needs a live wallet handle) and drive them through
    /// the real `_free` entry point — the same helper the cleanup
    /// path uses, so success-path and error-path releases can't drift.
    #[test]
    fn derive_keys_for_index_free_zeroizes_and_resets() {
        let secret = [0x5Au8; 32];
        let rows = vec![make_owned_row(secret), make_owned_row(secret)];
        let mut boxed = rows.into_boxed_slice();
        let items_ptr = boxed.as_mut_ptr();
        let items_count = boxed.len();
        std::mem::forget(boxed);

        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: items_ptr,
            count: items_count,
        };

        // SAFETY: `out.items` was detached above exactly as the
        // production derive path does; this is the sole free.
        unsafe { platform_wallet_derive_identity_keys_for_index_free(&mut out) };

        assert!(out.items.is_null());
        assert_eq!(out.count, 0);

        // Idempotent: a second free on the reset struct no-ops.
        unsafe { platform_wallet_derive_identity_keys_for_index_free(&mut out) };
        assert!(out.items.is_null());
        assert_eq!(out.count, 0);
    }

    /// Build an `IdentityPubkeyFFI` borrowing `pubkey` for the caller's
    /// lifetime. AUTHENTICATION / MASTER (discriminants 0/0/0) so no contract
    /// bounds are required — the raw byte values match the DPP `Purpose` /
    /// `SecurityLevel` reprs.
    fn ffi_row(key_id: u32, pubkey: &[u8]) -> IdentityPubkeyFFI {
        IdentityPubkeyFFI {
            key_id,
            key_type: 0,       // KeyType::ECDSA_SECP256K1
            purpose: 0,        // Purpose::AUTHENTICATION
            security_level: 0, // SecurityLevel::MASTER
            pubkey_bytes: pubkey.as_ptr(),
            pubkey_len: pubkey.len(),
            read_only: false,
            contract_bounds_kind: 0,
            contract_bounds_id: ptr::null(),
            contract_bounds_document_type: ptr::null(),
        }
    }

    /// Two rows sharing a key id must be rejected, not silently collapsed to
    /// last-wins in the `BTreeMap`. This is the exact decoder the invitation-
    /// claim path (`platform_wallet_claim_invitation`) runs, so the guard
    /// covers that fifth caller too — not only the four Android registration
    /// paths.
    #[test]
    fn decode_identity_pubkeys_rejects_duplicate_key_ids() {
        let pk_a = [0x02u8; 33];
        let pk_b = [0x03u8; 33];
        let rows = [ffi_row(0, &pk_a), ffi_row(0, &pk_b)];
        // SAFETY: `rows` (and the pubkey arrays it borrows) outlive the call.
        let mut err = unsafe { decode_identity_pubkeys(rows.as_ptr(), rows.len()) }
            .expect_err("duplicate key id must be rejected");
        assert_eq!(err.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
        unsafe { platform_wallet_ffi_result_free(&mut err) };
    }

    /// A distinct key set still decodes cleanly after the duplicate guard —
    /// proves the refactor didn't regress the common path.
    #[test]
    fn decode_identity_pubkeys_accepts_distinct_key_ids() {
        let pk_a = [0x02u8; 33];
        let pk_b = [0x03u8; 33];
        let rows = [ffi_row(0, &pk_a), ffi_row(1, &pk_b)];
        // SAFETY: `rows` (and the pubkey arrays it borrows) outlive the call.
        let map = unsafe { decode_identity_pubkeys(rows.as_ptr(), rows.len()) }
            .expect("distinct key ids must decode");
        assert_eq!(map.len(), 2);
    }

    /// An ENCRYPTION key with no contract bounds is still rejected after the
    /// refactor (Drive scopes those keys to a contract, so an unbounded one is
    /// unusable). Confirms `decode_contract_bounds`' guard survives.
    #[test]
    fn decode_identity_pubkeys_rejects_unbounded_encryption_key() {
        let pk = [0x02u8; 33];
        let mut enc = ffi_row(4, &pk);
        enc.purpose = 1; // Purpose::ENCRYPTION
        enc.security_level = 3; // SecurityLevel::MEDIUM
                                // contract_bounds_kind stays 0 → unbounded → rejected.
        let rows = [ffi_row(0, &pk), enc];
        // SAFETY: `rows` (and the pubkey array it borrows) outlive the call.
        let mut err = unsafe { decode_identity_pubkeys(rows.as_ptr(), rows.len()) }
            .expect_err("unbounded encryption key must be rejected");
        assert_eq!(err.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
        unsafe { platform_wallet_ffi_result_free(&mut err) };
    }
}
