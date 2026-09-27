package org.dashfoundation.dashsdk.queries

import kotlinx.serialization.json.Json
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Coverage for the decoding of what the two protocol-version-14
 * `propertyConstraints` natives return
 * (`dash_sdk_data_contract_get_property_constraints` and
 * `dash_sdk_data_contract_check_property_constraints`).
 *
 * Rust parses and evaluates the rules (rs-sdk-ffi's own tests cover every
 * rule family and violation); Kotlin only decodes, so these tests pin the
 * JSON shapes, which match wasm-dpp2's and the Swift SDK's key for key
 * (`DocumentPropertyConstraintsTests.swift` holds the same cases). Pure JVM:
 * no native library is loaded.
 */
class DocumentPropertyConstraintsTest {

    /**
     * The rules of an `offer` type as the FFI reports them (rs-sdk-ffi's
     * `should_list_every_rule_in_name_order_with_what_it_reads`), abridged to
     * three rules.
     */
    private val rulesJson = """
        [
          {
            "name": "closedNeedsClosedAt",
            "readsOwner": false,
            "reads": [
              { "kind": "text", "path": "status" },
              { "kind": "presence", "path": "closedAt" }
            ],
            "rule": {
              "anyOf": [
                { "notEqual": ["status", { "const": "closed" }] },
                { "present": "closedAt" }
              ]
            }
          },
          {
            "name": "perUnitFee",
            "readsOwner": false,
            "reads": [
              { "kind": "value", "path": "price" },
              { "kind": "value", "path": "fee" }
            ],
            "rule": { "greaterThanOrEqual": [{ "divide": ["price", "fee"] }, 1] }
          },
          {
            "name": "sellerIsOwner",
            "readsOwner": true,
            "reads": [
              { "kind": "presence", "path": "sellerId" },
              { "kind": "identifier", "path": "sellerId" }
            ],
            "rule": {
              "anyOf": [{ "absent": "sellerId" }, { "equal": ["sellerId", "${'$'}ownerId"] }]
            }
          }
        ]
    """.trimIndent()

    // Rules

    @Test
    fun `should decode the rules in order with what they read`() {
        val rules = DocumentPropertyConstraint.listFromJson(rulesJson)

        assertEquals(listOf("closedNeedsClosedAt", "perUnitFee", "sellerIsOwner"), rules.map { it.name })
        assertEquals(
            listOf(
                PropertyConstraintRead("status", PropertyConstraintRead.Kind.Text),
                PropertyConstraintRead("closedAt", PropertyConstraintRead.Kind.Presence),
            ),
            rules[0].reads,
        )
        assertEquals(
            listOf(PropertyConstraintRead.Kind.Value, PropertyConstraintRead.Kind.Value),
            rules[1].reads.map { it.kind },
        )
        assertEquals(
            listOf(PropertyConstraintRead.Kind.Presence, PropertyConstraintRead.Kind.Identifier),
            rules[2].reads.map { it.kind },
        )
        assertEquals(listOf(false, false, true), rules.map { it.readsOwner })
    }

    /**
     * The rule is kept as the JSON the schema declares, compact with sorted
     * keys: array order, and so operand order, is untouched.
     */
    @Test
    fun `should keep each rule as its declared JSON`() {
        val rules = DocumentPropertyConstraint.listFromJson(rulesJson)

        assertEquals(
            """{"anyOf":[{"notEqual":["status",{"const":"closed"}]},{"present":"closedAt"}]}""",
            rules[0].ruleJson,
        )
        assertEquals("""{"greaterThanOrEqual":[{"divide":["price","fee"]},1]}""", rules[1].ruleJson)
        assertEquals(
            """{"anyOf":[{"absent":"sellerId"},{"equal":["sellerId","${'$'}ownerId"]}]}""",
            rules[2].ruleJson,
        )
    }

    @Test
    fun `should sort the keys of a rule declaring several`() {
        val rules = DocumentPropertyConstraint.listFromJson(
            """[{"name":"r","readsOwner":false,"reads":[],"rule":{"b":[2],"a":{"d":1,"c":0}}}]""",
        )

        assertEquals("""{"a":{"c":0,"d":1},"b":[2]}""", rules.single().ruleJson)
    }

    @Test
    fun `should indent the same rule for display`() {
        val rule = DocumentPropertyConstraint.listFromJson(rulesJson).first()

        val pretty = rule.prettyRuleJson
        assertTrue(pretty, pretty.contains("\n"))
        assertEquals(Json.parseToJsonElement(rule.ruleJson), Json.parseToJsonElement(pretty))
    }

    @Test
    fun `should show rule text that does not parse as it is`() {
        val rule = DocumentPropertyConstraint("r", "not json", emptyList(), readsOwner = false)

        assertEquals("not json", rule.prettyRuleJson)
    }

    @Test
    fun `should decode no rules to an empty list`() {
        assertEquals(emptyList<DocumentPropertyConstraint>(), DocumentPropertyConstraint.listFromJson("[]"))
    }

    /** A kind added by a later protocol version is kept by name rather than failing the whole list. */
    @Test
    fun `should keep an unknown read kind by its name`() {
        val rules = DocumentPropertyConstraint.listFromJson(
            """
            [{ "name": "r", "rule": { "present": "a" }, "readsOwner": false,
               "reads": [{ "path": "a", "kind": "somethingNew" }] }]
            """.trimIndent(),
        )

        val kind = rules.single().reads.single().kind
        assertEquals(PropertyConstraintRead.Kind.Other("somethingNew"), kind)
        assertEquals("somethingNew", kind.name)
    }

    @Test
    fun `should round trip every read kind name`() {
        val names = listOf("value", "presence", "text", "identifier")
        val kinds = listOf(
            PropertyConstraintRead.Kind.Value,
            PropertyConstraintRead.Kind.Presence,
            PropertyConstraintRead.Kind.Text,
            PropertyConstraintRead.Kind.Identifier,
        )

        assertEquals(kinds, names.map(PropertyConstraintRead.Kind::fromName))
        assertEquals(names, kinds.map { it.name })
    }

    @Test
    fun `should refuse malformed rules`() {
        val malformed = listOf(
            "not json",
            """{"name": "r"}""",
            """[{"name": "r", "rule": {"present": "a"}, "reads": []}]""",
            // A number is not a boolean
            """[{"name": "r", "rule": {"present": "a"}, "reads": [], "readsOwner": 1}]""",
            // Nor is a string
            """[{"name": "r", "rule": {"present": "a"}, "reads": [], "readsOwner": "true"}]""",
            // A name must be a string
            """[{"name": 7, "rule": {"present": "a"}, "reads": [], "readsOwner": false}]""",
            """[{"name": "r", "rule": {"present": "a"}, "reads": [{"path": "a"}], "readsOwner": false}]""",
            """[7]""",
        )
        for (json in malformed) {
            assertThrows(json, DashSdkError.SerializationError::class.java) {
                DocumentPropertyConstraint.listFromJson(json)
            }
        }
    }

    // Violations

    @Test
    fun `should decode a violation`() {
        val violation = PropertyConstraintViolation.fromJson(
            """{ "rule": "perUnitFee", "violation": "DivisionByZero", "message": "it divides by zero" }""",
        )

        assertEquals(
            PropertyConstraintViolation(
                rule = "perUnitFee",
                violation = PropertyConstraintViolation.Kind.DivisionByZero,
                message = "it divides by zero",
            ),
            violation,
        )
        assertEquals(
            "The document breaks the propertyConstraints rule \"perUnitFee\" (DivisionByZero): " +
                "it divides by zero.",
            violation?.description,
        )
    }

    @Test
    fun `should round trip every violation name`() {
        val names = listOf("NotMet", "Overflow", "DivisionByZero", "NegativeExponent", "NotAnInteger")
        val kinds = listOf(
            PropertyConstraintViolation.Kind.NotMet,
            PropertyConstraintViolation.Kind.Overflow,
            PropertyConstraintViolation.Kind.DivisionByZero,
            PropertyConstraintViolation.Kind.NegativeExponent,
            PropertyConstraintViolation.Kind.NotAnInteger,
        )

        assertEquals(kinds, names.map(PropertyConstraintViolation.Kind::fromName))
        assertEquals(names, kinds.map { it.name })
        assertEquals(
            PropertyConstraintViolation.Kind.Other("Later"),
            PropertyConstraintViolation.Kind.fromName("Later"),
        )
    }

    @Test
    fun `should read null as every rule holding`() {
        assertNull(PropertyConstraintViolation.fromJson("null"))
    }

    @Test
    fun `should refuse malformed violations`() {
        for (json in listOf("", "[]", "\"NotMet\"", """{"rule": "r", "violation": "NotMet"}""")) {
            assertThrows(json, DashSdkError.SerializationError::class.java) {
                PropertyConstraintViolation.fromJson(json)
            }
        }
    }
}
