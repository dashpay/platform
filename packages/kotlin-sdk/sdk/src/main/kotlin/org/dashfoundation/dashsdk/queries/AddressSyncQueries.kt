package org.dashfoundation.dashsdk.queries

import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.AddressHistoryNative
import org.dashfoundation.dashsdk.ffi.BlobReader
import org.dashfoundation.dashsdk.wallet.op

/**
 * The address-sync queries of the query catalog: raw, network-wide reads with no wallet
 * filter. Port of the `getRecentBalanceChanges`, `getCompactedBalanceChanges`,
 * `getTrunkState`, `getBranchState` and `branchQueryDepth` members of Swift's `Addresses`
 * (`Sources/SwiftDashSDK/Address/Addresses.swift`). Each call is one C function of
 * rs-sdk-ffi and runs through the SDK's query gate on a worker thread.
 */
class AddressSyncQueries internal constructor(private val sdk: Sdk) {

    /** Address balance changes of every address from [startHeight] (one server page). */
    suspend fun recentBalanceChanges(startHeight: Long): List<BlockAddressBalanceChanges> = sdk.queryGate.op {
        require(startHeight >= 0) { "startHeight must not be negative" }
        decodeRecentBalanceChanges(
            mapNativeErrors { AddressHistoryNative.recentBalanceChanges(sdk.handle, startHeight) },
        )
    }

    /** Compacted address balance changes from [startBlockHeight] (at most one server page). */
    suspend fun compactedBalanceChanges(startBlockHeight: Long): List<CompactedBlockRange> = sdk.queryGate.op {
        require(startBlockHeight >= 0) { "startBlockHeight must not be negative" }
        decodeCompactedBalanceChanges(
            mapNativeErrors { AddressHistoryNative.compactedBalanceChanges(sdk.handle, startBlockHeight) },
        )
    }

    /** The address tree's trunk state. */
    suspend fun trunkState(): AddressTrunkState = sdk.queryGate.op {
        decodeTrunkState(mapNativeErrors { AddressHistoryNative.trunkState(sdk.handle) })
    }

    /**
     * One branch of the address tree under [key] at [depth], checked against [expectedHash]
     * (32 bytes) at [checkpointHeight]. Whether [depth] is allowed is decided by the query
     * (Rust and the server), whose error is thrown; a mistyped hash or a stale checkpoint
     * fails with a proof error.
     */
    suspend fun branchState(
        key: ByteArray,
        depth: Int,
        expectedHash: ByteArray,
        checkpointHeight: Long,
    ): AddressBranchState = sdk.queryGate.op {
        require(expectedHash.size == 32) { "expectedHash must be 32 bytes, got ${expectedHash.size}" }
        require(depth >= 0 && checkpointHeight >= 0) { "depth and checkpointHeight must not be negative" }
        decodeBranchState(
            mapNativeErrors {
                AddressHistoryNative.branchState(sdk.handle, key, depth, expectedHash, checkpointHeight)
            },
        )
    }

    /**
     * The depth to pass to [branchState] for a leaf boundary of about [estimatedCount]
     * elements (0 when unknown): the depth the address sync picks for such a leaf under the
     * SDK's platform version. Rust decides it (`dash_sdk_address_branch_query_depth`).
     */
    suspend fun branchQueryDepth(estimatedCount: Long): Int = sdk.queryGate.op {
        require(estimatedCount >= 0) { "estimatedCount must not be negative" }
        mapNativeErrors { AddressHistoryNative.branchQueryDepth(sdk.handle, estimatedCount) }
    }
}

/** The address-sync queries of this SDK (Swift: members of `sdk.addresses`). */
val Sdk.addressSync: AddressSyncQueries get() = AddressSyncQueries(this)

/** How a recent block changed an address's credits. */
enum class CreditOperation { SET, ADD }

/** One address's change in a block. [addressHex] is the raw address bytes in hex. */
data class AddressBalanceChange(val addressHex: String, val operation: CreditOperation, val credits: Long)

/** The address balance changes of one block. */
data class BlockAddressBalanceChanges(val blockHeight: Long, val changes: List<AddressBalanceChange>)

/** One address's change across a compacted range. */
data class CompactedAddressChange(val addressHex: String, val operation: Operation) {
    /** One addition and the block it happened in. */
    data class Entry(val blockHeight: Long, val credits: Long)

    /** The range's change: a final set value, or a list of additions. */
    sealed interface Operation {
        data class SetCredits(val credits: Long) : Operation

        data class AddToCredits(val entries: List<Entry>) : Operation
    }
}

/** The compacted address balance changes of a block range. */
data class CompactedBlockRange(
    val startBlockHeight: Long,
    val endBlockHeight: Long,
    val changes: List<CompactedAddressChange>,
)

/** An address found at the trunk or in a branch, with its nonce and balance. */
data class AddressTreeElement(val keyHex: String, val nonce: Long, val balance: Long)

/**
 * A subtree that needs a branch query: its key, the hash the branch must prove to, and its
 * estimated element count (0 when unknown).
 */
data class AddressLeafBoundary(val keyHex: String, val hashHex: String, val estimatedCount: Long)

/** The address tree's trunk state. */
data class AddressTrunkState(
    val checkpointHeight: Long,
    val elements: List<AddressTreeElement>,
    val leafBoundaries: List<AddressLeafBoundary>,
)

/** One branch of the address tree. */
data class AddressBranchState(
    val elements: List<AddressTreeElement>,
    val leafBoundaries: List<AddressLeafBoundary>,
)

// ── Decoders (layouts in rs-unified-sdk-jni/src/address_history.rs) ──────

internal fun decodeRecentBalanceChanges(blob: ByteArray): List<BlockAddressBalanceChanges> {
    val reader = BlobReader(blob, "recent balance changes")
    val blocks = List(reader.count()) {
        val height = reader.u64()
        val changes = List(reader.count()) {
            val address = reader.bytes().toHexLower()
            val operation = when (val op = reader.u8()) {
                0 -> CreditOperation.SET
                1 -> CreditOperation.ADD
                else -> throw DashSdkError.SerializationError("Unknown credit operation $op")
            }
            AddressBalanceChange(address, operation, reader.u64())
        }
        BlockAddressBalanceChanges(height, changes)
    }
    reader.finish()
    return blocks
}

internal fun decodeCompactedBalanceChanges(blob: ByteArray): List<CompactedBlockRange> {
    val reader = BlobReader(blob, "compacted balance changes")
    val ranges = List(reader.count()) {
        val start = reader.u64()
        val end = reader.u64()
        val changes = List(reader.count()) {
            val address = reader.bytes().toHexLower()
            val op = reader.u8()
            val setValue = reader.u64()
            val entries = List(reader.count()) {
                CompactedAddressChange.Entry(blockHeight = reader.u64(), credits = reader.u64())
            }
            val operation = when (op) {
                0 -> CompactedAddressChange.Operation.SetCredits(setValue)
                1 -> CompactedAddressChange.Operation.AddToCredits(entries)
                else -> throw DashSdkError.SerializationError("Unknown compacted operation $op")
            }
            CompactedAddressChange(address, operation)
        }
        CompactedBlockRange(start, end, changes)
    }
    reader.finish()
    return ranges
}

internal fun decodeTrunkState(blob: ByteArray): AddressTrunkState {
    val reader = BlobReader(blob, "trunk state")
    val checkpoint = reader.u64()
    val (elements, leaves) = reader.elementsAndLeaves()
    reader.finish()
    return AddressTrunkState(checkpoint, elements, leaves)
}

internal fun decodeBranchState(blob: ByteArray): AddressBranchState {
    val reader = BlobReader(blob, "branch state")
    val (elements, leaves) = reader.elementsAndLeaves()
    reader.finish()
    return AddressBranchState(elements, leaves)
}

private fun BlobReader.elementsAndLeaves(): Pair<List<AddressTreeElement>, List<AddressLeafBoundary>> {
    val elements = List(count()) {
        AddressTreeElement(keyHex = bytes().toHexLower(), nonce = u32(), balance = u64())
    }
    val leaves = List(count()) {
        AddressLeafBoundary(keyHex = bytes().toHexLower(), hashHex = fixed(32).toHexLower(), estimatedCount = u64())
    }
    return elements to leaves
}

private fun ByteArray.toHexLower(): String = joinToString("") { "%02x".format(it) }
