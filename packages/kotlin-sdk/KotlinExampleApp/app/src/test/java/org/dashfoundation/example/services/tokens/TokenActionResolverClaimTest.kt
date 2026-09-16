package org.dashfoundation.example.services.tokens

import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.example.util.Base58
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Claim-row eligibility tests for [TokenActionResolver] — the designated
 * `newTokensDestinationIdentity` (perpetual) path plus pre-programmed
 * recipients named in the contract's `distributions` map
 * (`{"$formatVersion":"0","distributions":{"<timestampMs>":{"<base58>":amount}}}`,
 * the shape [TokenMaterializer] persists).
 */
class TokenActionResolverClaimTest {

    private val recipientId = ByteArray(32) { (it + 1).toByte() }
    private val recipientBase58 = Base58.encode(recipientId)
    private val strangerId = ByteArray(32) { (it + 101).toByte() }

    private fun identity(id: ByteArray = recipientId) =
        IdentityEntity(identityId = id, networkRaw = 0)

    private fun token(
        preProgrammed: String? = null,
        perpetual: String? = null,
        destination: ByteArray? = null,
        mintingAllowChoosing: Boolean = true,
    ) = TokenEntity(
        id = ByteArray(36),
        contractId = ByteArray(32),
        position = 0,
        name = "Test Token",
        baseSupply = "1000",
        perpetualDistribution = perpetual,
        preProgrammedDistribution = preProgrammed,
        newTokensDestinationIdentity = destination,
        mintingAllowChoosingDestination = mintingAllowChoosing,
        hasDistribution = preProgrammed != null || perpetual != null,
    )

    private fun claim(token: TokenEntity, identity: IdentityEntity): TokenActionPermission =
        TokenActionResolver.resolve(token, identity, contract = null)
            .first { it.kind == TokenActionKind.CLAIM }
            .permission

    private fun preProgrammedJson(recipient: String = recipientBase58): String =
        """{"${'$'}formatVersion":"0","distributions":{"1750000000000":{"$recipient":5000}}}"""

    @Test
    fun `no distribution schedule is denied`() {
        assertEquals(
            TokenActionPermission.Denied("Token has no distribution schedule"),
            claim(token(), identity()),
        )
    }

    @Test
    fun `designated destination identity is allowed`() {
        assertTrue(
            claim(
                token(perpetual = "{}", destination = recipientId),
                identity(),
            ).isAllowed,
        )
    }

    @Test
    fun `pre-programmed recipient is allowed`() {
        assertTrue(
            claim(token(preProgrammed = preProgrammedJson()), identity()).isAllowed,
        )
    }

    @Test
    fun `pre-programmed recipient is allowed even when minting destination is pinned elsewhere`() {
        assertTrue(
            claim(
                token(
                    preProgrammed = preProgrammedJson(),
                    destination = strangerId,
                    mintingAllowChoosing = false,
                ),
                identity(),
            ).isAllowed,
        )
    }

    @Test
    fun `pre-programmed recipient in a later release is allowed`() {
        val json = """
            {"${'$'}formatVersion":"0","distributions":{
              "1000":{"${Base58.encode(strangerId)}":1},
              "2000":{"$recipientBase58":5000}
            }}
        """.trimIndent()
        assertTrue(claim(token(preProgrammed = json), identity()).isAllowed)
    }

    @Test
    fun `V0-wrapped pre-programmed distribution is unwrapped`() {
        assertTrue(
            claim(
                token(preProgrammed = """{"V0":${preProgrammedJson()}}"""),
                identity(),
            ).isAllowed,
        )
    }

    @Test
    fun `non-recipient of a pre-programmed-only token is denied`() {
        assertEquals(
            TokenActionPermission.Denied("Not a recipient of any pre-programmed release"),
            claim(token(preProgrammed = preProgrammedJson()), identity(strangerId)),
        )
    }

    @Test
    fun `garbage pre-programmed JSON denies instead of crashing`() {
        assertEquals(
            TokenActionPermission.Denied("Not a recipient of any pre-programmed release"),
            claim(token(preProgrammed = "not json"), identity()),
        )
    }

    @Test
    fun `perpetual-only non-designated identity keeps the existing denials`() {
        assertEquals(
            TokenActionPermission.Denied("Not the designated distribution recipient"),
            claim(
                token(perpetual = "{}", destination = strangerId, mintingAllowChoosing = false),
                identity(),
            ),
        )
        assertEquals(
            TokenActionPermission.Denied("Distribution eligibility not yet evaluated"),
            claim(
                token(perpetual = "{}", destination = strangerId),
                identity(),
            ),
        )
    }
}
