package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.AddressHistoryNative
import org.dashfoundation.dashsdk.ffi.BlobReader

// Port of the balance history part of `ManagedPlatformAddressWallet.swift`
// (packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet).

/**
 * The balance change history of a wallet's platform addresses, from one
 * [platformAddressBalanceHistory] call. Port of Swift's
 * `ManagedPlatformAddressWallet.BalanceHistory`.
 *
 * @property recent changes in Platform's recent (not yet compacted) zone, newest block first.
 * @property compacted compacted ranges, newest range first. Empty unless compacted history
 *   was asked for; no block here is also in [recent].
 * @property provedHeight the height the read reached (decided in Rust). Pass it as the
 *   `minProvedHeight` of the next read of a history that shows this one, so that read never
 *   uses an older page.
 */
data class PlatformAddressBalanceHistory(
    val recent: List<BalanceHistoryEntry>,
    val compacted: List<CompactedBalanceHistoryEntry>,
    val provedHeight: Long,
)

/**
 * One balance change of a wallet address in Platform's recent zone. Port of Swift's
 * `BalanceHistoryEntry`.
 *
 * @property addressType 0 = P2PKH, 1 = P2SH.
 * @property hashHex lowercase hex of the 20-byte address hash.
 * @property address the DIP-18 bech32m form of the address on the wallet's network, encoded
 *   by Rust.
 */
data class BalanceHistoryEntry(
    val blockHeight: Long,
    val addressType: Int,
    val hashHex: String,
    val address: String,
    val change: Change,
) {
    /** How the block changed the balance. */
    sealed interface Change {
        val credits: Long

        /** The block set the balance to [credits]. */
        data class Set(override val credits: Long) : Change

        /** The block added [credits]. */
        data class Added(override val credits: Long) : Change
    }
}

/**
 * One compacted balance change of a wallet address: a block range merged into one change.
 * Port of Swift's `CompactedBalanceHistoryEntry`.
 *
 * @property addressType 0 = P2PKH, 1 = P2SH.
 * @property hashHex lowercase hex of the 20-byte address hash.
 * @property address the bech32m form of the address on the wallet's network, encoded by Rust.
 */
data class CompactedBalanceHistoryEntry(
    val startBlockHeight: Long,
    val endBlockHeight: Long,
    val addressType: Int,
    val hashHex: String,
    val address: String,
    val change: Change,
) {
    /** One addition inside the range. */
    data class Addition(val blockHeight: Long, val credits: Long)

    /** How the range changed the balance. */
    sealed interface Change {
        /** The range ended with the balance set to [credits]. */
        data class SetFinal(val credits: Long) : Change

        /**
         * The range only added to the balance: [total] (computed in Rust) and each addition,
         * newest block first.
         */
        data class Additions(val total: Long, val entries: List<Addition>) : Change
    }
}

/**
 * Balance changes of this wallet's platform addresses (every platform payment account): the
 * recent zone, and the compacted zone too when [includeCompacted] is set. Port of Swift's
 * `ManagedPlatformAddressWallet.balanceHistory(includeCompacted:minProvedHeight:)`.
 *
 * [minProvedHeight] is the [PlatformAddressBalanceHistory.provedHeight] of the history already
 * shown (0 for a first read): the read starts its height floor there, so it never uses a page
 * older than what is on screen. A read that cannot get a page at its floor throws
 * [DashSdkError.PlatformWallet.BalanceHistoryInconsistentHeights] (code 83, retryable): every
 * attempt for one page got a successfully fetched, proof-verified reply below the floor.
 * Errors from the SDK itself (its own stale-node rejection, transport or proof failures) keep
 * their own mapping.
 *
 * Paging, the wallet-address filter, the height floor and its retries, the reconciliation of
 * the two zones (a recent block a compacted range covers shows once, in `compacted`), the
 * ordering, each range's total and each address's bech32m form all happen in
 * `platform-wallet` (`PlatformAddressWallet::balance_history`). Every page is proof-verified,
 * so the call goes through the network; it runs on [Dispatchers.IO]. A read-only wallet-handle
 * call, so it is not fenced by the manager's teardown gate (as [ManagedPlatformWallet.groups]).
 *
 * @throws IllegalArgumentException if [minProvedHeight] is negative.
 * @throws DashSdkError for a failed read, or a result this build cannot decode.
 */
suspend fun ManagedPlatformWallet.platformAddressBalanceHistory(
    includeCompacted: Boolean,
    minProvedHeight: Long,
): PlatformAddressBalanceHistory {
    require(minProvedHeight >= 0) { "minProvedHeight must not be negative, got $minProvedHeight" }
    val walletHandle = handle
    return withContext(Dispatchers.IO) {
        val blob = mapNativeErrors {
            AddressHistoryNative.balanceHistory(walletHandle, includeCompacted, minProvedHeight)
        }
        decodePlatformAddressBalanceHistory(blob)
    }
}

/**
 * Decode the BLOB `AddressHistoryNative.balanceHistory` returns (layout in
 * `rs-unified-sdk-jni/src/address_history.rs`), in Rust's order. An unknown change kind or an
 * entry without its address fails the whole history, as Swift's decoder does.
 */
internal fun decodePlatformAddressBalanceHistory(blob: ByteArray): PlatformAddressBalanceHistory {
    val reader = BlobReader(blob, "balance history")
    val provedHeight = reader.u64()
    val recent = List(reader.count()) {
        val blockHeight = reader.u64()
        val addressType = reader.u8()
        val hashHex = reader.fixed(20).toHexLower()
        val address = reader.address()
        val kind = reader.u8()
        val credits = reader.u64()
        val change = when (kind) {
            0 -> BalanceHistoryEntry.Change.Set(credits)
            1 -> BalanceHistoryEntry.Change.Added(credits)
            else -> throw DashSdkError.SerializationError("Unknown balance change kind $kind")
        }
        BalanceHistoryEntry(blockHeight, addressType, hashHex, address, change)
    }
    val compacted = List(reader.count()) {
        val start = reader.u64()
        val end = reader.u64()
        val addressType = reader.u8()
        val hashHex = reader.fixed(20).toHexLower()
        val address = reader.address()
        val kind = reader.u8()
        val credits = reader.u64()
        val additions = List(reader.count()) {
            CompactedBalanceHistoryEntry.Addition(blockHeight = reader.u64(), credits = reader.u64())
        }
        val change = when (kind) {
            0 -> CompactedBalanceHistoryEntry.Change.SetFinal(credits)
            1 -> CompactedBalanceHistoryEntry.Change.Additions(total = credits, entries = additions)
            else -> throw DashSdkError.SerializationError("Unknown compacted balance change kind $kind")
        }
        CompactedBalanceHistoryEntry(start, end, addressType, hashHex, address, change)
    }
    reader.finish()
    return PlatformAddressBalanceHistory(recent, compacted, provedHeight)
}

private fun BlobReader.address(): String = str().ifEmpty {
    throw DashSdkError.SerializationError("Balance history entry without an address")
}

private fun ByteArray.toHexLower(): String = joinToString("") { "%02x".format(it) }
