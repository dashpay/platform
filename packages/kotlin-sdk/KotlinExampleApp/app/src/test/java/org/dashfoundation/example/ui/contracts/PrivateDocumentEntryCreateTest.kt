package org.dashfoundation.example.ui.contracts

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.PrivateDocumentSenderAuth
import org.dashfoundation.example.util.Base58
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The create screen's own logic (TEST_PLAN DOC-19): how an outcome is shown, when Create may be
 * tapped, and how the typed recipient is read. Everything else the screen shows is Rust's.
 */
class PrivateDocumentEntryCreateTest {

    private fun inputs(recipientText: String = "abc", field: String = "hi") = PrivateEntryCreateInputs(
        walletHex = "aa",
        senderHex = "bb",
        recipientText = recipientText,
        recipientKeyId = 1L,
        auth = PrivateDocumentSenderAuth.ANONYMOUS,
        fields = listOf(mapOf("note" to field), emptyMap<String, Boolean>(), emptySet<String>(), emptyMap<String, List<String>>()),
    )

    @Test
    fun shouldShowACreateThatMayHaveGoneThroughAsUnconfirmedNeverAsFailed() {
        for (error in listOf(
            DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed("not proved"),
            DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed("no result"),
            DashSdkError.StateTransitionResultUnconfirmed("no result"),
        )) {
            assertEquals(
                PrivateEntryCreateOutcome.Unconfirmed(PrivateEntryCreateOutcome.UNCONFIRMED_MESSAGE),
                PrivateEntryCreateOutcome.of(error),
            )
        }
    }

    @Test
    fun shouldShowANetworkBeforeProtocolVersion15AsNotActive() {
        assertEquals(
            PrivateEntryCreateOutcome.NotActive,
            PrivateEntryCreateOutcome.of(DashSdkError.PlatformWallet.ProtocolVersionNotActive("below 15")),
        )
    }

    @Test
    fun shouldShowARefusalVerbatim() {
        assertEquals(
            PrivateEntryCreateOutcome.Failed("the properties do not satisfy \"note\""),
            PrivateEntryCreateOutcome.of(DashSdkError.InvalidParameter("the properties do not satisfy \"note\"")),
        )
    }

    @Test
    fun shouldHoldCreateWhileTheFormIsTheOneThatMayHaveGoneThrough() {
        val submitted = inputs()
        fun canSubmit(now: PrivateEntryCreateInputs, held: PrivateEntryCreateInputs?) =
            canSubmitPrivateEntryCreate(
                walletLoaded = true,
                hasSender = true,
                keysApply = true,
                hasRecipientKey = true,
                inputs = now,
                heldInputs = held,
            )
        assertTrue(canSubmit(submitted, held = null))
        assertFalse(canSubmit(inputs(), held = submitted))
        assertTrue(canSubmit(inputs(field = "other"), held = submitted))
        assertTrue(canSubmit(inputs(recipientText = "abd"), held = submitted))
    }

    @Test
    fun shouldNotOfferCreateWithoutAWalletSenderOrRecipientKey() {
        val now = inputs()
        assertFalse(canSubmitPrivateEntryCreate(false, true, true, true, now, null))
        assertFalse(canSubmitPrivateEntryCreate(true, false, true, true, now, null))
        assertFalse(canSubmitPrivateEntryCreate(true, true, false, true, now, null))
        assertFalse(canSubmitPrivateEntryCreate(true, true, true, false, now, null))
    }

    @Test
    fun shouldReadTheRecipientAsBase58OrHex() {
        val id = ByteArray(32) { it.toByte() }
        assertArrayEquals(id, parsePrivateEntryRecipientId(Base58.encode(id)))
        assertArrayEquals(id, parsePrivateEntryRecipientId(id.joinToString("") { "%02x".format(it) }))
        assertArrayEquals(id, parsePrivateEntryRecipientId("  ${Base58.encode(id)} "))
        assertNull(parsePrivateEntryRecipientId(""))
        assertNull(parsePrivateEntryRecipientId("not an id"))
        assertNull(parsePrivateEntryRecipientId(Base58.encode(ByteArray(31) { 1 })))
    }
}
