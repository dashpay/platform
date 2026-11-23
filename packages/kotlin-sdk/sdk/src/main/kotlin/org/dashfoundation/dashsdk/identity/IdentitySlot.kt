package org.dashfoundation.dashsdk.identity

/**
 * An identity slot the wallet leased to a new registration
 * ([IdentityRegistration.acquireRegistrationSlot]): derive the identity keys
 * at [identityIndex] and pass the slot to the registration call, or release
 * it when the registration is abandoned. The lease lives in memory only.
 *
 * @property slotToken opaque: the bits Rust handed out.
 */
data class RegistrationSlot(
    val identityIndex: Int,
    val slotToken: Long,
)

/**
 * The identity slot the wallet leased to a claim of one invitation
 * ([IdentityRegistration.acquireInvitationClaimSlot]). The lease is stored
 * durably and keyed by the invitation's funding outpoint, so a retried claim
 * of the same invitation gets the same [identityIndex].
 *
 * @property invitationOutPoint the invitation's funding outpoint: 32-byte raw
 *   txid, then the 4-byte little-endian vout.
 * @property submitted an earlier attempt of this claim may have reached
 *   Platform; the slot is then freed only by the claim's outcome.
 */
class InvitationClaimSlot(
    val identityIndex: Int,
    invitationOutPoint: ByteArray,
    val submitted: Boolean,
) {
    private val outPoint = invitationOutPoint.copyOf()

    val invitationOutPoint: ByteArray get() = outPoint.copyOf()

    init {
        require(identityIndex >= 0) { "identityIndex must be non-negative, got $identityIndex" }
        require(outPoint.size == OUT_POINT_SIZE) {
            "invitationOutPoint must be $OUT_POINT_SIZE bytes, got ${outPoint.size}"
        }
    }

    internal companion object {
        private const val OUT_POINT_SIZE = 36
        private const val SIZE = 4 + OUT_POINT_SIZE + 1

        /**
         * Decode the blob `IdentityNative.acquireInvitationClaimSlot`
         * returns: `u32` identity index (big-endian), the 36-byte outpoint,
         * then `u8` submitted.
         */
        fun decode(blob: ByteArray): InvitationClaimSlot {
            require(blob.size == SIZE) { "claim slot blob must be $SIZE bytes, got ${blob.size}" }
            val index = (0 until 4).fold(0L) { acc, i -> (acc shl 8) or (blob[i].toLong() and 0xFF) }
            require(index <= Int.MAX_VALUE) { "identityIndex $index exceeds Int range" }
            return InvitationClaimSlot(
                identityIndex = index.toInt(),
                invitationOutPoint = blob.copyOfRange(4, 4 + OUT_POINT_SIZE),
                submitted = blob[SIZE - 1].toInt() != 0,
            )
        }
    }
}
