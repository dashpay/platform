//! FFI for address-funded identity registration.
//!
//! Exposes `IdentityWallet::register_from_addresses` through a single C
//! entry point. Inputs arrive as a flat array of `(addressType, hash20,
//! nonce, credits)` tuples. The optional refund output is expressed as
//! a sibling triple plus an `has_output` flag.
//!
//! The caller supplies the BIP-39 mnemonic (typically from iOS
//! Keychain) plus an optional passphrase. This layer:
//!
//! 1. Derives a seed from the mnemonic.
//! 2. Derives every DIP-9 identity-authentication pubkey at
//!    `m/9'/coin'/5'/0'/ECDSA'/identity_index'/key_index'` for
//!    `key_index ∈ 0..key_count`.
//! 3. Builds a placeholder `Identity` carrying those pubkeys.
//! 4. Spins up `SeedBackedIdentitySigner` + `SeedBackedPlatformAddressSigner`
//!    against the same seed.
//! 5. Hands everything to the thin-wrapper
//!    `IdentityWallet::register_from_addresses`.
//!
//! The wallet struct stays key-material-free (WatchOnly /
//! ExternalSignable). The seed + intermediate xprivs live only for
//! the duration of this call — wrapped in `Zeroizing` where the
//! upstream types support it.
//!
//! Returns the newly-created identity through two out-params:
//! * `out_identity_id` — the 32-byte platform identifier.
//! * `out_identity_handle` — a handle into `MANAGED_IDENTITY_STORAGE`
//!   pointing at a `platform_wallet::ManagedIdentity` wrapping the
//!   Identity + its HD `identity_index`. The caller owns the handle
//!   and must free it via `managed_identity_destroy` when done.

use dash_sdk::platform::FetchMany;
use dashcore::secp256k1::Secp256k1;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::{AddressNonce, Identifier};
use drive_proof_verifier::types::AddressInfo;
use key_wallet::bip32::{
    ChildNumber, DerivationPath, ExtendedPrivKey, ExtendedPubKey, KeyDerivationType,
};
use key_wallet::dip9::{
    IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
};
use key_wallet::mnemonic::{Language, Mnemonic};
use key_wallet::Network;
use platform_wallet::wallet::signer::{SeedBackedIdentitySigner, SeedBackedPlatformAddressSigner};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;
use zeroize::Zeroizing;

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;

/// Flat input entry matching the SDK `put_with_address_funding`
/// shape. One row per contributing Platform Payment address.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IdentityInputAddressFFI {
    /// Address type discriminant (0 = P2PKH, 1 = P2SH). Matches the
    /// encoding used by `PlatformAddressFFI`.
    pub address_type: u8,
    /// 20-byte address hash.
    pub hash: [u8; 20],
    /// Current anti-replay nonce for the address.
    pub nonce: u32,
    /// Credits to spend from this address for the new identity.
    pub credits: u64,
}

/// Optional refund / change output paired with the input list. When
/// `has_output` is false the remaining fields are ignored.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IdentityOutputAddressFFI {
    pub has_output: bool,
    pub address_type: u8,
    pub hash: [u8; 20],
    pub credits: u64,
}

/// Register a new identity funded by Platform-address balances.
///
/// On success both `out_identity_id` (32 bytes) and
/// `out_identity_handle` are populated. The returned handle points at
/// a freshly-inserted `ManagedIdentity` in `MANAGED_IDENTITY_STORAGE`
/// wrapping the new identity together with `identity_index`. The
/// caller owns the handle and must release it via
/// `managed_identity_destroy`.
///
/// Note: the wallet's internal `IdentityManager` also receives a copy
/// of the same identity (via
/// `IdentityWallet::register_from_addresses`), so this handle is a
/// convenience for the caller to query right after creation — the
/// canonical source of truth remains the wallet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_register_identity_from_addresses(
    wallet_handle: Handle,
    identity_index: u32,
    key_count: u32,
    // BIP-39 mnemonic phrase (UTF-8, English). Used to derive
    // the DIP-9 auth keys + sign both the new identity and each
    // spent platform address. Not retained beyond this call.
    mnemonic: *const c_char,
    // Optional BIP-39 passphrase. Pass NULL for the empty
    // passphrase.
    passphrase: *const c_char,
    inputs: *const IdentityInputAddressFFI,
    inputs_count: usize,
    // Pointer rather than by-value because a 32-byte C struct
    // straddles the "passed by register vs. by stack" boundary
    // differently across toolchains. Swift + Rust agree on
    // pointer ABI, so we dodge that question entirely.
    output: *const IdentityOutputAddressFFI,
    out_identity_id: *mut [u8; 32],
    out_identity_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    // Distinct messages per pointer so the caller can tell which
    // invariant was violated. Swift currently surfaces `.nullPointer`
    // generically; the detail here makes the alert actionable.
    let invariant_violation: Option<&'static str> = if inputs.is_null() {
        Some("`inputs` pointer is null")
    } else if inputs_count == 0 {
        Some("`inputs_count` is zero")
    } else if mnemonic.is_null() {
        Some("`mnemonic` pointer is null")
    } else if out_identity_id.is_null() {
        Some("`out_identity_id` pointer is null")
    } else if out_identity_handle.is_null() {
        Some("`out_identity_handle` pointer is null")
    } else if key_count == 0 {
        Some("`key_count` must be >= 1")
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

    // Decode mnemonic + passphrase.
    let mnemonic_str = match CStr::from_ptr(mnemonic).to_str() {
        Ok(s) => s,
        Err(_) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorUtf8Conversion,
                    "mnemonic is not valid UTF-8",
                );
            }
            return PlatformWalletFFIResult::ErrorUtf8Conversion;
        }
    };
    let passphrase_str: &str = if passphrase.is_null() {
        ""
    } else {
        match CStr::from_ptr(passphrase).to_str() {
            Ok(s) => s,
            Err(_) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        "passphrase is not valid UTF-8",
                    );
                }
                return PlatformWalletFFIResult::ErrorUtf8Conversion;
            }
        }
    };
    let mnemonic_obj = match Mnemonic::from_phrase(mnemonic_str, Language::English) {
        Ok(m) => m,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!("invalid mnemonic: {}", e),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };
    let seed = Zeroizing::new(mnemonic_obj.to_seed(passphrase_str));

    // Decode the inputs array into the SDK's expected map shape.
    let entries = slice::from_raw_parts(inputs, inputs_count);
    let mut input_map: BTreeMap<PlatformAddress, (AddressNonce, Credits)> = BTreeMap::new();
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
        input_map.insert(address, (entry.nonce, entry.credits));
    }

    // `output` is allowed to be null — it means "no refund, any
    // residual credits stay with the new identity". Swift always
    // passes a real pointer, but we keep the null branch so the
    // ABI is forgiving to other callers.
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

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity_wallet = wallet.identity();
            let address_wallet = wallet.platform();
            let network: Network = wallet.sdk().network;

            // ---- Derive DIP-9 auth pubkeys from the seed ------------------
            // The seed lives in `Zeroizing` until end of this closure. The
            // intermediate xprivs are derived on the stack and dropped per
            // iteration; they don't impl Zeroize upstream (noted).
            let base_path: DerivationPath = match network {
                Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
                _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
            }
            .into();
            let key_type_index: u32 = KeyDerivationType::ECDSA.into();
            let secp = Secp256k1::new();

            let master = match ExtendedPrivKey::new_master(network, &*seed) {
                Ok(m) => m,
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("failed to derive master key from seed: {}", e),
                        );
                    }
                    return PlatformWalletFFIResult::ErrorWalletOperation;
                }
            };

            let mut keys_map: BTreeMap<u32, IdentityPublicKey> = BTreeMap::new();
            for key_index in 0..key_count {
                let full_path = base_path.extend([
                    ChildNumber::from_hardened_idx(key_type_index).unwrap_or_else(|_| {
                        ChildNumber::from_hardened_idx(0).expect("0' is always valid")
                    }),
                    match ChildNumber::from_hardened_idx(identity_index) {
                        Ok(cn) => cn,
                        Err(e) => {
                            if !out_error.is_null() {
                                *out_error = PlatformWalletFFIError::new(
                                    PlatformWalletFFIResult::ErrorInvalidParameter,
                                    format!("invalid identity_index: {}", e),
                                );
                            }
                            return PlatformWalletFFIResult::ErrorInvalidParameter;
                        }
                    },
                    match ChildNumber::from_hardened_idx(key_index) {
                        Ok(cn) => cn,
                        Err(e) => {
                            if !out_error.is_null() {
                                *out_error = PlatformWalletFFIError::new(
                                    PlatformWalletFFIResult::ErrorInvalidParameter,
                                    format!("invalid key_index: {}", e),
                                );
                            }
                            return PlatformWalletFFIResult::ErrorInvalidParameter;
                        }
                    },
                ]);

                let ext_priv = match master.derive_priv(&secp, &full_path) {
                    Ok(p) => p,
                    Err(e) => {
                        if !out_error.is_null() {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("failed to derive auth key at index {}: {}", key_index, e),
                            );
                        }
                        return PlatformWalletFFIResult::ErrorWalletOperation;
                    }
                };
                let ext_pub = ExtendedPubKey::from_priv(&secp, &ext_priv);
                let compressed_pubkey = ext_pub.public_key.serialize();

                let security_level = if key_index == 0 {
                    SecurityLevel::MASTER
                } else {
                    SecurityLevel::HIGH
                };

                keys_map.insert(
                    key_index,
                    IdentityPublicKey::V0(IdentityPublicKeyV0 {
                        id: key_index,
                        purpose: Purpose::AUTHENTICATION,
                        security_level,
                        contract_bounds: None,
                        key_type: KeyType::ECDSA_SECP256K1,
                        read_only: false,
                        data: BinaryData::new(compressed_pubkey.to_vec()),
                        disabled_at: None,
                    }),
                );
            }

            // ---- Build placeholder Identity + signers ---------------------
            let placeholder = Identity::V0(IdentityV0 {
                id: Identifier::default(),
                public_keys: keys_map,
                balance: 0,
                revision: 0,
            });

            let identity_signer = SeedBackedIdentitySigner::new(&*seed, network, identity_index);
            let address_signer =
                SeedBackedPlatformAddressSigner::new(&*seed, network, address_wallet);

            // ---- Refresh nonces from Platform -----------------------------
            // Ignore the caller-supplied nonces in `input_map` — they come
            // from SwiftData/BLAST and may be stale. Match the SDK's
            // `withdraw_address_funds` pattern: fetch the on-chain
            // "last-used" nonce via `AddressInfo::fetch_many`, then
            // increment by 1 to produce the "next-to-use" value Platform
            // expects. The caller-supplied nonce is preserved in the FFI
            // struct only as a forward-compat hint; nothing reads it.
            let sdk = wallet.sdk();
            let addresses: BTreeSet<PlatformAddress> = input_map.keys().copied().collect();
            let fresh_inputs_result: Result<
                BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
                String,
            > = runtime().block_on(async {
                let infos = AddressInfo::fetch_many(sdk, addresses.clone())
                    .await
                    .map_err(|e| format!("failed to fetch address nonces: {}", e))?;
                let mut fresh: BTreeMap<PlatformAddress, (AddressNonce, Credits)> = BTreeMap::new();
                for (addr, (_caller_nonce, credits)) in input_map.iter() {
                    let info = infos.get(addr).and_then(|maybe| maybe.as_ref());
                    // Platform returns `None` for addresses that have
                    // never been touched — nonce = 0, so first
                    // transition uses 1 (= 0 + 1). Treat missing as 0.
                    let current_nonce = info.map(|i| i.nonce).unwrap_or(0);
                    fresh.insert(*addr, (current_nonce + 1, *credits));
                }
                Ok(fresh)
            });
            let fresh_inputs = match fresh_inputs_result {
                Ok(f) => f,
                Err(msg) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            msg,
                        );
                    }
                    return PlatformWalletFFIResult::ErrorWalletOperation;
                }
            };

            // ---- Submit ---------------------------------------------------
            let result = runtime().block_on(identity_wallet.register_from_addresses(
                &placeholder,
                fresh_inputs,
                output_map,
                identity_index,
                &identity_signer,
                &address_signer,
                None,
            ));

            match result {
                Ok(identity) => {
                    let id_bytes: [u8; 32] = identity.id().to_buffer();
                    *out_identity_id = id_bytes;

                    // Wrap the new identity in a ManagedIdentity and
                    // hand the caller a handle into the global FFI
                    // storage. The wallet's IdentityManager keeps its
                    // own copy — this is a convenience reference for
                    // the caller that avoids a follow-up lookup.
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
