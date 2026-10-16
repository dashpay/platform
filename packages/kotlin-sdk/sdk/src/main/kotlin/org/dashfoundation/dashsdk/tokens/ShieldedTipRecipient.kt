package org.dashfoundation.dashsdk.tokens

/** Verified recipient snapshot to bind payment confirmation to an identity and address. */
class ShieldedTipRecipient(identityId: ByteArray, address: ByteArray) {
    private val identityBytes = identityId.copyOf()
    private val addressBytes = address.copyOf()
    val identityId: ByteArray get() = identityBytes.copyOf()
    val address: ByteArray get() = addressBytes.copyOf()
    init {
        require(identityBytes.size == 32)
        require(addressBytes.size == 43)
    }
    override fun equals(other: Any?): Boolean = other is ShieldedTipRecipient &&
        identityBytes.contentEquals(other.identityBytes) && addressBytes.contentEquals(other.addressBytes)
    override fun hashCode(): Int = 31 * identityBytes.contentHashCode() + addressBytes.contentHashCode()
}
