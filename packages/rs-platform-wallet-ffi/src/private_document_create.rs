//! FFI bindings for creating an entry of any private document type
//! (protocol version 15).
//!
//! - [`platform_wallet_manager_private_document_recipient_keys`]: the keys of
//!   an identity an entry may be sealed to, as the network proves the
//!   identity now (`private_document_recipient_key_choices`).
//! - [`platform_wallet_manager_private_document_create_fee`]: the fee a
//!   create of some entries of a type reserves from the notes it spends
//!   (`private_document_create_fee`).
//! - [`platform_wallet_manager_create_private_document_entry`]: seal one
//!   entry from one of the wallet's identities, inside the resolver-backed
//!   signer, and append it to the type's store, paying from a shielded
//!   account (`PlatformWallet::create_private_document_entry`). Built with the
//!   `shielded` feature only.
//!
//! Which keys, which sender key, the schema check, the fee and the outcome
//! are Rust's; these exports marshal.

use std::ffi::CStr;
use std::os::raw::c_char;

use platform_wallet::{
    private_document_create_fee, private_document_recipient_key_choices,
    PrivateDocumentEntryCreated, PrivateDocumentRecipientKey, PrivateDocumentSenderAuth,
};

use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{check_ptr, unwrap_result_or_return};

#[cfg(feature = "shielded")]
use platform_wallet::wallet::shielded::CachedOrchardProver;
#[cfg(feature = "shielded")]
use platform_wallet::NewPrivateDocumentEntry;
#[cfg(feature = "shielded")]
use rs_sdk_ffi::MnemonicResolverHandle;

#[cfg(feature = "shielded")]
use crate::dashpay::resolver_contact_crypto_provider;
#[cfg(feature = "shielded")]
use crate::identity_keys_from_mnemonic::resolve_seed_from_resolver;
#[cfg(feature = "shielded")]
use crate::private_contact_request::seed_mismatch_result;
#[cfg(feature = "shielded")]
use crate::shielded_send::{
    catch_spend_panic, map_spend_result, poke_sync_on_unconfirmed, resolve_wallet_and_coordinator,
};

/// A key of an identity an entry of a private document type may be sealed
/// to (`PrivateDocumentRecipientKey`).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PrivateDocumentRecipientKeyFFI {
    /// The key's id.
    pub key_id: u32,
    /// Its purpose, as dpp numbers it: 1 `ENCRYPTION`, 2 `DECRYPTION`.
    pub purpose: u8,
}

// Expected `PrivateDocumentRecipientKeyFFI` layout:
//
//   0..=3   key_id   u32
//   4       purpose  u8
//   5..=7   (tail padding to 4)
//
// Total size = 8, alignment = 4.
const _: [u8; 8] = [0u8; std::mem::size_of::<PrivateDocumentRecipientKeyFFI>()];
const _: [u8; 4] = [0u8; std::mem::align_of::<PrivateDocumentRecipientKeyFFI>()];
const _: () = assert!(std::mem::offset_of!(PrivateDocumentRecipientKeyFFI, purpose) == 4);

impl From<PrivateDocumentRecipientKey> for PrivateDocumentRecipientKeyFFI {
    fn from(key: PrivateDocumentRecipientKey) -> Self {
        Self {
            key_id: key.key_id,
            purpose: key.purpose as u8,
        }
    }
}

/// The keys [`platform_wallet_manager_private_document_recipient_keys`]
/// wrote: `count` keys at `keys`, or null and 0 for none. Release with
/// [`platform_wallet_private_document_recipient_keys_free`].
#[repr(C)]
#[derive(Debug)]
pub struct PrivateDocumentRecipientKeysFFI {
    pub keys: *mut PrivateDocumentRecipientKeyFFI,
    pub count: usize,
}

impl PrivateDocumentRecipientKeysFFI {
    /// No keys, owning nothing.
    pub fn empty() -> Self {
        Self {
            keys: std::ptr::null_mut(),
            count: 0,
        }
    }

    /// `keys` handed over as an exact-length boxed slice.
    fn from_keys(keys: Vec<PrivateDocumentRecipientKey>) -> Self {
        if keys.is_empty() {
            return Self::empty();
        }
        let keys: Box<[PrivateDocumentRecipientKeyFFI]> =
            keys.into_iter().map(Into::into).collect();
        let count = keys.len();
        Self {
            keys: Box::into_raw(keys) as *mut PrivateDocumentRecipientKeyFFI,
            count,
        }
    }
}

/// What [`platform_wallet_manager_create_private_document_entry`] created
/// (`PrivateDocumentEntryCreated`).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PrivateDocumentEntryCreatedFFI {
    /// The entry's commitment, which a read of the store reports it with
    /// (`PrivateDocumentEntryFFI::commitment`).
    pub commitment: [u8; 32],
    /// The sender's key the entry was sealed with: its outgoing viewing key
    /// recovers the entry, and a deniable MAC names it.
    pub sender_key_id: u32,
}

// Expected `PrivateDocumentEntryCreatedFFI` layout:
//
//   0..=31   commitment     [u8; 32]
//   32..=35  sender_key_id  u32
//
// Total size = 36, alignment = 4.
const _: [u8; 36] = [0u8; std::mem::size_of::<PrivateDocumentEntryCreatedFFI>()];
const _: [u8; 4] = [0u8; std::mem::align_of::<PrivateDocumentEntryCreatedFFI>()];
const _: () = assert!(std::mem::offset_of!(PrivateDocumentEntryCreatedFFI, sender_key_id) == 32);

impl From<PrivateDocumentEntryCreated> for PrivateDocumentEntryCreatedFFI {
    fn from(created: PrivateDocumentEntryCreated) -> Self {
        Self {
            commitment: created.commitment,
            sender_key_id: created.sender_key_id,
        }
    }
}

/// The sender authentication `auth_mode` names: 0 anonymous, 1 deniable MAC.
/// A signed entry (2, as an opened entry reports DashPay's) is not created.
pub(crate) fn sender_auth(
    auth_mode: u8,
) -> Result<PrivateDocumentSenderAuth, PlatformWalletFFIResult> {
    match auth_mode {
        0 => Ok(PrivateDocumentSenderAuth::Anonymous),
        1 => Ok(PrivateDocumentSenderAuth::DeniableMac),
        2 => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "a signed private document entry is not created: which keys may sign for a \
             private type is not decided",
        )),
        other => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("unknown private document auth mode {other} (expected 0 or 1)"),
        )),
    }
}

/// The SDK of the manager `handle`, or the invalid-handle result.
fn manager_sdk(handle: Handle) -> Result<std::sync::Arc<dash_sdk::Sdk>, PlatformWalletFFIResult> {
    PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| manager.sdk_arc())
        .ok_or_else(|| {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidHandle,
                format!("invalid manager handle: {handle}"),
            )
        })
}

/// What a host should know after a read here panicked: nothing was sent or
/// kept, so asking again is safe.
const READ_PANIC_GUIDANCE: &str = "Nothing was sent or kept; ask again.";

/// The keys of the identity `identity_id`, as the network proves it now,
/// that an entry of any private document type may be sealed to: its enabled
/// `ECDSA_SECP256K1` `DECRYPTION` and `ENCRYPTION` keys, by key id
/// (`private_document_recipient_key_choices`). Written to `*out_keys`; an
/// identity with no such key gives none.
///
/// An identity the network does not hold is `ErrorIdentityNotFound`; an
/// unknown `handle` is `ErrorInvalidHandle`. `*out_keys` is reset to the
/// empty value first; release it with
/// [`platform_wallet_private_document_recipient_keys_free`].
///
/// # Safety
/// - `identity_id` must point to 32 readable bytes.
/// - `out_keys` must be a valid, writable pointer that owns no earlier
///   result.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_private_document_recipient_keys(
    handle: Handle,
    identity_id: *const u8,
    out_keys: *mut PrivateDocumentRecipientKeysFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_keys);
    unsafe { *out_keys = PrivateDocumentRecipientKeysFFI::empty() };
    catch_panic_to_code(
        "private document recipient keys",
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        READ_PANIC_GUIDANCE,
        || {
            check_ptr!(identity_id);
            let identity_id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });
            let sdk = match manager_sdk(handle) {
                Ok(sdk) => sdk,
                Err(result) => return result,
            };
            let keys = unwrap_result_or_return!(block_on_worker(async move {
                private_document_recipient_key_choices(&sdk, identity_id).await
            }));
            unsafe { *out_keys = PrivateDocumentRecipientKeysFFI::from_keys(keys) };
            PlatformWalletFFIResult::ok()
        },
    )
}

/// Release what [`platform_wallet_manager_private_document_recipient_keys`]
/// wrote to `keys`, and reset it to the empty value, so a second call does
/// nothing. Null is ignored.
///
/// # Safety
/// `keys`, when non-null, must point to a `PrivateDocumentRecipientKeysFFI`
/// that is the empty value or was filled by
/// `platform_wallet_manager_private_document_recipient_keys` and not freed
/// since.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_private_document_recipient_keys_free(
    keys: *mut PrivateDocumentRecipientKeysFFI,
) {
    let Some(keys) = (unsafe { keys.as_mut() }) else {
        return;
    };
    if !keys.keys.is_null() && keys.count > 0 {
        // Mirror image of `from_keys`: an exact-length boxed slice.
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(keys.keys, keys.count)) });
    }
    *keys = PrivateDocumentRecipientKeysFFI::empty();
}

/// The fee, in credits, a create of `entry_count` entries of the private
/// type `document_type_name` of `data_contract_id` reserves from the notes
/// when it spends one or two of them (two Orchard actions), computed at the
/// manager's SDK protocol version, with the type's entry size from the
/// contract as the network proves it now (`private_document_create_fee`): the
/// least the spend pays. Written to `*out_fee`.
///
/// Errors: a type that is not private, or an entry count of 0 or above what
/// one transition appends (16 at protocol version 15), with
/// `ErrorInvalidParameter`; a network below protocol version 15 with
/// `ErrorProtocolVersionNotActive`; an unknown `handle` with
/// `ErrorInvalidHandle`.
///
/// # Safety
/// - `data_contract_id` must point to 32 readable bytes.
/// - `document_type_name` must be a valid NUL-terminated UTF-8 C string.
/// - `out_fee` must point to 8 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_private_document_create_fee(
    handle: Handle,
    data_contract_id: *const u8,
    document_type_name: *const c_char,
    entry_count: u32,
    out_fee: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_fee);
    unsafe { *out_fee = 0 };
    catch_panic_to_code(
        "private document create fee",
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        READ_PANIC_GUIDANCE,
        || {
            check_ptr!(data_contract_id);
            check_ptr!(document_type_name);
            let data_contract_id =
                unwrap_result_or_return!(unsafe { read_identifier(data_contract_id) });
            let document_type_name =
                unwrap_result_or_return!(unsafe { CStr::from_ptr(document_type_name) }.to_str())
                    .to_string();
            let sdk = match manager_sdk(handle) {
                Ok(sdk) => sdk,
                Err(result) => return result,
            };
            let fee = unwrap_result_or_return!(block_on_worker(async move {
                private_document_create_fee(
                    &sdk,
                    data_contract_id,
                    &document_type_name,
                    entry_count as usize,
                )
                .await
            }));
            unsafe { *out_fee = fee };
            PlatformWalletFFIResult::ok()
        },
    )
}

/// Create one entry of the private document type `document_type_name` of
/// `data_contract_id` from the wallet's identity `sender_identity_id` to the
/// key `recipient_key_id` of the identity `recipient_identity_id`, paying
/// for it from `account`'s shielded notes
/// (`PrivateDocumentCreateFromShieldedPool`, type 29): nothing on chain names
/// either identity (`PlatformWallet::create_private_document_entry`).
///
/// `properties_json` is the entry's properties, a JSON object keyed by
/// property name, as a document create takes them (byte arrays as hex or
/// base64 strings, identifiers as base58 or hex strings); Rust checks them
/// against the type's schema. `auth_mode` is 0 for an entry that names no
/// sender, 1 for one that names the sender with a deniable MAC; 2 (signed)
/// is refused. The entry is sealed inside the resolver-backed signer with the
/// sender's first enabled `ECDSA_SECP256K1` `ENCRYPTION` key whose identity
/// key slot derives it, so a read of the store
/// (`platform_wallet_read_private_document_entries`) recovers it as sent.
/// `mnemonic_resolver_handle` supplies both that key and the transient
/// Orchard spend authority. Writes the entry's commitment and sender key to
/// `*out_created`, reset to zeros first.
///
/// Errors: what is refused before anything is paid (a sender the wallet
/// does not hold or without such a key, a recipient key that is not an
/// enabled `DECRYPTION` or `ENCRYPTION` key, properties the schema refuses,
/// an unknown auth mode) with `ErrorInvalidParameter` or
/// `ErrorIdentityNotFound` and the reason; a resolver whose seed is not the
/// wallet's with `ErrorInvalidParameter`; a network below protocol version
/// 15 with `ErrorProtocolVersionNotActive`; the rest as the other note
/// spends: `ErrorShieldedSpendUnconfirmed` means the entry may already be in
/// the store, do not create it again.
///
/// # Safety
/// - `wallet_id_bytes`, `data_contract_id`, `sender_identity_id` and
///   `recipient_identity_id` must each point to 32 readable bytes.
/// - `mnemonic_resolver_handle` must be a valid, non-destroyed handle; the
///   caller retains ownership and pins it for the call.
/// - `document_type_name` and `properties_json` must be valid NUL-terminated
///   UTF-8 C strings.
/// - `out_created` must be a valid, writable pointer.
#[cfg(feature = "shielded")]
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_create_private_document_entry(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    account: u32,
    data_contract_id: *const u8,
    document_type_name: *const c_char,
    sender_identity_id: *const u8,
    recipient_identity_id: *const u8,
    recipient_key_id: u32,
    auth_mode: u8,
    properties_json: *const c_char,
    out_created: *mut PrivateDocumentEntryCreatedFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_created);
    unsafe { *out_created = PrivateDocumentEntryCreatedFFI::default() };
    // A panic in the proving task must not reach this `extern "C"` frame; the
    // notes may already be spent when it strikes, so the ambiguous
    // spend-unconfirmed contract applies (see `catch_spend_panic`).
    catch_spend_panic(CREATE_OPERATION, || unsafe {
        create_private_document_entry_inner(
            handle,
            wallet_id_bytes,
            mnemonic_resolver_handle,
            account,
            data_contract_id,
            document_type_name,
            sender_identity_id,
            recipient_identity_id,
            recipient_key_id,
            auth_mode,
            properties_json,
            out_created,
        )
    })
}

/// The operation name the create export reports.
#[cfg(feature = "shielded")]
const CREATE_OPERATION: &str = "private document create";

/// Body of [`platform_wallet_manager_create_private_document_entry`], as an
/// ordinary Rust function so a panic unwinds into [`catch_spend_panic`].
///
/// # Safety
/// Identical contract to the export that calls it.
#[cfg(feature = "shielded")]
#[allow(clippy::too_many_arguments)]
unsafe fn create_private_document_entry_inner(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    account: u32,
    data_contract_id: *const u8,
    document_type_name: *const c_char,
    sender_identity_id: *const u8,
    recipient_identity_id: *const u8,
    recipient_key_id: u32,
    auth_mode: u8,
    properties_json: *const c_char,
    out_created: *mut PrivateDocumentEntryCreatedFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(data_contract_id);
    check_ptr!(document_type_name);
    check_ptr!(properties_json);

    let mut wallet_id = [0u8; 32];
    unsafe { std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32) };
    let auth = match sender_auth(auth_mode) {
        Ok(auth) => auth,
        Err(result) => return result,
    };
    let new = NewPrivateDocumentEntry {
        data_contract_id: unwrap_result_or_return!(unsafe { read_identifier(data_contract_id) }),
        document_type_name: unwrap_result_or_return!(
            unsafe { CStr::from_ptr(document_type_name) }.to_str()
        )
        .to_string(),
        sender_identity_id: unwrap_result_or_return!(unsafe {
            read_identifier(sender_identity_id)
        }),
        recipient_identity_id: unwrap_result_or_return!(unsafe {
            read_identifier(recipient_identity_id)
        }),
        recipient_key_id,
        auth,
        properties_json: unwrap_result_or_return!(
            unsafe { CStr::from_ptr(properties_json) }.to_str()
        )
        .to_string(),
    };

    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(pair) => pair,
        Err(result) => return result,
    };
    let seed = match resolve_seed_from_resolver(mnemonic_resolver_handle, &wallet_id) {
        Ok(seed) => seed,
        Err(result) => return result,
    };
    // SAFETY: the caller pins the resolver handle for the duration of this
    // call.
    let crypto = unsafe {
        resolver_contact_crypto_provider(mnemonic_resolver_handle, wallet_id, wallet.network())
    };

    // The proof runs on a worker thread: Halo 2 synthesis recurses past the
    // iOS dispatch-thread stack.
    let result = block_on_worker(async move {
        let prover = CachedOrchardProver::new();
        let result = wallet
            .create_private_document_entry(
                &coordinator,
                seed.as_ref(),
                account,
                &new,
                &crypto,
                &prover,
            )
            .await;
        poke_sync_on_unconfirmed(&result, handle);
        result
    });
    match result {
        Ok(created) => {
            unsafe { *out_created = created.into() };
            PlatformWalletFFIResult::ok()
        }
        Err(error) => seed_mismatch_result(error)
            .unwrap_or_else(|error| map_spend_result(Err(error), CREATE_OPERATION)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::Purpose;

    #[test]
    fn should_marshal_the_sender_auth_modes_and_refuse_a_signed_one() {
        assert_eq!(
            sender_auth(0).ok(),
            Some(PrivateDocumentSenderAuth::Anonymous)
        );
        assert_eq!(
            sender_auth(1).ok(),
            Some(PrivateDocumentSenderAuth::DeniableMac)
        );
        for mode in [2, 3, 255] {
            let result = sender_auth(mode).expect_err("refused");
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            );
        }
    }

    #[test]
    fn should_marshal_and_free_the_recipient_keys() {
        let mut keys = PrivateDocumentRecipientKeysFFI::from_keys(vec![
            PrivateDocumentRecipientKey {
                key_id: 1,
                purpose: Purpose::ENCRYPTION,
            },
            PrivateDocumentRecipientKey {
                key_id: 2,
                purpose: Purpose::DECRYPTION,
            },
        ]);
        assert_eq!(keys.count, 2);
        let slice = unsafe { std::slice::from_raw_parts(keys.keys, keys.count) };
        assert_eq!(
            slice,
            &[
                PrivateDocumentRecipientKeyFFI {
                    key_id: 1,
                    purpose: 1
                },
                PrivateDocumentRecipientKeyFFI {
                    key_id: 2,
                    purpose: 2
                },
            ]
        );
        unsafe { platform_wallet_private_document_recipient_keys_free(&mut keys) };
        assert!(keys.keys.is_null());
        assert_eq!(keys.count, 0);
        // A second free, and a free of null, do nothing.
        unsafe { platform_wallet_private_document_recipient_keys_free(&mut keys) };
        unsafe { platform_wallet_private_document_recipient_keys_free(std::ptr::null_mut()) };

        let none = PrivateDocumentRecipientKeysFFI::from_keys(vec![]);
        assert!(none.keys.is_null());
        assert_eq!(none.count, 0);
    }

    #[test]
    fn should_marshal_a_created_entry() {
        let created: PrivateDocumentEntryCreatedFFI = PrivateDocumentEntryCreated {
            commitment: [5; 32],
            sender_key_id: 3,
        }
        .into();
        assert_eq!(
            created,
            PrivateDocumentEntryCreatedFFI {
                commitment: [5; 32],
                sender_key_id: 3
            }
        );
    }

    #[test]
    fn should_refuse_an_unknown_manager_handle() {
        let mut fee = 7u64;
        let name = std::ffi::CString::new("note").expect("a C string");
        let result = unsafe {
            platform_wallet_manager_private_document_create_fee(
                u64::MAX,
                [0u8; 32].as_ptr(),
                name.as_ptr(),
                1,
                &mut fee,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert_eq!(fee, 0);

        let mut keys = PrivateDocumentRecipientKeysFFI::empty();
        let result = unsafe {
            platform_wallet_manager_private_document_recipient_keys(
                u64::MAX,
                [0u8; 32].as_ptr(),
                &mut keys,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert!(keys.keys.is_null());
    }
}
