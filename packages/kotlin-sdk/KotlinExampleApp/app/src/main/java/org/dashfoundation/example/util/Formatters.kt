package org.dashfoundation.example.util

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import java.text.DateFormat
import java.util.Date
import java.util.Locale

/**
 * Display formatters shared by the contracts and storage screens — the
 * Kotlin counterparts of `AppDate`, `ByteCountFormatter`, and the
 * truncate-middle helpers scattered through the SwiftExampleApp views.
 */

/** Lenient JSON instance shared by the contract/document screens. */
@OptIn(kotlinx.serialization.ExperimentalSerializationApi::class)
val LenientJson: Json = Json {
    ignoreUnknownKeys = true
    isLenient = true
    prettyPrint = true
    prettyPrintIndent = "  "
}

fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }

fun String.hexToBytes(): ByteArray =
    chunked(2).map { it.toInt(16).toByte() }.toByteArray()

/** `abcdefgh...uvwxyz` middle truncation (← `truncateMiddle` in ContractsTabView.swift). */
fun truncateMiddle(s: String, head: Int = 8, tail: Int = 6): String {
    if (s.length <= head + tail + 1) return s
    return "${s.take(head)}...${s.takeLast(tail)}"
}

/** Binary byte-count string (← `ByteCountFormatter`, `.binary` count style). */
fun formatByteCount(count: Long): String {
    if (count < 1024) return "$count bytes"
    val kb = count / 1024.0
    if (kb < 1024) return String.format(Locale.US, "%.1f KB", kb)
    return String.format(Locale.US, "%.1f MB", kb / 1024.0)
}

/** Static relative-time string (← `AppDate.relative`; never auto-refreshes). */
fun formatRelative(date: Date): String {
    val deltaSeconds = (System.currentTimeMillis() - date.time) / 1000
    return when {
        deltaSeconds < 0 -> "in the future"
        deltaSeconds < 60 -> "just now"
        deltaSeconds < 3_600 -> "${deltaSeconds / 60}m ago"
        deltaSeconds < 86_400 -> "${deltaSeconds / 3_600}h ago"
        else -> "${deltaSeconds / 86_400}d ago"
    }
}

/** Abbreviated date+time (← `AppDate.formatted(dateStyle:timeStyle:)`). */
fun formatDate(date: Date): String =
    DateFormat.getDateTimeInstance(DateFormat.MEDIUM, DateFormat.SHORT).format(date)

/**
 * Duffs → "1,234.5678 DASH" (1 DASH = 1e8 duffs) — the Kotlin counterpart
 * of the `formatBalance` helpers in WalletDetailView / AccountListView.
 */
fun formatDuffs(duffs: Long): String = formatDashAmount(duffs.toDouble() / 100_000_000.0)

/**
 * Platform credits → DASH (1 DASH = 1e11 credits) — counterpart of the
 * `formatCredits` helpers in WalletDetailView / CoreContentView.
 */
fun formatCredits(credits: Long): String =
    formatDashAmount(credits.toDouble() / 100_000_000_000.0)

private fun formatDashAmount(dash: Double): String {
    val formatter = java.text.DecimalFormat("#,##0.########")
    return "${formatter.format(dash)} DASH"
}

/**
 * Decimal DASH string → duffs, `Decimal`-backed like Swift's
 * `parseTokenAmount(_:decimals: 8)` (SendViewModel.swift) so "0.0001"
 * deterministically yields exactly 10_000. Returns null when the text is
 * unparseable, non-positive, has more than 8 fractional digits, or
 * overflows [Long].
 */
fun parseDashToDuffs(text: String): Long? = try {
    val duffs = java.math.BigDecimal(text.trim())
        .movePointRight(8)
        .toBigIntegerExact()
    if (duffs.signum() > 0 && duffs.bitLength() < 63) duffs.toLong() else null
} catch (_: NumberFormatException) {
    null
} catch (_: ArithmeticException) {
    null
}

/**
 * Decimal DASH string → Platform credits (1 DASH = 1e11), the credits-scale
 * sibling of [parseDashToDuffs] — Swift's `parseTokenAmount(_:decimals: 11)`
 * backing `SendViewModel.amountCredits`. Used by every send flow that
 * settles on the credits ledger (shielded transfer / unshield / withdraw).
 * Returns null when the text is unparseable, non-positive, has more than 11
 * fractional digits, or overflows [Long].
 */
fun parseDashToCredits(text: String): Long? = try {
    val credits = java.math.BigDecimal(text.trim())
        .movePointRight(11)
        .toBigIntegerExact()
    if (credits.signum() > 0 && credits.bitLength() < 63) credits.toLong() else null
} catch (_: NumberFormatException) {
    null
} catch (_: ArithmeticException) {
    null
}

/** Pretty-print a JSON string; returns the input unchanged when unparseable. */
fun prettyPrintJson(raw: String): String = try {
    LenientJson.encodeToString(JsonElement.serializer(), LenientJson.parseToJsonElement(raw))
} catch (_: Exception) {
    raw
}
