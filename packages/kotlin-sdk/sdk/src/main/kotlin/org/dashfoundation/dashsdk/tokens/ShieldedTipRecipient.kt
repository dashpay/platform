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

/** Rust's decision whether a failed tip may be sent again. */
enum class ShieldedTipDisposition {
    /** Nothing was broadcast, or it was definitively rejected: review and send again. */
    NOT_SENT,

    /** The payment may have executed or may still execute: do not send it again. */
    UNRESOLVED;

    internal companion object {
        /** The native `SHIELDED_TIP_DISPOSITION_NOT_SENT`; every other value is unresolved. */
        const val NATIVE_NOT_SENT = 1

        fun fromNative(value: Int): ShieldedTipDisposition =
            if (value == NATIVE_NOT_SENT) NOT_SENT else UNRESOLVED
    }
}

/** A tip send that failed; [disposition] says whether it may be sent again. */
class ShieldedTipFailure(val disposition: ShieldedTipDisposition, cause: Throwable) :
    Exception(cause.message, cause)

/** How a tip recipient compares with the one last confirmed for the same name from this wallet. */
sealed class ShieldedTipRecipientChange {
    object New : ShieldedTipRecipientChange()
    object Unchanged : ShieldedTipRecipientChange()
    class Changed(val previous: ShieldedTipRecipient) : ShieldedTipRecipientChange()
}

/**
 * What a user confirms before tipping a name. [unresolved] means an earlier payment to
 * this address is unsettled, and a send is refused until shielded sync settles it.
 */
class ShieldedTipReview(
    val recipient: ShieldedTipRecipient,
    val change: ShieldedTipRecipientChange,
    val unresolved: Boolean,
) {
    internal companion object {
        fun fromNative(bytes: ByteArray): ShieldedTipReview {
            check(bytes.size == 152) { "Invalid tip review response" }
            val recipient = ShieldedTipRecipient(bytes.copyOfRange(0, 32), bytes.copyOfRange(32, 75))
            val change = when (bytes[75].toInt()) {
                0 -> ShieldedTipRecipientChange.New
                1 -> ShieldedTipRecipientChange.Unchanged
                else -> ShieldedTipRecipientChange.Changed(
                    ShieldedTipRecipient(bytes.copyOfRange(76, 108), bytes.copyOfRange(108, 151)),
                )
            }
            return ShieldedTipReview(recipient, change, bytes[151].toInt() != 0)
        }
    }
}

/**
 * A wallet identity's dedicated tip account. Show its balance as the identity's only
 * when [verified]: unverified, the account may belong to another identity.
 */
data class IdentityTipAccount(val account: Int, val verified: Boolean)

/** An identity's tip balance as the wallet can vouch for it; decided by Rust. */
sealed class IdentityTipBalance {
    /** This session has not proved the identity holds its slot. */
    object Unverified : IdentityTipBalance()

    /** Verified, but the account's ledger is not established yet. */
    data class Unavailable(val account: Int) : IdentityTipBalance()

    /** Spendable credits of the identity's tip account. */
    data class Available(val account: Int, val credits: Long) : IdentityTipBalance()

    internal companion object {
        fun fromNative(values: LongArray): IdentityTipBalance = when (values[0]) {
            2L -> Available(values[1].toInt(), values[2])
            1L -> Unavailable(values[1].toInt())
            else -> Unverified
        }
    }
}
