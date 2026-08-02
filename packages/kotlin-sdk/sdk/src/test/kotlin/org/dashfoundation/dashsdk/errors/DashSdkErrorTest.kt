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
    fun unknownNativeCodesFallBackToInternalError() {
        // A code in the rs-sdk-ffi range (< the platform-wallet offset) with
        // no dedicated mapping stays an InternalError.
        val mapped = DashSdkError.fromNative(DashSDKException(234, "?"))
        assertTrue(mapped is DashSdkError.InternalError)
    }

    @Test
    fun retryabilityFollowsErrorClass() {
        assertTrue(DashSdkError.NetworkError("x").isRetryable)
        assertTrue(DashSdkError.Timeout("x").isRetryable)
        assertFalse(DashSdkError.InvalidParameter("x").isRetryable)
    }

    @Test
    fun platformWalletInvalidParameterIsTypedAndPreservesTheCoreMessage() {
        val offset = DashSdkError.PLATFORM_WALLET_CODE_OFFSET
        val coreMessage = "rust-owned txMetadata validation detail"

        val mapped = runCatching {
            mapNativeErrors<Unit> { throw DashSDKException(offset + 2, coreMessage) }
        }.exceptionOrNull()

        assertTrue(mapped is DashSdkError.PlatformWallet.InvalidParameter)
        assertEquals(coreMessage, mapped?.message)
    }

    /**
     * Adding a narrower type must not break callers that already branch on the
     * generic platform-wallet error and inspect its native code.
     */
    @Test
    fun platformWalletInvalidParameterRemainsCompatibleWithTheGenericFallback() {
        val offset = DashSdkError.PLATFORM_WALLET_CODE_OFFSET
        val mapped = DashSdkError.fromNative(DashSDKException(offset + 2, "invalid input"))

        assertTrue(mapped is DashSdkError.PlatformWallet.InvalidParameter)
        assertTrue(mapped is DashSdkError.PlatformWallet.Generic)
        assertEquals(2, (mapped as DashSdkError.PlatformWallet.Generic).nativeCode)
    }

    @Test
    fun platformWalletCodesMapToPlatformWalletSubtree() {
        val offset = DashSdkError.PLATFORM_WALLET_CODE_OFFSET
        // Retry-semantics-bearing + typed platform-wallet codes.
        val invalidHandle = DashSdkError.fromNative(DashSDKException(offset + 1, "bad handle"))
        assertTrue(invalidHandle is DashSdkError.PlatformWallet.InvalidHandle)

        val walletOp = DashSdkError.fromNative(DashSDKException(offset + 6, "op failed"))
        assertTrue(walletOp is DashSdkError.PlatformWallet.WalletOperation)
        // Distinct from the rs-sdk-ffi CryptoError that shares raw code 6.
        assertFalse(walletOp is DashSdkError.CryptoError)

        listOf(7, 8, 98).forEach { code ->
            val notFound = DashSdkError.fromNative(DashSDKException(offset + code, "missing"))
            assertTrue("platform-wallet code $code must be typed NotFound", notFound is DashSdkError.NotFound)
            assertEquals("missing", notFound.message)
        }

        val noAnchor = DashSdkError.fromNative(DashSDKException(offset + 19, "mid-block tree"))
        assertTrue(noAnchor is DashSdkError.PlatformWallet.ShieldedNoRecordedAnchor)
        assertTrue("ShieldedNoRecordedAnchor is retryable", noAnchor.isRetryable)

        val spendUnconfirmed =
            DashSdkError.fromNative(DashSDKException(offset + 18, "ambiguous spend"))
        assertTrue(spendUnconfirmed is DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed)
        assertFalse(
            "ShieldedSpendUnconfirmed must NOT be retryable (notes stay reserved)",
            spendUnconfirmed.isRetryable,
        )
        // The message must warn against retrying, like the broadcast sibling.
        assertTrue(spendUnconfirmed.message!!.contains("do NOT retry"))

        val broadcastUnconfirmed =
            DashSdkError.fromNative(DashSDKException(offset + 20, "ambiguous broadcast"))
        assertTrue(
            broadcastUnconfirmed is DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed,
        )

        val coreInsufficientFunds =
            DashSdkError.fromNative(DashSDKException(offset + 22, "inputs reserved"))
        assertTrue(coreInsufficientFunds is DashSdkError.PlatformWallet.CoreInsufficientFunds)

        val recoveryCodes = mapOf(
            23 to DashSdkError.PlatformWallet.AssetLockNotTracked::class,
            24 to DashSdkError.PlatformWallet.AssetLockAlreadyConsumed::class,
            25 to DashSdkError.PlatformWallet.AssetLockFundingMismatch::class,
        )
        recoveryCodes.forEach { (code, expected) ->
            val mapped = DashSdkError.fromNative(DashSDKException(offset + code, "recovery"))
            assertEquals("platform-wallet code $code", expected, mapped::class)
        }
        assertFalse(
            "TransactionBroadcastUnconfirmed must NOT be retryable",
            broadcastUnconfirmed.isRetryable,
        )
        // The message must warn against retrying (distinct from the anchor case).
        assertTrue(broadcastUnconfirmed.message!!.contains("do NOT retry"))
    }

    @Test
    fun unmappedPlatformWalletCodesFallBackToGeneric() {
        val offset = DashSdkError.PLATFORM_WALLET_CODE_OFFSET
        // ErrorUnknown = 99 has no dedicated type → Generic carrying the code.
        val mapped = DashSdkError.fromNative(DashSDKException(offset + 99, "boom"))
        assertTrue(mapped is DashSdkError.PlatformWallet.Generic)
        assertEquals(99, (mapped as DashSdkError.PlatformWallet.Generic).nativeCode)
        assertEquals("boom", mapped.message)
        assertFalse("Generic platform-wallet errors are not retryable", mapped.isRetryable)
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

    @Test
    fun platformWalletNotFoundConvertsAtThePublicBoundary() {
        val error = runCatching {
            mapNativeErrors {
                throw DashSDKException(
                    DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 98,
                    "wallet not found",
                )
            }
        }.exceptionOrNull()

        assertTrue(error is DashSdkError.NotFound)
        assertEquals("wallet not found", error?.message)
    }
}
