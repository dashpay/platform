use std::os::raw::c_char;

// Single source of truth for the network type across the Rust-side
// wallet stack and the FFI boundary. `Network` is the typed enum;
// `FFINetwork` is the `#[repr(C)]` mirror cbindgen emits for callers.
// Every FFI entry point in this crate takes / returns `FFINetwork`;
// internal Rust code converts via `Network::from(ffi)` / `ffi.into()`.
pub use dash_network::ffi::FFINetwork;
pub use dash_network::Network;

/// Block time structure
#[repr(C)]
pub struct BlockTime {
    pub height: u64,
    pub core_height: u32,
    pub timestamp: u64,
}

impl From<platform_wallet::BlockTime> for BlockTime {
    fn from(bt: platform_wallet::BlockTime) -> Self {
        Self {
            height: bt.height,
            core_height: bt.core_height,
            timestamp: bt.timestamp,
        }
    }
}

impl From<BlockTime> for platform_wallet::BlockTime {
    fn from(bt: BlockTime) -> Self {
        Self {
            height: bt.height,
            core_height: bt.core_height,
            timestamp: bt.timestamp,
        }
    }
}

/// Read a 32-byte identifier from a `*const u8` buffer.
///
/// Centralised conversion helper used by every FFI entry point that
/// accepts an identifier id by pointer (the new ABI — see the
/// EXC_BAD_ACCESS write-up: `IdentifierBytes`-by-value is broken
/// across `@_silgen_name` because Swift's struct ABI ≠ AAPCS64 for
/// >16-byte aggregates).
///
/// `ptr` must point at exactly 32 readable bytes for the duration of
/// the call. Returns `None` on null pointer.
#[inline]
#[allow(clippy::result_large_err)]
pub unsafe fn read_identifier(
    ptr: *const u8,
) -> Result<dpp::prelude::Identifier, dpp::ProtocolError> {
    if ptr.is_null() {
        return Err(dpp::ProtocolError::IdentifierError(
            "identifier pointer is null".to_string(),
        ));
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr, 32) };
    dpp::prelude::Identifier::from_bytes(bytes)
        .map_err(|_| dpp::ProtocolError::IdentifierError("Invalid identifier bytes".to_string()))
}

/// Write a 32-byte identifier into a `*mut u8` out-buffer. Caller
/// must guarantee `ptr` is writable for at least 32 bytes.
///
/// Pairs with [`read_identifier`] — every formerly-`IdentifierBytes`
/// out-param became `*mut u8` under the same ABI fix.
#[inline]
pub unsafe fn write_identifier(ptr: *mut u8, id: &dpp::prelude::Identifier) {
    debug_assert!(!ptr.is_null());
    let bytes = id.to_buffer();
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, 32);
    }
}

/// Contact request structure
#[repr(C)]
pub struct ContactRequest {
    /// 32-byte identifier (raw bytes, not a struct, to keep this
    /// struct ABI-compatible with the C-side cbindgen view).
    pub identity_id: [u8; 32],
    pub label: *mut c_char,
    pub timestamp: u64,
}

/// Established contact structure
#[repr(C)]
pub struct EstablishedContact {
    pub identity_id: [u8; 32],
    pub label: *mut c_char,
    pub established_at: u64,
}

/// Array wrapper for returning multiple identifiers.
///
/// `items` points at a contiguous `[[u8; 32]; count]` buffer — flat
/// 32-byte rows, no per-row struct wrapper. The previous
/// `IdentifierBytes` row wrapper was removed in the
/// pointer-passing ABI fix; same byte layout, simpler shape on both
/// sides of the boundary.
#[repr(C)]
pub struct IdentifierArray {
    pub items: *mut [u8; 32],
    pub count: usize,
}

impl IdentifierArray {
    /// FFI-safe empty sentinel: a null pointer with a zero count.
    ///
    /// The `out_array: *mut IdentifierArray` entry points publish this
    /// before any fallible work (e.g. a handle-storage lookup) so an error
    /// return never leaves the out-param holding uninitialized stack bytes —
    /// `platform_wallet_identifier_array_free` reconstructs a `Vec` from any
    /// non-null pointer / non-zero count pair, so a cleanup-on-error caller
    /// would otherwise free garbage. The `(null, 0)` sentinel is skipped by
    /// that free path.
    pub fn empty() -> Self {
        Self {
            items: std::ptr::null_mut(),
            count: 0,
        }
    }

    pub fn new(identifiers: Vec<dpp::prelude::Identifier>) -> Self {
        let count = identifiers.len();
        if count == 0 {
            return Self::empty();
        }

        let mut items: Vec<[u8; 32]> = identifiers.into_iter().map(|id| id.to_buffer()).collect();

        let ptr = items.as_mut_ptr();
        std::mem::forget(items);

        Self { items: ptr, count }
    }

    /// Build the array from raw 32-byte rows (e.g. proTxHashes), which are not
    /// platform `Identifier`s. Same ownership contract as [`Self::new`]: the
    /// buffer is heap-leaked here and reclaimed by
    /// [`platform_wallet_identifier_array_free`].
    pub fn from_hashes(hashes: Vec<[u8; 32]>) -> Self {
        let count = hashes.len();
        if count == 0 {
            return Self::empty();
        }

        let mut items = hashes;
        let ptr = items.as_mut_ptr();
        std::mem::forget(items);

        Self { items: ptr, count }
    }
}

/// Free identifier array.
///
/// Pointer-only signature: the caller hands ownership back via a
/// pointer rather than passing the struct by value. Was previously
/// `(array: IdentifierArray)` — by-value was hitting the Swift /
/// AAPCS64 calling-convention mismatch for 16-byte aggregates.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identifier_array_free(array: *mut IdentifierArray) {
    if array.is_null() {
        return;
    }
    let array = unsafe { &mut *array };
    if !array.items.is_null() && array.count > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(array.items, array.count, array.count);
        }
    }
    array.items = std::ptr::null_mut();
    array.count = 0;
}

/// Free a C string
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_time_conversion() {
        let bt = platform_wallet::BlockTime {
            height: 100,
            core_height: 200,
            timestamp: 1234567890,
        };

        let ffi_bt: BlockTime = bt.into();
        assert_eq!(ffi_bt.height, 100);
        assert_eq!(ffi_bt.core_height, 200);
        assert_eq!(ffi_bt.timestamp, 1234567890);

        let back: platform_wallet::BlockTime = ffi_bt.into();
        assert_eq!(back.height, 100);
        assert_eq!(back.core_height, 200);
        assert_eq!(back.timestamp, 1234567890);
    }

    #[test]
    fn test_identifier_array_empty() {
        let array = IdentifierArray::new(vec![]);
        assert!(array.items.is_null());
        assert_eq!(array.count, 0);
    }

    #[test]
    fn test_identifier_array_with_items() {
        unsafe {
            let id1 = dpp::prelude::Identifier::random();
            let id2 = dpp::prelude::Identifier::random();

            let mut array = IdentifierArray::new(vec![id1, id2]);
            assert!(!array.items.is_null());
            assert_eq!(array.count, 2);

            platform_wallet_identifier_array_free(&mut array);
            assert!(array.items.is_null());
            assert_eq!(array.count, 0);
        }
    }

    #[test]
    fn test_read_identifier_round_trip() {
        unsafe {
            let id = dpp::prelude::Identifier::random();
            let buf: [u8; 32] = id.to_buffer();
            let read = read_identifier(buf.as_ptr()).expect("identifier should parse");
            assert_eq!(read, id);
        }
    }

    #[test]
    fn test_read_identifier_null() {
        unsafe {
            let res = read_identifier(std::ptr::null());
            assert!(res.is_err());
        }
    }
}
