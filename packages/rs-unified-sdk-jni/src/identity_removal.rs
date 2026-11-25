//! JNI export removing an identity from the device's live wallets. It
//! marshals ONE C function of `platform-wallet-ffi`:
//!
//! * `removeIdentityFromDevice` -> `platform_wallet_manager_remove_identity_from_device`.
//!
//! Rust checks the identity's contact request guards in every wallet holding
//! it, removes it from each, and emits the removal to the host persister; a
//! refusal throws `DashSDKException` with the platform-wallet code and nothing
//! is removed.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.IdentityRemovalNative`.

use crate::decision_proposals::read_bytes32;
use crate::support::{guard, take_pwffi_error};
use jni::objects::{JByteArray, JClass};
use jni::sys::{jboolean, jlong, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::identity_removal::platform_wallet_manager_remove_identity_from_device;

/// Remove `identityId` (32 bytes) from every wallet of the manager that
/// manages it. Returns true when a wallet managed and removed it, false when
/// none did. Throws, removing nothing, on a refusal, an unknown handle, or an
/// id that is not 32 bytes.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_IdentityRemovalNative_removeIdentityFromDevice(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    identity_id: JByteArray,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| {
        let Some(identity_id) = read_bytes32(env, &identity_id, "identityId") else {
            return JNI_FALSE;
        };
        let mut was_managed = false;
        let result = unsafe {
            platform_wallet_manager_remove_identity_from_device(
                manager_handle as Handle,
                &identity_id,
                &mut was_managed,
            )
        };
        if take_pwffi_error(env, result) {
            return JNI_FALSE;
        }
        if was_managed {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}
