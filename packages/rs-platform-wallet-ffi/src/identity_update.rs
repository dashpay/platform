//! FFI bindings for identity update (add / disable keys) driven by an
//! external `SignerHandle`.
//!
//! The MASTER auth key signs the `IdentityUpdateTransition` via the
//! supplied `signer_handle` (typically the iOS-side `KeychainSigner`).

use std::convert::TryFrom;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::KeyID;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition::public_key_in_creation::accessors::{
    IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV1Getters,
};
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::{StateTransition, StateTransitionType};
use platform_wallet::wallet::identity::network::{
    check_document_type_name, decode_state_transition,
};
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::identity_registration_with_signer::{
    decode_contract_bounds, key_with_row_limits, IdentityPubkeyFFI,
};
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Owned C representation of one public key carried by a parsed
/// `IdentityUpdateTransition`.
///
/// The `data_ptr` buffer and optional `contract_bounds_document_type`
/// string are heap allocations owned by Rust and must be released via
/// [`platform_wallet_parse_identity_update_transition_free`].
#[repr(C)]
pub struct ParsedIdentityUpdatePublicKeyFFI {
    pub key_id: u32,
    pub key_type: u8,
    pub purpose: u8,
    pub security_level: u8,
    pub read_only: bool,
    pub data_ptr: *mut u8,
    pub data_len: usize,
    /// 0 = none, 1 = SingleContract, 2 = SingleContractDocumentType,
    /// 3 = ContractGroup (the id is the contract group id).
    pub contract_bounds_kind: u8,
    pub contract_bounds_id: [u8; 32],
    pub contract_bounds_document_type: *mut c_char,
    /// Whether the key is registered with a `total_budget` (protocol
    /// version 14, `IdentityPublicKeyInCreation::V1`).
    pub has_total_budget: bool,
    /// Credits the key may take from the identity over its lifetime, when
    /// `has_total_budget`.
    pub total_budget: u64,
    /// Whether the key is registered with an `expires_at`.
    pub has_expires_at: bool,
    /// Block time in milliseconds from which the key can no longer sign,
    /// when `has_expires_at`.
    pub expires_at: u64,
}

/// Owned C representation of the inspectable parts of a parsed
/// `IdentityUpdateTransition`.
#[repr(C)]
pub struct ParsedIdentityUpdateFFI {
    pub identity_id: [u8; 32],
    pub add_public_keys: *mut ParsedIdentityUpdatePublicKeyFFI,
    pub add_public_keys_count: usize,
    pub disable_public_key_ids: *mut u32,
    pub disable_public_key_ids_count: usize,
}

impl Default for ParsedIdentityUpdateFFI {
    fn default() -> Self {
        Self {
            identity_id: [0u8; 32],
            add_public_keys: ptr::null_mut(),
            add_public_keys_count: 0,
            disable_public_key_ids: ptr::null_mut(),
            disable_public_key_ids_count: 0,
        }
    }
}

fn parse_identity_update_transition_bytes(
    bytes: &[u8],
) -> Result<IdentityUpdateTransition, PlatformWalletFFIResult> {
    // Tolerates Yappr's untagged identity updates next to standard tagged bytes.
    let (state_transition, _tagged_bytes) =
        decode_state_transition(bytes, &[StateTransitionType::IdentityUpdate])
            .map_err(crate::parse_state_transition::deserialization_error)?;

    match state_transition {
        StateTransition::IdentityUpdate(identity_update) => Ok(identity_update),
        other => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("Expected IdentityUpdateTransition, got {}", other.name()),
        )),
    }
}

/// Reporting a narrower bound than the transition declares would let the user
/// approve a broader scope than the one they were shown, so a document type
/// that cannot be shown as is is an error rather than a fallback to the
/// contract-only bound. The name is the approval sheet's own field, so it must
/// pass the same consensus-pattern check the state transition summary applies.
fn encode_contract_bounds(
    bounds: Option<&ContractBounds>,
) -> Result<(u8, [u8; 32], *mut c_char), PlatformWalletFFIResult> {
    match bounds {
        Some(ContractBounds::SingleContract { id }) => Ok((1u8, id.to_buffer(), ptr::null_mut())),
        Some(ContractBounds::SingleContractDocumentType {
            id,
            document_type_name,
        }) => {
            // The consensus pattern excludes NUL, so a name that passes it is
            // always a valid C string.
            let value = check_document_type_name(document_type_name)
                .map_err(|error| error.to_string())
                .and_then(|()| {
                    CString::new(document_type_name.as_str()).map_err(|error| error.to_string())
                })
                .map_err(|error| {
                    PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorInvalidParameter,
                        format!("Contract-bounds document type: {error}"),
                    )
                })?;
            Ok((2u8, id.to_buffer(), value.into_raw()))
        }
        Some(ContractBounds::ContractGroup { id }) => Ok((3u8, id.to_buffer(), ptr::null_mut())),
        None => Ok((0u8, [0u8; 32], ptr::null_mut())),
    }
}

/// Frees the owned buffers behind already-projected keys. Shared by the free
/// entry point and the error path in [`project_parsed_identity_update`], which
/// has to release what it allocated before the caller ever sees the struct.
///
/// # Safety
/// Every non-null `data_ptr` / `contract_bounds_document_type` must be a
/// pointer this module allocated and has not freed yet.
unsafe fn free_parsed_public_keys(keys: &mut [ParsedIdentityUpdatePublicKeyFFI]) {
    for key in keys.iter_mut() {
        if !key.data_ptr.is_null() && key.data_len > 0 {
            let data_slice = slice::from_raw_parts_mut(key.data_ptr, key.data_len);
            let _ = Box::from_raw(data_slice as *mut [u8]);
            key.data_ptr = ptr::null_mut();
            key.data_len = 0;
        }

        if !key.contract_bounds_document_type.is_null() {
            let _ = CString::from_raw(key.contract_bounds_document_type);
            key.contract_bounds_document_type = ptr::null_mut();
        }
    }
}

pub(crate) fn project_parsed_identity_update(
    identity_id: &Identifier,
    public_keys_to_add: &[IdentityPublicKeyInCreation],
    public_key_ids_to_disable: &[KeyID],
) -> Result<ParsedIdentityUpdateFFI, PlatformWalletFFIResult> {
    let identity_id = identity_id.to_buffer();

    let mut add_public_keys_vec: Vec<ParsedIdentityUpdatePublicKeyFFI> =
        Vec::with_capacity(public_keys_to_add.len());

    for public_key in public_keys_to_add.iter() {
        // Encoded before the key data is boxed, so a rejected bound leaves
        // nothing of this key to release.
        let (contract_bounds_kind, contract_bounds_id, contract_bounds_document_type) =
            match encode_contract_bounds(public_key.contract_bounds()) {
                Ok(bounds) => bounds,
                Err(error) => {
                    // The caller never receives this struct, so nothing else
                    // will ever free the keys projected so far.
                    unsafe { free_parsed_public_keys(&mut add_public_keys_vec) };
                    return Err(error);
                }
            };

        let data = public_key.data().as_slice().to_vec().into_boxed_slice();
        let data_len = data.len();
        let data_ptr = Box::into_raw(data) as *mut u8;

        add_public_keys_vec.push(ParsedIdentityUpdatePublicKeyFFI {
            key_id: public_key.id(),
            key_type: public_key.key_type() as u8,
            purpose: public_key.purpose() as u8,
            security_level: public_key.security_level() as u8,
            read_only: public_key.read_only(),
            data_ptr,
            data_len,
            contract_bounds_kind,
            contract_bounds_id,
            contract_bounds_document_type,
            has_total_budget: public_key.total_budget().is_some(),
            total_budget: public_key.total_budget().unwrap_or_default(),
            has_expires_at: public_key.expires_at().is_some(),
            expires_at: public_key.expires_at().unwrap_or_default(),
        });
    }

    let add_public_keys_count = add_public_keys_vec.len();
    let add_public_keys = if add_public_keys_count == 0 {
        ptr::null_mut()
    } else {
        Box::into_raw(add_public_keys_vec.into_boxed_slice())
            as *mut ParsedIdentityUpdatePublicKeyFFI
    };

    let disable_public_key_ids_vec = public_key_ids_to_disable.to_vec();
    let disable_public_key_ids_count = disable_public_key_ids_vec.len();
    let disable_public_key_ids = if disable_public_key_ids_count == 0 {
        ptr::null_mut()
    } else {
        Box::into_raw(disable_public_key_ids_vec.into_boxed_slice()) as *mut u32
    };

    Ok(ParsedIdentityUpdateFFI {
        identity_id,
        add_public_keys,
        add_public_keys_count,
        disable_public_key_ids,
        disable_public_key_ids_count,
    })
}

/// Deserializes a raw `IdentityUpdateTransition` (as carried by a DashConnect
/// `dash-st:` QR) into its inspectable parts.
///
/// Accepts both normal tagged DPP state-transition bytes and an identity update
/// serialized without the `StateTransition` variant tag (Yappr's framing);
/// trailing bytes are refused.
///
/// Does NOT sign and does NOT broadcast — the caller validates the result and
/// rebuilds the transition through the normal signing path.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_parse_identity_update_transition(
    transition_bytes: *const u8,
    transition_len: usize,
    out: *mut ParsedIdentityUpdateFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(transition_bytes);
    check_ptr!(out);

    *out = ParsedIdentityUpdateFFI::default();

    let bytes = slice::from_raw_parts(transition_bytes, transition_len);
    let transition = unwrap_result_or_return!(parse_identity_update_transition_bytes(bytes));
    *out = unwrap_result_or_return!(project_parsed_identity_update(
        &transition.identity_id(),
        transition.public_keys_to_add(),
        transition.public_key_ids_to_disable(),
    ));
    PlatformWalletFFIResult::ok()
}

/// Frees a parsed transition previously returned by
/// [`platform_wallet_parse_identity_update_transition`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_parse_identity_update_transition_free(
    out: *mut ParsedIdentityUpdateFFI,
) {
    if out.is_null() {
        return;
    }

    let parsed = &mut *out;

    if !parsed.add_public_keys.is_null() && parsed.add_public_keys_count > 0 {
        let keys = slice::from_raw_parts_mut(parsed.add_public_keys, parsed.add_public_keys_count);
        free_parsed_public_keys(keys);
        let _ = Box::from_raw(keys as *mut [ParsedIdentityUpdatePublicKeyFFI]);
    }

    if !parsed.disable_public_key_ids.is_null() && parsed.disable_public_key_ids_count > 0 {
        let disable_ids = slice::from_raw_parts_mut(
            parsed.disable_public_key_ids,
            parsed.disable_public_key_ids_count,
        );
        let _ = Box::from_raw(disable_ids as *mut [u32]);
    }

    *parsed = ParsedIdentityUpdateFFI::default();
}

/// Update an identity by adding new public keys and/or disabling
/// existing key IDs, signing the resulting `IdentityUpdateTransition`
/// with the supplied `signer_handle`.
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `identity_id` must be null (refused) or point at a 32-byte buffer.
/// - When `add_public_keys_count` is non-zero, `add_public_keys` must
///   point at a valid, aligned `[IdentityPubkeyFFI; add_public_keys_count]`
///   array whose rows satisfy the contract documented on
///   [`IdentityPubkeyFFI`].
/// - When `disable_public_key_ids_count` is non-zero,
///   `disable_public_key_ids` must point at a valid, aligned
///   `[u32; disable_public_key_ids_count]` array.
/// - `signer_handle` must be a valid, non-destroyed `SignerHandle` from
///   rs-sdk-ffi's signer constructors (typically `KeychainSigner.handle`,
///   made by `dash_sdk_signer_create_with_ctx`). It is borrowed until the
///   call returns and this function does NOT destroy it. Its callbacks run
///   on a runtime worker thread while the calling thread blocks, so they
///   must be safe to call from any thread, must not wait on the calling
///   thread, and must not call back into this FFI.
/// - Every buffer above stays owned by the caller and must stay valid and
///   unmodified until the call returns; none is retained after it.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_update_identity_with_signer(
    wallet_handle: Handle,
    identity_id: *const u8,
    add_public_keys: *const IdentityPubkeyFFI,
    add_public_keys_count: usize,
    disable_public_key_ids: *const u32,
    disable_public_key_ids_count: usize,
    signer_handle: *mut SignerHandle,
) -> PlatformWalletFFIResult {
    update_identity_with_signer(
        wallet_handle,
        identity_id,
        add_public_keys,
        add_public_keys_count,
        disable_public_key_ids,
        disable_public_key_ids_count,
        None,
        signer_handle,
    )
}

/// [`platform_wallet_update_identity_with_signer`], with the revision the
/// update must carry. When the cached identity would give the transition
/// another revision (it moved since the caller read it), the call fails
/// before anything is signed or broadcast, so a caller that recorded
/// `revision` knows the only revision this update can land at. Read the
/// revision to record from [`platform_wallet_identity_next_update_revision`].
/// When Drive refuses the update for its revision and proofs show the
/// refused transition's nonce spent and the identity below that revision,
/// the call fails with `ErrorIdentityRevisionRefused` (63) and the next
/// call to that reader gives the revision Drive takes. Any other failure
/// keeps its own code.
///
/// # Safety
/// Same pointer, length and lifetime rules as
/// [`platform_wallet_update_identity_with_signer`].
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_update_identity_with_signer_at_revision(
    wallet_handle: Handle,
    identity_id: *const u8,
    add_public_keys: *const IdentityPubkeyFFI,
    add_public_keys_count: usize,
    disable_public_key_ids: *const u32,
    disable_public_key_ids_count: usize,
    revision: u64,
    signer_handle: *mut SignerHandle,
) -> PlatformWalletFFIResult {
    update_identity_with_signer(
        wallet_handle,
        identity_id,
        add_public_keys,
        add_public_keys_count,
        disable_public_key_ids,
        disable_public_key_ids_count,
        Some(revision),
        signer_handle,
    )
}

/// The revision the next update of `identity_id` carries, into
/// `out_revision`: one above the cached identity's, or, after
/// Drive refused the last update for its revision below the cache's
/// (`ErrorIdentityRevisionRefused` for a pinned update), one above the
/// proved revision while the cache is still where it was then. The note
/// lives in memory: after a restart this gives one above the cache again.
/// Record it before submitting and pass it to
/// [`platform_wallet_update_identity_with_signer_at_revision`].
///
/// # Safety
/// `identity_id` must point to 32 readable bytes and `out_revision` to a
/// writable `u64`, both for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_next_update_revision(
    wallet_handle: Handle,
    identity_id: *const u8,
    out_revision: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_revision);
    *out_revision = 0;
    let id = unwrap_result_or_return!(read_identifier(identity_id));

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move { identity_wallet.next_update_revision(&id).await })
    });
    let result = unwrap_option_or_return!(option);
    *out_revision = unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

#[allow(clippy::too_many_arguments)]
unsafe fn update_identity_with_signer(
    wallet_handle: Handle,
    identity_id: *const u8,
    add_public_keys: *const IdentityPubkeyFFI,
    add_public_keys_count: usize,
    disable_public_key_ids: *const u32,
    disable_public_key_ids_count: usize,
    revision: Option<u64>,
    signer_handle: *mut SignerHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);

    let id = unwrap_result_or_return!(read_identifier(identity_id));

    let add_keys: Vec<IdentityPublicKey> =
        if add_public_keys.is_null() || add_public_keys_count == 0 {
            Vec::new()
        } else {
            let rows: &[IdentityPubkeyFFI] =
                slice::from_raw_parts(add_public_keys, add_public_keys_count);
            let mut keys: Vec<IdentityPublicKey> = Vec::with_capacity(rows.len());
            for (i, row) in rows.iter().enumerate() {
                let key_type = unwrap_result_or_return!(KeyType::try_from(row.key_type));
                let purpose = unwrap_result_or_return!(Purpose::try_from(row.purpose));
                let security_level =
                    unwrap_result_or_return!(SecurityLevel::try_from(row.security_level));
                if row.pubkey_bytes.is_null() || row.pubkey_len == 0 {
                    return PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorNullPointer,
                        format!("add_public_keys[{i}].pubkey_bytes is null or empty"),
                    );
                }
                let pubkey_bytes: Vec<u8> =
                    slice::from_raw_parts(row.pubkey_bytes, row.pubkey_len).to_vec();

                let contract_bounds = unwrap_result_or_return!(decode_contract_bounds(
                    row,
                    purpose,
                    i,
                    "add_public_keys"
                ));

                keys.push(key_with_row_limits(
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
                    row,
                ));
            }
            keys
        };

    let disable_ids: Vec<u32> =
        if disable_public_key_ids.is_null() || disable_public_key_ids_count == 0 {
            Vec::new()
        } else {
            slice::from_raw_parts(disable_public_key_ids, disable_public_key_ids_count).to_vec()
        };

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity_wallet
                .update_identity_with_external_signer_at_revision(
                    &id,
                    add_keys,
                    disable_ids,
                    revision,
                    signer,
                    None,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
    use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
    use dpp::identity::Identity;
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use dpp::version::PlatformVersion;
    use platform_wallet::test_support::test_platform_wallet_manager;
    use rs_sdk_ffi::{
        dash_sdk_signer_create_with_ctx, dash_sdk_signer_destroy, SignCompletionCallback,
    };
    use std::ffi::c_void;
    use std::ffi::CStr;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::runtime::runtime;

    /// Counts every call the update makes into a signer created through
    /// `dash_sdk_signer_create_with_ctx`; `ctx` points at the counter.
    unsafe extern "C" fn counting_sign_async(
        ctx: *const c_void,
        _pubkey_bytes: *const u8,
        _pubkey_len: usize,
        _key_type: u8,
        _data: *const u8,
        _data_len: usize,
        completion_ctx: *mut c_void,
        completion: SignCompletionCallback,
    ) {
        (*(ctx as *const AtomicUsize)).fetch_add(1, Ordering::SeqCst);
        let message = CString::new("counting signer").unwrap();
        completion(completion_ctx, ptr::null(), 0, 0, message.as_ptr());
    }

    unsafe extern "C" fn counting_can_sign(
        ctx: *const c_void,
        _pubkey_bytes: *const u8,
        _pubkey_len: usize,
        _key_type: u8,
    ) -> bool {
        (*(ctx as *const AtomicUsize)).fetch_add(1, Ordering::SeqCst);
        true
    }

    /// A signer handle whose every callback bumps `calls`. Destroy it with
    /// `dash_sdk_signer_destroy` before `calls` goes out of scope.
    fn counting_signer(calls: &AtomicUsize) -> *mut SignerHandle {
        unsafe {
            dash_sdk_signer_create_with_ctx(
                calls as *const AtomicUsize as *mut c_void,
                counting_sign_async,
                counting_can_sign,
                None,
            )
        }
    }

    fn message_of(result: &PlatformWalletFFIResult) -> String {
        assert!(!result.message.is_null());
        unsafe { CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    }

    /// A wallet handle whose cached identity `identity_id` is at `revision`
    /// and holds a signable MASTER key (id 0), so an update that got past
    /// the revision check would consult the signer. The manager owns the
    /// wallet and must outlive the handle.
    fn wallet_handle_at_revision(identity_id: Identifier, revision: u64) -> (impl Sized, Handle) {
        runtime().block_on(async {
            let (manager, wallet_id) = test_platform_wallet_manager().await;
            let wallet = manager.get_wallet(&wallet_id).await.expect("test wallet");
            let mut identity =
                Identity::default_versioned(PlatformVersion::latest()).expect("identity");
            identity.set_id(identity_id);
            identity.add_public_key(
                IdentityPublicKeyV0 {
                    id: 0,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::MASTER,
                    key_type: KeyType::ECDSA_SECP256K1,
                    data: vec![0x02; KeyType::ECDSA_SECP256K1.default_size()].into(),
                    ..Default::default()
                }
                .into(),
            );
            identity.set_revision(revision);
            {
                let mut wm = wallet.wallet_manager().write().await;
                wm.get_wallet_info_mut(&wallet_id)
                    .expect("wallet info")
                    .identity_manager
                    .add_identity(identity, 0, wallet_id, wallet.persister())
                    .expect("add identity");
            }
            let handle = PLATFORM_WALLET_STORAGE.insert(wallet);
            (manager, handle)
        })
    }

    /// The reader gives one above the cached revision for an identity the
    /// wallet holds, and refuses a null out-pointer and an unknown identity.
    #[test]
    fn should_read_the_next_update_revision_through_the_export() {
        let identity_id = Identifier::from([0x22; 32]);
        let id_bytes = identity_id.to_buffer();
        let (_manager, handle) = wallet_handle_at_revision(identity_id, 5);

        let mut revision = u64::MAX;
        let result = unsafe {
            platform_wallet_identity_next_update_revision(handle, id_bytes.as_ptr(), &mut revision)
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::Success,
            "{}",
            message_of(&result)
        );
        assert_eq!(revision, 6);

        let result = unsafe {
            platform_wallet_identity_next_update_revision(
                handle,
                id_bytes.as_ptr(),
                ptr::null_mut(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        let unknown = Identifier::from([0x23; 32]).to_buffer();
        let mut revision = u64::MAX;
        let result = unsafe {
            platform_wallet_identity_next_update_revision(handle, unknown.as_ptr(), &mut revision)
        };
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(revision, 0);
    }

    /// The `_at_revision` export hands its revision to the wallet's guard: a
    /// revision the cached identity would not give the transition comes back
    /// as `ErrorIdentityRevisionMismatch` (62) before the signer is
    /// consulted. The matching revision gets past the guard to the signer
    /// (and then fails at the mock SDK), so the zero count comes from the
    /// refusal, not from the fixture.
    #[test]
    fn should_forward_the_revision_to_the_guard_before_signing() {
        let identity_id = Identifier::from([0x21; 32]);
        let id_bytes = identity_id.to_buffer();
        let disable_ids = [1u32];
        let call = |handle: Handle, revision: u64, signer: *mut SignerHandle| unsafe {
            platform_wallet_update_identity_with_signer_at_revision(
                handle,
                id_bytes.as_ptr(),
                ptr::null(),
                0,
                disable_ids.as_ptr(),
                disable_ids.len(),
                revision,
                signer,
            )
        };

        // Cached at 5, so the transition would carry 6.
        let (_manager, handle) = wallet_handle_at_revision(identity_id, 5);
        for asked in [5, 7] {
            let calls = AtomicUsize::new(0);
            let signer = counting_signer(&calls);
            let result = call(handle, asked, signer);
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorIdentityRevisionMismatch,
                "asked {asked}: {}",
                message_of(&result)
            );
            assert_eq!(result.code as i32, 62);
            assert_eq!(calls.load(Ordering::SeqCst), 0, "asked {asked}");
            unsafe { dash_sdk_signer_destroy(signer) };
        }

        let calls = AtomicUsize::new(0);
        let signer = counting_signer(&calls);
        let result = call(handle, 6, signer);
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert_ne!(
            result.code,
            PlatformWalletFFIResultCode::ErrorIdentityRevisionMismatch,
            "{}",
            message_of(&result)
        );
        assert!(calls.load(Ordering::SeqCst) > 0);
        unsafe { dash_sdk_signer_destroy(signer) };

        PLATFORM_WALLET_STORAGE.remove(handle);
    }

    const YAPPR_TAGLESS_IDENTITY_UPDATE_FIXTURE_HEX: &str = concat!(
        "0089fd6ddba75136a4fea02dc7d89ef0ca5bcc32ccf12fb8da6a1a03740567ae72",
        "01010200060200020000145e24e38a86e720f61757647996957e322686abb70000",
        "07000103000021035e8cfb0785b54e8902a3dc17bdaad8a5738c6019a18ebc527f",
        "79d1c64a27826a4120dc911df1d1e6cccf8c95ec0d423c928433397933de6dd9ad",
        "006bc40dc0334d6d270d50c6d5e2dcdc5560e40487ddfe28bd1066d0729fad4b26",
        "f92ae33f12a04b00000000"
    );

    fn fixture_transition_bytes() -> Vec<u8> {
        let identity_id = Identifier::from([0x11; 32]);
        let contract_id = Identifier::from([0x44; 32]);

        let transition = StateTransition::IdentityUpdate(
            IdentityUpdateTransitionV0 {
                signature: BinaryData::new(vec![0x99; 65]),
                signature_public_key_id: 3,
                identity_id,
                revision: 7,
                nonce: 9,
                add_public_keys: vec![
                    IdentityPublicKeyInCreationV0 {
                        id: 17,
                        key_type: KeyType::ECDSA_SECP256K1,
                        purpose: Purpose::AUTHENTICATION,
                        security_level: SecurityLevel::HIGH,
                        read_only: false,
                        data: BinaryData::new(vec![0x02; 33]),
                        signature: BinaryData::new(vec![0xaa; 65]),
                        contract_bounds: None,
                    }
                    .into(),
                    IdentityPublicKeyInCreationV0 {
                        id: 18,
                        key_type: KeyType::ECDSA_SECP256K1,
                        purpose: Purpose::ENCRYPTION,
                        security_level: SecurityLevel::HIGH,
                        read_only: true,
                        data: BinaryData::new(vec![0x03; 33]),
                        signature: BinaryData::new(vec![0xbb; 65]),
                        contract_bounds: Some(ContractBounds::SingleContractDocumentType {
                            id: contract_id,
                            document_type_name: "profile".to_string(),
                        }),
                    }
                    .into(),
                ],
                disable_public_keys: vec![4, 8],
                user_fee_increase: 2,
            }
            .into(),
        );

        transition
            .serialize_to_bytes()
            .expect("fixture transition serializes")
    }

    #[test]
    fn parses_tagged_identity_update_transition() {
        let bytes = fixture_transition_bytes();
        let mut out = ParsedIdentityUpdateFFI::default();

        let result = unsafe {
            platform_wallet_parse_identity_update_transition(bytes.as_ptr(), bytes.len(), &mut out)
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.identity_id, [0x11; 32]);
        assert_eq!(out.add_public_keys_count, 2);
        assert_eq!(out.disable_public_key_ids_count, 2);

        let keys = unsafe { slice::from_raw_parts(out.add_public_keys, out.add_public_keys_count) };
        assert_eq!(keys[0].key_id, 17);
        assert!(!keys[0].read_only);
        assert!(keys[1].read_only);
        assert_eq!(keys[1].contract_bounds_kind, 2);
        assert_eq!(keys[1].contract_bounds_id, [0x44; 32]);
        let doc_type = unsafe {
            std::ffi::CStr::from_ptr(keys[1].contract_bounds_document_type)
                .to_str()
                .expect("doc type utf8")
        };
        assert_eq!(doc_type, "profile");

        let disable_ids = unsafe {
            slice::from_raw_parts(out.disable_public_key_ids, out.disable_public_key_ids_count)
        };
        assert_eq!(disable_ids, &[4, 8]);

        unsafe { platform_wallet_parse_identity_update_transition_free(&mut out) };
        assert!(out.add_public_keys.is_null());
        assert!(out.disable_public_key_ids.is_null());
    }

    #[test]
    fn parses_yappr_tagless_identity_update_transition() {
        let tagless = hex::decode(YAPPR_TAGLESS_IDENTITY_UPDATE_FIXTURE_HEX)
            .expect("valid Yappr fixture hex");
        let mut out = ParsedIdentityUpdateFFI::default();

        let result = unsafe {
            platform_wallet_parse_identity_update_transition(
                tagless.as_ptr(),
                tagless.len(),
                &mut out,
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(
            out.identity_id.as_slice(),
            hex::decode("89fd6ddba75136a4fea02dc7d89ef0ca5bcc32ccf12fb8da6a1a03740567ae72")
                .expect("valid identity id hex")
                .as_slice()
        );
        assert_eq!(out.disable_public_key_ids_count, 0);
        assert_eq!(out.add_public_keys_count, 2);

        let keys = unsafe { slice::from_raw_parts(out.add_public_keys, out.add_public_keys_count) };

        assert_eq!(keys[0].key_id, 6);
        assert_eq!(
            Purpose::try_from(keys[0].purpose).expect("recognized purpose"),
            Purpose::AUTHENTICATION
        );
        assert_eq!(
            SecurityLevel::try_from(keys[0].security_level).expect("recognized security level"),
            SecurityLevel::HIGH
        );
        assert_eq!(
            KeyType::try_from(keys[0].key_type).expect("recognized key type"),
            KeyType::ECDSA_HASH160
        );
        assert!(!keys[0].read_only);
        assert_eq!(keys[0].contract_bounds_kind, 0);
        let key0_data = unsafe { slice::from_raw_parts(keys[0].data_ptr, keys[0].data_len) };
        assert_eq!(
            key0_data,
            hex::decode("5e24e38a86e720f61757647996957e322686abb7")
                .expect("valid authentication key hex")
                .as_slice()
        );

        assert_eq!(keys[1].key_id, 7);
        assert_eq!(
            Purpose::try_from(keys[1].purpose).expect("recognized purpose"),
            Purpose::ENCRYPTION
        );
        assert_eq!(
            SecurityLevel::try_from(keys[1].security_level).expect("recognized security level"),
            SecurityLevel::MEDIUM
        );
        assert_eq!(
            KeyType::try_from(keys[1].key_type).expect("recognized key type"),
            KeyType::ECDSA_SECP256K1
        );
        assert!(!keys[1].read_only);
        // The real DashConnect ENCRYPTION key is intentionally unbounded, so
        // this guards the decoder widening that now accepts absent bounds.
        assert_eq!(keys[1].contract_bounds_kind, 0);
        let key1_data = unsafe { slice::from_raw_parts(keys[1].data_ptr, keys[1].data_len) };
        assert_eq!(
            key1_data,
            hex::decode("035e8cfb0785b54e8902a3dc17bdaad8a5738c6019a18ebc527f79d1c64a27826a")
                .expect("valid encryption key hex")
                .as_slice()
        );

        unsafe { platform_wallet_parse_identity_update_transition_free(&mut out) };
    }

    #[test]
    fn rejects_malformed_identity_update_transition_bytes() {
        let bytes = [0xde, 0xad, 0xbe, 0xef];
        let mut out = ParsedIdentityUpdateFFI::default();

        let result = unsafe {
            platform_wallet_parse_identity_update_transition(bytes.as_ptr(), bytes.len(), &mut out)
        };

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorDeserialization
        );
        assert!(out.add_public_keys.is_null());
        assert!(out.disable_public_key_ids.is_null());
        assert_eq!(out.add_public_keys_count, 0);
        assert_eq!(out.disable_public_key_ids_count, 0);
    }

    #[test]
    fn rejects_contract_bounds_document_type_that_cannot_cross_the_ffi() {
        // A document type carrying an interior NUL cannot be handed over as a
        // C string. Reporting the contract-only bound instead would show the
        // user a broader scope than the transition declares, so this is an
        // error rather than a narrowing.
        let transition =
            dpp::state_transition::identity_update_transition::IdentityUpdateTransition::V0(
                IdentityUpdateTransitionV0 {
                    signature: BinaryData::new(vec![0x99; 65]),
                    signature_public_key_id: 3,
                    identity_id: Identifier::from([0x11; 32]),
                    revision: 7,
                    nonce: 9,
                    add_public_keys: vec![
                        // Projected successfully, so its boxed data buffer and
                        // contract-bounds string are live when the next key
                        // fails. That makes the error path's
                        // `free_parsed_public_keys` run over a non-empty
                        // vector — with only the failing key present it would
                        // free nothing and a leak here would go unnoticed.
                        IdentityPublicKeyInCreationV0 {
                            id: 16,
                            key_type: KeyType::ECDSA_SECP256K1,
                            purpose: Purpose::ENCRYPTION,
                            security_level: SecurityLevel::MEDIUM,
                            read_only: false,
                            data: BinaryData::new(vec![0x03; 33]),
                            signature: BinaryData::new(vec![0xcc; 65]),
                            contract_bounds: Some(ContractBounds::SingleContractDocumentType {
                                id: Identifier::from([0x55; 32]),
                                document_type_name: "profile".to_string(),
                            }),
                        }
                        .into(),
                        IdentityPublicKeyInCreationV0 {
                            id: 17,
                            key_type: KeyType::ECDSA_SECP256K1,
                            purpose: Purpose::ENCRYPTION,
                            security_level: SecurityLevel::MEDIUM,
                            read_only: false,
                            data: BinaryData::new(vec![0x02; 33]),
                            signature: BinaryData::new(vec![0xaa; 65]),
                            contract_bounds: Some(ContractBounds::SingleContractDocumentType {
                                id: Identifier::from([0x44; 32]),
                                document_type_name: "pro\0file".to_string(),
                            }),
                        }
                        .into(),
                    ],
                    disable_public_keys: vec![],
                    user_fee_increase: 0,
                },
            );

        // `ParsedIdentityUpdateFFI` is a raw-pointer C struct with no `Debug`,
        // so the success case is rejected by hand rather than with `expect_err`.
        let error = match project_parsed_identity_update(
            &transition.identity_id(),
            transition.public_keys_to_add(),
            transition.public_key_ids_to_disable(),
        ) {
            Ok(_) => panic!("a document type with an interior NUL must not be narrowed"),
            Err(error) => error,
        };

        assert_eq!(
            error.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
    }

    /// The legacy parser shows a document-type bound in its own field, so a
    /// name outside the consensus pattern is refused through the public entry
    /// point, as the state transition summary refuses it.
    #[test]
    fn rejects_contract_bounds_document_type_outside_the_consensus_pattern() {
        for name in ["a\nb", "post\u{202e}", "", &"x".repeat(65)] {
            let transition: StateTransition = IdentityUpdateTransitionV0 {
                identity_id: Identifier::from([0x11; 32]),
                revision: 1,
                nonce: 1,
                add_public_keys: vec![IdentityPublicKeyInCreationV0 {
                    id: 17,
                    key_type: KeyType::ECDSA_SECP256K1,
                    purpose: Purpose::ENCRYPTION,
                    security_level: SecurityLevel::MEDIUM,
                    read_only: false,
                    data: BinaryData::new(vec![0x02; 33]),
                    signature: BinaryData::new(vec![0xaa; 65]),
                    contract_bounds: Some(ContractBounds::SingleContractDocumentType {
                        id: Identifier::from([0x44; 32]),
                        document_type_name: name.to_string(),
                    }),
                }
                .into()],
                ..Default::default()
            }
            .into();
            let bytes = transition.serialize_to_bytes().unwrap();
            let mut out = ParsedIdentityUpdateFFI::default();

            let result = unsafe {
                platform_wallet_parse_identity_update_transition(
                    bytes.as_ptr(),
                    bytes.len(),
                    &mut out,
                )
            };
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "{name:?}"
            );
            assert!(out.add_public_keys.is_null(), "{name:?}");
        }
    }

    #[test]
    fn rejects_truncated_identity_update_transition_bytes() {
        let mut bytes = fixture_transition_bytes();
        bytes.truncate(bytes.len() - 7);
        let mut out = ParsedIdentityUpdateFFI::default();

        let result = unsafe {
            platform_wallet_parse_identity_update_transition(bytes.as_ptr(), bytes.len(), &mut out)
        };

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorDeserialization
        );
        assert!(out.add_public_keys.is_null());
        assert!(out.disable_public_key_ids.is_null());
        assert_eq!(out.add_public_keys_count, 0);
        assert_eq!(out.disable_public_key_ids_count, 0);
    }
}
