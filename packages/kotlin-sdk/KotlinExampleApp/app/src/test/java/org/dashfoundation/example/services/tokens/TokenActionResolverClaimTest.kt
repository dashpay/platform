package org.dashfoundation.example.services.tokens

import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.dashsdk.tokens.TokenDistributionType
import org.dashfoundation.example.util.Base58
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Claim-row eligibility tests for [TokenActionResolver] — the perpetual
 * distribution's `distributionRecipient` (what Drive's wrong-claimant check
 * reads; the minting destination plays no part) plus pre-programmed
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
        contract: DataContractEntity? = null,
    ): TokenActionPermission =
        TokenActionResolver.resolve(token, identity, contract, oncePerIdentityClaimed)
            .first { it.kind == TokenActionKind.CLAIM }
            .permission

    private fun preferred(
        token: TokenEntity,
        identity: IdentityEntity,
        oncePerIdentityClaimed: Boolean = false,
    ): TokenDistributionType? =
        TokenActionResolver.preferredClaimDistribution(token, identity, null, oncePerIdentityClaimed)

    private fun preProgrammedJson(recipient: String = recipientBase58): String =
        """{"${'$'}formatVersion":"0","distributions":{"1750000000000":{"$recipient":5000}}}"""

    /** A stored perpetual block in rs-dpp's shape, paid to [recipient]. */
    private fun perpetualJson(
        recipient: String = """{"${'$'}type":"identity","identity":"$recipientBase58"}""",
    ): String =
        """{"${'$'}formatVersion":"0","distributionType":{"${'$'}type":"blockBasedDistribution",""" +
            """"interval":10,"function":{"${'$'}type":"fixedAmount","amount":100}},""" +
            """"distributionRecipient":$recipient}"""

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
    fun `perpetual distribution paid to the identity is allowed`() {
        assertTrue(claim(token(perpetual = perpetualJson()), identity()).isAllowed)
        assertEquals(
            listOf(TokenDistributionType.PERPETUAL),
            claimable(token(perpetual = perpetualJson()), identity()),
        )
    }

    @Test
    fun `perpetual distribution paid to the identity is allowed whatever the minting destination`() {
        // QA case TOK-07: the recipient is the acting identity while new
        // tokens go elsewhere (or nowhere), which Drive does not look at.
        assertTrue(
            claim(
                token(perpetual = perpetualJson(), destination = strangerId, mintingAllowChoosing = false),
                identity(),
            ).isAllowed,
        )
    }

    @Test
    fun `minting destination alone does not make a perpetual distribution claimable`() {
        val paidToStranger = perpetualJson(
            """{"${'$'}type":"identity","identity":"${Base58.encode(strangerId)}"}""",
        )
        assertEquals(
            TokenActionPermission.Denied("Not the designated distribution recipient"),
            claim(token(perpetual = paidToStranger, destination = recipientId), identity()),
        )
    }

    @Test
    fun `perpetual distribution paid to the contract owner is allowed only for the owner`() {
        val ownerPaid = token(perpetual = perpetualJson("""{"${'$'}type":"contractOwner"}"""))
        val contract = DataContractEntity(
            id = ByteArray(32),
            name = "C",
            serializedContract = ByteArray(0),
            networkRaw = 0,
            ownerId = recipientId,
        )
        assertTrue(claim(ownerPaid, identity(), contract = contract).isAllowed)
        assertEquals(
            TokenActionPermission.Denied("Not the designated distribution recipient"),
            claim(ownerPaid, identity(strangerId), contract = contract),
        )
        assertEquals(
            TokenActionPermission.Denied("Distribution recipient (contract owner) unknown locally"),
            claim(ownerPaid, identity()),
        )
    }

    @Test
    fun `perpetual recipients the app cannot evaluate say so`() {
        assertEquals(
            TokenActionPermission.Denied(
                "Distribution pays evonodes by participation; eligibility not evaluated by this app",
            ),
            claim(
                token(perpetual = perpetualJson("""{"${'$'}type":"evonodesByParticipation"}""")),
                identity(),
            ),
        )
        assertEquals(
            TokenActionPermission.Denied("Distribution recipient 'someoneNew' not recognised by this app"),
            claim(token(perpetual = perpetualJson("""{"${'$'}type":"someoneNew"}""")), identity()),
        )
        assertEquals(
            TokenActionPermission.Denied(
                "Distribution recipient 'missing distributionRecipient' not recognised by this app",
            ),
            claim(token(perpetual = "{}"), identity()),
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
                    perpetual = perpetualJson(),
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
                    perpetual = perpetualJson(),
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

    private fun claimable(
        token: TokenEntity,
        identity: IdentityEntity,
        oncePerIdentityClaimed: Boolean = false,
    ): List<TokenDistributionType> =
        TokenActionResolver.claimableDistributions(token, identity, null, oncePerIdentityClaimed)

    @Test
    fun `only the kinds the identity is eligible for are offered`() {
        val perpetualAndOnce = token(
            perpetual = perpetualJson(),
            oncePerIdentity = oncePerIdentityJson(),
        )
        // A stranger is only eligible through the once-per-identity kind:
        // offering perpetual would enable a paid wrong-claimant rejection.
        assertEquals(
            listOf(TokenDistributionType.ONCE_PER_IDENTITY),
            claimable(perpetualAndOnce, identity(strangerId)),
        )
        assertEquals(
            TokenDistributionType.ONCE_PER_IDENTITY,
            preferred(perpetualAndOnce, identity(strangerId)),
        )
        // The identity the perpetual distribution pays keeps perpetual first.
        assertEquals(
            listOf(TokenDistributionType.PERPETUAL, TokenDistributionType.ONCE_PER_IDENTITY),
            claimable(perpetualAndOnce, identity()),
        )
        assertEquals(TokenDistributionType.PERPETUAL, preferred(perpetualAndOnce, identity()))

        val preProgrammedAndOnce =
            token(preProgrammed = preProgrammedJson(), oncePerIdentity = oncePerIdentityJson())
        assertEquals(
            listOf(TokenDistributionType.PRE_PROGRAMMED, TokenDistributionType.ONCE_PER_IDENTITY),
            claimable(preProgrammedAndOnce, identity()),
        )
        assertEquals(
            listOf(TokenDistributionType.ONCE_PER_IDENTITY),
            claimable(preProgrammedAndOnce, identity(strangerId)),
        )
        assertEquals(
            TokenDistributionType.ONCE_PER_IDENTITY,
            preferred(token(oncePerIdentity = oncePerIdentityJson()), identity(strangerId)),
        )
    }

    @Test
    fun `a spent once-per-identity claim leaves a stranger nothing to claim`() {
        val perpetualAndOnce = token(
            perpetual = perpetualJson(),
            oncePerIdentity = oncePerIdentityJson(),
        )
        // The perpetual distribution pays someone else, so it is not a fallback.
        assertEquals(
            emptyList<TokenDistributionType>(),
            claimable(perpetualAndOnce, identity(strangerId), oncePerIdentityClaimed = true),
        )
        assertNull(preferred(perpetualAndOnce, identity(strangerId), oncePerIdentityClaimed = true))
        assertNull(
            preferred(
                token(oncePerIdentity = oncePerIdentityJson()),
                identity(strangerId),
                oncePerIdentityClaimed = true,
            ),
        )
        // The identity it pays keeps the perpetual kind after spending the claim.
        assertEquals(
            listOf(TokenDistributionType.PERPETUAL),
            claimable(perpetualAndOnce, identity(), oncePerIdentityClaimed = true),
        )
    }
}
