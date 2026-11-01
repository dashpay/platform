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

/**
 * The account a shielded tip is paid from. Rust decides which account an
 * identity's source selects, from the slot the wallet seed proves it holds.
 */
sealed class ShieldedTipSource {
    /** The identity's dedicated tip account. */
    class Identity(identityId: ByteArray) : ShieldedTipSource() {
        private val identityBytes = identityId.copyOf()
        val identityId: ByteArray get() = identityBytes.copyOf()
        init {
            require(identityBytes.size == 32) { "identity id must be 32 bytes, got ${identityBytes.size}" }
        }
        override fun equals(other: Any?): Boolean =
            other is Identity && identityBytes.contentEquals(other.identityBytes)
        override fun hashCode(): Int = identityBytes.contentHashCode()
    }

    /** An ordinary account, or the tip account of a retired identity. */
    data class Account(val index: Int) : ShieldedTipSource() {
        init {
            require(index >= 0) { "account must be non-negative, got $index" }
        }
    }
}
