package org.dashfoundation.example.services.tokens

import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.dashsdk.tokens.TokenDistributionType
import org.dashfoundation.example.util.Base58
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Claim-row eligibility tests for [TokenActionResolver] — the designated
 * `newTokensDestinationIdentity` (perpetual) path plus pre-programmed
 * recipients named in the contract's `distributions` map
 * (`{"$formatVersion":"0","distributions":{"<timestampMs>":{"<base58>":amount}}}`,
 * the shape [TokenMaterializer] persists), and the once-per-identity kind
 * (protocol version 14) that makes every identity eligible once.
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
        oncePerIdentity: String? = null,
    ) = TokenEntity(
        id = ByteArray(36),
        contractId = ByteArray(32),
        position = 0,
        name = "Test Token",
        baseSupply = "1000",
        perpetualDistribution = perpetual,
        preProgrammedDistribution = preProgrammed,
        oncePerIdentityDistribution = oncePerIdentity,
        newTokensDestinationIdentity = destination,
        mintingAllowChoosingDestination = mintingAllowChoosing,
        hasDistribution = preProgrammed != null || perpetual != null || oncePerIdentity != null,
    )

    private fun claim(
        token: TokenEntity,
        identity: IdentityEntity,
        oncePerIdentityClaimed: Boolean = false,
    ): TokenActionPermission =
        TokenActionResolver.resolve(token, identity, contract = null, oncePerIdentityClaimed)
            .first { it.kind == TokenActionKind.CLAIM }
            .permission

    private fun preferred(
        token: TokenEntity,
        identity: IdentityEntity,
        oncePerIdentityClaimed: Boolean = false,
    ): TokenDistributionType? =
        TokenActionResolver.preferredClaimDistribution(token, identity, oncePerIdentityClaimed)

    private fun preProgrammedJson(recipient: String = recipientBase58): String =
        """{"${'$'}formatVersion":"0","distributions":{"1750000000000":{"$recipient":5000}}}"""

    private fun oncePerIdentityJson(amount: String = "5000"): String =
        """{"${'$'}formatVersion":"0","amount":$amount}"""

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
    fun `once-per-identity distribution allows any identity`() {
        assertTrue(claim(token(oncePerIdentity = oncePerIdentityJson()), identity()).isAllowed)
        assertTrue(
            claim(token(oncePerIdentity = oncePerIdentityJson()), identity(strangerId)).isAllowed,
        )
    }

    @Test
    fun `once-per-identity distribution allows a non-recipient of the pre-programmed releases`() {
        assertTrue(
            claim(
                token(preProgrammed = preProgrammedJson(), oncePerIdentity = oncePerIdentityJson()),
                identity(strangerId),
            ).isAllowed,
        )
    }

    @Test
    fun `claimed once-per-identity distribution is denied when it was the only reason`() {
        assertEquals(
            TokenActionPermission.Denied("Already claimed the once-per-identity distribution"),
            claim(
                token(oncePerIdentity = oncePerIdentityJson()),
                identity(strangerId),
                oncePerIdentityClaimed = true,
            ),
        )
        // Alongside a perpetual distribution paid to someone else it is still the
        // spent claim that explains the denial, not a recipient mismatch.
        assertEquals(
            TokenActionPermission.Denied("Already claimed the once-per-identity distribution"),
            claim(
                token(
                    perpetual = "{}",
                    destination = recipientId,
                    oncePerIdentity = oncePerIdentityJson(),
                ),
                identity(strangerId),
                oncePerIdentityClaimed = true,
            ),
        )
    }

    @Test
    fun `claimed once-per-identity distribution keeps the other kinds claimable`() {
        assertTrue(
            claim(
                token(
                    perpetual = "{}",
                    destination = recipientId,
                    oncePerIdentity = oncePerIdentityJson(),
                ),
                identity(),
                oncePerIdentityClaimed = true,
            ).isAllowed,
        )
        assertTrue(
            claim(
                token(preProgrammed = preProgrammedJson(), oncePerIdentity = oncePerIdentityJson()),
                identity(),
                oncePerIdentityClaimed = true,
            ).isAllowed,
        )
    }

    @Test
    fun `preferred kind is the one that makes the identity eligible`() {
        val perpetualAndOnce = token(
            perpetual = "{}",
            destination = recipientId,
            oncePerIdentity = oncePerIdentityJson(),
        )
        // A stranger is only eligible through the once-per-identity kind:
        // defaulting to perpetual would be a paid wrong-claimant rejection.
        assertEquals(
            TokenDistributionType.ONCE_PER_IDENTITY,
            preferred(perpetualAndOnce, identity(strangerId)),
        )
        // The identity the perpetual distribution pays keeps perpetual first.
        assertEquals(TokenDistributionType.PERPETUAL, preferred(perpetualAndOnce, identity()))

        val preProgrammedAndOnce =
            token(preProgrammed = preProgrammedJson(), oncePerIdentity = oncePerIdentityJson())
        assertEquals(
            TokenDistributionType.PRE_PROGRAMMED,
            preferred(preProgrammedAndOnce, identity()),
        )
        assertEquals(
            TokenDistributionType.ONCE_PER_IDENTITY,
            preferred(preProgrammedAndOnce, identity(strangerId)),
        )
        assertEquals(
            TokenDistributionType.ONCE_PER_IDENTITY,
            preferred(token(oncePerIdentity = oncePerIdentityJson()), identity(strangerId)),
        )
    }

    @Test
    fun `claimed once-per-identity distribution is no longer offered`() {
        val perpetualAndOnce = token(
            perpetual = "{}",
            destination = recipientId,
            oncePerIdentity = oncePerIdentityJson(),
        )
        assertEquals(
            listOf(TokenDistributionType.PERPETUAL),
            TokenActionResolver.claimableDistributions(
                perpetualAndOnce, oncePerIdentityClaimed = true,
            ),
        )
        // Nothing left that makes the stranger eligible: fall back to what exists.
        assertEquals(
            TokenDistributionType.PERPETUAL,
            preferred(perpetualAndOnce, identity(strangerId), oncePerIdentityClaimed = true),
        )
        assertNull(
            preferred(
                token(oncePerIdentity = oncePerIdentityJson()),
                identity(strangerId),
                oncePerIdentityClaimed = true,
            ),
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
