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
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::serialization::PlatformDeserializable;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::StateTransition;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::identity_registration_with_signer::{decode_contract_bounds, IdentityPubkeyFFI};
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

const IDENTITY_UPDATE_VARIANT_TAG: u8 = 6;

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
    /// 0 = none, 1 = SingleContract, 2 = SingleContractDocumentType.
    pub contract_bounds_kind: u8,
    pub contract_bounds_id: [u8; 32],
    pub contract_bounds_document_type: *mut c_char,
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
) -> Result<
    dpp::state_transition::identity_update_transition::IdentityUpdateTransition,
    PlatformWalletFFIResult,
> {
    let mut prefixed = Vec::with_capacity(bytes.len() + 1);
    prefixed.push(IDENTITY_UPDATE_VARIANT_TAG);
    prefixed.extend_from_slice(bytes);

    // A leading variant tag usually means the payload is already framed as a
    // state transition, and Yappr's tagless framing needs the tag prepended.
    // Neither test is conclusive — a tagless body can start with the tag byte
    // by coincidence — so the likelier framing is only tried first, and the
    // other one is still tried before the payload is rejected.
    let (first, first_label, second, second_label) =
        if bytes.first().copied() == Some(IDENTITY_UPDATE_VARIANT_TAG) {
            (bytes, "as-is", prefixed.as_slice(), "variant tag prepended")
        } else {
            (prefixed.as_slice(), "variant tag prepended", bytes, "as-is")
        };

    let state_transition = match StateTransition::deserialize_from_bytes(first) {
        Ok(state_transition) => state_transition,
        Err(first_error) => match StateTransition::deserialize_from_bytes(second) {
            Ok(state_transition) => state_transition,
            Err(second_error) => {
                return Err(PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorDeserialization,
                    format!(
                        "Failed to deserialize IdentityUpdateTransition in either framing \
                         ({first_label}: {first_error}; {second_label}: {second_error})"
                    ),
                ));
            }
        },
    };

    match state_transition {
        StateTransition::IdentityUpdate(identity_update) => Ok(identity_update),
        other => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("Expected IdentityUpdateTransition, got {other:?}"),
        )),
    }
}

/// Reporting a narrower bound than the transition declares would let the user
/// approve a broader scope than the one they were shown, so a document type
/// that cannot cross the FFI as a C string is an error rather than a fallback
/// to the contract-only bound.
fn encode_contract_bounds(
    bounds: Option<&ContractBounds>,
) -> Result<(u8, [u8; 32], *mut c_char), PlatformWalletFFIResult> {
    match bounds {
        Some(ContractBounds::SingleContract { id }) => Ok((1u8, id.to_buffer(), ptr::null_mut())),
        Some(ContractBounds::SingleContractDocumentType {
            id,
            document_type_name,
        }) => match CString::new(document_type_name.as_str()) {
            Ok(value) => Ok((2u8, id.to_buffer(), value.into_raw())),
            Err(error) => Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!(
                    "Contract-bounds document type name cannot be represented as a C string: \
                     {error}"
                ),
            )),
        },
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

fn project_parsed_identity_update(
    transition: &dpp::state_transition::identity_update_transition::IdentityUpdateTransition,
) -> Result<ParsedIdentityUpdateFFI, PlatformWalletFFIResult> {
    let identity_id = transition.identity_id().to_buffer();

    let public_keys_to_add = transition.public_keys_to_add();
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
        });
    }

    let add_public_keys_count = add_public_keys_vec.len();
    let add_public_keys = if add_public_keys_count == 0 {
        ptr::null_mut()
    } else {
        Box::into_raw(add_public_keys_vec.into_boxed_slice())
            as *mut ParsedIdentityUpdatePublicKeyFFI
    };

    let disable_public_key_ids_vec = transition.public_key_ids_to_disable().to_vec();
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
/// Accepts both normal tagged DPP state-transition bytes and Yappr's tagless
/// framing, where the positional bincode enum variant tag `6` has to be
/// prepended before deserialization.
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
    *out = unwrap_result_or_return!(project_parsed_identity_update(&transition));
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

                keys.push(IdentityPublicKey::V0(IdentityPublicKeyV0 {
                    id: row.key_id,
                    purpose,
                    security_level,
                    contract_bounds,
                    key_type,
                    read_only: row.read_only,
                    data: BinaryData::new(pubkey_bytes),
                    disabled_at: None,
                }));
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
                .update_identity_with_external_signer(&id, add_keys, disable_ids, signer, None)
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
    use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;

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
        let tagged = fixture_transition_bytes();
        let tagless = tagged[1..].to_vec();
        let mut out = ParsedIdentityUpdateFFI::default();

        let result = unsafe {
            platform_wallet_parse_identity_update_transition(
                tagless.as_ptr(),
                tagless.len(),
                &mut out,
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.identity_id, [0x11; 32]);
        assert_eq!(out.add_public_keys_count, 2);

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
                            document_type_name: "pro\0file".to_string(),
                        }),
                    }
                    .into()],
                    disable_public_keys: vec![],
                    user_fee_increase: 0,
                },
            );

        // `ParsedIdentityUpdateFFI` is a raw-pointer C struct with no `Debug`,
        // so the success case is rejected by hand rather than with `expect_err`.
        let error = match project_parsed_identity_update(&transition) {
            Ok(_) => panic!("a document type with an interior NUL must not be narrowed"),
            Err(error) => error,
        };

        assert_eq!(
            error.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
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
