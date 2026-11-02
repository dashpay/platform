package org.dashfoundation.dashsdk.voting

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Decoding of what the decision proposal natives return, and the encoding of what they take
 * (the vote choice, an edit's url change).
 *
 * Rust writes every value (rs-sdk-ffi's `decision_proposal/json.rs` tests pin the same
 * shapes); Kotlin only decodes, so these tests pin the JSON as rs-sdk-ffi writes it, key for
 * key. Pure JVM: no native library is loaded.
 */
class DecisionProposalModelsTest {

    private val proposalId = "8vLzWy3oPYAwJ9W8TjWZ3vSJMy7bDqyuvbMvzQMxVjPN"
    private val ownerId = "CQwyTH5AnQ5YQVGeYhBP8ihLdMKKbS6ydzN2E8wRWMNn"

    private val proposalJson = """
        {"id":"$proposalId","ownerId":"$ownerId","revision":1,
         "createdAt":1700000000000,"updatedAt":1700000000000,
         "title":"Raise the block size?","description":"Yes doubles it; no keeps it.",
         "url":"https://forum.dash.org/t/block-size",
         "passPercent":60,"minimumTurnoutPercent":10,"endDate":1701209600000}
    """.trimIndent()

    private val votePollJson = """
        {"${'$'}type":"yesNoVotePoll",
         "purpose":{"${'$'}type":"decisionProposal","documentId":"$proposalId"},
         "supermajorityNumerator":60,"supermajorityDenominator":100,
         "minimumVotingPower":{"${'$'}type":"fractionOfTotal","numerator":10,"denominator":100,"rounding":"up"}}
    """.trimIndent()

    @Test
    fun shouldDecodeAProposalWithEveryField() {
        assertEquals(
            DecisionProposal(
                id = proposalId,
                ownerId = ownerId,
                revision = 1,
                createdAtMs = 1_700_000_000_000,
                updatedAtMs = 1_700_000_000_000,
                title = "Raise the block size?",
                description = "Yes doubles it; no keeps it.",
                url = "https://forum.dash.org/t/block-size",
                passPercent = 60,
                minimumTurnoutPercent = 10,
                endDateMs = 1_701_209_600_000,
            ),
            DecisionProposal.fromJson(proposalJson),
        )
    }

    @Test
    fun shouldDecodeAnAbsentUrlAndAbsentSystemFieldsAsNull() {
        val proposal = DecisionProposal.fromJson(
            """
            {"id":"$proposalId","ownerId":"$ownerId","revision":null,"createdAt":null,
             "updatedAt":null,"title":"t","description":"d","url":null,
             "passPercent":50,"minimumTurnoutPercent":0,"endDate":1701209600000}
            """.trimIndent(),
        )
        assertNull(proposal.url)
        assertNull(proposal.revision)
        assertNull(proposal.createdAtMs)
        assertNull(proposal.updatedAtMs)
    }

    @Test
    fun shouldDecodeAListingInItsOrder() {
        val second = proposalJson.replace(proposalId, ownerId)
        val listing = DecisionProposal.listFromJson("[$proposalJson,$second]")
        assertEquals(listOf(proposalId, ownerId), listing.map { it.id })
        assertEquals(emptyList<DecisionProposal>(), DecisionProposal.listFromJson("[]"))
    }

    @Test
    fun shouldDecodeAStartedPollWithItsTalliesAndRule() {
        val state = YesNoVotePollState.fromJson(
            """
            {"votePoll":$votePollJson,"status":"started",
             "startBlock":{"height":7,"coreHeight":42,"timeMs":7000,"epoch":3},
             "yesVotingPower":5,"noVotingPower":1,"abstainVotingPower":4}
            """.trimIndent(),
        )
        assertEquals(
            YesNoVotePollState(
                votePoll = YesNoVotePoll(
                    purpose = YesNoVotePollPurpose.DecisionProposal(proposalId),
                    supermajorityNumerator = 60,
                    supermajorityDenominator = 100,
                    minimumVotingPower = YesNoMinimumVotingPower.FractionOfTotal(
                        numerator = 10,
                        denominator = 100,
                        rounding = VotingPowerRounding.UP,
                    ),
                ),
                status = YesNoVotePollStatus.Started(
                    PlatformBlockInfo(height = 7, coreHeight = 42, timeMs = 7_000, epoch = 3),
                ),
                yesVotingPower = 5,
                noVotingPower = 1,
                abstainVotingPower = 4,
            ),
            state,
        )
        assertNull(state.result)
    }

    @Test
    fun shouldDecodeAFinishedPollWithItsResult() {
        val state = YesNoVotePollState.fromJson(
            """
            {"votePoll":$votePollJson,"status":"finished",
             "result":{"passed":true,"yesVotingPower":30,"noVotingPower":10,
                       "abstainVotingPower":8,"requiredVotingPower":20,
                       "startBlock":{"height":7,"coreHeight":42,"timeMs":7000,"epoch":3},
                       "finalizationBlock":{"height":9,"coreHeight":42,"timeMs":9000,"epoch":3}},
             "yesVotingPower":0,"noVotingPower":0,"abstainVotingPower":0}
            """.trimIndent(),
        )
        val result = YesNoVotePollResult(
            passed = true,
            yesVotingPower = 30,
            noVotingPower = 10,
            abstainVotingPower = 8,
            requiredVotingPower = 20,
            startBlock = PlatformBlockInfo(height = 7, coreHeight = 42, timeMs = 7_000, epoch = 3),
            finalizationBlock = PlatformBlockInfo(height = 9, coreHeight = 42, timeMs = 9_000, epoch = 3),
        )
        assertEquals(YesNoVotePollStatus.Finished(result), state.status)
        assertEquals(result, state.result)
        assertEquals(0L, state.yesVotingPower)
        assertEquals(0L, state.noVotingPower)
        assertEquals(0L, state.abstainVotingPower)
    }

    @Test
    fun shouldDecodeAnAbsoluteMinimumVotingPower() {
        val state = YesNoVotePollState.fromJson(
            """
            {"votePoll":${votePollJson.replace(
                """{"${'$'}type":"fractionOfTotal","numerator":10,"denominator":100,"rounding":"up"}""",
                """{"${'$'}type":"absolute","votingPower":400}""",
            )},"status":"started",
             "startBlock":{"height":7,"coreHeight":42,"timeMs":7000,"epoch":3},
             "yesVotingPower":0,"noVotingPower":0,"abstainVotingPower":0}
            """.trimIndent(),
        )
        assertEquals(
            YesNoMinimumVotingPower.Absolute(votingPower = 400),
            state.votePoll.minimumVotingPower,
        )
    }

    @Test
    fun shouldDecodeEveryRoundingName() {
        val names = mapOf(
            "down" to VotingPowerRounding.DOWN,
            "up" to VotingPowerRounding.UP,
            "downPlusOne" to VotingPowerRounding.DOWN_PLUS_ONE,
            "upMinusOne" to VotingPowerRounding.UP_MINUS_ONE,
        )
        for ((name, rounding) in names) {
            val state = YesNoVotePollState.fromJson(
                """
                {"votePoll":${votePollJson.replace("\"up\"", "\"$name\"")},"status":"started",
                 "startBlock":{"height":7,"coreHeight":42,"timeMs":7000,"epoch":3},
                 "yesVotingPower":0,"noVotingPower":0,"abstainVotingPower":0}
                """.trimIndent(),
            )
            val minimum = state.votePoll.minimumVotingPower as YesNoMinimumVotingPower.FractionOfTotal
            assertEquals(name, rounding, minimum.rounding)
        }
    }

    @Test
    fun shouldDecodeAMasternodeAnswerForEveryChoice() {
        for (choice in YesNoAbstainVoteChoice.entries) {
            assertEquals(
                MasternodeYesNoVote(choice = choice, timesVoted = 2),
                MasternodeYesNoVote.fromJson("""{"choice":"${choice.jsonName}","timesVoted":2}"""),
            )
        }
    }

    /** The `vote_choice` byte of `dash_sdk_decision_proposal_cast_vote`. */
    @Test
    fun shouldEncodeTheVoteChoiceAsTheFfiByte() {
        assertEquals(0, YesNoAbstainVoteChoice.YES.discriminant)
        assertEquals(1, YesNoAbstainVoteChoice.NO.discriminant)
        assertEquals(2, YesNoAbstainVoteChoice.ABSTAIN.discriminant)
        assertEquals("yes", YesNoAbstainVoteChoice.YES.jsonName)
        assertEquals("no", YesNoAbstainVoteChoice.NO.jsonName)
        assertEquals("abstain", YesNoAbstainVoteChoice.ABSTAIN.jsonName)
        assertEquals(3, YesNoAbstainVoteChoice.entries.size)
    }

    /** The `url` and `remove_url` of `DecisionProposalEditFFI`, one shape per change. */
    @Test
    fun shouldEncodeEveryUrlChangeAsTheEditTakesIt() {
        assertEquals(
            NativeUrlChange(url = null, removeUrl = false),
            DecisionProposalEdit.UrlChange.Keep.toNative(),
        )
        assertEquals(
            NativeUrlChange(url = "https://forum.dash.org/t/block-size", removeUrl = false),
            DecisionProposalEdit.UrlChange.Set("https://forum.dash.org/t/block-size").toNative(),
        )
        assertEquals(
            NativeUrlChange(url = null, removeUrl = true),
            DecisionProposalEdit.UrlChange.Remove.toNative(),
        )
        assertEquals(DecisionProposalEdit.UrlChange.Keep, DecisionProposalEdit().url)
    }

    @Test
    fun shouldRefuseJsonMissingARequiredField() {
        val error = assertThrows(DashSdkError.SerializationError::class.java) {
            DecisionProposal.fromJson(proposalJson.replace("\"endDate\"", "\"closesAt\""))
        }
        assertTrue(error.message.orEmpty(), error.message.orEmpty().contains("endDate"))
    }

    @Test
    fun shouldRefuseAFieldOfTheWrongType() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            DecisionProposal.fromJson(proposalJson.replace("\"passPercent\":60", "\"passPercent\":\"60\""))
        }
    }

    @Test
    fun shouldRefuseAnUnknownPollStatusOrChoice() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            YesNoVotePollState.fromJson(
                """{"votePoll":$votePollJson,"status":"paused",
                   "yesVotingPower":0,"noVotingPower":0,"abstainVotingPower":0}""",
            )
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            MasternodeYesNoVote.fromJson("""{"choice":"maybe","timesVoted":1}""")
        }
    }

    @Test
    fun shouldRefuseTextThatIsNotJson() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            DecisionProposal.listFromJson("not json")
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            DecisionProposal.listFromJson(proposalJson)
        }
    }
}
