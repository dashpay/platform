package org.dashfoundation.dashsdk.tokens

import java.math.BigInteger

private val U64_MAX_BIG_INTEGER = BigInteger("18446744073709551615")

/** Convert a protocol u64 to the unchanged signed `jlong` raw-bit carrier. */
internal fun ULong.toNativeLongBits(): Long = toLong()

/** Decode the unchanged signed `jlong` raw-bit carrier as a protocol u64. */
internal fun Long.fromNativeLongBits(): ULong = toULong()

/**
 * Java-accessible full-domain u64 conversion over the same raw `jlong`
 * carrier used by Kotlin and JNI. Java callers should use
 * [Tokens.javaAmounts] for token actions and can use these helpers when a
 * persisted/native raw carrier must be inspected directly.
 */
object TokenAmountInterop {
    @JvmStatic
    fun toRawLongBits(value: BigInteger): Long = value.toProtocolULong().toLong()

    @JvmStatic
    fun fromRawLongBits(rawBits: Long): BigInteger = BigInteger(rawBits.toULong().toString())
}

internal fun BigInteger.toProtocolULong(): ULong {
    require(signum() >= 0 && this <= U64_MAX_BIG_INTEGER) {
        "token amount must be in 0..$U64_MAX_BIG_INTEGER, got $this"
    }
    return toString().toULong()
}
