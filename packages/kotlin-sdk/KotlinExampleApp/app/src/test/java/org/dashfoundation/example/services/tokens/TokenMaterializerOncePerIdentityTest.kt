package org.dashfoundation.example.services.tokens

import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * [TokenMaterializer] coverage for the `oncePerIdentityDistribution` block
 * (protocol version 14): the raw block lands in
 * [TokenEntity.oncePerIdentityDistribution], flips `hasDistribution`, and
 * [TokenOncePerIdentityDistribution.parse] reads the amount back as a
 * decimal string whether the contract encoded it as a number or a string,
 * and only inside the protocol's 1 to i64::MAX range.
 */
class TokenMaterializerOncePerIdentityTest {

    private val contractId = ByteArray(32) { 0xCD.toByte() }

    private fun contract(distributionRules: String): DataContractEntity = DataContractEntity(
        id = contractId,
        name = "Fixture",
        serializedContract =
            """{"tokens":{"0":{"baseSupply":0,"distributionRules":$distributionRules}}}"""
                .encodeToByteArray(),
        networkRaw = 0,
    )

    private fun parseSingleToken(distributionRules: String): TokenEntity {
        val tokens = TokenMaterializer.parse(contract(distributionRules))
        assertEquals(1, tokens.size)
        return tokens.single()
    }

    private fun block(amount: String): String =
        """{"oncePerIdentityDistribution":{"${'$'}formatVersion":"0","amount":$amount}}"""

    @Test
    fun `numeric amount is persisted raw and parses to a decimal string`() {
        val token = parseSingleToken(block("5000"))

        val raw = token.oncePerIdentityDistribution
        assertNotNull(raw)
        assertTrue(token.hasDistribution)
        assertEquals("5000", TokenOncePerIdentityDistribution.parse(raw)?.amount)
    }

    @Test
    fun `string amount parses to the same decimal string`() {
        val token = parseSingleToken(block("\"12345\""))

        assertEquals(
            "12345",
            TokenOncePerIdentityDistribution.parse(token.oncePerIdentityDistribution)?.amount,
        )
    }

    @Test
    fun `the largest protocol amount survives verbatim as number and as string`() {
        val max = Long.MAX_VALUE.toString() // i64::MAX, the most rs-dpp admits

        val asNumber = parseSingleToken(block(max))
        assertEquals(
            max,
            TokenOncePerIdentityDistribution.parse(asNumber.oncePerIdentityDistribution)?.amount,
        )

        val asString = parseSingleToken(block("\"$max\""))
        assertEquals(
            max,
            TokenOncePerIdentityDistribution.parse(asString.oncePerIdentityDistribution)?.amount,
        )
    }

    @Test
    fun `parse rejects amounts outside the protocol range`() {
        // rs-dpp admits 1..=i64::MAX, so nothing else can come from a contract on chain.
        assertNull(TokenOncePerIdentityDistribution.parse("""{"amount":0}"""))
        assertNull(TokenOncePerIdentityDistribution.parse("""{"amount":"9223372036854775808"}"""))
        assertNull(TokenOncePerIdentityDistribution.parse("""{"amount":"18446744073709551615"}"""))
        assertNull(TokenOncePerIdentityDistribution.parse("""{"amount":"18446744073709551616"}"""))
        assertEquals("1", TokenOncePerIdentityDistribution.parse("""{"amount":1}""")?.amount)
    }

    @Test
    fun `absent block leaves the column null and hasDistribution false`() {
        val token = parseSingleToken("""{"mintingAllowChoosingDestination":true}""")

        assertNull(token.oncePerIdentityDistribution)
        assertFalse(token.hasDistribution)
    }

    @Test
    fun `block alongside a pre-programmed distribution keeps both`() {
        val preProgrammed =
            """{"${'$'}formatVersion":"0","distributions":{"1750000000000":{"abc":1}}}"""
        val token = parseSingleToken(
            """{"preProgrammedDistribution":$preProgrammed,""" +
                """"oncePerIdentityDistribution":{"${'$'}formatVersion":"0","amount":9}}""",
        )

        assertNotNull(token.preProgrammedDistribution)
        assertEquals(
            "9",
            TokenOncePerIdentityDistribution.parse(token.oncePerIdentityDistribution)?.amount,
        )
        assertTrue(token.hasDistribution)
    }

    @Test
    fun `parse rejects a block without a valid amount`() {
        assertNull(TokenOncePerIdentityDistribution.parse("""{"${'$'}formatVersion":"0"}"""))
        assertNull(TokenOncePerIdentityDistribution.parse("""{"amount":"abc"}"""))
        assertNull(TokenOncePerIdentityDistribution.parse("""{"amount":-1}"""))
        assertNull(TokenOncePerIdentityDistribution.parse("""{"amount":1.5}"""))
        assertNull(TokenOncePerIdentityDistribution.parse("not json"))
        assertNull(TokenOncePerIdentityDistribution.parse(""))
        assertNull(TokenOncePerIdentityDistribution.parse(null))
    }

    @Test
    fun `already-claimed rejection is recognised by code and by message`() {
        assertTrue(
            OncePerIdentityClaimStore.isAlreadyClaimed(
                IllegalStateException("consensus error 40722: claim rejected"),
            ),
        )
        assertTrue(
            OncePerIdentityClaimStore.isAlreadyClaimed(
                RuntimeException(
                    "claim failed",
                    IllegalStateException(
                        "Token claim error: identity 'a' already claimed the " +
                            "once-per-identity distribution of token 'b' at 100",
                    ),
                ),
            ),
        )
        assertFalse(
            OncePerIdentityClaimStore.isAlreadyClaimed(
                IllegalStateException("Token mint past max supply"),
            ),
        )
    }

    @Test
    fun `V0-wrapped block is unwrapped`() {
        assertEquals("7", TokenOncePerIdentityDistribution.parse("""{"V0":{"amount":7}}""")?.amount)
    }
}
