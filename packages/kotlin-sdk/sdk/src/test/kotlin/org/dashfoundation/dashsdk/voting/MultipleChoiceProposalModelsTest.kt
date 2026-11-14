package org.dashfoundation.dashsdk.voting

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Decoding of what the multiple choice proposal natives return, and the encoding of what
 * they take (the vote choice, an edit's changes).
 *
 * Rust writes every value (rs-sdk-ffi's `decision_proposal/multiple_choice/json.rs` tests
 * pin the same shapes); Kotlin only decodes, so these tests pin the JSON as rs-sdk-ffi
 * writes it, key for key. Pure JVM: no native library is loaded.
 */
class MultipleChoiceProposalModelsTest {

    private val proposalId = "8vLzWy3oPYAwJ9W8TjWZ3vSJMy7bDqyuvbMvzQMxVjPN"
    private val ownerId = "CQwyTH5AnQ5YQVGeYhBP8ihLdMKKbS6ydzN2E8wRWMNn"
    private val choiceId = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk"
    private val otherChoiceId = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"
    private val describedId = "7NYmEKQsYtniQRUmxwdPGeVcirMoPh5ZPyAKz8BWFy3r"
    private val describedContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"

    private val proposalJson = """
        {"id":"$proposalId","ownerId":"$ownerId","revision":1,
         "createdAt":1700000000000,"updatedAt":1700000000000,
         "title":"Which block size?",
         "descriptionDocument":{"id":"$describedId","contractId":"$describedContractId","documentType":"post"},
         "url":"https://forum.dash.org/t/block-size","open":true}
    """.trimIndent()

    private val choiceJson = """
        {"id":"$choiceId","ownerId":"$ownerId","proposalId":"$proposalId","revision":null,
         "createdAt":1700000000000,"updatedAt":null,
         "title":"Double it","description":"Twice the size it is today."}
    """.trimIndent()

    private val choice = ProposalChoice(
        id = choiceId,
        ownerId = ownerId,
        proposalId = proposalId,
        revision = null,
        createdAtMs = 1_700_000_000_000,
        updatedAtMs = null,
        title = "Double it",
        description = "Twice the size it is today.",
    )

    @Test
    fun shouldDecodeAProposalWithEveryField() {
        assertEquals(
            MultipleChoiceProposal(
                id = proposalId,
                ownerId = ownerId,
                revision = 1,
                createdAtMs = 1_700_000_000_000,
                updatedAtMs = 1_700_000_000_000,
                title = "Which block size?",
                descriptionDocument = MultipleChoiceProposal.DescriptionDocument(
                    id = describedId,
                    contractId = describedContractId,
                    documentType = "post",
                ),
                url = "https://forum.dash.org/t/block-size",
                open = true,
            ),
            MultipleChoiceProposal.fromJson(proposalJson),
        )
    }

    @Test
    fun shouldDecodeAnAbsentDescriptionDocumentUrlAndSystemFieldsAsNull() {
        val proposal = MultipleChoiceProposal.fromJson(
            """
            {"id":"$proposalId","ownerId":"$ownerId","revision":null,"createdAt":null,
             "updatedAt":null,"title":"t","descriptionDocument":null,"url":null,"open":false}
            """.trimIndent(),
        )
        assertNull(proposal.descriptionDocument)
        assertNull(proposal.url)
        assertNull(proposal.revision)
        assertNull(proposal.createdAtMs)
        assertNull(proposal.updatedAtMs)
        assertEquals(false, proposal.open)
    }

    @Test
    fun shouldDecodeAListingInItsOrder() {
        val second = proposalJson.replace(proposalId, otherChoiceId)
        val listing = MultipleChoiceProposal.listFromJson("[$proposalJson,$second]")
        assertEquals(listOf(proposalId, otherChoiceId), listing.map { it.id })
        assertEquals(emptyList<MultipleChoiceProposal>(), MultipleChoiceProposal.listFromJson("[]"))
    }

    @Test
    fun shouldDecodeAChoiceWithEveryFieldAndAnAbsentDescriptionAsNull() {
        assertEquals(choice, ProposalChoice.fromJson(choiceJson))
        val bare = ProposalChoice.fromJson(
            choiceJson.replace("\"Twice the size it is today.\"", "null"),
        )
        assertNull(bare.description)
        assertNull(bare.revision)
        assertNull(bare.updatedAtMs)
    }

    @Test
    fun shouldDecodeARunningVoteWithItsChoicesAndTallies() {
        val vote = MultipleChoiceProposalVote.fromJson(
            """
            {"choices":[{"id":"$choiceId","choice":$choiceJson,"voteTally":12},
                        {"id":"$otherChoiceId","choice":null,"voteTally":null}],
             "abstainVoteTally":3,"noneOfTheChoicesVoteTally":1,"result":null}
            """.trimIndent(),
        )
        assertEquals(
            MultipleChoiceProposalVote(
                choices = listOf(
                    MultipleChoiceProposalVote.ChoiceInVote(id = choiceId, choice = choice, voteTally = 12),
                    MultipleChoiceProposalVote.ChoiceInVote(id = otherChoiceId, choice = null, voteTally = null),
                ),
                abstainVoteTally = 3,
                noneOfTheChoicesVoteTally = 1,
                result = null,
            ),
            vote,
        )
    }

    @Test
    fun shouldDecodeHowAFinishedVoteEndedForEveryOutcome() {
        val outcomes = listOf(
            Triple("keptChoice", "\"$choiceId\"", MultipleChoiceProposalVote.Outcome.KEPT_CHOICE),
            Triple("noneOfTheChoices", "null", MultipleChoiceProposalVote.Outcome.NONE_OF_THE_CHOICES),
            Triple("noWinner", "null", MultipleChoiceProposalVote.Outcome.NO_WINNER),
        )
        for ((name, keptChoiceId, outcome) in outcomes) {
            val vote = MultipleChoiceProposalVote.fromJson(
                """
                {"choices":[{"id":"$choiceId","choice":null,"voteTally":9}],
                 "abstainVoteTally":null,"noneOfTheChoicesVoteTally":4,
                 "result":{"outcome":"$name","keptChoiceId":$keptChoiceId,
                           "finalizationBlock":{"height":9,"coreHeight":42,"timeMs":9000,"epoch":3}}}
                """.trimIndent(),
            )
            assertEquals(
                name,
                MultipleChoiceProposalVote(
                    choices = listOf(
                        MultipleChoiceProposalVote.ChoiceInVote(id = choiceId, choice = null, voteTally = 9),
                    ),
                    abstainVoteTally = null,
                    noneOfTheChoicesVoteTally = 4,
                    result = MultipleChoiceProposalVote.Result(
                        outcome = outcome,
                        keptChoiceId = choiceId.takeIf { outcome == MultipleChoiceProposalVote.Outcome.KEPT_CHOICE },
                        finalizationBlock = PlatformBlockInfo(
                            height = 9,
                            coreHeight = 42,
                            timeMs = 9_000,
                            epoch = 3,
                        ),
                    ),
                ),
                vote,
            )
        }
        assertEquals(3, MultipleChoiceProposalVote.Outcome.entries.size)
    }

    @Test
    fun shouldDecodeAVoteWithNoChoiceAsEmpty() {
        assertEquals(
            MultipleChoiceProposalVote(
                choices = emptyList(),
                abstainVoteTally = null,
                noneOfTheChoicesVoteTally = null,
                result = null,
            ),
            MultipleChoiceProposalVote.fromJson(
                """{"choices":[],"abstainVoteTally":null,"noneOfTheChoicesVoteTally":null,"result":null}""",
            ),
        )
    }

    @Test
    fun shouldDecodeAMasternodeAnswerForEveryChoice() {
        assertEquals(
            MultipleChoiceVoteChoice.Choice(choiceId),
            MultipleChoiceVoteChoice.fromJson("""{"choice":"choice","choiceId":"$choiceId"}"""),
        )
        assertEquals(
            MultipleChoiceVoteChoice.NoneOfTheChoices,
            MultipleChoiceVoteChoice.fromJson("""{"choice":"noneOfTheChoices","choiceId":null}"""),
        )
        assertEquals(
            MultipleChoiceVoteChoice.Abstain,
            MultipleChoiceVoteChoice.fromJson("""{"choice":"abstain","choiceId":null}"""),
        )
    }

    /** The `vote_choice` byte and `choice_id` of `dash_sdk_multiple_choice_proposal_cast_vote`. */
    @Test
    fun shouldEncodeTheVoteChoiceAsTheFfiByteAndChoiceId() {
        assertEquals(
            NativeMultipleChoiceVote(discriminant = 0, choiceId = choiceId),
            MultipleChoiceVoteChoice.Choice(choiceId).toNative(),
        )
        assertEquals(
            NativeMultipleChoiceVote(discriminant = 1, choiceId = null),
            MultipleChoiceVoteChoice.NoneOfTheChoices.toNative(),
        )
        assertEquals(
            NativeMultipleChoiceVote(discriminant = 2, choiceId = null),
            MultipleChoiceVoteChoice.Abstain.toNative(),
        )
    }

    /** A field and its `remove_` flag of `MultipleChoiceProposalEditFFI`, one shape per change. */
    @Test
    fun shouldEncodeEveryChangeAsTheEditTakesIt() {
        assertEquals(
            NativeChange<String>(value = null, remove = false),
            (MultipleChoiceProposalEdit.Change.Keep as MultipleChoiceProposalEdit.Change<String>).toNative(),
        )
        assertEquals(
            NativeChange(value = "https://forum.dash.org/t/block-size", remove = false),
            MultipleChoiceProposalEdit.Change.Set("https://forum.dash.org/t/block-size").toNative(),
        )
        assertEquals(
            NativeChange<String>(value = null, remove = true),
            (MultipleChoiceProposalEdit.Change.Remove as MultipleChoiceProposalEdit.Change<String>).toNative(),
        )

        val described = DescriptionDocumentParams(ByteArray(32) { 5 }, ByteArray(32) { 6 }, "post")
        assertEquals(
            NativeChange(value = described, remove = false),
            MultipleChoiceProposalEdit.Change.Set(described).toNative(),
        )

        val untouched = MultipleChoiceProposalEdit()
        assertNull(untouched.title)
        assertEquals(MultipleChoiceProposalEdit.Change.Keep, untouched.descriptionDocument)
        assertEquals(MultipleChoiceProposalEdit.Change.Keep, untouched.url)
    }

    /** The input holds byte arrays, so it compares them by content. */
    @Test
    fun shouldCompareADescriptionDocumentParamsByItsContent() {
        val described = DescriptionDocumentParams(ByteArray(32) { 5 }, ByteArray(32) { 6 }, "post")
        val same = DescriptionDocumentParams(ByteArray(32) { 5 }, ByteArray(32) { 6 }, "post")
        assertEquals(described, same)
        assertEquals(described.hashCode(), same.hashCode())
        assertNotEquals(described, DescriptionDocumentParams(ByteArray(32) { 7 }, ByteArray(32) { 6 }, "post"))
        assertNotEquals(described, DescriptionDocumentParams(ByteArray(32) { 5 }, ByteArray(32) { 7 }, "post"))
        assertNotEquals(described, DescriptionDocumentParams(ByteArray(32) { 5 }, ByteArray(32) { 6 }, "note"))
    }

    /** The two amounts cross as decimal strings of credits, up to `u64::MAX`. */
    @Test
    fun shouldDecodeACreditAmountFromItsDecimalString() {
        assertEquals(20_000_000_000uL, creditsFromDecimal("20000000000"))
        assertEquals(ULong.MAX_VALUE, creditsFromDecimal("18446744073709551615"))
        for (malformed in listOf("", "-1", "2e10", "18446744073709551616", "\"20000000000\"")) {
            assertThrows(malformed, DashSdkError.SerializationError::class.java) {
                creditsFromDecimal(malformed)
            }
        }
    }

    @Test
    fun shouldRefuseJsonMissingARequiredField() {
        val error = assertThrows(DashSdkError.SerializationError::class.java) {
            MultipleChoiceProposal.fromJson(proposalJson.replace("\"open\"", "\"isOpen\""))
        }
        assertTrue(error.message.orEmpty(), error.message.orEmpty().contains("open"))
        assertThrows(DashSdkError.SerializationError::class.java) {
            ProposalChoice.fromJson(choiceJson.replace("\"proposalId\"", "\"proposal\""))
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            MultipleChoiceProposalVote.fromJson(
                """{"abstainVoteTally":null,"noneOfTheChoicesVoteTally":null,"result":null}""",
            )
        }
    }

    @Test
    fun shouldRefuseAFieldOfTheWrongType() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            MultipleChoiceProposal.fromJson(proposalJson.replace("\"open\":true", "\"open\":\"true\""))
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            MultipleChoiceProposal.fromJson(
                proposalJson.replace(Regex("\"descriptionDocument\":\\{[^}]*\\}"), "\"descriptionDocument\":\"post\""),
            )
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            MultipleChoiceProposalVote.fromJson(
                """{"choices":["$choiceId"],"abstainVoteTally":null,
                   "noneOfTheChoicesVoteTally":null,"result":null}""",
            )
        }
    }

    @Test
    fun shouldRefuseAnUnknownOutcomeOrChoice() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            MultipleChoiceProposalVote.fromJson(
                """{"choices":[],"abstainVoteTally":null,"noneOfTheChoicesVoteTally":null,
                   "result":{"outcome":"tied","keptChoiceId":null,
                             "finalizationBlock":{"height":9,"coreHeight":42,"timeMs":9000,"epoch":3}}}""",
            )
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            MultipleChoiceVoteChoice.fromJson("""{"choice":"maybe","choiceId":null}""")
        }
        // A vote for a choice names it.
        assertThrows(DashSdkError.SerializationError::class.java) {
            MultipleChoiceVoteChoice.fromJson("""{"choice":"choice","choiceId":null}""")
        }
    }

    @Test
    fun shouldRefuseTextThatIsNotJson() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            MultipleChoiceProposal.listFromJson("not json")
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            MultipleChoiceProposal.listFromJson(proposalJson)
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            ProposalChoice.fromJson("[]")
        }
    }
}
