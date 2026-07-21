package org.dashfoundation.dashsdk.persistence

/**
 * A protocol `u64` stored as an order-preserving, fixed-width big-endian BLOB.
 *
 * SQLite has no unsigned 64-bit integer type: storing the raw bits in an
 * `INTEGER` makes values in `2^63..u64::MAX` negative and breaks predicates
 * and ordering. The eight-byte representation used here sorts in the same
 * order as [ULong] when SQLite compares BLOBs lexicographically.
 */
data class UInt64Value(val value: ULong) : Comparable<UInt64Value> {
    override fun compareTo(other: UInt64Value): Int = value.compareTo(other.value)

    fun toBigEndianBytes(): ByteArray = ByteArray(SIZE_BYTES) { index ->
        (value shr ((SIZE_BYTES - 1 - index) * Byte.SIZE_BITS)).toByte()
    }

    /** Raw signed carrier used only at the JNI boundary. */
    fun toRawLongBits(): Long = value.toLong()

    companion object {
        const val SIZE_BYTES: Int = Long.SIZE_BYTES
        val ZERO: UInt64Value = UInt64Value(0u)

        fun fromRawLongBits(value: Long): UInt64Value = UInt64Value(value.toULong())

        fun fromBigEndianBytes(bytes: ByteArray): UInt64Value {
            require(bytes.size == SIZE_BYTES) {
                "u64 storage must contain exactly $SIZE_BYTES bytes, got ${bytes.size}"
            }
            var value = 0uL
            bytes.forEach { byte -> value = (value shl Byte.SIZE_BITS) or byte.toUByte().toULong() }
            return UInt64Value(value)
        }
    }
}
