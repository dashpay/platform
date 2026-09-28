package org.dashfoundation.example.ui.contracts

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonObject
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.queries.DocumentPropertyConstraint
import org.dashfoundation.dashsdk.queries.PropertyConstraintRead
import org.dashfoundation.dashsdk.queries.PropertyConstraintTotalRead
import org.dashfoundation.dashsdk.queries.PropertyConstraintViolation
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test

/**
 * Pins the screen-side `propertyConstraints` glue (protocol version 14) behind
 * the DocumentTypeDetailsScreen section and the CreateDocumentScreen
 * pre-check. Rust reads and judges the rules; these tests stand in for it
 * with plain lambdas, so they pin only when the app asks, what it does with
 * the answer, and that a check which cannot run never blocks a create
 * (consensus judges the document either way). Pure JVM: no native library.
 */
class PropertyConstraintsTest {

    private val schema = buildJsonObject {
        put("type", "object")
        putJsonObject("propertyConstraints") {
            putJsonObject("perUnitFee") { put("present", "fee") }
        }
    }

    private val plainSchema = buildJsonObject { put("type", "object") }

    private val stored = byteArrayOf(1, 2, 3)

    private val rule = DocumentPropertyConstraint(
        name = "sellerIsOwner",
        ruleJson = """{"anyOf":[{"absent":"sellerId"},{"equal":["sellerId","${'$'}ownerId"]}]}""",
        reads = listOf(
            PropertyConstraintRead("sellerId", PropertyConstraintRead.Kind.Presence),
            PropertyConstraintRead("sellerId", PropertyConstraintRead.Kind.Identifier),
            PropertyConstraintRead("sellerId", PropertyConstraintRead.Kind.Presence),
        ),
        readsOwner = true,
    )

    /** A rule reading system times and heights, one twice, as Rust lists them. */
    private val timedRule = DocumentPropertyConstraint(
        name = "settledAfterTransfer",
        ruleJson = """{"greaterThan":["${'$'}updatedAt","${'$'}transferredAtCoreBlockHeight"]}""",
        reads = emptyList(),
        readsOwner = false,
        readsSystem = listOf("${'$'}updatedAt", "${'$'}transferredAtCoreBlockHeight", "${'$'}updatedAt"),
    )

    private val violation = PropertyConstraintViolation(
        rule = "perUnitFee",
        violation = PropertyConstraintViolation.Kind.DivisionByZero,
        message = "it divides by zero",
    )

    @Test
    fun `should see the keyword only where the schema declares it`() {
        assertTrue(documentTypeDeclaresPropertyConstraints(schema))
        assertFalse(documentTypeDeclaresPropertyConstraints(plainSchema))
        assertFalse(documentTypeDeclaresPropertyConstraints(null))
    }

    // Document type screen

    @Test
    fun `should hide the section without asking Rust when the schema declares no rules`() = runTest {
        val section = loadPropertyConstraintsSection(plainSchema, stored) {
            fail("a schema without the keyword must not be read")
            emptyList()
        }

        assertEquals(PropertyConstraintsSection.Hidden, section)
    }

    @Test
    fun `should list the rules Rust reads from the stored contract`() = runTest {
        var readBytes: ByteArray? = null
        val section = loadPropertyConstraintsSection(schema, stored) { bytes ->
            readBytes = bytes
            listOf(rule)
        }

        assertEquals(PropertyConstraintsSection.Rules(listOf(rule)), section)
        assertArrayEquals(stored, readBytes)
    }

    /** Below protocol version 14 Rust reports no rules for a type declaring some. */
    @Test
    fun `should say the rules are not enforced when Rust reads none`() = runTest {
        val section = loadPropertyConstraintsSection(schema, stored) { emptyList() }

        assertEquals(PropertyConstraintsSection.NotEnforced, section)
    }

    @Test
    fun `should say why the rules cannot be read`() = runTest {
        val noSdk = loadPropertyConstraintsSection(schema, stored, read = null)
        assertEquals(
            PropertyConstraintsSection.Unavailable("Connect to a network to read the property constraints."),
            noSdk,
        )

        for (missing in listOf(null, ByteArray(0))) {
            val noBytes = loadPropertyConstraintsSection(schema, missing) {
                fail("nothing can be read without the stored serialization")
                emptyList()
            }
            assertTrue(
                "$noBytes",
                (noBytes as PropertyConstraintsSection.Unavailable).reason.contains("download it again"),
            )
        }

        val failed = loadPropertyConstraintsSection(schema, stored) {
            throw DashSdkError.SerializationError("Failed to deserialize contract")
        }
        assertEquals(
            PropertyConstraintsSection.Unavailable(
                "Could not read the property constraints: Failed to deserialize contract",
            ),
            failed,
        )
    }

    // Create pre-check

    @Test
    fun `should pass a document whose type declares no rules without asking Rust`() = runTest {
        val preCheck = propertyConstraintPreCheck(plainSchema, stored) {
            fail("a schema without the keyword must not be checked")
            null
        }

        assertEquals(PropertyConstraintPreCheck.Passed, preCheck)
    }

    @Test
    fun `should stop a document that breaks a rule`() = runTest {
        var checkedBytes: ByteArray? = null
        val preCheck = propertyConstraintPreCheck(schema, stored) { bytes ->
            checkedBytes = bytes
            violation
        }

        assertEquals(PropertyConstraintPreCheck.Broken(violation), preCheck)
        assertArrayEquals(stored, checkedBytes)
    }

    @Test
    fun `should pass a document that meets every rule`() = runTest {
        assertEquals(
            PropertyConstraintPreCheck.Passed,
            propertyConstraintPreCheck(schema, stored) { null },
        )
    }

    @Test
    fun `should let the create proceed when the check cannot run`() = runTest {
        assertEquals(
            PropertyConstraintPreCheck.Skipped("no SDK is connected"),
            propertyConstraintPreCheck(schema, stored, check = null),
        )
        for (missing in listOf(null, ByteArray(0))) {
            assertEquals(
                PropertyConstraintPreCheck.Skipped("the data contract has no stored serialization"),
                propertyConstraintPreCheck(schema, missing) {
                    fail("nothing can be checked without the stored serialization")
                    null
                },
            )
        }
        assertEquals(
            PropertyConstraintPreCheck.Skipped("Document type 'offer' not found in the data contract"),
            propertyConstraintPreCheck(schema, stored) {
                throw DashSdkError.NotFound("Document type 'offer' not found in the data contract")
            },
        )
    }

    /** An app bundling a native library built before the two exports must neither crash nor block. */
    @Test
    fun `should neither crash nor block on a native library without the exports`() = runTest {
        val missing = UnsatisfiedLinkError("dataContractCheckPropertyConstraints")

        assertEquals(
            PropertyConstraintPreCheck.Skipped(NATIVE_LIBRARY_PREDATES_RULES),
            propertyConstraintPreCheck(schema, stored) { throw missing },
        )
        assertEquals(
            PropertyConstraintsSection.Unavailable(NATIVE_LIBRARY_PREDATES_RULES),
            loadPropertyConstraintsSection(schema, stored) { throw missing },
        )
    }

    @Test
    fun `should not swallow a cancellation`() = runTest {
        try {
            propertyConstraintPreCheck(schema, stored) { throw CancellationException("left the screen") }
            fail("the cancellation must propagate")
        } catch (e: CancellationException) {
            assertEquals("left the screen", e.message)
        }
    }

    // Display

    @Test
    fun `should list each read once with its kind`() {
        assertEquals("sellerId (presence), sellerId (identifier)", propertyConstraintReadsText(rule))
        assertEquals(
            "title (length), tags (count), labels (elements)",
            propertyConstraintReadsText(
                DocumentPropertyConstraint(
                    name = "sizes",
                    ruleJson = "{}",
                    reads = listOf(
                        PropertyConstraintRead("title", PropertyConstraintRead.Kind.Length),
                        PropertyConstraintRead("tags", PropertyConstraintRead.Kind.Count),
                        PropertyConstraintRead("labels", PropertyConstraintRead.Kind.Elements),
                    ),
                    readsOwner = false,
                ),
            ),
        )
    }

    @Test
    fun `should list each system value a rule reads once`() {
        assertEquals(
            "Reads ${'$'}updatedAt, ${'$'}transferredAtCoreBlockHeight",
            propertyConstraintSystemReadsText(timedRule),
        )
    }

    @Test
    fun `should show no system line for a rule reading none`() {
        assertNull(propertyConstraintSystemReadsText(rule))
    }

    /** A rule reading a count by owner twice, a whole-type count and a sum by category. */
    private val totalledRule = DocumentPropertyConstraint(
        name = "withinLimits",
        ruleJson = "{}",
        reads = emptyList(),
        readsOwner = true,
        readsTotals = listOf(
            PropertyConstraintTotalRead(
                PropertyConstraintTotalRead.Kind.CountOf,
                "listing",
                null,
                listOf("${'$'}ownerId"),
            ),
            PropertyConstraintTotalRead(
                PropertyConstraintTotalRead.Kind.SumOf,
                "listing",
                "price",
                listOf("category", "status"),
            ),
            PropertyConstraintTotalRead(
                PropertyConstraintTotalRead.Kind.CountOf,
                "listing",
                null,
                listOf("${'$'}ownerId"),
            ),
            PropertyConstraintTotalRead(
                PropertyConstraintTotalRead.Kind.CountOf,
                "listing",
                null,
                emptyList(),
            ),
        ),
    )

    /** Each total once, its filter keys after "by"; the SwiftExampleApp builds the same text. */
    @Test
    fun `should list each total a rule reads once`() {
        assertEquals(
            "Reads totals: countOf listing by ${'$'}ownerId; sumOf price of listing by " +
                "category, status; countOf listing",
            propertyConstraintTotalsText(totalledRule),
        )
        assertNull(propertyConstraintTotalsText(timedRule))
    }

    /** The note under a rule reading a total; the SwiftExampleApp shows the same text. */
    @Test
    fun `should say the check before sending cannot catch a rule reading a total`() {
        assertEquals(
            "The platform reads these totals when the document is sent; the check before " +
                "sending does not, so it cannot catch this rule.",
            PROPERTY_CONSTRAINT_TOTALS_NOTE,
        )
    }

    /** The note under a rule reading a system value; the SwiftExampleApp shows the same text. */
    @Test
    fun `should say which writes the update and transfer times answer to`() {
        assertEquals(
            "A price update is judged against the rules reading ${'$'}updatedAt or its block " +
                "heights, and a transfer or purchase against those reading ${'$'}transferredAt or " +
                "its block heights.",
            PROPERTY_CONSTRAINT_SYSTEM_READS_NOTE,
        )
    }

    @Test
    fun `should name the rule the violation and the reason in the alert`() {
        assertEquals(
            "Rule: perUnitFee\nViolation: DivisionByZero\nReason: it divides by zero",
            propertyConstraintViolationAlert(violation),
        )
        assertEquals(
            "Rule: r\nViolation: Later\nReason: m",
            propertyConstraintViolationAlert(
                PropertyConstraintViolation("r", PropertyConstraintViolation.Kind.Other("Later"), "m"),
            ),
        )
    }
}
