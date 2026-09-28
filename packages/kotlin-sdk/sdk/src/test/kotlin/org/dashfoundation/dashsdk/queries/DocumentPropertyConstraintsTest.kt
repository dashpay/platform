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
            "readsSystem": [],
            "rule": {
              "ifThen": [
                { "equal": ["status", { "const": "closed" }] },
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
            "readsSystem": [],
            "rule": { "greaterThanOrEqual": [{ "divide": ["price", "fee"] }, 1] }
          },
          {
            "name": "sellerIsOwner",
            "readsOwner": true,
            "reads": [
              { "kind": "presence", "path": "sellerId" },
              { "kind": "identifier", "path": "sellerId" }
            ],
            "readsSystem": [],
            "rule": {
              "anyOf": [{ "absent": "sellerId" }, { "equal": ["sellerId", "${'$'}ownerId"] }]
            }
          }
        ]
    """.trimIndent()

    /**
     * Rules reading sizes and the elements of an array, as the FFI reports
     * them (wasm-dpp2's `DocumentPropertyConstraints.spec.ts` holds the same
     * rules): a `byteLength` operand reads a string's size, a `count` operand
     * an array's items, and a `contains` the array it looks in.
     */
    private val sizeAndElementRulesJson = """
        [
          {
            "name": "notUsed",
            "readsOwner": false,
            "reads": [{ "kind": "elements", "path": "labels" }],
            "readsSystem": [],
            "rule": { "not": { "contains": ["labels", { "const": "used" }] } }
          },
          {
            "name": "tagsWithinLimit",
            "readsOwner": false,
            "reads": [
              { "kind": "count", "path": "tags" },
              { "kind": "value", "path": "maxTags" }
            ],
            "readsSystem": [],
            "rule": { "lessThanOrEqual": [{ "count": "tags" }, "maxTags"] }
          },
          {
            "name": "titleBytes",
            "readsOwner": false,
            "reads": [{ "kind": "length", "path": "title" }],
            "readsSystem": [],
            "rule": { "lessThanOrEqual": [{ "byteLength": "title" }, 12] }
          }
        ]
    """.trimIndent()

    /**
     * Rules reading system times and heights (rs-sdk-ffi's
     * `should_read_the_clock_for_system_times_and_skip_block_heights`), plus
     * one reading an update time twice and a Core height, in declared order.
     */
    private val systemRulesJson = """
        [
          {
            "name": "endsAfterCreation",
            "readsOwner": false,
            "reads": [{ "kind": "value", "path": "endsAt" }],
            "readsSystem": ["${'$'}createdAt"],
            "rule": { "greaterThan": ["endsAt", "${'$'}createdAt"] }
          },
          {
            "name": "listedAfterHeight10",
            "readsOwner": false,
            "reads": [],
            "readsSystem": ["${'$'}createdAtBlockHeight"],
            "rule": { "greaterThanOrEqual": ["${'$'}createdAtBlockHeight", 10] }
          },
          {
            "name": "settledAfterTransfer",
            "readsOwner": false,
            "reads": [],
            "readsSystem": [
              "${'$'}updatedAt",
              "${'$'}transferredAtCoreBlockHeight",
              "${'$'}updatedAt"
            ],
            "rule": {
              "allOf": [
                { "greaterThan": ["${'$'}updatedAt", 0] },
                { "greaterThan": ["${'$'}transferredAtCoreBlockHeight", 0] },
                { "lessThan": ["${'$'}updatedAt", 4102444800000] }
              ]
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
        assertEquals(List(3) { emptyList<String>() }, rules.map { it.readsSystem })
    }

    @Test
    fun `should decode size and element reads as length count and elements`() {
        val rules = DocumentPropertyConstraint.listFromJson(sizeAndElementRulesJson)

        assertEquals(listOf("notUsed", "tagsWithinLimit", "titleBytes"), rules.map { it.name })
        assertEquals(
            listOf(PropertyConstraintRead("labels", PropertyConstraintRead.Kind.Elements)),
            rules[0].reads,
        )
        assertEquals(
            listOf(
                PropertyConstraintRead("tags", PropertyConstraintRead.Kind.Count),
                PropertyConstraintRead("maxTags", PropertyConstraintRead.Kind.Value),
            ),
            rules[1].reads,
        )
        assertEquals(
            listOf(PropertyConstraintRead("title", PropertyConstraintRead.Kind.Length)),
            rules[2].reads,
        )
        assertEquals("""{"lessThanOrEqual":[{"byteLength":"title"},12]}""", rules[2].ruleJson)
    }

    /** The names come through as Rust reports them, in declared order, a repeat kept. */
    @Test
    fun `should decode the system times and heights each rule reads`() {
        val rules = DocumentPropertyConstraint.listFromJson(systemRulesJson)

        assertEquals(
            listOf(
                listOf("${'$'}createdAt"),
                listOf("${'$'}createdAtBlockHeight"),
                listOf("${'$'}updatedAt", "${'$'}transferredAtCoreBlockHeight", "${'$'}updatedAt"),
            ),
            rules.map { it.readsSystem },
        )
        assertEquals(listOf(PropertyConstraintRead("endsAt", PropertyConstraintRead.Kind.Value)), rules[0].reads)
        assertEquals(emptyList<PropertyConstraintRead>(), rules[1].reads)
        assertEquals(listOf(false, false, false), rules.map { it.readsOwner })
    }

    /**
     * An `ifThenElse` reads what every branch reads, whichever a document
     * takes: the descriptor rs-sdk-ffi's
     * `should_report_every_branch_of_an_if_then_else_and_judge_the_one_taken`
     * reports, the owner read in the then branch and the creation time in the
     * else branch.
     */
    @Test
    fun `should decode what every branch of an ifThenElse reads`() {
        val rule = DocumentPropertyConstraint.listFromJson(
            """
            [
              {
                "name": "openEndedSoldByOwner",
                "readsOwner": true,
                "reads": [
                  { "kind": "presence", "path": "endsAt" },
                  { "kind": "identifier", "path": "sellerId" },
                  { "kind": "value", "path": "endsAt" }
                ],
                "readsSystem": ["${'$'}createdAt"],
                "rule": {
                  "ifThenElse": [
                    { "absent": "endsAt" },
                    { "equal": ["sellerId", "${'$'}ownerId"] },
                    { "greaterThan": ["endsAt", "${'$'}createdAt"] }
                  ]
                }
              }
            ]
            """.trimIndent(),
        ).single()

        assertEquals("openEndedSoldByOwner", rule.name)
        assertEquals(
            listOf(
                PropertyConstraintRead("endsAt", PropertyConstraintRead.Kind.Presence),
                PropertyConstraintRead("sellerId", PropertyConstraintRead.Kind.Identifier),
                PropertyConstraintRead("endsAt", PropertyConstraintRead.Kind.Value),
            ),
            rule.reads,
        )
        assertTrue(rule.readsOwner)
        assertEquals(listOf("${'$'}createdAt"), rule.readsSystem)
        assertEquals(
            """{"ifThenElse":[{"absent":"endsAt"},{"equal":["sellerId","${'$'}ownerId"]},""" +
                """{"greaterThan":["endsAt","${'$'}createdAt"]}]}""",
            rule.ruleJson,
        )
    }

    /** A native library built before `readsSystem` leaves the key out. */
    @Test
    fun `should read a rule without readsSystem as reading no system value`() {
        val rules = DocumentPropertyConstraint.listFromJson(
            """[{"name":"r","readsOwner":false,"reads":[],"rule":{"present":"a"}}]""",
        )

        assertEquals(emptyList<String>(), rules.single().readsSystem)
        assertEquals(
            DocumentPropertyConstraint("r", """{"present":"a"}""", emptyList(), readsOwner = false),
            rules.single(),
        )
    }

    /**
     * The rule is kept as the JSON the schema declares, compact with sorted
     * keys: array order, and so operand order, is untouched.
     */
    @Test
    fun `should keep each rule as its declared JSON`() {
        val rules = DocumentPropertyConstraint.listFromJson(rulesJson)

        assertEquals(
            """{"ifThen":[{"equal":["status",{"const":"closed"}]},{"present":"closedAt"}]}""",
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
        val names = listOf("value", "presence", "text", "identifier", "length", "count", "elements")
        val kinds = listOf(
            PropertyConstraintRead.Kind.Value,
            PropertyConstraintRead.Kind.Presence,
            PropertyConstraintRead.Kind.Text,
            PropertyConstraintRead.Kind.Identifier,
            PropertyConstraintRead.Kind.Length,
            PropertyConstraintRead.Kind.Count,
            PropertyConstraintRead.Kind.Elements,
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

    /** Present, `readsSystem` must be an array of strings; only a missing key means none. */
    @Test
    fun `should refuse a malformed readsSystem`() {
        val malformed = listOf(
            // A single name is not an array
            "\"\$createdAt\"",
            "null",
            "true",
            """{"${'$'}createdAt": true}""",
            // Each entry must be a string
            "[1]",
            "[null]",
            """["${'$'}createdAt", 7]""",
            """[["${'$'}createdAt"]]""",
        )
        for (readsSystem in malformed) {
            val json =
                """[{"name":"r","readsOwner":false,"reads":[],"readsSystem":$readsSystem,"rule":{"present":"a"}}]"""
            assertThrows(json, DashSdkError.SerializationError::class.java) {
                DocumentPropertyConstraint.listFromJson(json)
            }
        }
    }

    /**
     * The totals rules read, as rs-sdk-ffi's
     * `should_list_the_totals_a_rule_reads_and_leave_it_unjudged` reports them:
     * a whole-type count, a count by owner, a sum by category, and none.
     */
    private val totalRulesJson = """
        [
          {
            "name": "allListings",
            "readsOwner": false,
            "reads": [],
            "readsSystem": [],
            "readsTotals": [{ "kind": "countOf", "documentType": "listing", "filter": [] }],
            "rule": { "lessThan": [{ "countOf": ["listing"] }, 1000] }
          },
          {
            "name": "atMostTwoPerOwner",
            "readsOwner": true,
            "reads": [],
            "readsSystem": [],
            "readsTotals": [
              { "kind": "countOf", "documentType": "listing", "filter": ["${'$'}ownerId"] }
            ],
            "rule": {
              "lessThanOrEqual": [{ "countOf": ["listing", { "${'$'}ownerId": "${'$'}ownerId" }] }, 2]
            }
          },
          {
            "name": "categoryBudget",
            "readsOwner": false,
            "reads": [{ "kind": "value", "path": "category" }],
            "readsSystem": [],
            "readsTotals": [
              {
                "kind": "sumOf",
                "documentType": "listing",
                "property": "price",
                "filter": ["category"]
              }
            ],
            "rule": {
              "lessThanOrEqual": [{ "sumOf": ["listing", "price", { "category": "category" }] }, 250]
            }
          },
          {
            "name": "priceCap",
            "readsOwner": false,
            "reads": [{ "kind": "value", "path": "price" }],
            "readsSystem": [],
            "readsTotals": [],
            "rule": { "lessThanOrEqual": ["price", 1000] }
          }
        ]
    """.trimIndent()

    // Totals

    @Test
    fun `should decode the totals each rule reads`() {
        val rules = DocumentPropertyConstraint.listFromJson(totalRulesJson)

        assertEquals(
            listOf("allListings", "atMostTwoPerOwner", "categoryBudget", "priceCap"),
            rules.map { it.name },
        )
        assertEquals(
            listOf(
                listOf(
                    PropertyConstraintTotalRead(
                        PropertyConstraintTotalRead.Kind.CountOf,
                        "listing",
                        null,
                        emptyList(),
                    ),
                ),
                listOf(
                    PropertyConstraintTotalRead(
                        PropertyConstraintTotalRead.Kind.CountOf,
                        "listing",
                        null,
                        listOf("${'$'}ownerId"),
                    ),
                ),
                listOf(
                    PropertyConstraintTotalRead(
                        PropertyConstraintTotalRead.Kind.SumOf,
                        "listing",
                        "price",
                        listOf("category"),
                    ),
                ),
                emptyList(),
            ),
            rules.map { it.readsTotals },
        )
        assertEquals(listOf(false, true, false, false), rules.map { it.readsOwner })
        assertEquals(
            listOf(PropertyConstraintRead("category", PropertyConstraintRead.Kind.Value)),
            rules[2].reads,
        )
    }

    /** A native library built before `readsTotals` leaves the key out. */
    @Test
    fun `should read a rule without readsTotals as reading no total`() {
        val rules = DocumentPropertyConstraint.listFromJson(
            """[{"name":"r","readsOwner":false,"reads":[],"readsSystem":[],"rule":{"present":"a"}}]""",
        )

        assertEquals(emptyList<PropertyConstraintTotalRead>(), rules.single().readsTotals)
        assertEquals(
            DocumentPropertyConstraint("r", """{"present":"a"}""", emptyList(), readsOwner = false),
            rules.single(),
        )
    }

    @Test
    fun `should round trip every total kind name and keep an unknown one`() {
        for (kind in listOf(PropertyConstraintTotalRead.Kind.CountOf, PropertyConstraintTotalRead.Kind.SumOf)) {
            assertEquals(kind, PropertyConstraintTotalRead.Kind.fromName(kind.name))
        }
        assertEquals(
            PropertyConstraintTotalRead.Kind.Other("averageOf"),
            PropertyConstraintTotalRead.Kind.fromName("averageOf"),
        )
    }

    @Test
    fun `should refuse a malformed readsTotals`() {
        val malformed = listOf(
            "null",
            "{}",
            "[1]",
            "[null]",
            // Every total names its kind, type and filter
            """[{"documentType":"listing","filter":[]}]""",
            """[{"kind":"countOf","filter":[]}]""",
            """[{"kind":"countOf","documentType":"listing"}]""",
            // The filter lists strings, and a property is one
            """[{"kind":"countOf","documentType":"listing","filter":"${'$'}ownerId"}]""",
            """[{"kind":"countOf","documentType":"listing","filter":[7]}]""",
            """[{"kind":"sumOf","documentType":"listing","property":7,"filter":[]}]""",
            """[{"kind":7,"documentType":"listing","filter":[]}]""",
        )
        for (readsTotals in malformed) {
            val json =
                """[{"name":"r","readsOwner":false,"reads":[],"readsSystem":[],"readsTotals":$readsTotals,"rule":{"present":"a"}}]"""
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
