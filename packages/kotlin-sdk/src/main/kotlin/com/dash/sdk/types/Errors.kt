package com.dash.sdk.types

/**
 * Base exception for all SDK errors
 */
sealed class DashSDKException(message: String, cause: Throwable? = null) : Exception(message, cause) {
    /**
     * Invalid parameter passed to function
     */
    class InvalidParameterException(message: String) : DashSDKException(message)

    /**
     * SDK not initialized or in invalid state
     */
    class InvalidStateException(message: String) : DashSDKException(message)

    /**
     * Network error occurred
     */
    class NetworkException(message: String, cause: Throwable? = null) : DashSDKException(message, cause)

    /**
     * Serialization/deserialization error
     */
    class SerializationException(message: String, cause: Throwable? = null) : DashSDKException(message, cause)

    /**
     * Platform protocol error
     */
    class ProtocolException(message: String) : DashSDKException(message)

    /**
     * Cryptographic operation failed
     */
    class CryptoException(message: String) : DashSDKException(message)

    /**
     * Resource not found
     */
    class NotFoundException(message: String) : DashSDKException(message)

    /**
     * Operation timed out
     */
    class TimeoutException(message: String) : DashSDKException(message)

    /**
     * Feature not implemented
     */
    class NotImplementedException(message: String) : DashSDKException(message)

    /**
     * Internal error
     */
    class InternalException(message: String, cause: Throwable? = null) : DashSDKException(message, cause)

    /**
     * Unknown error
     */
    class UnknownException(message: String, cause: Throwable? = null) : DashSDKException(message, cause)
}