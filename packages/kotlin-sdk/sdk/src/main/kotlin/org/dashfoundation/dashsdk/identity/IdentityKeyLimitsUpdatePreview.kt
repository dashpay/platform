package org.dashfoundation.dashsdk.identity

/**
 * What raising one key's limits (protocol version 14) would do, as the wallet answers it:
 * port of Swift's `ManagedPlatformWallet.IdentityKeyLimitsUpdatePreview`
 * (`Sources/SwiftDashSDK/PlatformWallet/ManagedPlatformWallet.swift`).
 *
 * Every value is Rust's (`platform_wallet::IdentityKeyLimitsUpdatePreview`). A preview asked
 * with no change carries the key's current limits and the key that would sign; one asked
 * with a change also carries the absolute limits the update would set. Hand it back to
 * [IdentityUpdates.updateKeyLimitsAsReviewed] to sign exactly what it shows.
 *
 * Credits and milliseconds are `u64` on the wire and [ULong] here, so no value is narrowed.
 *
 * @property identityId the identity whose key the update raises (32 bytes).
 * @property keyId the key the update raises.
 * @property signingKeyId the key that would sign the update.
 * @property currentTotalBudget the total budget the key carries now, in credits.
 * @property currentExpiresAt the expiry the key carries now, block time in milliseconds.
 * @property newTotalBudget the total budget the update would set, in credits.
 * @property newExpiresAt the expiry the update would set, block time in milliseconds.
 */
class IdentityKeyLimitsUpdatePreview(
    identityId: ByteArray,
    val keyId: Int,
    val signingKeyId: Int,
    val currentTotalBudget: ULong?,
    val currentExpiresAt: ULong?,
    val newTotalBudget: ULong?,
    val newExpiresAt: ULong?,
) {
    init {
        require(identityId.size == 32) { "identityId must be 32 bytes, got ${identityId.size}" }
    }

    private val identityIdBytes: ByteArray = identityId.copyOf()

    /** A copy of the identity id, so a caller cannot change the preview it reviewed. */
    val identityId: ByteArray get() = identityIdBytes.copyOf()

    /** The six JNI slots this preview crosses back in (layout on [fromSlots]). */
    internal fun toSlots(): LongArray {
        var flags = 0L
        if (currentTotalBudget != null) flags = flags or HAS_CURRENT_TOTAL_BUDGET
        if (currentExpiresAt != null) flags = flags or HAS_CURRENT_EXPIRES_AT
        if (newTotalBudget != null) flags = flags or HAS_NEW_TOTAL_BUDGET
        if (newExpiresAt != null) flags = flags or HAS_NEW_EXPIRES_AT
        return longArrayOf(
            signingKeyId.toUInt().toLong(),
            flags,
            (currentTotalBudget ?: 0uL).toLong(),
            (currentExpiresAt ?: 0uL).toLong(),
            (newTotalBudget ?: 0uL).toLong(),
            (newExpiresAt ?: 0uL).toLong(),
        )
    }

    override fun equals(other: Any?): Boolean =
        other is IdentityKeyLimitsUpdatePreview &&
            identityIdBytes.contentEquals(other.identityIdBytes) &&
            keyId == other.keyId &&
            signingKeyId == other.signingKeyId &&
            currentTotalBudget == other.currentTotalBudget &&
            currentExpiresAt == other.currentExpiresAt &&
            newTotalBudget == other.newTotalBudget &&
            newExpiresAt == other.newExpiresAt

    override fun hashCode(): Int {
        var result = identityIdBytes.contentHashCode()
        result = 31 * result + keyId
        result = 31 * result + signingKeyId
        result = 31 * result + (currentTotalBudget?.hashCode() ?: 0)
        result = 31 * result + (currentExpiresAt?.hashCode() ?: 0)
        result = 31 * result + (newTotalBudget?.hashCode() ?: 0)
        result = 31 * result + (newExpiresAt?.hashCode() ?: 0)
        return result
    }

    override fun toString(): String =
        "IdentityKeyLimitsUpdatePreview(keyId=$keyId, signingKeyId=$signingKeyId, " +
            "currentTotalBudget=$currentTotalBudget, currentExpiresAt=$currentExpiresAt, " +
            "newTotalBudget=$newTotalBudget, newExpiresAt=$newExpiresAt)"

    internal companion object {
        const val SLOTS = 6
        const val HAS_CURRENT_TOTAL_BUDGET = 1L
        const val HAS_CURRENT_EXPIRES_AT = 1L shl 1
        const val HAS_NEW_TOTAL_BUDGET = 1L shl 2
        const val HAS_NEW_EXPIRES_AT = 1L shl 3
        private const val KNOWN_FLAGS =
            HAS_CURRENT_TOTAL_BUDGET or HAS_CURRENT_EXPIRES_AT or HAS_NEW_TOTAL_BUDGET or
                HAS_NEW_EXPIRES_AT

        /**
         * Decode the slots `IdentityKeyLimitsNative.previewKeyLimitsUpdate` returns for key
         * [keyId] of [identityId]: `[signingKeyId, flags, currentTotalBudget,
         * currentExpiresAt, newTotalBudget, newExpiresAt]`, each the `u32`/`u64` bit
         * pattern, with `flags` bits 0..3 saying which of the four limits are present (in
         * that order). Mirrors `preview_slots` in `rs-unified-sdk-jni/src/identity_key_limits.rs`.
         *
         * @throws IllegalArgumentException for slots of another shape, a signing key id past
         *   `u32`, or a flag bit this build does not know.
         */
        fun fromSlots(identityId: ByteArray, keyId: Int, slots: LongArray): IdentityKeyLimitsUpdatePreview {
            require(slots.size == SLOTS) { "a key limits preview has $SLOTS slots, got ${slots.size}" }
            val signingKeyId = slots[0]
            require(signingKeyId in 0L..UInt.MAX_VALUE.toLong()) {
                "signing key id $signingKeyId is not a u32"
            }
            val flags = slots[1]
            require(flags and KNOWN_FLAGS.inv() == 0L) { "unknown key limits preview flags $flags" }
            fun value(flag: Long, slot: Int): ULong? =
                if (flags and flag != 0L) slots[slot].toULong() else null
            return IdentityKeyLimitsUpdatePreview(
                identityId = identityId,
                keyId = keyId,
                signingKeyId = signingKeyId.toUInt().toInt(),
                currentTotalBudget = value(HAS_CURRENT_TOTAL_BUDGET, 2),
                currentExpiresAt = value(HAS_CURRENT_EXPIRES_AT, 3),
                newTotalBudget = value(HAS_NEW_TOTAL_BUDGET, 4),
                newExpiresAt = value(HAS_NEW_EXPIRES_AT, 5),
            )
        }
    }
}
