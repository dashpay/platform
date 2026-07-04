package org.dashfoundation.dashsdk.errors

import org.dashfoundation.dashsdk.ffi.DashSDKException

/**
 * Public error hierarchy of the Kotlin SDK — the Android analog of the
 * Swift SDK's `UserFacingError`/`SDKError` split, keyed off the native
 * `DashSDKErrorCode` values (`rs-sdk-ffi/src/error.rs`).
 *
 * The JNI layer throws the internal [DashSDKException]; public API entry
 * points convert it via [fromNative] so callers only ever see this type.
 */
sealed class DashSdkError(
    message: String,
    cause: Throwable? = null,
) : Exception(message, cause) {

    /** Whether retrying the same operation can plausibly succeed. */
    open val isRetryable: Boolean get() = false

    class InvalidParameter(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class InvalidState(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class NetworkError(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause) {
        override val isRetryable: Boolean get() = true
    }

    class SerializationError(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class ProtocolError(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class CryptoError(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class NotFound(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class Timeout(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause) {
        override val isRetryable: Boolean get() = true
    }

    class NotImplemented(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class DriveInternalError(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    class InternalError(message: String, cause: Throwable? = null) :
        DashSdkError(message, cause)

    companion object {
        /** Map a native error code + message into the public hierarchy. */
        fun fromNative(e: DashSDKException): DashSdkError {
            val message = e.message ?: "Unknown SDK error"
            return when (e.code) {
                1 -> InvalidParameter(message, e)
                2 -> InvalidState(message, e)
                3 -> NetworkError(message, e)
                4 -> SerializationError(message, e)
                5 -> ProtocolError(message, e)
                6 -> CryptoError(message, e)
                7 -> NotFound(message, e)
                8 -> Timeout(message, e)
                9 -> NotImplemented(message, e)
                10 -> DriveInternalError(message, e)
                else -> InternalError(message, e)
            }
        }
    }
}

/**
 * Run [block], converting any [DashSDKException] escaping the native layer
 * into the public [DashSdkError] hierarchy. Every public SDK entry point
 * that calls an `external fun` goes through this.
 */
inline fun <T> mapNativeErrors(block: () -> T): T =
    try {
        block()
    } catch (e: DashSDKException) {
        throw DashSdkError.fromNative(e)
    }
