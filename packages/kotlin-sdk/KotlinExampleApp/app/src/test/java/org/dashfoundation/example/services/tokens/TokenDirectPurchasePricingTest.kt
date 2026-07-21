package org.dashfoundation.example.services.tokens

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.math.BigInteger

/**
 * Parsing + cost-resolution tests for [TokenDirectPurchasePricing] — the
 * model behind the Direct Purchase form's cost preview. The `costFor`
 * expectations mirror Drive's `token_direct_purchase_transition_action` v0
 * transformer (`required_price = perTokenPrice × token_count`, highest tier
 * `≤ amount`, under-minimum / empty schedule reject).
 */
class TokenDirectPurchasePricingTest {

    private val tokenId = "TokenIdBase58AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"

    private fun single(price: Long) =
        """{"$tokenId":{"type":"single_price","price":$price}}"""

    private fun setPrices(vararg tiers: Pair<Long, Long>): String {
        val entries = tiers.joinToString(",") { (amount, price) ->
            """{"amount":$amount,"price":$price}"""
        }
        return """{"$tokenId":{"type":"set_prices","prices":[$entries]}}"""
    }

    // ── Parsing ────────────────────────────────────────────────────────

    @Test
    fun `single price parses`() {
        val pricing = TokenDirectPurchasePricing.parse(single(100), tokenId)
        assertEquals(
            TokenDirectPurchasePricing.SinglePrice(BigInteger.valueOf(100)),
            pricing,
        )
    }

    @Test
    fun `set prices parse preserving tiers`() {
        val pricing = TokenDirectPurchasePricing.parse(
            setPrices(1L to 100L, 10L to 80L, 100L to 50L),
            tokenId,
        )
        fun tier(amount: Long, price: Long) = TokenDirectPurchasePricing.SetPrices.Tier(
            BigInteger.valueOf(amount), BigInteger.valueOf(price),
        )
        assertEquals(
            TokenDirectPurchasePricing.SetPrices(listOf(tier(1, 100), tier(10, 80), tier(100, 50))),
            pricing,
        )
    }

    @Test
    fun `null entry means no price`() {
        assertNull(TokenDirectPurchasePricing.parse("""{"$tokenId":null}""", tokenId))
    }

    @Test
    fun `missing token key means no price`() {
        val other = """{"other":{"type":"single_price","price":5}}"""
        assertNull(TokenDirectPurchasePricing.parse(other, tokenId))
    }

    @Test
    fun `empty set prices schedule means no price`() {
        assertNull(TokenDirectPurchasePricing.parse(setPrices(), tokenId))
    }

    @Test
    fun `garbage json means no price`() {
        assertNull(TokenDirectPurchasePricing.parse("not json", tokenId))
        assertNull(TokenDirectPurchasePricing.parse("[1,2,3]", tokenId))
    }

    @Test
    fun `u64 price beyond long range parses via big integer`() {
        // 2^63 + 1 — outside signed Long, must survive parsing.
        val big = "9223372036854775809"
        val pricing = TokenDirectPurchasePricing.parse(
            """{"$tokenId":{"type":"single_price","price":$big}}""",
            tokenId,
        )
        assertEquals(TokenDirectPurchasePricing.SinglePrice(BigInteger(big)), pricing)
    }

    // ── Cost resolution ────────────────────────────────────────────────

    @Test
    fun `single price cost is price times amount`() {
        val pricing = TokenDirectPurchasePricing.parse(single(250), tokenId)!!
        assertEquals(1_000uL, pricing.costFor(4u))
    }

    @Test
    fun `set prices picks highest tier at or below amount`() {
        val pricing = TokenDirectPurchasePricing.parse(
            setPrices(1L to 100L, 10L to 80L, 100L to 50L),
            tokenId,
        )!!
        // Buying 50 matches tier 10 (price 80) => 80 * 50.
        assertEquals(4_000uL, pricing.costFor(50u))
        // Exact boundary hits its own tier.
        assertEquals(800uL, pricing.costFor(10u))
        // Above the top tier uses the top tier.
        assertEquals(50_000uL, pricing.costFor(1_000u))
    }

    @Test
    fun `amount below minimum tier is not purchasable`() {
        val pricing = TokenDirectPurchasePricing.parse(setPrices(5L to 200L, 10L to 150L), tokenId)!!
        assertNull(pricing.costFor(2u))
        assertEquals(BigInteger.valueOf(5), pricing.minimumPurchaseAmount)
    }

    @Test
    fun `free tier produces zero expected cost`() {
        assertEquals(0uL, TokenDirectPurchasePricing.parse(single(0), tokenId)!!.costFor(10u))
    }

    @Test
    fun `zero amount is not purchasable`() {
        val pricing = TokenDirectPurchasePricing.parse(single(100), tokenId)!!
        assertNull(pricing.costFor(0u))
    }

    @Test
    fun `amount above protocol u48 maximum is not purchasable`() {
        val pricing = TokenDirectPurchasePricing.parse(single(1), tokenId)!!
        assertEquals(281_474_976_710_655uL, pricing.costFor(281_474_976_710_655uL))
        assertNull(pricing.costFor(281_474_976_710_656uL))
    }

    @Test
    fun `single price total beyond u64 saturates like Drive`() {
        // price 2^62, amount 4 => 2^64.
        val pricing = TokenDirectPurchasePricing.parse(single(1L shl 62), tokenId)!!
        assertEquals(ULong.MAX_VALUE, pricing.costFor(4u))
    }

    @Test
    fun `set prices total beyond u64 is not purchasable like Drive`() {
        val overflowPrice = (BigInteger.ONE shl 62).toString()
        val pricing = TokenDirectPurchasePricing.parse(
            """{"$tokenId":{"type":"set_prices","prices":[{"amount":1,"price":$overflowPrice}]}}""",
            tokenId,
        )!!

        assertNull(pricing.costFor(4u))
    }

    @Test
    fun `cost may use the high half of u64`() {
        val pricing = TokenDirectPurchasePricing.parse(single(1L shl 62), tokenId)!!
        assertEquals(1uL shl 63, pricing.costFor(2u))
    }

    @Test
    fun `single price minimum is one`() {
        val pricing = TokenDirectPurchasePricing.parse(single(100), tokenId)!!
        assertEquals(BigInteger.ONE, pricing.minimumPurchaseAmount)
    }
}
