#![allow(non_camel_case_types)]

use std::os::raw::{c_char, c_int};

#[no_mangle]
pub extern "C" fn dash_spv_ffi_config_add_peer(
    config: *mut dash_spv_ffi::FFIClientConfig,
    addr: *const c_char,
) -> c_int {
    // Safety: Forward call to underlying FFI; pointers are opaque
    unsafe { dash_spv_ffi::dash_spv_ffi_config_add_peer(config, addr) }
}

#[no_mangle]
pub extern "C" fn dash_spv_ffi_config_set_restrict_to_configured_peers(
    config: *mut dash_spv_ffi::FFIClientConfig,
    restrict_peers: bool,
) -> c_int {
    // Safety: Forward call to underlying FFI; pointers are opaque
    unsafe {
        dash_spv_ffi::dash_spv_ffi_config_set_restrict_to_configured_peers(config, restrict_peers)
    }
}
