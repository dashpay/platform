package org.dashfoundation.dashsdk.identity

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.IdentityWithdrawalsNative
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import java.nio.BufferUnderflowException
import java.nio.ByteBuffer

// Port of `IdentityWithdrawals.swift` (packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet).

/**
 * The withdrawals contract status of a withdrawal, mirrored from the code
 * `platform_wallet_identity_withdrawals` reports. A code this build does not know is refused
 * when decoding, never mapped to a nearby case. Port of Swift's `WithdrawalStatus`.
 *
 * @property code the withdrawals contract status code.
 */
enum class WithdrawalStatus(val code: Int) {
    /** Waiting in Platform state to be pooled. */
    QUEUED(0),

    /** Pooled into an asset unlock transaction, waiting for a quorum to sign. */
    POOLED(1),

    /**
     * Platform built the asset unlock transaction and queued it for a quorum to sign and for
     * broadcast to Core. This status does not show whether either has happened: it stays
     * broadcasted after Core receives the signed transaction.
     */
    BROADCASTED(2),

    /** Platform saw the transaction in a chain-locked Core block. */
    COMPLETE(3),

    /** Core never mined the transaction in time; Platform signs it again. */
    EXPIRED(4),

    /**
     * The signed transaction can never be mined (its payout is below Core's dust threshold).
     */
    FAILED(5),
    ;

    companion object {
        /** The status with [code], or null for a code this build does not know. */
        fun fromCode(code: Int): WithdrawalStatus? = entries.firstOrNull { it.code == code }
    }
}

/**
 * One withdrawal of an identity's credits to Core, as the withdrawals system contract stores
 * it. Rust decides [status], [statusLabel] and [isFinal]; this value only carries them. Port
 * of Swift's `IdentityWithdrawal`.
 *
 * @property id the id of the withdrawal document (32 bytes).
 * @property ownerId the identity the credits were withdrawn from (32 bytes).
 * @property amountCredits the credits withdrawn, before the Core transaction fee.
 * @property coreFeePerByte the Core fee rate the withdrawal offered, in duffs per byte.
 * @property statusLabel the status as Rust names it ("Queued", "Pooled", ...).
 * @property isFinal whether the withdrawal reached a status it never leaves (complete or
 *   failed), as Rust decided it. An expired withdrawal is not final.
 * @property destination the Core address the withdrawal pays on the wallet's network, or null
 *   when the output script is not an address.
 * @property outputScript the Core output script the withdrawal pays.
 * @property transactionIndex the index of the asset unlock transaction that carries the
 *   withdrawal to Core, once Platform pooled it.
 * @property transactionSignHeight the Core chain-locked height Platform built the asset unlock
 *   transaction at (its request height), once it built it. Not proof that a quorum signed it
 *   or that Core accepted it.
 * @property createdAtMs when the withdrawal was submitted, block time in Unix milliseconds.
 * @property updatedAtMs when its status last changed, block time in Unix milliseconds.
 */
class IdentityWithdrawal(
    val id: ByteArray,
    val ownerId: ByteArray,
    val amountCredits: Long,
    val coreFeePerByte: Long,
    val status: WithdrawalStatus,
    val statusLabel: String,
    val isFinal: Boolean,
    val destination: String?,
    val outputScript: ByteArray,
    val transactionIndex: Long?,
    val transactionSignHeight: Long?,
    val createdAtMs: Long,
    val updatedAtMs: Long,
) {
    /** Lowercase hex of [id]. */
    val idHex: String get() = id.toHexLower()

    /** Lowercase hex of [outputScript]. */
    val outputScriptHex: String get() = outputScript.toHexLower()

    override fun equals(other: Any?): Boolean =
        other is IdentityWithdrawal &&
            id.contentEquals(other.id) &&
            ownerId.contentEquals(other.ownerId) &&
            amountCredits == other.amountCredits &&
            coreFeePerByte == other.coreFeePerByte &&
            status == other.status &&
            statusLabel == other.statusLabel &&
            isFinal == other.isFinal &&
            destination == other.destination &&
            outputScript.contentEquals(other.outputScript) &&
            transactionIndex == other.transactionIndex &&
            transactionSignHeight == other.transactionSignHeight &&
            createdAtMs == other.createdAtMs &&
            updatedAtMs == other.updatedAtMs

    override fun hashCode(): Int = id.contentHashCode()

    override fun toString(): String =
        "IdentityWithdrawal(id=$idHex, amountCredits=$amountCredits, status=$status, " +
            "statusLabel=$statusLabel, isFinal=$isFinal, destination=$destination)"
}

/**
 * Every withdrawal of [identityId] to Core, each listed once, most recently updated first,
 * read with proofs from the withdrawals system contract (protocol version 14 and later).
 * Rust reads every page at one block height. When no read of the listing stays at one block
 * height in a few attempts this throws an error that is safe to retry: call it again. Any
 * identity can be asked for, not only the wallet's. Withdrawals from Platform addresses and
 * from the shielded pool are not found: the contract stores them under its own owner.
 *
 * Port of Swift's `ManagedPlatformWallet.identityWithdrawals(identityId:)`. A read-only
 * wallet-handle call, so it is not fenced by the manager's teardown gate (as
 * [ManagedPlatformWallet.groups]). Runs on [Dispatchers.IO].
 *
 * @throws IllegalArgumentException if [identityId] is not 32 bytes.
 * @throws DashSdkError for a failed read, or a row this build cannot decode.
 */
suspend fun ManagedPlatformWallet.identityWithdrawals(identityId: ByteArray): List<IdentityWithdrawal> {
    require(identityId.size == 32) {
        "identityId must contain exactly 32 bytes (got ${identityId.size})"
    }
    val walletHandle = handle
    return withContext(Dispatchers.IO) {
        val blob = mapNativeErrors {
            IdentityWithdrawalsNative.identityWithdrawals(walletHandle, identityId)
        }
        decodeIdentityWithdrawals(blob)
    }
}

/**
 * Decode the BLOB `IdentityWithdrawalsNative.identityWithdrawals` returns (layout in
 * `rs-unified-sdk-jni/src/identity_withdrawals.rs`), in the order Rust gave. Every `has*`
 * flag gates its field: a 0 flag becomes null, never a zero. One row this build cannot decode
 * (an unknown status code, a missing status label, a truncated BLOB) fails the whole list,
 * as Swift's `IdentityWithdrawal.decodeRows` does.
 *
 * @throws DashSdkError.SerializationError for anything it cannot decode.
 */
internal fun decodeIdentityWithdrawals(blob: ByteArray): List<IdentityWithdrawal> {
    val buffer = ByteBuffer.wrap(blob) // big-endian
    try {
        val count = buffer.int
        if (count < 0) throw DashSdkError.SerializationError("withdrawal count $count is negative")
        val out = ArrayList<IdentityWithdrawal>(minOf(count, 1024))
        repeat(count) {
            val id = buffer.bytes(32)
            val ownerId = buffer.bytes(32)
            val amount = buffer.long
            val coreFeePerByte = buffer.int.toLong() and 0xFFFF_FFFFL
            val statusCode = buffer.get().toInt() and 0xFF
            val status = WithdrawalStatus.fromCode(statusCode)
                ?: throw DashSdkError.SerializationError("unknown withdrawal status code $statusCode")
            val label = buffer.utf8()
            if (label.isEmpty()) {
                throw DashSdkError.SerializationError("withdrawal row has no status label")
            }
            val isFinal = buffer.flag()
            val destination = if (buffer.flag()) buffer.utf8() else null
            val script = buffer.bytes(buffer.length())
            val transactionIndex = if (buffer.flag()) buffer.long else null
            val signHeight = if (buffer.flag()) buffer.int.toLong() and 0xFFFF_FFFFL else null
            val createdAt = buffer.long
            val updatedAt = buffer.long
            out += IdentityWithdrawal(
                id = id,
                ownerId = ownerId,
                amountCredits = amount,
                coreFeePerByte = coreFeePerByte,
                status = status,
                statusLabel = label,
                isFinal = isFinal,
                destination = destination,
                outputScript = script,
                transactionIndex = transactionIndex,
                transactionSignHeight = signHeight,
                createdAtMs = createdAt,
                updatedAtMs = updatedAt,
            )
        }
        if (buffer.hasRemaining()) {
            throw DashSdkError.SerializationError("${buffer.remaining()} bytes left after the withdrawal rows")
        }
        return out
    } catch (e: BufferUnderflowException) {
        throw DashSdkError.SerializationError("withdrawal rows are truncated", e)
    }
}

private fun ByteBuffer.bytes(length: Int): ByteArray = ByteArray(length).also { get(it) }

/** A `u32` length; refuses one past what is left so a corrupt length never allocates. */
private fun ByteBuffer.length(): Int {
    val length = int
    if (length < 0 || length > remaining()) {
        throw DashSdkError.SerializationError("length $length exceeds the ${remaining()} bytes left")
    }
    return length
}

private fun ByteBuffer.utf8(): String = String(bytes(length()), Charsets.UTF_8)

private fun ByteBuffer.flag(): Boolean = when (val value = get().toInt()) {
    0 -> false
    1 -> true
    else -> throw DashSdkError.SerializationError("flag byte $value is neither 0 nor 1")
}

private fun ByteArray.toHexLower(): String = joinToString("") { "%02x".format(it) }
