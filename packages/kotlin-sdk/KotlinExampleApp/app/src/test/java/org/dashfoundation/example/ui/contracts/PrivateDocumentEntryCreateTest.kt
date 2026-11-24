package org.dashfoundation.example.ui.contracts

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.PrivateDocumentRecipientKey
import org.dashfoundation.dashsdk.wallet.PrivateDocumentSenderAuth
import org.dashfoundation.example.util.Base58
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * The create screen's own logic (TEST_PLAN DOC-19): how an outcome is shown, what a tap sends
 * (judged on the live form), how the typed recipient is read and that strings go as typed.
 * Everything else the screen shows is Rust's.
 */
class PrivateDocumentEntryCreateTest {

    private val recipient = ByteArray(32) { it.toByte() }
    private val keys = listOf(
        PrivateDocumentRecipientKey(keyId = 1, purpose = 1),
        PrivateDocumentRecipientKey(keyId = 2, purpose = 2),
    )

    private fun request(
        recipientText: String = Base58.encode(recipient),
        keysFor: ByteArray? = recipient,
        keyId: Long? = 1L,
        wallet: String? = "AA",
        sender: String? = "bb",
    ) = privateEntryCreateRequest(
        walletIdHex = wallet,
        senderIdentityIdHex = sender,
        contractIdHex = "CC",
        documentTypeName = "note",
        recipientText = recipientText,
        recipientKeysFor = keysFor,
        recipientKeys = keys,
        recipientKeyId = keyId,
        auth = PrivateDocumentSenderAuth.ANONYMOUS,
        propertiesJson = "{}",
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
    fun shouldBuildTheSameRequestForEverySpellingOfTheRecipient() {
        val base58 = request()
        val hex = request(recipientText = recipient.joinToString("") { "%02x".format(it) })
        val padded = request(recipientText = "  ${Base58.encode(recipient)} ")
        assertEquals(base58, hex)
        assertEquals(base58, padded)
        assertEquals("aa", base58?.walletIdHex)
        assertEquals("cc", base58?.contractIdHex)
        assertEquals(recipient.joinToString("") { "%02x".format(it) }, base58?.recipientIdentityIdHex)
    }

    @Test
    fun shouldBuildNoRequestForARecipientOtherThanTheOneWhoseKeysWereLoaded() {
        // The field was edited after the keys were loaded: the old keys never go with the new text
        assertNull(request(recipientText = Base58.encode(ByteArray(32) { 9 })))
        assertNull(request(keysFor = null))
        assertNull(request(recipientText = "not an id"))
    }

    @Test
    fun shouldBuildNoRequestWithoutAWalletASenderOrALoadedKey() {
        assertNull(request(wallet = null))
        assertNull(request(sender = null))
        assertNull(request(keyId = null))
        assertNull(request(keyId = 7L))
    }

    @Test
    fun shouldSendAStringPropertyExactlyAsTyped() {
        val properties = Json.parseToJsonElement(
            """{"note":{"type":"string"},"tag":{"type":"string"},"empty":{"type":"string"}}""",
        ).jsonObject
        val json = buildPropertiesJson(
            properties = properties,
            required = emptySet(),
            textValues = mapOf("note" to "  hello  ", "tag" to "   ", "empty" to ""),
            boolValues = emptyMap(),
            touchedBools = emptySet(),
            trimStrings = false,
        )
        assertEquals("""{"note":"  hello  ","tag":"   "}""", json)
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
