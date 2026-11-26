package org.dashfoundation.dashsdk.queries

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.PlatformBlocksNative
import org.dashfoundation.dashsdk.wallet.op

/**
 * The block and state transition status reads (protocol version 15) — port of the
 * `SDK.getBlocks`, `getOwnerTransactionsInBlock`, `getRecentStateTransitionStatus` and
 * `getStateTransitionStatusByHash` methods in Swift's `FFI/PlatformQueryExtensions.swift`.
 *
 * rs-sdk sends each request, verifies the proofs when the SDK proves (the default) and refuses
 * a network below protocol version 15; rs-sdk-ffi writes the answer as JSON, which this class
 * decodes one to one into the types in this file (Swift `PlatformBlocks.swift`). Whether an
 * answer is proved is Rust's word, carried in each type's `proved`; nothing here decides it.
 *
 * Errors: the SDK's refusals as [DashSdkError] (a malformed identity id or hash is
 * [DashSdkError.InvalidParameter] before anything is sent), and an answer that does not
 * decode as [DashSdkError.SerializationError].
 */
class PlatformBlocks internal constructor(private val sdk: Sdk) {

    /**
     * Up to [count] consecutive blocks from [startHeight], each with every state transition it
     * holds and its result (`dash_sdk_system_get_blocks`). The node accepts a count from 1 to
     * 10 and refuses any other. Neither may be negative.
     */
    suspend fun getBlocks(startHeight: Long, count: Int): PlatformBlockList = sdk.queryGate.op {
        decode(
            "dash_sdk_system_get_blocks",
            mapNativeErrors { PlatformBlocksNative.getBlocks(sdk.handle, startHeight, count) },
        )
    }

    /**
     * The state transitions the identity [ownerId] (base58) owns in the block at [height]
     * (`dash_sdk_system_get_owner_transactions_in_block`). Proved per transition, not proved
     * complete: [getBlocks] gives the block's whole list.
     */
    suspend fun getOwnerTransactionsInBlock(height: Long, ownerId: String): OwnerTransactionsInBlock =
        sdk.queryGate.op {
            decode(
                "dash_sdk_system_get_owner_transactions_in_block",
                mapNativeErrors {
                    PlatformBlocksNative.getOwnerTransactionsInBlock(sdk.handle, height, ownerId)
                },
            )
        }

    /**
     * What one node says of the transition with [transitionHash] (64 hex characters), without
     * waiting (`dash_sdk_system_get_recent_state_transition_status`). Unproved.
     *
     * Privacy: the hash goes to this SDK's nodes, not to the node the transition was broadcast
     * to, so asking links the caller to that transition in front of them. Use it for public
     * transitions only, not for shielded or private document transitions.
     */
    suspend fun getRecentStateTransitionStatus(transitionHash: String): NodeTransitionStatus =
        sdk.queryGate.op {
            decode(
                "dash_sdk_system_get_recent_state_transition_status",
                mapNativeErrors {
                    PlatformBlocksNative.getRecentStateTransitionStatus(sdk.handle, transitionHash)
                },
            )
        }

    /**
     * The status of the transition with [transitionHash] (64 hex characters), proved by its
     * block when the node says it is committed and that block holds it
     * (`dash_sdk_system_get_state_transition_status_by_hash`).
     *
     * Privacy: as [getRecentStateTransitionStatus], for public transitions only.
     */
    suspend fun getStateTransitionStatusByHash(transitionHash: String): TransitionStatusByHash =
        sdk.queryGate.op {
            decode(
                "dash_sdk_system_get_state_transition_status_by_hash",
                mapNativeErrors {
                    PlatformBlocksNative.getStateTransitionStatusByHash(sdk.handle, transitionHash)
                },
            )
        }

    internal companion object {
        /** Ignores keys a newer rs-sdk-ffi adds, as Swift's `JSONDecoder` does. */
        val json = Json { ignoreUnknownKeys = true }

        /** [answer], the JSON [function] wrote, decoded as [T]. */
        inline fun <reified T> decode(function: String, answer: String?): T {
            if (answer == null) {
                throw DashSdkError.SerializationError("$function returned no answer")
            }
            return try {
                json.decodeFromString<T>(answer)
            } catch (e: SerializationException) {
                throw DashSdkError.SerializationError("$function answer did not decode: ${e.message}", e)
            } catch (e: IllegalArgumentException) {
                throw DashSdkError.SerializationError("$function answer did not decode: ${e.message}", e)
            }
        }
    }
}

/** A state transition a block holds, with the result of its execution. */
@Serializable
data class PlatformBlockTransaction(
    /** The SHA-256 of the serialized transition, hex: the hash Tenderdash knows it by. */
    val hash: String,
    /** 0 if it executed, otherwise the code of the error it paid for. */
    val code: Long,
    /** Whether the transition executed, as rs-sdk reads [code] (`BlockTransaction::executed`). */
    val executed: Boolean,
    @SerialName("gas_used") val gasUsed: Long,
    /** The data of its execution result, hex. */
    @SerialName("result_data") val resultData: String,
    /** The transition type, e.g. `IdentityCreditTransfer`. */
    val type: String,
    /** The identity the transition reports as its own, base58; null when it has none. */
    @SerialName("owner_id") val ownerId: String? = null,
)

/** A block of the Platform chain with every state transition it holds, in block order. */
@Serializable
data class PlatformBlock(
    val height: Long,
    /** The block hash, hex. */
    val hash: String,
    /** The block time, in milliseconds since the Unix epoch. */
    @SerialName("time_ms") val timeMs: Long,
    @SerialName("core_chain_locked_height") val coreChainLockedHeight: Long,
    /** Platform's state root after this block, hex. */
    @SerialName("app_hash") val appHash: String,
    /** The masternode that proposed the block, hex as the block carries it. */
    @SerialName("proposer_pro_tx_hash") val proposerProTxHash: String,
    val transactions: List<PlatformBlockTransaction>,
)

/**
 * Consecutive blocks from a start height (`dash_sdk_system_get_blocks`; Swift
 * `PlatformBlocks`). When [proved], each block is the one a validator quorum committed at its
 * height. There can be fewer blocks than asked for, and nothing proves a block left off the
 * end does not exist.
 */
@Serializable
data class PlatformBlockList(
    val proved: Boolean,
    val blocks: List<PlatformBlock>,
)

/**
 * The state transitions one identity owns in one block
 * (`dash_sdk_system_get_owner_transactions_in_block`). When [proved], each transition is in
 * that block with that result and is the owner's, but the list is not proved complete: a node
 * can leave some out.
 */
@Serializable
data class OwnerTransactionsInBlock(
    val proved: Boolean,
    val height: Long,
    val transactions: List<PlatformBlockTransaction>,
)

/** The error a node reports for a committed transition, unproved. */
@Serializable
data class NodeReportedTransitionError(
    val code: Long,
    val message: String,
    /** The consensus error the node attached, as text; null when none decoded. */
    @SerialName("consensus_error") val consensusError: String? = null,
)

/**
 * One node's view of a transition (`dash_sdk_system_get_recent_state_transition_status`).
 * Always unproved: never decide anything about funds on it.
 */
@Serializable
data class NodeTransitionStatus(
    /** Always false. */
    val proved: Boolean,
    val status: Status,
    /** The height of the block the node says holds it, when committed. */
    val height: Long? = null,
    /** The error the node reports it failed with, when committed and failed. */
    val error: NodeReportedTransitionError? = null,
    /** The node's latest block height when it answered. */
    @SerialName("node_height") val nodeHeight: Long? = null,
) {
    @Serializable
    enum class Status {
        @SerialName("committed") COMMITTED,
        @SerialName("pending") PENDING,
        @SerialName("unknown") UNKNOWN,
    }
}

/**
 * A transition's status from its hash, proved where the network can prove it
 * (`dash_sdk_system_get_state_transition_status_by_hash`). Only [Status.IN_BLOCK] is proved.
 */
@Serializable
data class TransitionStatusByHash(
    val proved: Boolean,
    val status: Status,
    /** [Status.IN_BLOCK] only. */
    @SerialName("block_height") val blockHeight: Long? = null,
    /** [Status.IN_BLOCK] only, hex. */
    @SerialName("block_hash") val blockHash: String? = null,
    /** [Status.IN_BLOCK] only, milliseconds since the Unix epoch. */
    @SerialName("block_time_ms") val blockTimeMs: Long? = null,
    /** [Status.IN_BLOCK] only: the transition with its result. */
    val transaction: PlatformBlockTransaction? = null,
    /** The height the node named ([Status.COMMITTED_NOT_PROVED], [Status.COMMITTED_REFUTED]). */
    val height: Long? = null,
    /** [Status.COMMITTED_NOT_PROVED] only: the error the node reports, unproved. */
    val error: NodeReportedTransitionError? = null,
    /** The node's latest block height when it answered (every status but [Status.IN_BLOCK]). */
    @SerialName("node_height") val nodeHeight: Long? = null,
) {
    @Serializable
    enum class Status {
        /** Proved: the block a validator quorum committed at [blockHeight] holds it. */
        @SerialName("in_block") IN_BLOCK,

        /** The node says committed at [height], and no proved block came back for it. */
        @SerialName("committed_not_proved") COMMITTED_NOT_PROVED,

        /** The node says committed at [height], and the proved block there does not hold it. */
        @SerialName("committed_refuted") COMMITTED_REFUTED,

        /** The node says it is in its mempool. */
        @SerialName("pending") PENDING,

        /** The node knows nothing of it. */
        @SerialName("unknown") UNKNOWN,
    }
}
