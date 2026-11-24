package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.BlobReader

/**
 * Whom a shielded tip to a username pays, as Rust resolved it
 * (`ShieldedTipRecipient` in
 * `packages/rs-platform-wallet/src/wallet/shielded/tip_payment.rs`). A review
 * ([PlatformWalletManager.reviewShieldedTip]) returns it and a send
 * ([PlatformWalletManager.sendShieldedTip]) takes it back unchanged, so a
 * name or a profile that changed in between cannot redirect the payment.
 *
 * A snapshot: the bytes are copied in at construction and copied out on
 * every read, so what a send passes to Rust is what the review returned,
 * whatever the caller did with its own arrays since.
 *
 * @param identityId the 32-byte id of the identity the username resolves to.
 * @param address the raw 43-byte Orchard address (11-byte diversifier,
 *   32-byte `pk_d`) that identity's profile publishes.
 */
class ShieldedTipRecipient(
    identityId: ByteArray,
    address: ByteArray,
) {
    private val identityIdBytes: ByteArray = identityId.copyOf()
    private val addressBytes: ByteArray = address.copyOf()

    /** A copy of the 32-byte identity id. */
    val identityId: ByteArray get() = identityIdBytes.copyOf()

    /** A copy of the raw 43-byte Orchard address. */
    val address: ByteArray get() = addressBytes.copyOf()

    override fun equals(other: Any?): Boolean =
        other is ShieldedTipRecipient &&
            identityIdBytes.contentEquals(other.identityIdBytes) &&
            addressBytes.contentEquals(other.addressBytes)

    override fun hashCode(): Int =
        31 * identityIdBytes.contentHashCode() + addressBytes.contentHashCode()

    override fun toString(): String =
        "ShieldedTipRecipient(identityId=${identityIdBytes.toHex()}, " +
            "address=${addressBytes.toHex()})"

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }
}

/**
 * How a tip recipient compares with the one this wallet last confirmed for
 * the same name. Rust makes the comparison; nothing here compares recipients.
 *
 * A recipient is confirmed when a send to it is admitted, before the
 * transfer is built: none of these says a tip was paid.
 */
sealed interface ShieldedTipRecipientChange {
    /** No recipient was confirmed for the name from this wallet before. */
    data object New : ShieldedTipRecipientChange

    /** The same identity and address as last confirmed. */
    data object Unchanged : ShieldedTipRecipientChange

    /**
     * The name now resolves to another identity, or its profile publishes
     * another address.
     *
     * @param previous the recipient last confirmed for the name.
     */
    data class Changed(val previous: ShieldedTipRecipient) : ShieldedTipRecipientChange
}

/**
 * What the user confirms before tipping a name
 * ([PlatformWalletManager.reviewShieldedTip]).
 *
 * @param recipient whom the name pays now. A send takes it back unchanged.
 * @param change how it compares with the recipient last confirmed for the
 *   name.
 * @param unresolved an earlier shielded payment from this wallet to the
 *   recipient may still execute, so a send to it is refused
 *   ([DashSdkError.PlatformWallet.ShieldedTipPending]) until a shielded sync
 *   resolves that payment.
 */
data class ShieldedTipReview(
    val recipient: ShieldedTipRecipient,
    val change: ShieldedTipRecipientChange,
    val unresolved: Boolean,
)

/** The account a tip is paid from. Rust resolves and checks it. */
sealed interface ShieldedTipSource {
    /**
     * The identity's own shielded account (its identity account), at the
     * slot the seed proves the identity holds when the tip is sent.
     *
     * @param identityId the 32-byte id of a wallet identity.
     */
    data class Identity(val identityId: ByteArray) : ShieldedTipSource {
        override fun equals(other: Any?): Boolean =
            other is Identity && identityId.contentEquals(other.identityId)

        override fun hashCode(): Int = identityId.contentHashCode()
    }

    /**
     * An account named by its number: an ordinary account, or the identity
     * account of a slot that holds no identity. Rust refuses the account of
     * a slot that holds an identity; that identity pays through [Identity].
     */
    data class Account(val account: Int) : ShieldedTipSource
}

/**
 * The shielded balance of a wallet identity's own account, as far as the
 * wallet can vouch for it ([PlatformWalletManager.shieldedIdentityBalance]).
 * A host shows these states; it does not add up stored notes.
 */
sealed interface ShieldedIdentityBalance {
    /**
     * The seed has not proved in this session which account is the
     * identity's. [PlatformWalletManager.prepareShieldedIdentityAddress]
     * proves it.
     */
    data object Unverified : ShieldedIdentityBalance

    /**
     * The account is proved, but its ledger is not established (not
     * restored or scanned yet, as after a shielded Clear). Its balance is
     * not known, which is not the same as zero.
     *
     * @param account the identity's shielded account.
     */
    data class Unavailable(val account: Int) : ShieldedIdentityBalance

    /**
     * @param account the identity's shielded account.
     * @param credits its spendable credits, without the notes that pending
     *   spends reserve.
     */
    data class Available(val account: Int, val credits: Long) : ShieldedIdentityBalance
}

/** Bytes of an identity id in the BLOBs below. */
private const val IDENTITY_ID_BYTES = 32

/** Bytes of a raw Orchard address in the BLOBs below. */
private const val SHIELDED_ADDRESS_BYTES = 43

/**
 * Decode the BLOB `FundingNative.reviewShieldedTip` returns (layout in
 * `rs-unified-sdk-jni/src/funding.rs`): the fields of the C
 * `ShieldedTipReviewFFI` in its order. The previous recipient is read only
 * for a changed one; the C struct zeroes it otherwise. A change kind or a
 * flag this build does not know fails the review instead of reading as
 * another one.
 */
internal fun decodeShieldedTipReview(blob: ByteArray): ShieldedTipReview {
    val reader = BlobReader(blob, "shielded tip review")
    val recipient = reader.shieldedTipRecipient()
    val change = reader.u8()
    val previous = reader.shieldedTipRecipient()
    val unresolved = reader.u8()
    reader.finish()
    return ShieldedTipReview(
        recipient = recipient,
        change = when (change) {
            0 -> ShieldedTipRecipientChange.New
            1 -> ShieldedTipRecipientChange.Unchanged
            2 -> ShieldedTipRecipientChange.Changed(previous)
            else -> throw DashSdkError.SerializationError("Unknown shielded tip recipient change $change")
        },
        unresolved = when (unresolved) {
            0 -> false
            1 -> true
            else -> throw DashSdkError.SerializationError("Unknown shielded tip unresolved flag $unresolved")
        },
    )
}

private fun BlobReader.shieldedTipRecipient(): ShieldedTipRecipient =
    ShieldedTipRecipient(
        identityId = fixed(IDENTITY_ID_BYTES),
        address = fixed(SHIELDED_ADDRESS_BYTES),
    )

/**
 * Decode the BLOB `FundingNative.shieldedIdentityBalance` returns (layout in
 * `rs-unified-sdk-jni/src/funding.rs`): the fields of the C
 * `ShieldedIdentityBalanceFFI` in its order. A state this build does not
 * know, or an account an `Int` cannot carry, fails the read instead of
 * reading as a balance.
 */
internal fun decodeShieldedIdentityBalance(blob: ByteArray): ShieldedIdentityBalance {
    val reader = BlobReader(blob, "shielded identity balance")
    val state = reader.u8()
    val account = reader.u32()
    val credits = reader.u64()
    reader.finish()
    return when (state) {
        0 -> ShieldedIdentityBalance.Unverified
        1 -> ShieldedIdentityBalance.Unavailable(shieldedAccount(account))
        2 -> ShieldedIdentityBalance.Available(shieldedAccount(account), credits)
        else -> throw DashSdkError.SerializationError("Unknown shielded identity balance state $state")
    }
}

private fun shieldedAccount(account: Long): Int {
    if (account > Int.MAX_VALUE) {
        throw DashSdkError.SerializationError("Shielded account $account does not fit an Int")
    }
    return account.toInt()
}
