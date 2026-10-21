package org.dashfoundation.example.services.tokens

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
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
 * [TokenOncePerIdentityDistribution.parse] reads the u64 amount back as a
 * decimal string whether the contract encoded it as a number or a string.
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
    fun `amounts above Long MAX_VALUE survive verbatim as number and as string`() {
        val huge = "18446744073709551615" // UInt64.max

        val asNumber = parseSingleToken(block(huge))
        assertEquals(
            huge,
            TokenOncePerIdentityDistribution.parse(asNumber.oncePerIdentityDistribution)?.amount,
        )

        val asString = parseSingleToken(block("\"$huge\""))
        assertEquals(
            huge,
            TokenOncePerIdentityDistribution.parse(asString.oncePerIdentityDistribution)?.amount,
        )
    }

    @Test
    fun `parse checks the u64 carrier and leaves the protocol range to Rust`() {
        // rs-dpp validates 1..=i64::MAX at registration; the app does not mirror that
        // rule, it only refuses what is not a raw u64 at all.
        assertEquals("0", TokenOncePerIdentityDistribution.parse("""{"amount":0}""")?.amount)
        assertEquals(
            "9223372036854775808",
            TokenOncePerIdentityDistribution.parse("""{"amount":"9223372036854775808"}""")?.amount,
        )
        assertNull(TokenOncePerIdentityDistribution.parse("""{"amount":"18446744073709551616"}"""))
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
    fun `already-claimed rejection is recognised by its consensus code`() {
        val rejection = claimFailure(consensusCode = 40722, consensusKind = STATE_KIND)
        assertTrue(OncePerIdentityClaimStore.isAlreadyClaimed(rejection))
        // Wrapped by a caller on the way up, the SDK error is still found.
        assertTrue(
            OncePerIdentityClaimStore.isAlreadyClaimed(RuntimeException("claim failed", rejection)),
        )
    }

    @Test
    fun `another consensus rejection is not the already-claimed one`() {
        // `TokenNotForDirectSale`, the state error one code below.
        assertFalse(
            OncePerIdentityClaimStore.isAlreadyClaimed(
                claimFailure(consensusCode = 40721, consensusKind = STATE_KIND),
            ),
        )
    }

    @Test
    fun `error text is never read as the already-claimed rejection`() {
        // A failure that was not a consensus rejection, whatever its message
        // says: the code as a number of its own, the sentence rs-dpp renders,
        // and the digits inside a timestamp.
        listOf(
            "consensus error 40722: claim rejected",
            "Token claim error: identity 'a' already claimed the " +
                "once-per-identity distribution of token 'b' at 100",
            "Token mint past max supply: 1758140722000",
        ).forEach { message ->
            assertFalse(
                message,
                OncePerIdentityClaimStore.isAlreadyClaimed(
                    DashSdkError.fromNative(DashSDKException(UNKNOWN_WALLET_ERROR, message)),
                ),
            )
            assertFalse(
                message,
                OncePerIdentityClaimStore.isAlreadyClaimed(IllegalStateException(message)),
            )
        }
    }

    @Test
    fun `V0-wrapped block is unwrapped`() {
        assertEquals("7", TokenOncePerIdentityDistribution.parse("""{"V0":{"amount":7}}""")?.amount)
    }

    /** What `wallet.tokens.claim` throws for a claim Platform rejected. */
    private fun claimFailure(consensusCode: Int, consensusKind: Int): DashSdkError =
        DashSdkError.fromNative(
            DashSDKException(
                UNKNOWN_WALLET_ERROR,
                "Token operation failed: Token claim failed: state transition broadcast error",
                consensusCode,
                consensusKind,
            ),
        )

    private companion object {
        /** `PlatformWalletFFIResultCode::ErrorUnknown`, as the JNI bridge throws it. */
        const val UNKNOWN_WALLET_ERROR = DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 99

        /** `PlatformWalletFFIConsensusErrorKind::State`. */
        const val STATE_KIND = 4
    }
}
