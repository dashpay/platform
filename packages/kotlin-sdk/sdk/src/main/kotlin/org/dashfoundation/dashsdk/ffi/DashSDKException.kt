package org.dashfoundation.dashsdk.ffi

import org.dashfoundation.dashsdk.errors.ConsensusErrorKind
import org.dashfoundation.dashsdk.errors.PaymentDelivery
import org.dashfoundation.dashsdk.errors.PlatformConsensusError

/**
 * Exception thrown by the native layer (`rs-unified-sdk-jni`) when an FFI call
 * returns an error. Constructed from Rust via JNI: the `(Int, String)`,
 * `(Int, String, Int, Int)` and `(Int, String, Int, Int, Int)` constructor
 * signatures must stay in sync with
 * `rs-unified-sdk-jni/src/support.rs::throw_sdk_exception`,
 * `throw_sdk_consensus_exception` and `throw_pwffi_result`.
 *
 * [code] values mirror `DashSDKErrorCode` in `rs-sdk-ffi/src/error.rs`:
 * 0=Success, 1=InvalidParameter, 2=InvalidState, 3=NetworkError,
 * 4=SerializationError, 5=ProtocolError, 6=CryptoError, 7=NotFound,
 * 8=Timeout, 9=NotImplemented, 10=DriveInternalError,
 * 11=StateTransitionResultUnconfirmed, 99=InternalError.
 *
 * [consensusError] is the consensus rejection behind the failure when the
 * native result or `DashSDKError` carried one, `null` otherwise.
 * [paymentDelivery] is whether a failed payment may have left the wallet, as
 * a platform-wallet result classified it; `null` for an rs-sdk-ffi error.
 *
 * Internal: the public API maps this into the
 * [org.dashfoundation.dashsdk.errors.DashSdkError] hierarchy.
 */
class DashSDKException(
    val code: Int,
    message: String,
    val consensusError: PlatformConsensusError?,
    val paymentDelivery: PaymentDelivery? = null,
) : Exception(message) {
    constructor(code: Int, message: String) : this(code, message, null)

    /**
     * JNI entry for a failure that is a consensus rejection. [consensusKind]
     * is the discriminant of the native kind: `PlatformWalletFFIConsensusErrorKind`
     * for a platform-wallet result, `DashSDKConsensusErrorKind` for an
     * rs-sdk-ffi error. The two share their values.
     */
    constructor(code: Int, message: String, consensusCode: Int, consensusKind: Int) : this(
        code,
        message,
        PlatformConsensusError(consensusCode, ConsensusErrorKind.fromNative(consensusKind)),
    )

    /**
     * JNI entry for a platform-wallet result: its consensus rejection
     * ([consensusCode] `0` when it carries none) and [paymentDelivery], the
     * `PlatformWalletFFIPaymentDelivery` discriminant.
     */
    constructor(
        code: Int,
        message: String,
        consensusCode: Int,
        consensusKind: Int,
        paymentDelivery: Int,
    ) : this(
        code,
        message,
        if (consensusCode == 0) {
            null
        } else {
            PlatformConsensusError(consensusCode, ConsensusErrorKind.fromNative(consensusKind))
        },
        PaymentDelivery.fromNative(paymentDelivery),
    )
}
