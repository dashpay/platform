package org.dashfoundation.example.ui.tokens

import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.dashfoundation.example.util.LenientJson
import org.junit.Assert.assertEquals
import org.junit.Test

class QuickBasicTokenScreenTest {

    @Test
    fun `token schema preserves full u64 supplies as JSON numbers`() {
        val json = synthesizeTokenSchemas(
            singular = "coin",
            plural = "coins",
            shouldCapitalize = false,
            decimals = 0,
            baseSupply = 1uL shl 63,
            maxSupply = ULong.MAX_VALUE,
        )
        val token = LenientJson.parseToJsonElement(json).jsonObject["0"]!!.jsonObject

        assertEquals("9223372036854775808", token["baseSupply"]!!.jsonPrimitive.content)
        assertEquals("18446744073709551615", token["maxSupply"]!!.jsonPrimitive.content)
        // Values are numeric JSON literals, not quoted strings.
        assertEquals(false, token["baseSupply"]!!.jsonPrimitive.isString)
        assertEquals(false, token["maxSupply"]!!.jsonPrimitive.isString)
    }
}
