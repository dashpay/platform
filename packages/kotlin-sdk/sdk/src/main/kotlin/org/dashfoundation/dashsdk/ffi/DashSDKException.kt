package org.dashfoundation.dashsdk.ffi

import org.dashfoundation.dashsdk.errors.ConsensusErrorKind
import org.dashfoundation.dashsdk.errors.PlatformConsensusError

/**
 * Exception thrown by the native layer (`rs-unified-sdk-jni`) when an FFI call
 * returns an error. Constructed from Rust via JNI: the `(Int, String)` and
 * `(Int, String, Int, Int)` constructor signatures must stay in sync with
 * `rs-unified-sdk-jni/src/support.rs::throw_sdk_exception` and
 * `throw_sdk_consensus_exception`.
 *
 * [code] values mirror `DashSDKErrorCode` in `rs-sdk-ffi/src/error.rs`:
 * 0=Success, 1=InvalidParameter, 2=InvalidState, 3=NetworkError,
 * 4=SerializationError, 5=ProtocolError, 6=CryptoError, 7=NotFound,
 * 8=Timeout, 9=NotImplemented, 10=DriveInternalError, 99=InternalError.
 *
 * [consensusError] is the consensus rejection behind the failure when the
 * native result carried one, `null` otherwise.
 *
 * Internal: the public API maps this into the
 * [org.dashfoundation.dashsdk.errors.DashSdkError] hierarchy.
 */
class DashSDKException(
    val code: Int,
    message: String,
    val consensusError: PlatformConsensusError?,
) : Exception(message) {
    constructor(code: Int, message: String) : this(code, message, null)

    /**
     * JNI entry for a failure that is a consensus rejection. [consensusKind]
     * is the `PlatformWalletFFIConsensusErrorKind` discriminant.
     */
    constructor(code: Int, message: String, consensusCode: Int, consensusKind: Int) : this(
        code,
        message,
        PlatformConsensusError(consensusCode, ConsensusErrorKind.fromNative(consensusKind)),
    )
}
