package org.dashfoundation.example.ui.identity

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Wording and error routing of [GiveUpNameScreen]. Port of
 * `packages/swift-sdk/SwiftExampleApp/SwiftExampleAppTests/GiveUpNamePresentationTests.swift`.
 * Swift's main-name matching test has no counterpart here: the rule is
 * Rust's (`give_up_clears_main_dpns_name`), tested in platform-wallet and
 * bound in `FfiSmokeTest`.
 */
class GiveUpNamePresentationTest {
    @Test
    fun shouldReadAProtocolVersionRefusalAsNotAvailableWithTheSdkMessage() {
        val message = "broadcast_state_transition needs Platform protocol version 15 for deleting " +
            "or tombstoning a DPNS name; the network is proven at protocol version 14"
        assertEquals(
            GiveUpNamePhase.NotAvailable(message),
            GiveUpNameUi.loadFailure(DashSdkError.PlatformWallet.ProtocolVersionNotActive(message)),
        )
        val submitted = GiveUpNameUi.submitFailure(
            DashSdkError.PlatformWallet.ProtocolVersionNotActive(message),
        )
        assertTrue(submitted is GiveUpNameSubmitFailure.Failed)
        assertTrue(
            (submitted as GiveUpNameSubmitFailure.Failed).message
                .startsWith("Not available on this network yet."),
        )
    }

    @Test
    fun shouldAskForAnAppUpdateNotARetryWhenTheSdkIsPinned() {
        val message = "this SDK is pinned to an earlier protocol version"
        assertEquals(
            GiveUpNamePhase.SdkPinned(message),
            GiveUpNameUi.loadFailure(DashSdkError.PlatformWallet.SdkPinnedBelowProtocolVersion(message)),
        )
    }

    @Test
    fun shouldOfferNothingMoreAfterAnUnconfirmedGiveUp() {
        // The error's own message carries a do-not-retry suffix; the SDK's text leads it
        val submitted = GiveUpNameUi.submitFailure(
            DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed("no result in time"),
        )
        assertTrue(submitted is GiveUpNameSubmitFailure.Unconfirmed)
        assertTrue(
            (submitted as GiveUpNameSubmitFailure.Unconfirmed).message.startsWith("no result in time"),
        )
        // Reopened while the earlier give-up is unresolved: Rust refuses the read the same way
        val loaded = GiveUpNameUi.loadFailure(
            DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed(
                "an earlier give-up is unconfirmed",
            ),
        )
        assertTrue(loaded is GiveUpNamePhase.Unconfirmed)
        assertTrue(
            (loaded as GiveUpNamePhase.Unconfirmed).message
                .startsWith("an earlier give-up is unconfirmed"),
        )
    }

    @Test
    fun shouldShowOtherErrorsForRetry() {
        val load = GiveUpNameUi.loadFailure(DashSdkError.NotFound("DPNS name \"alice\" not found"))
        assertTrue(load is GiveUpNamePhase.Failed)
        assertTrue((load as GiveUpNamePhase.Failed).message.contains("alice"))

        val submit = GiveUpNameUi.submitFailure(
            DashSdkError.PlatformWallet.SigningKeyUnavailable("key 3 is watch-only"),
        )
        assertTrue(submit is GiveUpNameSubmitFailure.Failed)
        assertTrue((submit as GiveUpNameSubmitFailure.Failed).message.contains("watch-only"))
    }

    @Test
    fun shouldStateThatATombstoneIsPermanent() {
        assertTrue(GiveUpNameUi.tombstoneConfirmation("alice").startsWith("This is permanent."))
        assertTrue(GiveUpNameUi.TOMBSTONE_EXPLANATION.contains("nobody can undo it"))
        assertTrue(GiveUpNameUi.DELETE_EXPLANATION.contains("someone else"))
    }
}
