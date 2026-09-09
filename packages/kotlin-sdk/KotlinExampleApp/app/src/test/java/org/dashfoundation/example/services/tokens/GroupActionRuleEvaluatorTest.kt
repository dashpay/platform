package org.dashfoundation.example.services.tokens

import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Banner-state tests for [GroupActionRuleEvaluator] — the ported
 * `inferredGroupPosition` logic from `TokenMintActionView.swift:191-207`
 * — driven by rules-JSON fixtures in the shapes the app persists
 * ([ChangeControlRules.toJson]) and the raw contract shapes
 * ([ChangeControlRules.parse] normalizes: V0 wrapper, snake_case,
 * tagged-object takers).
 */
class GroupActionRuleEvaluatorTest {

    private fun rules(authorized: String): String =
        ChangeControlRules(authorizedToMakeChange = authorized).toJson()

    // ── inferredGroupPosition over canonical column JSON ───────────────

    @Test
    fun `null or blank rules yield single-signer`() {
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition(null, 1))
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition("", 1))
        assertEquals(
            GroupActionRuleEvaluator.BannerState.None,
            GroupActionRuleEvaluator.bannerState(null, 1),
        )
    }

    @Test
    fun `garbage rules JSON yields single-signer`() {
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition("not json", 1))
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition("[1,2,3]", 1))
    }

    @Test
    fun `NoOne and ContractOwner and Identity takers are not group-gated`() {
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition(rules("NoOne"), 3))
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition(rules("ContractOwner"), 3))
        assertNull(
            GroupActionRuleEvaluator.inferredGroupPosition(
                rules("Identity:36NAEMim7BpheLD5xo3AmnEs2AUCPzCSCPGL4hpFYqrJ"), 3,
            ),
        )
    }

    @Test
    fun `MainGroup resolves through the main control group position`() {
        assertEquals(
            2,
            GroupActionRuleEvaluator.inferredGroupPosition(rules("MainGroup"), 2),
        )
        assertEquals(
            GroupActionRuleEvaluator.BannerState.Propose(2),
            GroupActionRuleEvaluator.bannerState(rules("MainGroup"), 2),
        )
    }

    @Test
    fun `MainGroup without a configured main position is single-signer`() {
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition(rules("MainGroup"), null))
    }

    @Test
    fun `MainGroup with an out-of-u16-range main position is single-signer`() {
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition(rules("MainGroup"), 0x10000))
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition(rules("MainGroup"), -1))
    }

    @Test
    fun `Group prefix resolves its position`() {
        assertEquals(
            0,
            GroupActionRuleEvaluator.inferredGroupPosition(rules("Group:0"), null),
        )
        assertEquals(
            7,
            GroupActionRuleEvaluator.inferredGroupPosition(rules("Group:7"), 1),
        )
        assertEquals(
            GroupActionRuleEvaluator.BannerState.Propose(7),
            GroupActionRuleEvaluator.bannerState(rules("Group:7"), 1),
        )
    }

    @Test
    fun `Group prefix with junk or out-of-range position is single-signer`() {
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition(rules("Group:abc"), 1))
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition(rules("Group:"), 1))
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition(rules("Group:65536"), 1))
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition(rules("Group:-1"), 1))
    }

    @Test
    fun `u16 boundary position 65535 is accepted`() {
        assertEquals(
            65535,
            GroupActionRuleEvaluator.inferredGroupPosition(rules("Group:65535"), null),
        )
        assertEquals(
            65535,
            GroupActionRuleEvaluator.inferredGroupPosition(rules("MainGroup"), 65535),
        )
    }

    // ── Raw contract-JSON fixture shapes (parser normalization) ───────

    @Test
    fun `V0-wrapped snake_case rule parses and infers`() {
        val raw = """{"V0":{"authorized_to_make_change":"Group:4",""" +
            """"admin_action_takers":"NoOne"}}"""
        assertEquals(4, GroupActionRuleEvaluator.inferredGroupPosition(raw, null))
    }

    @Test
    fun `tagged-object Group taker normalizes to the Group grammar`() {
        val raw = """{"V0":{"authorizedToMakeChange":{"Group":5}}}"""
        val parsed = ChangeControlRules.parse(raw)
        assertEquals("Group:5", parsed?.authorizedToMakeChange)
        assertEquals(5, GroupActionRuleEvaluator.inferredGroupPosition(raw, null))
    }

    @Test
    fun `tagged-object Identity taker normalizes and is not group-gated`() {
        val raw = """{"authorizedToMakeChange":{"Identity":"abc123"}}"""
        val parsed = ChangeControlRules.parse(raw)
        assertEquals("Identity:abc123", parsed?.authorizedToMakeChange)
        assertNull(GroupActionRuleEvaluator.inferredGroupPosition(raw, 3))
    }

    // ── relevantGroupPositions sweep ───────────────────────────────────

    private fun token(
        manualMintingRules: String? = null,
        manualBurningRules: String? = null,
        freezeRules: String? = null,
        emergencyActionRules: String? = null,
        maxSupplyChangeRules: String? = null,
        distributionChangeRules: String? = null,
        mainControlGroupPosition: Int? = null,
    ) = TokenEntity(
        id = ByteArray(36),
        contractId = ByteArray(32),
        position = 0,
        name = "Test",
        baseSupply = "1000",
        manualMintingRules = manualMintingRules,
        manualBurningRules = manualBurningRules,
        freezeRules = freezeRules,
        emergencyActionRules = emergencyActionRules,
        maxSupplyChangeRules = maxSupplyChangeRules,
        distributionChangeRules = distributionChangeRules,
        mainControlGroupPosition = mainControlGroupPosition,
    )

    @Test
    fun `no group-capable rules yields no positions`() {
        assertEquals(
            emptyList<Int>(),
            GroupActionRuleEvaluator.relevantGroupPositions(
                token(manualMintingRules = rules("ContractOwner")),
            ),
        )
    }

    @Test
    fun `positions are collected across rules and deduped in first-hit order`() {
        val t = token(
            manualMintingRules = rules("Group:2"),
            manualBurningRules = rules("MainGroup"),
            freezeRules = rules("Group:2"),
            emergencyActionRules = rules("Group:1"),
            mainControlGroupPosition = 9,
        )
        assertEquals(listOf(2, 9, 1), GroupActionRuleEvaluator.relevantGroupPositions(t))
    }

    @Test
    fun `direct-purchase pricing rule inside the distribution bundle is swept`() {
        val bundle = TokenDistributionChangeRules(
            changeDirectPurchasePricingRules =
                ChangeControlRules(authorizedToMakeChange = "Group:6"),
        ).toJson()
        assertEquals(
            listOf(6),
            GroupActionRuleEvaluator.relevantGroupPositions(
                token(distributionChangeRules = bundle),
            ),
        )
    }

    @Test
    fun `max-supply change group is included in pending proposal positions`() {
        assertEquals(
            listOf(11),
            GroupActionRuleEvaluator.relevantGroupPositions(
                token(maxSupplyChangeRules = rules("Group:11")),
            ),
        )
    }
}
