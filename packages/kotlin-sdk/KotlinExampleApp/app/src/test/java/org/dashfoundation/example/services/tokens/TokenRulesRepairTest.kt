package org.dashfoundation.example.services.tokens

import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.example.util.Base58
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment

/**
 * [TokenMaterializer.rederiveRules] against a real in-memory Room database:
 * a row an older build stored with every rule as "NoOne" gets the rules its
 * stored contract names, and nothing else on the row changes.
 */
@RunWith(RobolectricTestRunner::class)
class TokenRulesRepairTest {

    private val contractId = ByteArray(32) { 0x44 }
    private val actor = Base58.encode(ByteArray(32) { 0x22 })
    private lateinit var db: DashDatabase

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(RuntimeEnvironment.getApplication())
    }

    @After
    fun tearDown() {
        db.close()
    }

    private fun rule(taker: String) =
        """{"${'$'}formatVersion":"0","authorizedToMakeChange":$taker,""" +
            """"adminActionTakers":{"${'$'}type":"contractOwner"},""" +
            """"changingAuthorizedActionTakersToNoOneAllowed":false,""" +
            """"changingAdminActionTakersToNoOneAllowed":false,""" +
            """"selfChangingAdminActionTakersAllowed":false}"""

    private val contract = DataContractEntity(
        id = contractId,
        name = "QA",
        serializedContract = (
            """{"tokens":{"0":{"baseSupply":1000,""" +
                """"manualMintingRules":${rule("""{"${'$'}type":"identity","identity":"$actor"}""")},""" +
                """"manualBurningRules":${rule("""{"${'$'}type":"group","position":0}""")}}}}"""
            ).encodeToByteArray(),
        networkRaw = 0,
        hasTokens = true,
    )

    /** The row as an older build wrote it, then paused on chain. */
    private fun seedOldRow() = runBlocking {
        db.dataContractDao().upsert(contract)
        val fresh = TokenMaterializer.parse(contract).single()
        val old = ChangeControlRules(
            authorizedToMakeChange = AuthorizedActionTakers.NO_ONE,
            adminActionTakers = AuthorizedActionTakers.NO_ONE,
        ).toJson()
        db.tokenDao().upsertToken(
            fresh.copy(manualMintingRules = old, manualBurningRules = old, isPaused = true, baseSupply = "777"),
        )
    }

    @Test
    fun `should rewrite rules stored as NoOne and keep the rest of the row`() = runBlocking {
        seedOldRow()
        val id = TokenMaterializer.tokenId(contractId, 0)

        val rewritten = TokenMaterializer.rederiveRules(listOf(contract), db.tokenDao())

        assertEquals(1, rewritten)
        val row = db.tokenDao().getTokenById(id)!!
        assertEquals("Identity:$actor", ChangeControlRules.parse(row.manualMintingRules)?.authorizedToMakeChange)
        assertEquals("Group:0", ChangeControlRules.parse(row.manualBurningRules)?.authorizedToMakeChange)
        assertTrue(row.isPaused)
        assertEquals("777", row.baseSupply)
        assertTrue(row.canManuallyMint)
    }

    @Test
    fun `should not write rows that already match and should repair them again after a downgrade`() =
        runBlocking {
            seedOldRow()
            assertEquals(1, TokenMaterializer.rederiveRules(listOf(contract), db.tokenDao()))
            assertEquals(0, TokenMaterializer.rederiveRules(listOf(contract), db.tokenDao()))

            // An older build writes the bad rows again; the next launch repairs them.
            seedOldRow()
            assertEquals(1, TokenMaterializer.rederiveRules(listOf(contract), db.tokenDao()))
        }

    @Test
    fun `should keep the stored paused flag when the contract is materialized again`() =
        runBlocking {
            db.dataContractDao().upsert(contract)
            val id = TokenMaterializer.tokenId(contractId, 0)

            // A new row takes the contract's startAsPaused (absent: false).
            TokenMaterializer.materialize(contract, db.tokenDao())
            assertFalse(db.tokenDao().getTokenById(id)!!.isPaused)

            // Paused on Platform, then the contract is re-imported or refreshed.
            db.tokenDao().updatePaused(id, true)
            TokenMaterializer.materialize(contract, db.tokenDao())
            assertTrue(db.tokenDao().getTokenById(id)!!.isPaused)

            // Resumed, then refreshed again.
            db.tokenDao().updatePaused(id, false)
            TokenMaterializer.materialize(contract, db.tokenDao())
            assertFalse(db.tokenDao().getTokenById(id)!!.isPaused)
        }
}
