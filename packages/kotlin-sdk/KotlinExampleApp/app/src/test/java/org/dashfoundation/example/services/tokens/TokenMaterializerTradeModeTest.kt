package org.dashfoundation.example.services.tokens

import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * [TokenMaterializer] coverage for `marketplaceRules.tradeMode`. rs-dpp
 * writes `TokenTradeMode` externally tagged: `NotTradeable` as a bare
 * string, `Tradeable` (protocol version 15) as an object holding its
 * `minFill` (a JSON number, or a decimal string above 2^53 - 1). The
 * variant name lands in [TokenEntity.tradeMode]; `minFill` is not kept. Any
 * other name is stored as `NotTradeable`.
 * Mirrors `DataContractParserTradeModeTests.swift`.
 */
class TokenMaterializerTradeModeTest {

    private val contractId = ByteArray(32) { 0xAB.toByte() }

    /** [tradeMode] is the raw JSON value of `marketplaceRules.tradeMode`. */
    private fun parseSingleToken(tradeMode: String): TokenEntity {
        val json = """{"tokens":{"0":{"baseSupply":0,"marketplaceRules":""" +
            """{"${'$'}formatVersion":"0","tradeMode":$tradeMode}}}}"""
        val contract = DataContractEntity(
            id = contractId,
            name = "Fixture",
            serializedContract = json.encodeToByteArray(),
            networkRaw = 0,
        )
        val tokens = TokenMaterializer.parse(contract)
        assertEquals(1, tokens.size)
        return tokens.single()
    }

    @Test
    fun `tradeable object form is stored as Tradeable`() {
        val token = parseSingleToken("""{"Tradeable":{"minFill":1000}}""")

        assertEquals("Tradeable", token.tradeMode)
    }

    @Test
    fun `tradeable with a string minFill is stored as Tradeable`() {
        val token = parseSingleToken("""{"Tradeable":{"minFill":"9223372036854775807"}}""")

        assertEquals("Tradeable", token.tradeMode)
    }

    @Test
    fun `bare NotTradeable string stays NotTradeable`() {
        val token = parseSingleToken("\"NotTradeable\"")

        assertEquals("NotTradeable", token.tradeMode)
    }

    @Test
    fun `an unknown bare mode is stored as NotTradeable`() {
        val token = parseSingleToken("\"Auctioned\"")

        assertEquals("NotTradeable", token.tradeMode)
    }

    @Test
    fun `an unknown tagged mode is stored as NotTradeable`() {
        val token = parseSingleToken("""{"Auctioned":{"minFill":1}}""")

        assertEquals("NotTradeable", token.tradeMode)
    }
}
