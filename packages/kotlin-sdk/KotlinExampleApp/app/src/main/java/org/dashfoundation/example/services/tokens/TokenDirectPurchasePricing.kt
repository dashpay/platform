package org.dashfoundation.example.services.tokens

import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.dashfoundation.example.util.LenientJson
import java.math.BigInteger

/**
 * A token's direct-purchase pricing schedule, parsed from the JSON that
 * `TokenQueries.directPurchasePrices` returns (shape emitted by the FFI's
 * `dash_sdk_token_get_direct_purchase_prices`):
 *
 * ```
 * {"<tokenIdBase58>": {"type":"single_price","price":<u64>}}
 * {"<tokenIdBase58>": {"type":"set_prices","prices":[{"amount":<u64>,"price":<u64>}, …]}}
 * {"<tokenIdBase58>": null}
 * ```
 *
 * This is the model behind the Direct Purchase form's cost preview. It
 * resolves the total credits a buyer must pay for a given raw token amount
 * using the *same* tier rule Drive applies when it validates the purchase
 * (`token_direct_purchase_transition_action` v0 transformer), so the
 * `expectedTotalCost` the form submits matches the chain's `required_price`:
 *
 *  - [SinglePrice]: `price × amount`.
 *  - [SetPrices]: the highest tier threshold `≤ amount` sets the per-token
 *    price and the total is `tierPrice × amount`. An amount below the
 *    smallest threshold is under the minimum sale amount (not purchasable);
 *    an empty schedule means the token isn't for direct sale.
 *
 * Prices are u64 credits on-chain, so parsing and multiplication go through
 * [BigInteger]. Drive saturates an overflowing [SinglePrice] multiplication,
 * while a [SetPrices] multiplication uses checked arithmetic and rejects the
 * purchase; this preview mirrors that distinction. A zero price is valid and
 * produces a zero expected cost, matching the native purchase contract.
 */
sealed interface TokenDirectPurchasePricing {

    /** One flat per-token price for any amount. */
    data class SinglePrice(val price: BigInteger) : TokenDirectPurchasePricing

    /** Tiered pricing: threshold → per-token price. */
    data class SetPrices(val tiers: List<Tier>) : TokenDirectPurchasePricing {
        /** A `(minimum amount, per-token price)` tier. */
        data class Tier(val amount: BigInteger, val price: BigInteger)
    }

    /**
     * Required total cost in credits to buy [amount] raw token units, or
     * `null` when the amount isn't purchasable at this schedule (below the
     * minimum sale amount, an empty schedule, or a tiered-price overflow.
     * Single-price overflow saturates to u64 max, matching Drive.
     */
    fun costFor(amount: ULong): ULong? {
        if (amount == 0uL) return null
        val count = BigInteger(amount.toString())
        if (count > MAX_TOKEN_COUNT) return null
        val (perToken, saturatesOnOverflow) = when (this) {
            is SinglePrice -> price to true
            is SetPrices -> (
                tiers
                    .filter { it.amount <= count }
                    .maxByOrNull { it.amount }
                    ?.price ?: return null
                ) to false
        }
        val total = perToken * count
        if (total.signum() < 0) return null
        if (total > MAX_U64 && !saturatesOnOverflow) return null
        return total.min(MAX_U64).toString().toULong()
    }

    /**
     * The smallest raw amount that can be purchased — `1` for a single
     * price, the lowest tier threshold for a tiered schedule. Used for the
     * "minimum purchase is N" hint.
     */
    val minimumPurchaseAmount: BigInteger
        get() = when (this) {
            is SinglePrice -> BigInteger.ONE
            is SetPrices -> tiers.minOfOrNull { it.amount } ?: BigInteger.ONE
        }

    companion object {
        private val MAX_U64 = BigInteger(ULong.MAX_VALUE.toString())
        // DPP's MAX_DISTRIBUTION_PARAM: token transition amounts are u48,
        // even though pricing and balance carriers use the full u64 domain.
        private val MAX_TOKEN_COUNT = BigInteger("281474976710655")

        /**
         * Parse the pricing for [canonicalTokenId] out of a
         * `directPurchasePrices` response, or `null` when the token has no
         * usable price (a `null`/missing entry, an empty tier list) or the
         * JSON can't be read.
         */
        fun parse(json: String, canonicalTokenId: String): TokenDirectPurchasePricing? {
            val entry = runCatching {
                LenientJson.parseToJsonElement(json).jsonObject[canonicalTokenId]
            }.getOrNull()
            val obj = entry as? JsonObject ?: return null
            return runCatching {
                when (obj["type"]?.jsonPrimitive?.content) {
                    "single_price" -> obj["price"]?.bigIntegerOrNull()?.let { SinglePrice(it) }
                    "set_prices" -> {
                        val tiers = obj["prices"]?.jsonArray.orEmpty().mapNotNull { element ->
                            val tier = element as? JsonObject ?: return@mapNotNull null
                            val amount = tier["amount"]?.bigIntegerOrNull()
                            val price = tier["price"]?.bigIntegerOrNull()
                            if (amount != null && price != null) {
                                SetPrices.Tier(amount, price)
                            } else {
                                null
                            }
                        }
                        if (tiers.isEmpty()) null else SetPrices(tiers)
                    }
                    else -> null
                }
            }.getOrNull()
        }

        private fun JsonElement.bigIntegerOrNull(): BigInteger? =
            runCatching { BigInteger(jsonPrimitive.content) }.getOrNull()
    }
}
