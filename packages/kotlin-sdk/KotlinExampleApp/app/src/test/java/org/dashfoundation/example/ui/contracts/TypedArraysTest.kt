package org.dashfoundation.example.ui.contracts

import java.math.BigInteger
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.add
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonArray
import kotlinx.serialization.json.putJsonObject
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test

/**
 * Pins the protocol-v14 typed array helpers behind the create and replace
 * forms ([documentTypedArray], [typedArrayElement], [typedArrayJson],
 * [typedArraySeedRows], [buildPropertiesJson]). The Rust sanitize step turns
 * base58 and hex strings into identifiers and bytes inside typed array
 * elements, but it does not turn the string "5" into an integer, so the form
 * must send each element as a JSON value of its own kind.
 */
class TypedArraysTest {

    private val identifierItems = buildJsonObject {
        put("type", "array")
        put("byteArray", true)
        put("minItems", 32)
        put("maxItems", 32)
        put("contentMediaType", "application/x.dash.dpp.identifier")
    }

    /** A charter-like document type with one typed array of every element kind. */
    private val schema: JsonObject = buildJsonObject {
        put("type", "object")
        putJsonObject("properties") {
            putJsonObject("members") {
                put("type", "array")
                put("items", identifierItems)
                put("maxItems", 16)
                put("uniqueItems", true)
                put("position", 0)
            }
            putJsonObject("scores") {
                put("type", "array")
                putJsonObject("items") {
                    put("type", "integer")
                    put("minimum", 0)
                    put("maximum", 100)
                }
                put("minItems", 1)
                put("maxItems", 8)
                put("position", 1)
            }
            putJsonObject("ratios") {
                put("type", "array")
                putJsonObject("items") {
                    put("type", "number")
                    put("minimum", 0)
                    put("maximum", 1)
                }
                put("maxItems", 4)
                put("position", 2)
            }
            putJsonObject("flags") {
                put("type", "array")
                putJsonObject("items") { put("type", "boolean") }
                put("maxItems", 4)
                put("position", 3)
            }
            putJsonObject("tags") {
                put("type", "array")
                putJsonObject("items") {
                    put("type", "string")
                    putJsonArray("enum") {
                        add("spam")
                        add("abuse")
                    }
                }
                put("maxItems", 2)
                put("position", 4)
            }
            putJsonObject("digests") {
                put("type", "array")
                putJsonObject("items") {
                    put("type", "array")
                    put("byteArray", true)
                    put("minItems", 4)
                    put("maxItems", 4)
                }
                put("maxItems", 4)
                put("position", 5)
            }
            putJsonObject("avatar") {
                put("type", "array")
                put("byteArray", true)
                put("maxItems", 64)
                put("position", 6)
            }
            putJsonObject("team") {
                put("type", "object")
                putJsonObject("properties") {
                    putJsonObject("leads") {
                        put("type", "array")
                        put("items", identifierItems)
                        put("maxItems", 4)
                        put("position", 0)
                    }
                }
                put("position", 7)
            }
        }
    }

    private val properties = schema["properties"]!!.jsonObject

    private fun typedArray(name: String): DocumentTypedArray =
        documentTypedArray(name, properties[name]!!.jsonObject)!!

    private val identifierA = Base58.encode(ByteArray(32) { 1 })
    private val identifierB = Base58.encode(ByteArray(32) { 2 })

    @Test
    fun shouldReadEveryElementKindWithItsBounds() {
        assertEquals(
            DocumentTypedArray("members", TypedArrayItem.IdentifierItem, null, 16, true),
            typedArray("members"),
        )
        assertEquals(
            DocumentTypedArray(
                "scores",
                TypedArrayItem.IntegerItem(BigInteger.ZERO, BigInteger.valueOf(100)),
                1,
                8,
                false,
            ),
            typedArray("scores"),
        )
        assertEquals(TypedArrayItem.NumberItem(0.0, 1.0), typedArray("ratios").items)
        assertEquals(TypedArrayItem.BooleanItem(), typedArray("flags").items)
        assertEquals(
            TypedArrayItem.StringItem(allowedValues = listOf("spam", "abuse")),
            typedArray("tags").items,
        )
        assertEquals(TypedArrayItem.ByteArrayItem(4, 4), typedArray("digests").items)
    }

    @Test
    fun shouldNotReadAByteArrayPropertyOrAPlainArrayAsATypedArray() {
        assertNull(documentTypedArray("avatar", properties["avatar"]!!.jsonObject))
        assertNull(documentTypedArray("plain", buildJsonObject { put("type", "array") }))
        assertNull(documentTypedArray("text", buildJsonObject { put("type", "string") }))
    }

    @Test
    fun shouldListEveryTypedArrayIncludingNestedOnesByDottedPath() {
        assertEquals(
            listOf("digests", "flags", "members", "ratios", "scores", "tags", "team.leads"),
            documentTypedArrays(schema).map { it.path },
        )
        assertTrue(documentTypedArrays(null).isEmpty())
    }

    @Test
    fun shouldConvertEachElementKindToAJsonValueOfItsOwnKind() {
        fun valid(item: TypedArrayItem, raw: String): JsonPrimitive =
            (typedArrayElement(item, raw) as TypedArrayElement.Valid).json

        assertEquals(JsonPrimitive(BigInteger.valueOf(42)), valid(typedArray("scores").items, " 42 "))
        assertEquals(JsonPrimitive(0.5), valid(typedArray("ratios").items, "0.5"))
        assertEquals(JsonPrimitive(true), valid(typedArray("flags").items, "true"))
        assertEquals(JsonPrimitive("spam"), valid(typedArray("tags").items, "spam"))
        assertEquals(JsonPrimitive(identifierA), valid(TypedArrayItem.IdentifierItem, identifierA))
        assertEquals(JsonPrimitive("deadbeef"), valid(typedArray("digests").items, "0xDEADBEEF"))
        // Numbers and booleans go out unquoted: sanitize will not parse strings.
        assertTrue(!valid(typedArray("scores").items, "7").isString)
        assertTrue(!valid(typedArray("flags").items, "false").isString)
    }

    @Test
    fun shouldRefuseElementTextConsensusWouldRefuse() {
        fun reason(item: TypedArrayItem, raw: String): String =
            (typedArrayElement(item, raw) as TypedArrayElement.Invalid).reason

        val scores = typedArray("scores").items
        assertEquals("not a whole number", reason(scores, "4.5"))
        assertEquals("above the maximum 100", reason(scores, "101"))
        assertEquals("below the minimum 0", reason(scores, "-1"))
        assertEquals("above the maximum 1.0", reason(typedArray("ratios").items, "1.5"))
        assertEquals("not true or false", reason(typedArray("flags").items, "yes"))
        assertEquals("not one of spam, abuse", reason(typedArray("tags").items, "other"))
        assertEquals("not a base58 identifier", reason(TypedArrayItem.IdentifierItem, "0OIl"))
        assertEquals("not hex bytes", reason(typedArray("digests").items, "xyz"))
        assertEquals("fewer than 4 bytes", reason(typedArray("digests").items, "dead"))
        assertEquals(
            "longer than 3 characters",
            reason(TypedArrayItem.StringItem(maxLength = 3), "four"),
        )
    }

    @Test
    fun shouldBuildTheArrayAndRefuseCountsRepeatsAndBadRows() {
        assertEquals(
            buildJsonArray {
                add(identifierA)
                add(identifierB)
            },
            typedArrayJson("members", typedArray("members"), listOf(identifierA, identifierB)),
        )
        assertRefused("scores needs at least 1 items, has 0") {
            typedArrayJson("scores", typedArray("scores"), emptyList())
        }
        assertRefused("tags allows at most 2 items, has 3") {
            typedArrayJson("tags", typedArray("tags"), listOf("spam", "spam", "abuse"))
        }
        assertRefused("members must not repeat an element: $identifierA") {
            typedArrayJson("members", typedArray("members"), listOf(identifierA, identifierA))
        }
        assertRefused("scores[1]: not a whole number") {
            typedArrayJson("scores", typedArray("scores"), listOf("1", "x"))
        }
    }

    @Test
    fun shouldSendTypedArraysFromTheListRowsAndOmitAnEmptyOptionalOne() {
        val json = buildPropertiesJson(
            properties = properties,
            required = setOf("scores"),
            textValues = emptyMap(),
            boolValues = emptyMap(),
            touchedBools = emptySet(),
            listValues = mapOf(
                "scores" to listOf("3", "99"),
                "flags" to listOf("true", "false"),
                "tags" to emptyList(),
            ),
        )
        val sent = LenientJson.parseToJsonElement(json).jsonObject
        assertEquals(
            buildJsonArray {
                add(3)
                add(99)
            },
            sent["scores"],
        )
        assertEquals(
            buildJsonArray {
                add(true)
                add(false)
            },
            sent["flags"],
        )
        assertEquals(setOf("scores", "flags"), sent.keys)
    }

    @Test
    fun shouldSeedReplaceRowsFromTheStoredDocumentWithBytesAsHex() {
        val stored = buildJsonArray {
            add("3q2+7w==")
            add("AAECAw==")
        }
        assertEquals(
            listOf("deadbeef", "00010203"),
            typedArraySeedRows(typedArray("digests").items, stored),
        )
        assertEquals(
            listOf("3", "99"),
            typedArraySeedRows(
                typedArray("scores").items,
                buildJsonArray {
                    add(3)
                    add(99)
                },
            ),
        )
        assertNull(typedArraySeedRows(typedArray("scores").items, JsonPrimitive(3)))
        assertNull(
            typedArraySeedRows(
                typedArray("scores").items,
                JsonArray(listOf(buildJsonObject { put("a", 1) })),
            ),
        )
    }

    @Test
    fun shouldStartNewRowsOnTheFirstEnumValueOrFalse() {
        assertEquals("spam", typedArrayNewRow(typedArray("tags").items))
        assertEquals("false", typedArrayNewRow(typedArray("flags").items))
        assertEquals("", typedArrayNewRow(typedArray("scores").items))
    }

    private fun assertRefused(message: String, block: () -> Unit) {
        try {
            block()
            fail("expected a refusal: $message")
        } catch (e: IllegalArgumentException) {
            assertEquals(message, e.message)
        }
    }
}
