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
    fun platformWalletCodesMapToPlatformWalletSubtree() {
        val offset = DashSdkError.PLATFORM_WALLET_CODE_OFFSET
        // Retry-semantics-bearing + typed platform-wallet codes.
        val invalidHandle = DashSdkError.fromNative(DashSDKException(offset + 1, "bad handle"))
        assertTrue(invalidHandle is DashSdkError.PlatformWallet.InvalidHandle)

        val walletOp = DashSdkError.fromNative(DashSDKException(offset + 6, "op failed"))
        assertTrue(walletOp is DashSdkError.PlatformWallet.WalletOperation)
        // Distinct from the rs-sdk-ffi CryptoError that shares raw code 6.
        assertFalse(walletOp is DashSdkError.CryptoError)

        listOf(7, 8).forEach { code ->
            val notFound = DashSdkError.fromNative(DashSDKException(offset + code, "missing"))
            assertTrue("platform-wallet code $code must be typed NotFound", notFound is DashSdkError.NotFound)
            assertEquals("missing", notFound.message)
        }

        // 98 (PlatformWalletFFIResultCode::NotFound, the blanket Option → result
        // miss) stays in the PlatformWallet subtree as the typed
        // PlatformWallet.NotFound — parity with Swift's PlatformWalletError
        // .notFound (also in the wallet-error family) — distinct from the typed
        // top-level NotFound that 7/8 map to. Local reads still recognize it at
        // the raw code via translateManagedIdentityNotFoundToZero (#4051) before
        // this mapping runs.
        val optionMiss = DashSdkError.fromNative(DashSDKException(offset + 98, "missing"))
        assertTrue(optionMiss is DashSdkError.PlatformWallet.NotFound)
        assertFalse(optionMiss is DashSdkError.NotFound)
        assertEquals("missing", optionMiss.message)

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

        // Code 43, NOT 37 (v4.2-dev's ErrorDocumentNotForSale) and NOT 32
        // (ErrorTransactionBuild — dashpay/platform#4247/#4256). This
        // assertion is the mirror's guard against the collision — if the Rust
        // discriminant is ever moved back onto a claimed number, the host
        // silently reclassifies an already-claimed invite as some other
        // branch's error. 43 matches the integration-branch allocation
        // already shipped in QA AARs, so it is frozen.
        val inviteClaimed =
            DashSdkError.fromNative(DashSDKException(offset + 43, "nullifier already spent"))
        assertTrue(inviteClaimed is DashSdkError.PlatformWallet.ShieldedInviteAlreadyClaimed)
        assertFalse(
            "ShieldedInviteAlreadyClaimed is TERMINAL — the note is consumed",
            inviteClaimed.isRetryable,
        )

        // Code 44 (ErrorShieldedScanBudgetExhausted, dashpay/platform#4306):
        // the claim scan paused at its per-attempt budget. The retryability
        // polarity is the entire contract — a host that saw this as terminal
        // would strand a funded claim whose note sits deep in the tree.
        val scanBudget =
            DashSdkError.fromNative(DashSDKException(offset + 44, "scan paused at 262144"))
        assertTrue(scanBudget is DashSdkError.PlatformWallet.ShieldedScanBudgetExhausted)
        assertTrue(
            "ShieldedScanBudgetExhausted is RETRYABLE — progress is checkpointed",
            scanBudget.isRetryable,
        )

        // Code 45 (ErrorShieldedLifecycleBusy, dashpay/platform#4313): the
        // claim was refused admission at the store — a concurrent Clear /
        // wallet removal holds it, or another claimant already holds this
        // invitation's claim-record key. Nothing was scanned, built or
        // broadcast, so the polarity matches 44's: a host that read this as
        // terminal would fail an invitation that is merely contended.
        val lifecycleBusy =
            DashSdkError.fromNative(
                DashSDKException(offset + 45, "shielded state is being cleared or removed"),
            )
        assertTrue(lifecycleBusy is DashSdkError.PlatformWallet.ShieldedLifecycleBusy)
        assertTrue(
            "ShieldedLifecycleBusy is RETRYABLE — nothing was consumed",
            lifecycleBusy.isRetryable,
        )

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

        // Definitive broadcast rejection (26) must reach callers as its own type,
        // NOT as Generic: it is the definitive counterpart to the ambiguous
        // TransactionBroadcastUnconfirmed (20), and on the deferred path the
        // reservation was released and the token consumed — so it is not
        // retryable in place, it must be rebuilt.
        val rejected = DashSdkError.fromNative(DashSDKException(offset + 26, "bad-txns-inputs-spent"))
        assertTrue(
            "code 26 must not fall through to Generic",
            rejected is DashSdkError.PlatformWallet.TransactionBroadcastRejected,
        )
        assertFalse(
            "TransactionBroadcastRejected must NOT be retryable in place (rebuild the payment)",
            rejected.isRetryable,
        )
        assertEquals("bad-txns-inputs-spent", rejected.message)

        // Deferred build/broadcast: the three sibling reservation-token failures
        // map to three distinct typed errors, none retryable.
        val agedOut = DashSdkError.fromNative(DashSDKException(offset + 34, "stale token 7"))
        assertTrue(agedOut is DashSdkError.PlatformWallet.StaleReservationToken)
        assertFalse(
            "StaleReservationToken must NOT be retryable (rebuild the payment)",
            agedOut.isRetryable,
        )
        assertEquals("stale token 7", agedOut.message)

        val consumed = DashSdkError.fromNative(DashSDKException(offset + 35, "already broadcast"))
        assertTrue(consumed is DashSdkError.PlatformWallet.ReservationTokenConsumed)
        assertFalse(
            "ReservationTokenConsumed must NOT be retryable (rebuild the payment)",
            consumed.isRetryable,
        )
        assertEquals("already broadcast", consumed.message)

        val walletMismatch =
            DashSdkError.fromNative(DashSDKException(offset + 36, "different generation"))
        assertTrue(walletMismatch is DashSdkError.PlatformWallet.ReservationWalletMismatch)
        assertFalse(
            "ReservationWalletMismatch must NOT be retryable (rebuild the payment)",
            walletMismatch.isRetryable,
        )
        assertEquals("different generation", walletMismatch.message)
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
    fun platformShieldCapacityCode41MapsTyped() {
        val message =
            "Platform shield capacity exceeded: available 3623849220, required 3623849221"
        val mapped = DashSdkError.fromNative(
            DashSDKException(
                DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 41,
                message,
            ),
        )

        assertTrue(mapped is DashSdkError.PlatformWallet.PlatformShieldCapacityExceeded)
        assertEquals(message, mapped.message)
        assertFalse(
            "unchanged amount must not be retried without refreshing preflight",
            mapped.isRetryable,
        )
    }

    @Test
    fun signingKeyUnavailableCode31MapsTyped() {
        // The STRUCTURED discriminator (dashpay/platform#4060 finding 7):
        // PlatformWalletFFIResultCode::ErrorSigningKeyUnavailable (31) maps
        // to the typed error on the code alone — no message inspection.
        val offset = DashSdkError.PLATFORM_WALLET_CODE_OFFSET
        val mapped = DashSdkError.fromNative(
            DashSDKException(offset + 31, "arbitrary human text, no marker"),
        )
        assertTrue(mapped is DashSdkError.PlatformWallet.SigningKeyUnavailable)
        assertFalse(mapped.isRetryable)
    }

    @Test
    fun signingKeyUnavailableIsRecognizedByItsMessageMarker() {
        val offset = DashSdkError.PLATFORM_WALLET_CODE_OFFSET
        val marker = DashSdkError.PlatformWallet.SigningKeyUnavailable.MESSAGE_MARKER
        // DEPRECATED fallback for the #4191 merge-order transition: at
        // #4191's revision the completion error travels as free text under
        // the catch-all codes (ErrorUnknown = 99 via the blanket
        // PlatformWalletError conversion, sometimes wrapped as
        // ErrorWalletOperation = 6) — both must keep surfacing typed until
        // the fallback's removal (#4052, #4060 finding 7). Mixed
        // old-native/new-Kotlin artifacts are unsupported (JNI arity
        // change), so that pairing is NOT what this covers.
        for (code in intArrayOf(6, 99)) {
            val mapped = DashSdkError.fromNative(
                DashSDKException(offset + code, "Signing failed: $marker deadbeef00112233…"),
            )
            assertTrue(
                "code $code with marker → SigningKeyUnavailable",
                mapped is DashSdkError.PlatformWallet.SigningKeyUnavailable,
            )
            assertFalse(mapped.isRetryable)
        }
        // Without the marker the catch-all mappings are untouched.
        val walletOp = DashSdkError.fromNative(DashSDKException(offset + 6, "op failed"))
        assertTrue(walletOp is DashSdkError.PlatformWallet.WalletOperation)
        val generic = DashSdkError.fromNative(DashSDKException(offset + 99, "boom"))
        assertTrue(generic is DashSdkError.PlatformWallet.Generic)
    }

    @Test
    fun signingKeyMarkerNeverOverridesRetrySemanticsCodes() {
        val offset = DashSdkError.PLATFORM_WALLET_CODE_OFFSET
        val marker = DashSdkError.PlatformWallet.SigningKeyUnavailable.MESSAGE_MARKER
        // A dedicated retry-semantics code keeps its type even if the Rust
        // message happens to embed the marker text.
        val mapped = DashSdkError.fromNative(
            DashSDKException(offset + 19, "anchor missing; $marker something"),
        )
        assertTrue(mapped is DashSdkError.PlatformWallet.ShieldedNoRecordedAnchor)
    }

    @Test
    fun platformWalletNotFoundCodeMapsToTypedWalletNotFound() {
        // PlatformWalletFFIResultCode::NotFound (98) — the code the Option →
        // result conversion emits for "requested <thing> not found". Maps to
        // the typed PlatformWallet.NotFound (Swift parity:
        // PlatformWalletError.notFound); Dashpay's managed-identity reads
        // translate the raw code to null before it ever escapes (#4051).
        val mapped = DashSdkError.fromNative(
            DashSDKException(
                DashSdkError.PLATFORM_WALLET_CODE_OFFSET +
                    DashSdkError.PLATFORM_WALLET_NOT_FOUND_CODE,
                "requested platform_wallet::identity::ManagedIdentity not found",
            ),
        )
        assertTrue(mapped is DashSdkError.PlatformWallet.NotFound)
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

        // Code 98 surfaces (through the public mapNativeErrors boundary) as the
        // typed PlatformWallet.NotFound — the wallet-family NotFound, exactly
        // how Swift surfaces PlatformWalletError.notFound, and NOT the
        // top-level NotFound reserved for rs-sdk-ffi codes 7/8 — so #4051's
        // raw-code translation stays the single place that turns an
        // unmanaged-identity miss into an absence.
        assertTrue(error is DashSdkError.PlatformWallet.NotFound)
        assertFalse(error is DashSdkError.NotFound)
        assertEquals("wallet not found", (error as DashSdkError).message)
    }
}
