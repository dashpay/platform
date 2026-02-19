//! FFI-compatible address provider implementation using callbacks

use super::types::DashSDKPendingAddressList;
use dash_sdk::platform::address_sync::{AddressFunds, AddressIndex, AddressKey, AddressProvider};
use std::os::raw::c_void;

/// Function pointer type for getting pending addresses
///
/// Returns a list of pending addresses that need to be synchronized.
/// The returned list must remain valid until the next call to this function
/// or until the sync operation completes.
pub type GetPendingAddressesFn =
    unsafe extern "C" fn(context: *mut c_void) -> DashSDKPendingAddressList;

/// Function pointer type for getting the gap limit
pub type GetGapLimitFn = unsafe extern "C" fn(context: *mut c_void) -> u32;

/// Function pointer type for checking if there are pending addresses
pub type HasPendingFn = unsafe extern "C" fn(context: *mut c_void) -> bool;

/// Optional function pointer type for checking if there are pending addresses.
/// Nullable — may be null (cbindgen translates this to a nullable C function pointer).
pub type OptionalHasPendingFn = Option<unsafe extern "C" fn(context: *mut c_void) -> bool>;

/// Function pointer type for getting the highest found index
///
/// Returns the highest found index, or u32::MAX if none found
pub type GetHighestFoundIndexFn = unsafe extern "C" fn(context: *mut c_void) -> u32;

/// Optional function pointer type for getting the highest found index.
/// Nullable — may be null (cbindgen translates this to a nullable C function pointer).
pub type OptionalGetHighestFoundIndexFn = Option<unsafe extern "C" fn(context: *mut c_void) -> u32>;

/// Function pointer type for handling a found address
///
/// Called when an address is found in the tree with a balance and nonce.
pub type OnAddressFoundFn = unsafe extern "C" fn(
    context: *mut c_void,
    index: u32,
    key: *const u8,
    key_len: usize,
    nonce: u32,
    balance: u64,
);

/// Function pointer type for handling an absent address
///
/// Called when an address is proven absent from the tree.
pub type OnAddressAbsentFn =
    unsafe extern "C" fn(context: *mut c_void, index: u32, key: *const u8, key_len: usize);

/// Optional destructor for cleanup
pub type DestroyProviderFn = unsafe extern "C" fn(context: *mut c_void);

/// Optional destructor for cleanup.
/// Nullable — may be null (cbindgen translates this to a nullable C function pointer).
pub type OptionalDestroyProviderFn = Option<unsafe extern "C" fn(context: *mut c_void)>;

/// VTable for address provider callbacks
#[repr(C)]
pub struct AddressProviderVTable {
    /// Get the gap limit for this provider
    pub gap_limit: GetGapLimitFn,

    /// Get currently pending addresses to synchronize
    pub pending_addresses: GetPendingAddressesFn,

    /// Called when an address is found with a balance
    pub on_address_found: OnAddressFoundFn,

    /// Called when an address is proven absent
    pub on_address_absent: OnAddressAbsentFn,

    /// Check if there are still pending addresses.
    /// May be null; if null the default implementation (pending_addresses is non-empty) is used.
    pub has_pending: OptionalHasPendingFn,

    /// Get the highest found index.
    /// May be null; if null returns None.
    pub highest_found_index: OptionalGetHighestFoundIndexFn,

    /// Optional destructor for cleanup. May be null.
    pub destroy: OptionalDestroyProviderFn,
}

/// FFI-compatible address provider using callbacks
#[repr(C)]
pub struct AddressProviderFFI {
    /// Opaque context pointer passed to all callbacks
    pub context: *mut c_void,

    /// Pointer to the vtable containing callback functions
    pub vtable: *const AddressProviderVTable,
}

// SAFETY: AddressProviderFFI only contains function pointers and an opaque context
// The actual thread safety depends on the callback implementations
unsafe impl Send for AddressProviderFFI {}

/// Wrapper that implements AddressProvider trait using FFI callbacks
pub(crate) struct CallbackAddressProvider<'a> {
    ffi: &'a mut AddressProviderFFI,
}

impl<'a> CallbackAddressProvider<'a> {
    pub fn new(ffi: &'a mut AddressProviderFFI) -> Self {
        Self { ffi }
    }
}

impl<'a> AddressProvider for CallbackAddressProvider<'a> {
    fn gap_limit(&self) -> AddressIndex {
        unsafe {
            let vtable = &*self.ffi.vtable;
            (vtable.gap_limit)(self.ffi.context)
        }
    }

    fn pending_addresses(&self) -> Vec<(AddressIndex, AddressKey)> {
        unsafe {
            let vtable = &*self.ffi.vtable;
            let list = (vtable.pending_addresses)(self.ffi.context);

            if list.addresses.is_null() || list.count == 0 {
                return Vec::new();
            }

            let addresses_slice = std::slice::from_raw_parts(list.addresses, list.count);
            let mut result = Vec::with_capacity(list.count);

            for entry in addresses_slice {
                if !entry.key.is_null() && entry.key_len > 0 {
                    let key = std::slice::from_raw_parts(entry.key, entry.key_len).to_vec();
                    result.push((entry.index, key));
                }
            }

            result
        }
    }

    fn on_address_found(&mut self, index: AddressIndex, key: &[u8], funds: AddressFunds) {
        unsafe {
            let vtable = &*self.ffi.vtable;
            (vtable.on_address_found)(
                self.ffi.context,
                index,
                key.as_ptr(),
                key.len(),
                funds.nonce,
                funds.balance,
            );
        }
    }

    fn on_address_absent(&mut self, index: AddressIndex, key: &[u8]) {
        unsafe {
            let vtable = &*self.ffi.vtable;
            (vtable.on_address_absent)(self.ffi.context, index, key.as_ptr(), key.len());
        }
    }

    fn has_pending(&self) -> bool {
        unsafe {
            let vtable = &*self.ffi.vtable;
            if let Some(has_pending) = vtable.has_pending {
                has_pending(self.ffi.context)
            } else {
                // Default implementation
                !self.pending_addresses().is_empty()
            }
        }
    }

    fn highest_found_index(&self) -> Option<AddressIndex> {
        unsafe {
            let vtable = &*self.ffi.vtable;
            if let Some(get_highest) = vtable.highest_found_index {
                let index = get_highest(self.ffi.context);
                if index == u32::MAX {
                    None
                } else {
                    Some(index)
                }
            } else {
                None
            }
        }
    }
}

/// Free a pending address list
///
/// This should be called to clean up memory after the sync operation is complete
/// if the caller allocated the list dynamically.
///
/// Note: This only frees the list structure, not the key data which should be
/// managed by the caller.
///
/// # Safety
/// - `list` must be a valid pointer to a DashSDKPendingAddressList
///   or null (no-op)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_pending_address_list_free(list: *mut DashSDKPendingAddressList) {
    if list.is_null() {
        return;
    }

    let list = Box::from_raw(list);
    if !list.addresses.is_null() && list.count > 0 {
        // Only free the addresses array, not the key data (which is owned by caller)
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            list.addresses,
            list.count,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test vtable with simple implementations
    unsafe extern "C" fn test_gap_limit(_ctx: *mut c_void) -> u32 {
        20
    }

    unsafe extern "C" fn test_pending_addresses(_ctx: *mut c_void) -> DashSDKPendingAddressList {
        DashSDKPendingAddressList::empty()
    }

    unsafe extern "C" fn test_on_found(
        _ctx: *mut c_void,
        _index: u32,
        _key: *const u8,
        _key_len: usize,
        _nonce: u32,
        _balance: u64,
    ) {
    }

    unsafe extern "C" fn test_on_absent(
        _ctx: *mut c_void,
        _index: u32,
        _key: *const u8,
        _key_len: usize,
    ) {
    }

    static TEST_VTABLE: AddressProviderVTable = AddressProviderVTable {
        gap_limit: test_gap_limit,
        pending_addresses: test_pending_addresses,
        on_address_found: test_on_found,
        on_address_absent: test_on_absent,
        has_pending: None,
        highest_found_index: None,
        destroy: None,
    };

    #[test]
    fn test_callback_provider_gap_limit() {
        let mut ffi = AddressProviderFFI {
            context: std::ptr::null_mut(),
            vtable: &TEST_VTABLE,
        };

        let provider = CallbackAddressProvider::new(&mut ffi);
        assert_eq!(provider.gap_limit(), 20);
    }

    #[test]
    fn test_callback_provider_empty_pending() {
        let mut ffi = AddressProviderFFI {
            context: std::ptr::null_mut(),
            vtable: &TEST_VTABLE,
        };

        let provider = CallbackAddressProvider::new(&mut ffi);
        assert!(provider.pending_addresses().is_empty());
    }
}
