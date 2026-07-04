package org.dashfoundation.dashsdk.errors

import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DashSdkErrorTest {

    @Test
    fun mapsNativeCodesToHierarchy() {
        // Codes mirror DashSDKErrorCode in rs-sdk-ffi/src/error.rs.
        val cases = mapOf(
            1 to DashSdkError.InvalidParameter::class,
            2 to DashSdkError.InvalidState::class,
            3 to DashSdkError.NetworkError::class,
            4 to DashSdkError.SerializationError::class,
            5 to DashSdkError.ProtocolError::class,
            6 to DashSdkError.CryptoError::class,
            7 to DashSdkError.NotFound::class,
            8 to DashSdkError.Timeout::class,
            9 to DashSdkError.NotImplemented::class,
            10 to DashSdkError.DriveInternalError::class,
            99 to DashSdkError.InternalError::class,
        )
        cases.forEach { (code, expected) ->
            val mapped = DashSdkError.fromNative(DashSDKException(code, "boom"))
            assertEquals("code $code", expected, mapped::class)
            assertEquals("boom", mapped.message)
        }
    }

    @Test
    fun unknownCodesFallBackToInternalError() {
        val mapped = DashSdkError.fromNative(DashSDKException(1234, "?"))
        assertTrue(mapped is DashSdkError.InternalError)
    }

    @Test
    fun retryabilityFollowsErrorClass() {
        assertTrue(DashSdkError.NetworkError("x").isRetryable)
        assertTrue(DashSdkError.Timeout("x").isRetryable)
        assertFalse(DashSdkError.InvalidParameter("x").isRetryable)
    }

    @Test
    fun mapNativeErrorsConvertsAtTheBoundary() {
        try {
            mapNativeErrors { throw DashSDKException(7, "identity not found") }
        } catch (e: DashSdkError) {
            assertTrue(e is DashSdkError.NotFound)
            return
        }
        throw AssertionError("expected DashSdkError")
    }
}
