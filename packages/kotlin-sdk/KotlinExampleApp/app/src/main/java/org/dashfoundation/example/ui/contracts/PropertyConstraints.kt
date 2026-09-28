package org.dashfoundation.example.ui.contracts

import kotlinx.coroutines.CancellationException
import kotlinx.serialization.json.JsonObject
import org.dashfoundation.dashsdk.queries.DocumentPropertyConstraint
import org.dashfoundation.dashsdk.queries.PropertyConstraintTotalRead
import org.dashfoundation.dashsdk.queries.PropertyConstraintViolation

// Screen-side glue for a document type's `propertyConstraints` rules
// (protocol version 14): named conditions every created or replaced document
// must meet, refused by consensus with error 10422 while the fee is still
// charged. Rust parses and judges the rules (`sdk.contracts.propertyConstraints`
// and `sdk.contracts.checkPropertyConstraints`); nothing here evaluates one.
// Port of the SwiftExampleApp's `DocumentTypeDetailsView` section and
// `CreateDocumentView` pre-check.

/**
 * Whether [schema] carries the `propertyConstraints` keyword. Says nothing
 * about the rules themselves: those are read by Rust.
 */
internal fun documentTypeDeclaresPropertyConstraints(schema: JsonObject?): Boolean =
    schema?.get("propertyConstraints") != null

/** What the document type screen shows for its rules. */
internal sealed interface PropertyConstraintsSection {
    /** Nothing: the schema declares no rules. */
    data object Hidden : PropertyConstraintsSection

    /** The rules Rust read, in name order (the order consensus checks them in). */
    data class Rules(val rules: List<DocumentPropertyConstraint>) : PropertyConstraintsSection

    /**
     * The schema declares rules, but the SDK's protocol version enforces none
     * (they take effect at protocol version 14).
     */
    data object NotEnforced : PropertyConstraintsSection

    /** The rules could not be read, with why. */
    data class Unavailable(val reason: String) : PropertyConstraintsSection
}

/**
 * Read the rules of a document type with [schema] from its contract's stored
 * platform serialization, [serializedContract], through [read] (null when no
 * SDK is connected).
 */
internal suspend fun loadPropertyConstraintsSection(
    schema: JsonObject?,
    serializedContract: ByteArray?,
    read: (suspend (serializedContract: ByteArray) -> List<DocumentPropertyConstraint>)?,
): PropertyConstraintsSection {
    if (!documentTypeDeclaresPropertyConstraints(schema)) return PropertyConstraintsSection.Hidden
    if (read == null) {
        return PropertyConstraintsSection.Unavailable(
            "Connect to a network to read the property constraints.",
        )
    }
    if (serializedContract == null || serializedContract.isEmpty()) {
        return PropertyConstraintsSection.Unavailable(
            "The data contract has no stored serialization; download it again to read " +
                "the property constraints.",
        )
    }
    return try {
        val rules = read(serializedContract)
        if (rules.isEmpty()) {
            PropertyConstraintsSection.NotEnforced
        } else {
            PropertyConstraintsSection.Rules(rules)
        }
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        PropertyConstraintsSection.Unavailable("Could not read the property constraints: ${e.message}")
    } catch (_: UnsatisfiedLinkError) {
        PropertyConstraintsSection.Unavailable(NATIVE_LIBRARY_PREDATES_RULES)
    }
}

/** The outcome of checking a document's rules before it is broadcast. */
internal sealed interface PropertyConstraintPreCheck {
    /** The document meets every rule, or its type declares none: send it. */
    data object Passed : PropertyConstraintPreCheck

    /** The document breaks [violation]'s rule: do not send it. */
    data class Broken(val violation: PropertyConstraintViolation) : PropertyConstraintPreCheck

    /**
     * The check could not run, for [reason]. The create goes ahead: consensus
     * judges the document either way.
     */
    data class Skipped(val reason: String) : PropertyConstraintPreCheck
}

/**
 * Judge a document to create, of a type with [schema], against its rules
 * through [check] (null when no SDK is connected), reading the contract's
 * stored platform serialization [serializedContract].
 */
internal suspend fun propertyConstraintPreCheck(
    schema: JsonObject?,
    serializedContract: ByteArray?,
    check: (suspend (serializedContract: ByteArray) -> PropertyConstraintViolation?)?,
): PropertyConstraintPreCheck {
    if (!documentTypeDeclaresPropertyConstraints(schema)) return PropertyConstraintPreCheck.Passed
    if (check == null) return PropertyConstraintPreCheck.Skipped("no SDK is connected")
    if (serializedContract == null || serializedContract.isEmpty()) {
        return PropertyConstraintPreCheck.Skipped("the data contract has no stored serialization")
    }
    return try {
        check(serializedContract)
            ?.let { PropertyConstraintPreCheck.Broken(it) }
            ?: PropertyConstraintPreCheck.Passed
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        PropertyConstraintPreCheck.Skipped(e.message ?: e.toString())
    } catch (_: UnsatisfiedLinkError) {
        PropertyConstraintPreCheck.Skipped(NATIVE_LIBRARY_PREDATES_RULES)
    }
}

/**
 * Why nothing could be read or checked when the bundled native library was
 * built before the two `propertyConstraints` exports: a stale `.so` must not
 * crash the screen or block a create.
 */
internal const val NATIVE_LIBRARY_PREDATES_RULES =
    "The native library predates property constraints; rebuild it to read them."

/** Each property [rule] reads with how it reads it, repeats dropped: `price (value), fee (value)`. */
internal fun propertyConstraintReadsText(rule: DocumentPropertyConstraint): String =
    rule.reads.distinct().joinToString(", ") { "${it.path} (${it.kind.name})" }

/**
 * The system times and heights [rule] reads, repeats dropped, as the line
 * shown under it: `Reads $createdAt, $updatedAt`. `null` for a rule reading
 * none.
 */
internal fun propertyConstraintSystemReadsText(rule: DocumentPropertyConstraint): String? =
    rule.readsSystem.distinct().takeIf { it.isNotEmpty() }?.joinToString(", ", prefix = "Reads ")

/**
 * The note under a rule reading a system time or height
 * ([propertyConstraintSystemReadsText] not `null`): which writes besides a
 * create or a replace consensus judges against such a rule. Only a reminder
 * of the protocol's behaviour, the same for every such rule: Rust decides.
 */
internal const val PROPERTY_CONSTRAINT_SYSTEM_READS_NOTE =
    "A price update is judged against the rules reading \$updatedAt or its block heights, " +
        "and a transfer or purchase against those reading \$transferredAt or its block heights."

/**
 * The line naming the `countOf` and `sumOf` totals [rule] reads, each once:
 * `Reads totals: countOf listing by $ownerId; sumOf price of listing by category`.
 * `null` for a rule reading none. The SwiftExampleApp builds the same text.
 */
internal fun propertyConstraintTotalsText(rule: DocumentPropertyConstraint): String? =
    rule.readsTotals
        .map(::totalText)
        .distinct()
        .takeIf { it.isNotEmpty() }
        ?.joinToString("; ", prefix = "Reads totals: ")

/**
 * One total as [propertyConstraintTotalsText] names it: its kind, the property
 * it sums (a `sumOf` only), its type, and the keys it filters by.
 */
private fun totalText(total: PropertyConstraintTotalRead): String = buildString {
    append(total.kind.name)
    total.property?.let { append(" $it of") }
    append(" ${total.documentType}")
    if (total.filter.isNotEmpty()) append(" by ${total.filter.joinToString(", ")}")
}

/**
 * The note under a rule reading a total ([propertyConstraintTotalsText] not
 * `null`): the check before sending reads no state, so it cannot catch the
 * rule. The same text as the SwiftExampleApp's, for cross-platform UAT.
 */
internal const val PROPERTY_CONSTRAINT_TOTALS_NOTE =
    "The platform reads these totals when the document is sent; the check before sending " +
        "does not, so it cannot catch this rule."

/** Title of the alert a broken rule raises instead of a broadcast. */
internal const val PROPERTY_CONSTRAINT_BROKEN_TITLE = "Not sent: a property constraint is broken"

/** Body of the alert a broken rule raises: the rule, the violation and the reason. */
internal fun propertyConstraintViolationAlert(violation: PropertyConstraintViolation): String =
    "Rule: ${violation.rule}\nViolation: ${violation.violation.name}\nReason: ${violation.message}"
