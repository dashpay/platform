//! Identity operations

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dash_sdk::platform::{Fetch, FetchMany};
use dpp::prelude::{Identifier, Identity};
use platform_value::{string_encoding::Encoding, Value};

use crate::sdk::SDKWrapper;
use crate::types::{IOSSDKIdentityInfo, IdentityHandle, SDKHandle};
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};

/// Fetch an identity by ID
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_fetch(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
) -> IOSSDKResult {
    if sdk_handle.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if identity_id.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Identity ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(e) => {
            return IOSSDKResult::error(FFIError::from(e).into());
        }
    };

    let id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ));
        }
    };

    let result = wrapper.runtime.block_on(async {
        Identity::fetch(&wrapper.sdk, id)
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(Some(identity)) => {
            let handle = Box::into_raw(Box::new(identity)) as *mut IdentityHandle;
            IOSSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Ok(None) => IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::NotFound,
            "Identity not found".to_string(),
        )),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Create a new identity
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_create(sdk_handle: *mut SDKHandle) -> IOSSDKResult {
    if sdk_handle.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);

    let result = wrapper.runtime.block_on(async {
        match wrapper.sdk.wallet() {
            Some(wallet) => wrapper
                .sdk
                .identities()
                .create()
                .await
                .map_err(FFIError::from),
            None => Err(FFIError::InvalidState("No wallet configured".to_string())),
        }
    });

    match result {
        Ok(identity) => {
            let handle = Box::into_raw(Box::new(identity)) as *mut IdentityHandle;
            IOSSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Top up an identity with credits
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_topup(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    amount: u64,
) -> *mut IOSSDKError {
    if sdk_handle.is_null() || identity_handle.is_null() {
        return Box::into_raw(Box::new(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Handle is null".to_string(),
        )));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let identity = &*(identity_handle as *const Identity);

    let result = wrapper.runtime.block_on(async {
        match wrapper.sdk.wallet() {
            Some(wallet) => wrapper
                .sdk
                .identities()
                .top_up(identity.id(), amount)
                .await
                .map_err(|e| FFIError::InternalError(e.to_string())),
            None => Err(FFIError::InvalidState("No wallet configured".to_string())),
        }
    });

    match result {
        Ok(_) => std::ptr::null_mut(),
        Err(e) => Box::into_raw(Box::new(e.into())),
    }
}

/// Get identity information
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_get_info(
    identity_handle: *const IdentityHandle,
) -> *mut IOSSDKIdentityInfo {
    if identity_handle.is_null() {
        return std::ptr::null_mut();
    }

    let identity = &*(identity_handle as *const Identity);

    let id_str = match CString::new(identity.id().to_string(Encoding::Base58)) {
        Ok(s) => s.into_raw(),
        Err(_) => return std::ptr::null_mut(),
    };

    let info = IOSSDKIdentityInfo {
        id: id_str,
        balance: identity.balance(),
        revision: identity.revision() as u64,
        public_keys_count: identity.public_keys().len() as u32,
    };

    Box::into_raw(Box::new(info))
}

/// Destroy an identity handle
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_destroy(handle: *mut IdentityHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut Identity);
    }
}

/// Register a name for an identity
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_register_name(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    name: *const c_char,
) -> *mut IOSSDKError {
    if sdk_handle.is_null() || identity_handle.is_null() || name.is_null() {
        return Box::into_raw(Box::new(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Invalid parameters".to_string(),
        )));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let identity = &*(identity_handle as *const Identity);

    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(e) => {
            return Box::into_raw(Box::new(FFIError::from(e).into()));
        }
    };

    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .names()
            .register(name_str, identity.id())
            .await
            .map_err(|e| FFIError::InternalError(e.to_string()))
    });

    match result {
        Ok(_) => std::ptr::null_mut(),
        Err(e) => Box::into_raw(Box::new(e.into())),
    }
}

/// Resolve a name to an identity
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_resolve_name(
    sdk_handle: *const SDKHandle,
    name: *const c_char,
) -> IOSSDKResult {
    if sdk_handle.is_null() || name.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Invalid parameters".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(e) => {
            return IOSSDKResult::error(FFIError::from(e).into());
        }
    };

    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .names()
            .resolve(name_str)
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(Some(identity)) => {
            let handle = Box::into_raw(Box::new(identity)) as *mut IdentityHandle;
            IOSSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Ok(None) => IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::NotFound,
            "Name not registered".to_string(),
        )),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}
