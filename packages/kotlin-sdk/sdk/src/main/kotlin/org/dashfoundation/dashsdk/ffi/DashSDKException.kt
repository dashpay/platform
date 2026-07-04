package org.dashfoundation.dashsdk.ffi

/**
 * Exception thrown by the native layer (`rs-unified-sdk-jni`) when an FFI call
 * returns an error. Constructed from Rust via JNI — the `(Int, String)`
 * constructor signature must stay in sync with
 * `rs-unified-sdk-jni/src/support.rs::throw_sdk_exception`.
 *
 * [code] values mirror `DashSDKErrorCode` in `rs-sdk-ffi/src/error.rs`:
 * 0=Success, 1=InvalidParameter, 2=InvalidState, 3=NetworkError,
 * 4=SerializationError, 5=ProtocolError, 6=CryptoError, 7=NotFound,
 * 8=Timeout, 9=NotImplemented, 10=DriveInternalError, 99=InternalError.
 *
 * Internal: the public API maps this into the
 * [org.dashfoundation.dashsdk.errors.DashSdkError] hierarchy.
 */
class DashSDKException(
    val code: Int,
    message: String,
) : Exception(message)
