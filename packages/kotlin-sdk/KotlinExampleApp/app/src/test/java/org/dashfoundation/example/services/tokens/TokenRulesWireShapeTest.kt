package org.dashfoundation.example.services.tokens

import kotlinx.serialization.json.JsonElement
import org.dashfoundation.dashsdk.persistence.dao.TokenDao
import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.lang.reflect.Proxy
import kotlinx.coroutines.runBlocking

/**
 * The action-taker and distribution-recipient shapes rs-dpp writes into
 * contract JSON, read back by the app.
 *
 * The wire fixtures are copied verbatim from rs-dpp's own per-variant
 * wire-shape tests, which pin what its custom `Serialize` impls emit:
 * - `json_round_trip_with_full_wire_shape_all_variants` in
 *   `packages/rs-dpp/src/data_contract/change_control_rules/authorized_action_takers.rs`
 * - `recipient_json_round_trip_with_full_wire_shape_all_variants` in
 *   `packages/rs-dpp/src/data_contract/associated_token/token_perpetual_distribution/distribution_recipient.rs`
 *
 * The contract fixture follows the token block of
 * `packages/rs-dpp/src/data_contract/conversion/json/mod.rs`'s
 * `from_json` tests.
 */
class TokenRulesWireShapeTest {

    // `Identifier::from([0x42; 32])` in both rs-dpp tests.
    private val rsDppIdB58 = "5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf"

    private fun json(text: String): JsonElement = LenientJson.parseToJsonElement(text)

    // ── AuthorizedActionTakers ───────────────────────────────────────

    private val takerFixtures = listOf(
        """{"${'$'}type": "noOne"}""" to AuthorizedActionTakers.NO_ONE,
        """{"${'$'}type": "contractOwner"}""" to AuthorizedActionTakers.CONTRACT_OWNER,
        """{"${'$'}type": "identity", "identity": "$rsDppIdB58"}""" to
            AuthorizedActionTakers.identity(rsDppIdB58),
        """{"${'$'}type": "mainGroup"}""" to AuthorizedActionTakers.MAIN_GROUP,
        """{"${'$'}type": "group", "position": 42}""" to AuthorizedActionTakers.group(42),
    )

    @Test
    fun `should read every AuthorizedActionTakers variant rs-dpp serializes`() {
        for ((wire, expected) in takerFixtures) {
            assertEquals(wire, expected, AuthorizedActionTakers.parse(json(wire)))
        }
    }

    @Test
    fun `should read both taker fields of a rule object in the rs-dpp shape`() {
        for ((wire, expected) in takerFixtures) {
            val rule = ChangeControlRules.parse(
                """
                {"${'$'}formatVersion":"0","authorizedToMakeChange":$wire,
                 "adminActionTakers":$wire,
                 "changingAuthorizedActionTakersToNoOneAllowed":true,
                 "changingAdminActionTakersToNoOneAllowed":false,
                 "selfChangingAdminActionTakersAllowed":true}
                """.trimIndent(),
            )!!
            assertEquals(expected, rule.authorizedToMakeChange)
            assertEquals(expected, rule.adminActionTakers)
            assertTrue(rule.changingAuthorizedActionTakersToNoOneAllowed)
            assertTrue(rule.selfChangingAdminActionTakersAllowed)
        }
    }

    @Test
    fun `should round-trip every variant through the stored rules column`() {
        for ((wire, _) in takerFixtures) {
            val parsed = ChangeControlRules.parse("""{"authorizedToMakeChange":$wire,"adminActionTakers":$wire}""")!!
            assertEquals(parsed, ChangeControlRules.parse(parsed.toJson()))
        }
    }

    @Test
    fun `should keep a taker it cannot read as unknown, never as no one`() {
        val cases = mapOf(
            """{"${'$'}type": "everyone"}""" to AuthorizedActionTakers.unknown("everyone"),
            """{"${'$'}type": "identity"}""" to AuthorizedActionTakers.unknown("identity"),
            """{"${'$'}type": "group", "position": "x"}""" to AuthorizedActionTakers.unknown("group"),
            "\"Somebody\"" to AuthorizedActionTakers.unknown("Somebody"),
            "7" to AuthorizedActionTakers.unknown("7"),
        )
        for ((wire, expected) in cases) {
            val parsed = AuthorizedActionTakers.parse(json(wire))
            assertEquals(wire, expected, parsed)
            assertNotEquals(wire, AuthorizedActionTakers.NO_ONE, parsed)
        }
        val missing = ChangeControlRules.parse("""{"adminActionTakers":{"${'$'}type":"noOne"}}""")!!
        assertEquals(AuthorizedActionTakers.unknown("missing"), missing.authorizedToMakeChange)
    }

    @Test
    fun `should still read the older externally tagged takers`() {
        assertEquals("Group:5", AuthorizedActionTakers.parse(json("""{"Group":5}""")))
        assertEquals("Identity:abc", AuthorizedActionTakers.parse(json("""{"Identity":"abc"}""")))
    }

    // ── TokenDistributionRecipient ───────────────────────────────────

    @Test
    fun `should read every TokenDistributionRecipient variant rs-dpp serializes`() {
        val cases = listOf(
            """{"${'$'}type": "contractOwner"}""" to TokenDistributionRecipient.ContractOwner,
            """{"${'$'}type": "identity", "identity": "$rsDppIdB58"}""" to
                TokenDistributionRecipient.Identity(rsDppIdB58),
            """{"${'$'}type": "evonodesByParticipation"}""" to
                TokenDistributionRecipient.EvonodesByParticipation,
        )
        for ((wire, expected) in cases) {
            assertEquals(wire, expected, TokenDistributionRecipient.parse(json(wire)))
            val block = """{"${'$'}formatVersion":"0","distributionType":{},"distributionRecipient":$wire}"""
            assertEquals(wire, expected, TokenDistributionRecipient.fromPerpetualDistribution(block))
        }
    }

    @Test
    fun `should keep a recipient it cannot read as unknown`() {
        assertEquals(
            TokenDistributionRecipient.Unknown("everyone"),
            TokenDistributionRecipient.parse(json("""{"${'$'}type":"everyone"}""")),
        )
        assertEquals(
            TokenDistributionRecipient.Unknown("identity"),
            TokenDistributionRecipient.parse(json("""{"${'$'}type":"identity"}""")),
        )
        assertEquals(
            TokenDistributionRecipient.Unknown("missing distributionRecipient"),
            TokenDistributionRecipient.fromPerpetualDistribution("""{"distributionType":{}}"""),
        )
        assertEquals(
            TokenDistributionRecipient.Unknown("unreadable perpetual distribution"),
            TokenDistributionRecipient.fromPerpetualDistribution("not json"),
        )
    }

    // ── Contract -> rows -> action list ──────────────────────────────

    private val ownerId = ByteArray(32) { 0x11 }
    private val actorId = ByteArray(32) { 0x22 }
    private val strangerId = ByteArray(32) { 0x33 }
    private val contractId = ByteArray(32) { 0x44 }

    private fun rule(taker: String, admin: String = """{"${'$'}type": "contractOwner"}""") = """
        {"${'$'}formatVersion": "0",
         "authorizedToMakeChange": $taker,
         "adminActionTakers": $admin,
         "changingAuthorizedActionTakersToNoOneAllowed": false,
         "changingAdminActionTakersToNoOneAllowed": false,
         "selfChangingAdminActionTakersAllowed": false}
    """.trimIndent()

    /**
     * The QA token: minting authorized to one identity, burning to group 0,
     * freezing to the contract owner, and a perpetual distribution paid to
     * the acting identity.
     */
    private fun contract(): DataContractEntity {
        val actor = Base58.encode(actorId)
        val identityTaker = """{"${'$'}type": "identity", "identity": "$actor"}"""
        val groupTaker = """{"${'$'}type": "group", "position": 0}"""
        val ownerTaker = """{"${'$'}type": "contractOwner"}"""
        val noOne = """{"${'$'}type": "noOne"}"""
        val json = """
            {"${'$'}formatVersion": "1", "ownerId": "${Base58.encode(ownerId)}",
             "groups": {"0": {"${'$'}formatVersion": "0",
                              "members": {"$actor": 1, "${Base58.encode(ownerId)}": 1},
                              "requiredPower": 2}},
             "tokens": {"0": {
               "${'$'}formatVersion": "0",
               "conventions": {"${'$'}formatVersion": "0", "decimals": 2, "localizations": {}},
               "baseSupply": 1000,
               "distributionRules": {
                 "${'$'}formatVersion": "0",
                 "perpetualDistribution": {
                   "${'$'}formatVersion": "0",
                   "distributionType": {"${'$'}type": "blockBasedDistribution", "interval": 10,
                                        "function": {"${'$'}type": "fixedAmount", "amount": 100}},
                   "distributionRecipient": $identityTaker
                 },
                 "perpetualDistributionRules": ${rule(ownerTaker)},
                 "preProgrammedDistribution": null,
                 "newTokensDestinationIdentity": null,
                 "newTokensDestinationIdentityRules": ${rule(ownerTaker)},
                 "mintingAllowChoosingDestination": true,
                 "mintingAllowChoosingDestinationRules": ${rule(ownerTaker)},
                 "changeDirectPurchasePricingRules": ${rule(noOne, noOne)}
               },
               "marketplaceRules": {"${'$'}formatVersion": "0", "tradeMode": "NotTradeable"},
               "manualMintingRules": ${rule(identityTaker)},
               "manualBurningRules": ${rule(groupTaker)},
               "freezeRules": ${rule(ownerTaker)},
               "unfreezeRules": ${rule(groupTaker)},
               "destroyFrozenFundsRules": ${rule(identityTaker)},
               "emergencyActionRules": ${rule(groupTaker)},
               "mainControlGroup": 0,
               "mainControlGroupCanBeModified": $noOne
             }}}
        """.trimIndent()
        return DataContractEntity(
            id = contractId,
            name = "QA",
            serializedContract = json.encodeToByteArray(),
            networkRaw = 0,
            ownerId = ownerId,
        )
    }

    private fun identity(id: ByteArray) = IdentityEntity(identityId = id, networkRaw = 0)

    private fun actions(token: TokenEntity, id: ByteArray): Map<TokenActionKind, TokenActionPermission> =
        TokenActionResolver.resolve(token, identity(id), contract())
            .associate { it.kind to it.permission }

    @Test
    fun `should store the rs-dpp takers in the rules columns`() {
        val token = TokenMaterializer.parse(contract()).single()
        val actor = Base58.encode(actorId)

        assertEquals("Identity:$actor", ChangeControlRules.parse(token.manualMintingRules)?.authorizedToMakeChange)
        assertEquals("Group:0", ChangeControlRules.parse(token.manualBurningRules)?.authorizedToMakeChange)
        assertEquals("ContractOwner", ChangeControlRules.parse(token.freezeRules)?.authorizedToMakeChange)
        assertEquals("ContractOwner", ChangeControlRules.parse(token.manualMintingRules)?.adminActionTakers)
        assertEquals(AuthorizedActionTakers.NO_ONE, token.mainControlGroupCanBeModified)
    }

    @Test
    fun `should open the rule-gated actions for the identities the contract authorizes`() {
        val token = TokenMaterializer.parse(contract()).single()

        val actor = actions(token, actorId)
        assertEquals(TokenActionPermission.Allowed, actor[TokenActionKind.MINT])
        assertEquals(TokenActionPermission.Allowed, actor[TokenActionKind.BURN])
        assertEquals(TokenActionPermission.Allowed, actor[TokenActionKind.UNFREEZE])
        assertEquals(TokenActionPermission.Allowed, actor[TokenActionKind.DESTROY_FROZEN_FUNDS])
        assertEquals(TokenActionPermission.Allowed, actor[TokenActionKind.PAUSE])
        assertEquals(TokenActionPermission.Allowed, actor[TokenActionKind.CLAIM])
        assertEquals(
            TokenActionPermission.Denied("Freezing: only the contract owner"),
            actor[TokenActionKind.FREEZE],
        )

        val owner = actions(token, ownerId)
        assertEquals(TokenActionPermission.Allowed, owner[TokenActionKind.FREEZE])
        assertEquals(TokenActionPermission.Allowed, owner[TokenActionKind.BURN])
        assertEquals(
            TokenActionPermission.Denied("Not the designated distribution recipient"),
            owner[TokenActionKind.CLAIM],
        )

        val stranger = actions(token, strangerId)
        assertEquals(TokenActionPermission.Denied("Manual burning: not in group 0"), stranger[TokenActionKind.BURN])
    }

    @Test
    fun `should describe an unrecognised taker instead of saying no one is authorized`() {
        val token = TokenEntity(
            id = ByteArray(36),
            contractId = contractId,
            position = 0,
            name = "T",
            baseSupply = "0",
            manualBurningRules = ChangeControlRules.parse(rule("""{"${'$'}type": "everyone"}"""))!!.toJson(),
            canManuallyBurn = true,
        )
        assertEquals(
            TokenActionPermission.Denied("Manual burning: authorization rule 'everyone' not recognised by this app"),
            actions(token, actorId)[TokenActionKind.BURN],
        )
    }

    @Test
    fun `should rewrite rows an older build stored as NoOne from the stored contract`() = runBlocking {
        val rewrites = mutableMapOf<String, Any?>()
        val dao = Proxy.newProxyInstance(
            TokenDao::class.java.classLoader,
            arrayOf(TokenDao::class.java),
        ) { _, method, args ->
            require(method.name == "rewriteRules") { "unexpected ${method.name}" }
            method.parameters.dropLast(1).forEachIndexed { i, _ -> rewrites["p$i"] = args[i] }
            1
        } as TokenDao

        val rewritten = TokenMaterializer.rederiveRules(listOf(contract()), dao)

        assertEquals(1, rewritten)
        val expected = TokenMaterializer.parse(contract()).single()
        // Parameter order of TokenDao.rewriteRules.
        assertTrue((rewrites["p0"] as ByteArray).contentEquals(expected.id))
        assertEquals(expected.manualMintingRules, rewrites["p3"])
        assertEquals(expected.manualBurningRules, rewrites["p4"])
        assertEquals(expected.freezeRules, rewrites["p5"])
        assertEquals(expected.mainControlGroupCanBeModified, rewrites["p11"])
        assertEquals(
            "Identity:${Base58.encode(actorId)}",
            ChangeControlRules.parse(rewrites["p3"] as String)?.authorizedToMakeChange,
        )
    }
}
