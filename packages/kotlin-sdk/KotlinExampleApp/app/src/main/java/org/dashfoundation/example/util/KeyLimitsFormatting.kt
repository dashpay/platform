package org.dashfoundation.example.util

import java.text.DateFormat
import java.text.DecimalFormatSymbols
import java.util.Calendar
import java.util.Date
import java.util.Locale
import java.util.TimeZone

/**
 * Renders a key's protocol 14 limits for the key screens, and turns what a person types or
 * picks into protocol units: port of `SwiftExampleApp/Utils/KeyLimitsFormatting.swift`.
 *
 * Display and input parsing only. Whether a key can be raised, whether an amount or an
 * expiry is acceptable, and which key signs are Rust's answers
 * (`IdentityUpdates.previewKeyLimitsUpdate`); nothing here refuses a zero amount or an
 * earlier expiry. Credits and milliseconds are `u64`, so they are [ULong] here.
 */
object KeyLimitsFormatting {
    /** 1 DASH = 100 000 000 duffs = 100 000 000 000 credits. */
    private const val CREDITS_PER_DASH: ULong = 100_000_000_000uL
    private const val CREDIT_DECIMALS = 11

    /** Credits rendered as DASH, trimming trailing zeros: 1_000_000_000 -> "0.01 DASH". */
    fun dash(credits: ULong): String = "${dashNumber(credits)} DASH"

    /** The DASH amount alone, at full credit precision: 1_000_000_000 -> "0.01". */
    fun dashNumber(credits: ULong): String {
        val whole = credits / CREDITS_PER_DASH
        val fraction = (credits % CREDITS_PER_DASH).toString()
            .padStart(CREDIT_DECIMALS, '0')
            .trimEnd('0')
        return if (fraction.isEmpty()) "$whole" else "$whole.$fraction"
    }

    /**
     * The inverse of [dashNumber]: DASH typed by a person, as credits. Exact digit
     * arithmetic, never floating point. The decimal separator is "." or [decimalSeparator]
     * (the locale's, which a decimal keyboard types). Surrounding whitespace is ignored.
     * Null for text that is not a plain unsigned decimal number, for more than 11 decimals
     * (finer than a credit), and for an amount past `u64` credits. Zero parses: whether it
     * is a raise is Rust's answer.
     */
    fun credits(
        fromDash: String,
        decimalSeparator: Char = DecimalFormatSymbols.getInstance().decimalSeparator,
    ): ULong? {
        var normalized = fromDash.trim()
        if (decimalSeparator != '.') normalized = normalized.replace(decimalSeparator, '.')
        val parts = normalized.split('.')
        if (parts.size !in 1..2) return null
        val wholeDigits = parts[0]
        val fractionDigits = if (parts.size == 2) parts[1] else ""
        if (wholeDigits.isEmpty() && fractionDigits.isEmpty()) return null
        if (!wholeDigits.all { it in '0'..'9' } || !fractionDigits.all { it in '0'..'9' }) return null
        if (fractionDigits.length > CREDIT_DECIMALS) return null

        val whole = if (wholeDigits.isEmpty()) 0uL else wholeDigits.toULongOrNull() ?: return null
        val fraction = fractionDigits.padEnd(CREDIT_DECIMALS, '0').toULong()
        if (whole > ULong.MAX_VALUE / CREDITS_PER_DASH) return null
        val wholeCredits = whole * CREDITS_PER_DASH
        if (fraction > ULong.MAX_VALUE - wholeCredits) return null
        return wholeCredits + fraction
    }

    /**
     * A picked instant as block time milliseconds, the unit of `expiresAt`. Null before
     * 1970, which the protocol cannot carry: the form says so instead of dropping the expiry.
     */
    fun millis(pickedMillis: Long): ULong? = if (pickedMillis >= 0) pickedMillis.toULong() else null

    /**
     * [currentMillis] moved to the calendar day [pickedDateUtcMillis] (a Material date
     * picker's selection: UTC midnight of that day), keeping its local time of day in [zone].
     */
    fun withDate(currentMillis: Long, pickedDateUtcMillis: Long, zone: TimeZone = TimeZone.getDefault()): Long {
        val day = Calendar.getInstance(UTC).apply { timeInMillis = pickedDateUtcMillis }
        return Calendar.getInstance(zone).apply {
            timeInMillis = currentMillis
            set(Calendar.YEAR, day.get(Calendar.YEAR))
            set(Calendar.DAY_OF_YEAR, 1)
            set(Calendar.MONTH, day.get(Calendar.MONTH))
            set(Calendar.DAY_OF_MONTH, day.get(Calendar.DAY_OF_MONTH))
        }.timeInMillis
    }

    /** [currentMillis] at [hour]:[minute]:00.000 local time in [zone], on the same local day. */
    fun withTime(currentMillis: Long, hour: Int, minute: Int, zone: TimeZone = TimeZone.getDefault()): Long =
        Calendar.getInstance(zone).apply {
            timeInMillis = currentMillis
            set(Calendar.HOUR_OF_DAY, hour)
            set(Calendar.MINUTE, minute)
            set(Calendar.SECOND, 0)
            set(Calendar.MILLISECOND, 0)
        }.timeInMillis

    /** The local calendar day of [millis] in [zone], as a Material date picker selects it. */
    fun dateUtcMillis(millis: Long, zone: TimeZone = TimeZone.getDefault()): Long {
        val local = Calendar.getInstance(zone).apply { timeInMillis = millis }
        return Calendar.getInstance(UTC).apply {
            clear()
            set(local.get(Calendar.YEAR), local.get(Calendar.MONTH), local.get(Calendar.DAY_OF_MONTH))
        }.timeInMillis
    }

    /** The local hour and minute of [millis] in [zone]. */
    fun timeOfDay(millis: Long, zone: TimeZone = TimeZone.getDefault()): Pair<Int, Int> {
        val local = Calendar.getInstance(zone).apply { timeInMillis = millis }
        return local.get(Calendar.HOUR_OF_DAY) to local.get(Calendar.MINUTE)
    }

    /**
     * What `Identities.fetchKeysRemainingBudgets` answered (`{"5": "1000", "6": null}`), by
     * key id: the credits left for a budgeted key, null for a key without a budget. Key ids
     * are read as `u32` and kept as their [Int] bit pattern, as Room stores them.
     */
    fun remainingBudgets(json: String?): Map<Int, ULong?> {
        if (json.isNullOrBlank()) return emptyMap()
        val obj = LenientJson.parseToJsonElement(json) as? kotlinx.serialization.json.JsonObject
            ?: return emptyMap()
        return obj.entries.mapNotNull { (key, value) ->
            val keyId = key.toUIntOrNull()?.toInt() ?: return@mapNotNull null
            val remaining = (value as? kotlinx.serialization.json.JsonPrimitive)
                ?.takeUnless { it is kotlinx.serialization.json.JsonNull }
                ?.content
                ?.toULongOrNull()
            keyId to remaining
        }.toMap()
    }

    private val UTC: TimeZone = TimeZone.getTimeZone("UTC")

    /** "0.004 of 0.01 DASH left" or, without a remaining figure, "0.01 DASH". */
    fun budget(total: ULong, remaining: ULong?): String =
        if (remaining == null) dash(total) else "${dashNumber(remaining)} of ${dash(total)} left"

    /**
     * Whether the key has expired at [nowMillis]. The expiry instant itself is already
     * expired, as consensus counts it. Display only: the preview checks the key against the
     * block time Platform proved.
     */
    fun isExpired(expiresAt: ULong, nowMillis: Long): Boolean =
        nowMillis >= 0 && nowMillis.toULong() >= expiresAt

    /** The expiry as an absolute local date and time. */
    fun expiryDate(
        expiresAt: ULong,
        locale: Locale = Locale.getDefault(),
        zone: TimeZone = TimeZone.getDefault(),
    ): String {
        val formatter = DateFormat.getDateTimeInstance(DateFormat.MEDIUM, DateFormat.SHORT, locale)
        formatter.timeZone = zone
        return formatter.format(Date(expiresAt.toLong()))
    }

    /** "Expires in 3 hours" or "Expired 2 days ago". */
    fun expiryRelative(expiresAt: ULong, nowMillis: Long): String {
        val relative = relative(expiresAt.toLong() - nowMillis)
        return if (isExpired(expiresAt, nowMillis)) "Expired $relative" else "Expires $relative"
    }

    /** "in 1 hour" / "2 days ago" in the largest whole unit, as Swift's relative formatter. */
    private fun relative(deltaMillis: Long): String {
        val seconds = kotlin.math.abs(deltaMillis) / 1000
        val (count, unit) = when {
            seconds >= YEAR -> seconds / YEAR to "year"
            seconds >= MONTH -> seconds / MONTH to "month"
            seconds >= WEEK -> seconds / WEEK to "week"
            seconds >= DAY -> seconds / DAY to "day"
            seconds >= HOUR -> seconds / HOUR to "hour"
            seconds >= MINUTE -> seconds / MINUTE to "minute"
            else -> seconds to "second"
        }
        val amount = "$count $unit${if (count == 1L) "" else "s"}"
        return if (deltaMillis > 0) "in $amount" else "$amount ago"
    }

    private const val MINUTE = 60L
    private const val HOUR = 60 * MINUTE
    private const val DAY = 24 * HOUR
    private const val WEEK = 7 * DAY
    private const val MONTH = 30 * DAY
    private const val YEAR = 365 * DAY
}
