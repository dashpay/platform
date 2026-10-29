//! The key layout a new identity is created with, marshalled from
//! [`platform_wallet::IdentityCreationKeyLayout`].
//!
//! Hosts read the layout here instead of keeping their own table: each row
//! names the key id, the DIP-9 key index to derive it at, and the role
//! (type, purpose, security level, contract bounds) to register it with.

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
use platform_wallet::{IdentityCreationKeyLayout, IdentityCreationKeySlot};

use crate::check_ptr;
use crate::error::*;
use crate::handle::{Handle, PLATFORM_WALLET_STORAGE};
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Which key layout an identity-creation flow registers. Mirrors
/// [`IdentityCreationKeyLayout`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityCreationKeyLayoutFFI {
    /// A new identity without DashPay keys; also what a resume of a tracked
    /// registration lock re-derives.
    NewIdentity = 0,
    /// A new identity with the DashPay ENCRYPTION / DECRYPTION keys.
    NewIdentityWithDashPayKeys = 1,
}

impl From<IdentityCreationKeyLayoutFFI> for IdentityCreationKeyLayout {
    fn from(layout: IdentityCreationKeyLayoutFFI) -> Self {
        match layout {
            IdentityCreationKeyLayoutFFI::NewIdentity => Self::NewIdentity,
            IdentityCreationKeyLayoutFFI::NewIdentityWithDashPayKeys => {
                Self::NewIdentityWithDashPayKeys
            }
        }
    }
}

impl From<IdentityCreationKeyLayout> for IdentityCreationKeyLayoutFFI {
    fn from(layout: IdentityCreationKeyLayout) -> Self {
        match layout {
            IdentityCreationKeyLayout::NewIdentity => Self::NewIdentity,
            IdentityCreationKeyLayout::NewIdentityWithDashPayKeys => {
                Self::NewIdentityWithDashPayKeys
            }
        }
    }
}

/// One key of the layout.
///
/// `contract_bounds_kind` uses the encoding of
/// [`crate::IdentityPubkeyFFI`]: `0` none, `1` `SingleContract`, `2`
/// `SingleContractDocumentType`. `contract_bounds_id` is zero when the kind
/// is `0`; `contract_bounds_document_type` is a Rust-owned NUL-terminated
/// string when the kind is `2`, null otherwise.
#[repr(C)]
pub struct IdentityCreationKeySlotFFI {
    pub key_id: u32,
    /// DIP-9 key index (`m/9'/coin'/5'/0'/0'/identity_index'/key_index'`).
    pub derivation_key_index: u32,
    pub key_type: u8,
    pub purpose: u8,
    pub security_level: u8,
    pub contract_bounds_kind: u8,
    pub contract_bounds_id: [u8; 32],
    pub contract_bounds_document_type: *mut c_char,
}

/// Heap-allocated array of [`IdentityCreationKeySlotFFI`] rows in key-id
/// order. Release with [`platform_wallet_identity_creation_key_layout_free`].
#[repr(C)]
pub struct IdentityCreationKeySlotsFFI {
    pub items: *mut IdentityCreationKeySlotFFI,
    pub count: usize,
}

impl IdentityCreationKeySlotFFI {
    fn from_slot(slot: &IdentityCreationKeySlot) -> Result<Self, PlatformWalletFFIResult> {
        let (contract_bounds_kind, contract_bounds_id, contract_bounds_document_type) =
            match &slot.contract_bounds {
                None => (0, [0u8; 32], ptr::null_mut()),
                Some(ContractBounds::SingleContract { id }) => (1, id.to_buffer(), ptr::null_mut()),
                Some(ContractBounds::SingleContractDocumentType {
                    id,
                    document_type_name,
                }) => {
                    let name = CString::new(document_type_name.as_str()).map_err(|e| {
                        PlatformWalletFFIResult::err(
                            PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                            format!("document type name contains a NUL byte: {e}"),
                        )
                    })?;
                    (2, id.to_buffer(), name.into_raw())
                }
                Some(ContractBounds::ContractGroup { .. }) => {
                    return Err(PlatformWalletFFIResult::err(
                        PlatformWalletFFIResultCode::ErrorSerialization,
                        "identity creation layout slot has contract-group bounds, which this \
                         row cannot carry",
                    ));
                }
            };
        Ok(Self {
            key_id: slot.key_id,
            derivation_key_index: slot.derivation_key_index(),
            key_type: slot.key_type as u8,
            purpose: slot.purpose as u8,
            security_level: slot.security_level as u8,
            contract_bounds_kind,
            contract_bounds_id,
            contract_bounds_document_type,
        })
    }

    /// Release the row's owned document type string.
    unsafe fn release(&mut self) {
        if !self.contract_bounds_document_type.is_null() {
            drop(CString::from_raw(self.contract_bounds_document_type));
            self.contract_bounds_document_type = ptr::null_mut();
        }
    }
}

/// Write the key layout an identity-creation flow registers to
/// `out_slots`, one row per key in key-id order.
///
/// On error `*out_slots` is left empty.
///
/// # Safety
/// `out_slots` must be a valid, writable pointer. Release a populated
/// result with [`platform_wallet_identity_creation_key_layout_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_creation_key_layout(
    layout: IdentityCreationKeyLayoutFFI,
    out_slots: *mut IdentityCreationKeySlotsFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_slots);
    *out_slots = IdentityCreationKeySlotsFFI {
        items: ptr::null_mut(),
        count: 0,
    };

    let mut rows = Vec::new();
    for slot in IdentityCreationKeyLayout::from(layout).slots() {
        match IdentityCreationKeySlotFFI::from_slot(&slot) {
            Ok(row) => rows.push(row),
            Err(e) => {
                for mut row in rows {
                    row.release();
                }
                return e;
            }
        }
    }

    let mut boxed = rows.into_boxed_slice();
    *out_slots = IdentityCreationKeySlotsFFI {
        items: boxed.as_mut_ptr(),
        count: boxed.len(),
    };
    std::mem::forget(boxed);
    PlatformWalletFFIResult::ok()
}

/// Release a layout written by [`platform_wallet_identity_creation_key_layout`]
/// and reset it to empty. Safe on an empty or null struct.
///
/// # Safety
/// `slots` must be null or point at a struct populated by
/// [`platform_wallet_identity_creation_key_layout`] that was not freed yet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_creation_key_layout_free(
    slots: *mut IdentityCreationKeySlotsFFI,
) {
    if slots.is_null() {
        return;
    }
    let slots = &mut *slots;
    if !slots.items.is_null() {
        let rows = Vec::from_raw_parts(slots.items, slots.count, slots.count);
        for mut row in rows {
            row.release();
        }
    }
    slots.items = ptr::null_mut();
    slots.count = 0;
}

/// Write the minimum asset-lock value (duffs) that registers `layout` at
/// the wallet's platform version to `out_duffs`, from the versioned fee
/// schedule (`IdentityWallet::identity_creation_min_funding_duffs`). Hosts
/// show and enforce it; Rust refuses a wallet-funded registration, or the
/// resume of a tracked lock, below it.
///
/// # Safety
/// `out_duffs` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_creation_min_funding_duffs(
    wallet_handle: Handle,
    layout: IdentityCreationKeyLayoutFFI,
    out_duffs: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_duffs);
    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        wallet
            .identity()
            .identity_creation_min_funding_duffs(layout.into())
    });
    let result = unwrap_option_or_return!(option);
    *out_duffs = unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

/// Minimum fees (credits) of an address-funded identity create, one per
/// input count: `items[n - 1]` is the fee with `n` inputs, and `count` is
/// the protocol's maximum input count. Release with
/// [`platform_wallet_identity_creation_from_addresses_min_fees_free`].
#[repr(C)]
pub struct IdentityCreationFeesFFI {
    pub items: *mut u64,
    pub count: usize,
}

/// Write the minimum fees of an address-funded identity create of `layout`
/// (no refund output) at the wallet's platform version to `out_fees`, from
/// the versioned fee schedule
/// (`IdentityWallet::identity_creation_from_addresses_min_fees`).
///
/// On error `*out_fees` is left empty.
///
/// # Safety
/// `out_fees` must be a valid, writable pointer. Release a populated result
/// with [`platform_wallet_identity_creation_from_addresses_min_fees_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_creation_from_addresses_min_fees(
    wallet_handle: Handle,
    layout: IdentityCreationKeyLayoutFFI,
    out_fees: *mut IdentityCreationFeesFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_fees);
    *out_fees = IdentityCreationFeesFFI {
        items: ptr::null_mut(),
        count: 0,
    };
    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        wallet
            .identity()
            .identity_creation_from_addresses_min_fees(layout.into())
    });
    let result = unwrap_option_or_return!(option);
    let fees = unwrap_result_or_return!(result);
    let mut boxed = fees.into_boxed_slice();
    *out_fees = IdentityCreationFeesFFI {
        items: boxed.as_mut_ptr(),
        count: boxed.len(),
    };
    std::mem::forget(boxed);
    PlatformWalletFFIResult::ok()
}

/// Release fees written by
/// [`platform_wallet_identity_creation_from_addresses_min_fees`] and reset
/// the struct to empty. Safe on an empty or null struct.
///
/// # Safety
/// `fees` must be null or point at a struct populated by
/// [`platform_wallet_identity_creation_from_addresses_min_fees`] that was
/// not freed yet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_creation_from_addresses_min_fees_free(
    fees: *mut IdentityCreationFeesFFI,
) {
    if fees.is_null() {
        return;
    }
    let fees = &mut *fees;
    if !fees.items.is_null() {
        drop(Vec::from_raw_parts(fees.items, fees.count, fees.count));
    }
    fees.items = ptr::null_mut();
    fees.count = 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contracts::SystemDataContract;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use platform_wallet::test_support::test_platform_wallet_manager;
    use std::ffi::CStr;

    use crate::runtime::runtime;

    fn read_layout(layout: IdentityCreationKeyLayoutFFI) -> Vec<(u32, u32, u8, u8, u8, u8)> {
        let mut out = IdentityCreationKeySlotsFFI {
            items: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe { platform_wallet_identity_creation_key_layout(layout, &mut out) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        let rows = unsafe { std::slice::from_raw_parts(out.items, out.count) }
            .iter()
            .map(|r| {
                (
                    r.key_id,
                    r.derivation_key_index,
                    r.key_type,
                    r.purpose,
                    r.security_level,
                    r.contract_bounds_kind,
                )
            })
            .collect();
        unsafe { platform_wallet_identity_creation_key_layout_free(&mut out) };
        assert!(out.items.is_null());
        assert_eq!(out.count, 0);
        rows
    }

    const SECP: u8 = KeyType::ECDSA_SECP256K1 as u8;
    const AUTH: u8 = Purpose::AUTHENTICATION as u8;

    #[test]
    fn should_marshal_the_new_identity_layout() {
        assert_eq!(
            read_layout(IdentityCreationKeyLayoutFFI::NewIdentity),
            vec![
                (0, 0, SECP, AUTH, SecurityLevel::MASTER as u8, 0),
                (1, 1, SECP, AUTH, SecurityLevel::CRITICAL as u8, 0),
                (2, 2, SECP, AUTH, SecurityLevel::HIGH as u8, 0),
                (
                    3,
                    3,
                    SECP,
                    Purpose::TRANSFER as u8,
                    SecurityLevel::CRITICAL as u8,
                    0
                ),
            ]
        );
    }

    #[test]
    fn should_marshal_the_dashpay_layout_with_contact_request_bounds() {
        let mut out = IdentityCreationKeySlotsFFI {
            items: ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            platform_wallet_identity_creation_key_layout(
                IdentityCreationKeyLayoutFFI::NewIdentityWithDashPayKeys,
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        let rows = unsafe { std::slice::from_raw_parts(out.items, out.count) };
        assert_eq!(rows.len(), 6);
        for (row, purpose) in [
            (&rows[4], Purpose::ENCRYPTION),
            (&rows[5], Purpose::DECRYPTION),
        ] {
            assert_eq!(row.purpose, purpose as u8);
            assert_eq!(row.security_level, SecurityLevel::MEDIUM as u8);
            assert_eq!(row.contract_bounds_kind, 2);
            assert_eq!(
                row.contract_bounds_id,
                SystemDataContract::Dashpay.id().to_buffer()
            );
            let name = unsafe { CStr::from_ptr(row.contract_bounds_document_type) };
            assert_eq!(name.to_str().unwrap(), "contactRequest");
        }
        unsafe { platform_wallet_identity_creation_key_layout_free(&mut out) };
    }

    #[test]
    fn should_tolerate_freeing_a_null_or_empty_layout() {
        unsafe { platform_wallet_identity_creation_key_layout_free(ptr::null_mut()) };
        let mut empty = IdentityCreationKeySlotsFFI {
            items: ptr::null_mut(),
            count: 0,
        };
        unsafe { platform_wallet_identity_creation_key_layout_free(&mut empty) };
    }

    #[test]
    fn should_marshal_the_rust_creation_minimums_for_each_layout() {
        let (manager, wallet) = runtime().block_on(async {
            let (manager, wallet_id) = test_platform_wallet_manager().await;
            let wallet = manager.get_wallet(&wallet_id).await.expect("test wallet");
            (manager, wallet)
        });
        let handle = PLATFORM_WALLET_STORAGE.insert(wallet.clone());
        for (ffi, layout) in [
            (
                IdentityCreationKeyLayoutFFI::NewIdentity,
                IdentityCreationKeyLayout::NewIdentity,
            ),
            (
                IdentityCreationKeyLayoutFFI::NewIdentityWithDashPayKeys,
                IdentityCreationKeyLayout::NewIdentityWithDashPayKeys,
            ),
        ] {
            let mut duffs = 0u64;
            let result = unsafe {
                platform_wallet_identity_creation_min_funding_duffs(handle, ffi, &mut duffs)
            };
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(
                duffs,
                wallet
                    .identity()
                    .identity_creation_min_funding_duffs(layout)
                    .expect("fee")
            );

            let mut fees = IdentityCreationFeesFFI {
                items: ptr::null_mut(),
                count: 0,
            };
            let result = unsafe {
                platform_wallet_identity_creation_from_addresses_min_fees(handle, ffi, &mut fees)
            };
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(
                unsafe { std::slice::from_raw_parts(fees.items, fees.count) },
                wallet
                    .identity()
                    .identity_creation_from_addresses_min_fees(layout)
                    .expect("fees")
                    .as_slice()
            );
            unsafe { platform_wallet_identity_creation_from_addresses_min_fees_free(&mut fees) };
            assert!(fees.items.is_null());
            assert_eq!(fees.count, 0);
        }

        PLATFORM_WALLET_STORAGE.remove(handle);
        let mut duffs = 0u64;
        let result = unsafe {
            platform_wallet_identity_creation_min_funding_duffs(
                handle,
                IdentityCreationKeyLayoutFFI::NewIdentity,
                &mut duffs,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
        drop(manager);
    }

    #[test]
    fn should_tolerate_freeing_null_or_empty_fees() {
        unsafe { platform_wallet_identity_creation_from_addresses_min_fees_free(ptr::null_mut()) };
        let mut empty = IdentityCreationFeesFFI {
            items: ptr::null_mut(),
            count: 0,
        };
        unsafe { platform_wallet_identity_creation_from_addresses_min_fees_free(&mut empty) };
    }
}
