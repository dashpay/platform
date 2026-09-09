package org.dashfoundation.dashsdk.persistence.converters

import androidx.room.TypeConverter
import org.dashfoundation.dashsdk.persistence.UInt64Value
import java.util.Date

/**
 * Room type converters for the Dash SDK persistence layer.
 *
 * Deliberately minimal — the entities were designed so that almost every
 * Swift type maps to a Room-native column type without conversion:
 *
 * - Swift `Data` → [ByteArray] (BLOB, stored verbatim; bincode / consensus /
 *   JSON blobs are passthrough and never re-encoded on this side).
 * - Swift `String` → [String].
 * - Swift `Bool` → [Boolean].
 * - Swift integer types → [Int] / [Long] (unsigned semantics documented on
 *   each field).
 * - Swift Codable token-rule structs → JSON [String] columns (UI-only
 *   payloads, see TokenEntity).
 *
 * `Date` is converted to epoch milliseconds. Protocol `u64` values use
 * [UInt64Value]'s fixed-width big-endian BLOB representation so the complete
 * unsigned domain round-trips and remains correctly ordered by SQLite.
 */
class Converters {

    /** Swift `Date` → epoch millis (`Long`). `null` round-trips as `null`. */
    @TypeConverter
    fun dateToEpochMillis(date: Date?): Long? = date?.time

    /** Epoch millis (`Long`) → `Date`. `null` round-trips as `null`. */
    @TypeConverter
    fun epochMillisToDate(millis: Long?): Date? = millis?.let { Date(it) }

    @TypeConverter
    fun uint64ToBigEndianBlob(value: UInt64Value?): ByteArray? = value?.toBigEndianBytes()

    @TypeConverter
    fun bigEndianBlobToUInt64(bytes: ByteArray?): UInt64Value? =
        bytes?.let(UInt64Value::fromBigEndianBytes)
}
