package org.dashfoundation.dashsdk.moderation

import java.io.File
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Decoding of what the contract moderation and fee pot natives return, and the list and
 * status bytes they take. Kotlin counterpart of Swift's `ContractModerationDecodingTests`:
 * the payloads are the ones rs-sdk-ffi's `contract_moderation/json.rs` tests pin, and the
 * kept fields fixture is the very file those tests and the Swift tests read. Pure JVM: no
 * native library is loaded.
 */
class ContractModerationModelsTest {

    private val id1 = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi"
    private val id2 = "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR"
    private val id3 = "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8"
    private val id4 = "GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq"
    private val id5 = "LbUiWL3xVV8hTFYBVdbTNrpDo41NKS6o3LHHuDzjfcY"
    private val id6 = "QWmroo4YnnMqYW3cnxWkFdaTxGD3P7vMSzwMHGbUzwF"
    private val id9 = "cGfHiC6Kgg3FpFZvgwGcswsCRtp4aBP2fzuXRQPizuN"
    private val id20 = "2MNus2KCpxwXnp19iyXNpWSFtBD2UGjQBAL8AbtywfT9"
    private val id21 = "2RJD1KnDRGEkvuFfAGrJ7PD28LRE9LRDjZznDywagzmr"
    private val id22 = "2VDW9dFE1ZXz4zWAbaBDQFynNVdRpQ73HyfSHMzBSL6Z"
    private val id30 = "32ZsJ2yJjwuoBiWE5xnZjG9tKmK3CubbmEzgkQLyQzgD"
    private val id31 = "36VASLSKLFD2KokjXG7V28veZvXEsyHRKefLonPaAKzv"

    private fun reason(text: String) = ContractModerationReason(null, text, null, null)

    private val fullReasonJson =
        """{"code":7,"text":"spam","documents":[{"documentTypeName":"post","documentId":"$id5"}],"reasonDocumentId":"$id6"}"""

    @Test
    fun shouldEncodeListsAndStatusesAsTheFfiBytes() {
        assertEquals(listOf(0, 1, 2), ContractModerationList.entries.map { it.ffiTag })
        assertEquals(
            listOf("banlist", "suspensions", "warnings"),
            ContractModerationList.entries.map { it.jsonName },
        )
        assertEquals(listOf(0, 1), ContractTeamActionStatus.entries.map { it.ffiTag })
    }

    @Test
    fun shouldDecodeAStatusOnEveryList() {
        val status = ContractModerationJson.status(
            """{"lists":["banlist","suspensions","warnings"],"banned":true,"banReason":$fullReasonJson,""" +
                """"suspendedUntil":1700000000000,"suspensionReason":{"code":null,"text":"cool off"},""" +
                """"warnings":[{"warnedAt":1600000000000,"reason":{"code":null,"text":"first"}}]}""",
        )
        assertEquals(ContractModerationList.entries.toList(), status.lists)
        assertEquals(true, status.banned)
        assertEquals(
            ContractModerationReason(7, "spam", listOf(ContractModerationDocument("post", id5)), id6),
            status.banReason,
        )
        assertEquals(1_700_000_000_000, status.suspendedUntilMs)
        assertEquals(reason("cool off"), status.suspensionReason)
        assertEquals(listOf(ContractWarning(1_600_000_000_000, reason("first"))), status.warnings)
    }

    /** A list that was not read leaves its fields absent: unknown, not clear. */
    @Test
    fun shouldDecodeAStatusOnSomeListsWithTheOthersAbsent() {
        val banlistOnly = ContractModerationJson.status("""{"lists":["banlist"],"banned":false}""")
        assertEquals(listOf(ContractModerationList.BANLIST), banlistOnly.lists)
        assertEquals(false, banlistOnly.banned)
        assertNull(banlistOnly.suspendedUntilMs)
        assertNull(banlistOnly.warnings)

        val clear = ContractModerationJson.status("""{"lists":["suspensions","warnings"],"warnings":[]}""")
        assertNull(clear.banned)
        assertNull(clear.suspendedUntilMs)
        assertEquals(emptyList<ContractWarning>(), clear.warnings)
    }

    @Test
    fun shouldRefuseAnUnknownList() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            ContractModerationJson.status("""{"lists":["mutes"]}""")
        }
    }

    /** A malformed page is named in the error by its start and length, not copied whole. */
    @Test
    fun shouldQuoteOnlyTheStartOfAMalformedPage() {
        val page = "[" + "1,".repeat(10_000) + "1]"
        val error = assertThrows(DashSdkError.SerializationError::class.java) {
            ContractModerationJson.entriesPage(page)
        }
        val message = error.message.orEmpty()
        assertTrue(message, message.length < 400)
        assertTrue(message, message.endsWith("... (${page.length} chars)"))
    }

    @Test
    fun shouldDecodeAnEntriesPageWithItsCursor() {
        val page = ContractModerationJson.entriesPage(
            """{"entries":[""" +
                """{"identityId":"$id3","until":1700000000000,"reason":{"code":null,"text":"cool off"}},""" +
                """{"identityId":"$id4","until":1700000000000,"reason":{"code":null,"text":"cool off"}}],""" +
                """"nextStartAfter":"$id4"}""",
        )
        assertEquals(listOf(id3, id4), page.entries.map { it.identityId })
        assertEquals(1_700_000_000_000, page.entries[0].untilMs)
        assertNull(page.entries[0].warnings)
        assertEquals(id4, page.nextStartAfter)

        val last = ContractModerationJson.entriesPage("""{"entries":[]}""")
        assertTrue(last.entries.isEmpty())
        assertNull(last.nextStartAfter)
    }

    @Test
    fun shouldDecodeAWarningEntryWithItsWarnings() {
        val entry = ContractModerationJson.entriesPage(
            """{"entries":[{"identityId":"$id3","reason":{"code":null,"text":"second"},""" +
                """"warnings":[{"warnedAt":1,"reason":{"code":null,"text":"first"}},""" +
                """{"warnedAt":2,"reason":{"code":null,"text":"second"}}]}]}""",
        ).entries.single()
        assertNull(entry.untilMs)
        assertEquals(reason("second"), entry.reason)
        assertEquals(listOf(1L, 2L), entry.warnings?.map { it.warnedAtMs })
    }

    @Test
    fun shouldDecodeRemovalsWithKeptFieldsRestorationAndCursor() {
        val hash = "ab".repeat(32)
        val page = ContractModerationJson.removalsPage(
            """{"removals":[""" +
                """{"documentId":"$id3","documentOwnerId":"$id20","moderatorId":"$id21",""" +
                """"reason":{"code":null,"text":"off topic"},"removedAt":1700000000000,""" +
                """"documentHash":"$hash",""" +
                """"keptFields":{"${'$'}createdAt":{"type":"uint","value":1600000000000},""" +
                """"author":{"type":"identifier","value":"$id9"},""" +
                """"hashtag":{"type":"string","value":"dash"},""" +
                """"meta":{"type":"object","value":{"tags":{"type":"array","value":[{"type":"string","value":"privacy"}]}}},""" +
                """"score":{"type":"int","value":-3}}},""" +
                """{"documentId":"$id4","documentOwnerId":"$id20","moderatorId":"$id21",""" +
                """"reason":{"code":null,"text":"off topic"},"removedAt":1700000000000,""" +
                """"documentHash":"$hash","restoredBy":"$id22","restoredAt":1700000100000,""" +
                """"keptFields":{}}],""" +
                """"nextStartAfter":"$id4"}""",
        )
        val removed = page.removals[0]
        val restored = page.removals[1]
        assertEquals(id3, removed.documentId)
        assertEquals(id20, removed.documentOwnerId)
        assertEquals(id21, removed.moderatorId)
        assertEquals(1_700_000_000_000, removed.removedAtMs)
        assertEquals(hash, removed.documentHash)
        assertNull(removed.restoredBy)
        assertNull(removed.restoredAtMs)
        assertEquals(
            mapOf(
                "\$createdAt" to ContractKeptFieldValue.UInt(1_600_000_000_000u),
                "author" to ContractKeptFieldValue.Identifier(id9),
                "hashtag" to ContractKeptFieldValue.Text("dash"),
                "meta" to ContractKeptFieldValue.Object(
                    mapOf(
                        "tags" to ContractKeptFieldValue.Array(
                            listOf(ContractKeptFieldValue.Text("privacy")),
                        ),
                    ),
                ),
                "score" to ContractKeptFieldValue.Int(-3),
            ),
            removed.keptFields,
        )
        assertEquals(id22, restored.restoredBy)
        assertEquals(1_700_000_100_000, restored.restoredAtMs)
        assertTrue(restored.keptFields.isEmpty())
        assertEquals(id4, page.nextStartAfter)
    }

    /**
     * The kept values the FFI's writer tests produce, read from the one fixture file
     * rs-sdk-ffi and the Swift tests share. A missing file fails the test rather than
     * skipping it.
     */
    @Test
    fun shouldDecodeTheSharedKeptFieldsFixtureByItsTags() {
        val fixture = keptFieldsFixture()
        val kept = Json.parseToJsonElement(fixture).jsonObject
            .mapValues { (_, value) -> ContractModerationJson.keptValue(value) }

        assertEquals(ContractKeptFieldValue.Bytes("AQID"), kept["bytes"])
        assertEquals(ContractKeptFieldValue.Bool(true), kept["flag"])
        assertEquals(ContractKeptFieldValue.Int(Long.MIN_VALUE), kept["i64min"])
        assertEquals(ContractKeptFieldValue.Identifier(id9), kept["id"])
        assertEquals(
            ContractKeptFieldValue.Int128("-170141183460469231731687303715884105728"),
            kept["int128"],
        )
        assertEquals(ContractKeptFieldValue.UInt(ULong.MAX_VALUE), kept["u64max"])
        assertEquals(
            ContractKeptFieldValue.UInt128("340282366920938463463374607431768211455"),
            kept["uint128"],
        )
        assertEquals(ContractKeptFieldValue.Double(1.0), kept["whole"])
        val negativeZero = kept["negativeZero"] as ContractKeptFieldValue.Double
        assertEquals(-1.0, Math.copySign(1.0, negativeZero.value), 0.0)
        assertEquals(
            ContractKeptFieldValue.Object(
                mapOf(
                    "list" to ContractKeptFieldValue.Array(
                        listOf(
                            ContractKeptFieldValue.UInt(7u),
                            ContractKeptFieldValue.Null,
                            ContractKeptFieldValue.Text("a"),
                        ),
                    ),
                ),
            ),
            kept["nested"],
        )
    }

    /** A signed and an unsigned integer of the same magnitude stay apart. */
    @Test
    fun shouldKeepSignedAndUnsignedIntegersApart() {
        val signed = ContractModerationJson.keptValue(Json.parseToJsonElement("""{"type":"int","value":5}"""))
        val unsigned = ContractModerationJson.keptValue(Json.parseToJsonElement("""{"type":"uint","value":5}"""))
        assertEquals(ContractKeptFieldValue.Int(5), signed)
        assertEquals(ContractKeptFieldValue.UInt(5u), unsigned)
        assertNotEquals(signed, unsigned)
    }

    @Test
    fun shouldRefuseAnUnknownKeptValueTag() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            ContractModerationJson.keptValue(Json.parseToJsonElement("""{"type":"float","value":1}"""))
        }
    }

    @Test
    fun shouldDecodeATeamActionsPageWithItsCursor() {
        val page = ContractModerationJson.teamActionsPage(
            """{"actions":[""" +
                """{"actionId":"$id3","proposerId":"$id30","proposedAt":1000,""" +
                """"event":{"type":"deleteSettledDocument","documentTypeName":"post","documentId":"$id31",""" +
                """"documentLastModifiedAt":10,"documentRevision":4,"reason":{"code":null,"text":"spam"}},""" +
                """"approvalCount":2},""" +
                """{"actionId":"$id4","proposerId":"$id30","proposedAt":1000,""" +
                """"event":{"type":"deleteSettledDocument","documentTypeName":"post","documentId":"$id31",""" +
                """"documentLastModifiedAt":10,"reason":{"code":null,"text":"spam"}},""" +
                """"approvalCount":2}],""" +
                """"nextStartAtActionId":"$id4","nextStartAtActionIdIncluded":false}""",
        )
        assertEquals(listOf(id3, id4), page.actions.map { it.actionId })
        assertEquals(id30, page.actions[0].proposerId)
        assertEquals(1_000L, page.actions[0].proposedAtMs)
        assertEquals(2L, page.actions[0].approvalCount)
        assertEquals(
            ContractTeamActionEvent.DeleteSettledDocument("post", id31, 10, 4, reason("spam")),
            page.actions[0].event,
        )
        assertNull((page.actions[1].event as ContractTeamActionEvent.DeleteSettledDocument).documentRevision)
        assertEquals(id4, page.nextStartAtActionId)
        assertEquals(false, page.nextStartAtActionIdIncluded)
    }

    @Test
    fun shouldRefuseAnUnknownTeamActionEvent() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            ContractModerationJson.teamActionsPage(
                """{"actions":[{"actionId":"$id3","proposerId":"$id30","proposedAt":1,""" +
                    """"event":{"type":"mint","documentId":"x"},"approvalCount":1}]}""",
            )
        }
    }

    @Test
    fun shouldDecodeSignersAndActionCounts() {
        assertEquals(
            listOf(id1, id2),
            ContractModerationJson.teamActionSigners("""{"signerIds":["$id1","$id2"]}""").signerIds,
        )
        assertTrue(ContractModerationJson.teamActionSigners("""{"signerIds":[]}""").signerIds.isEmpty())
        assertEquals(
            listOf(
                ContractModerationActionCount(id1, 3),
                ContractModerationActionCount(id2, 5),
            ),
            ContractModerationJson.actionCounts(
                """{"counts":[{"identityId":"$id1","count":3},{"identityId":"$id2","count":5}]}""",
            ).counts,
        )
    }

    /** A pot never paid out has no last claim; one paid out in epoch 0 has one. Credits use the whole u64 range. */
    @Test
    fun shouldDecodeFeePotsWithALastClaimOnlyWhenPaidOut() {
        val pots = ContractModerationJson.feePots(
            """{"owner":{"credits":10000000},""" +
                """"moderators":{"credits":18446744073709551615,"lastClaimEpoch":0,""" +
                """"lastClaimTimeMs":1700000000000,"lastClaimantId":"$id9"}}""",
        )
        assertEquals(10_000_000uL, pots.owner.credits)
        assertNull(pots.owner.lastClaimEpoch)
        assertNull(pots.owner.lastClaimTimeMs)
        assertEquals(ULong.MAX_VALUE, pots.moderators.credits)
        assertEquals(0, pots.moderators.lastClaimEpoch)
        assertEquals(1_700_000_000_000, pots.moderators.lastClaimTimeMs)
        assertEquals(id9, pots.moderators.lastClaimantId)
    }

    /** The shared fixture, found by walking up from the test's working directory to the repository. */
    private fun keptFieldsFixture(): String {
        val relative = "packages/swift-sdk/SwiftTests/SwiftDashSDKTests/Fixtures/ContractModeration/kept_fields.json"
        var dir: File? = File(System.getProperty("user.dir")).absoluteFile
        while (dir != null) {
            val candidate = File(dir, relative)
            if (candidate.isFile) return candidate.readText().trim()
            dir = dir.parentFile
        }
        throw AssertionError("the kept fields fixture $relative was not found")
    }
}
