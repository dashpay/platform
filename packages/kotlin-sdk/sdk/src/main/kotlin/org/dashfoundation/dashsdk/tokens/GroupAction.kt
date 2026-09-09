package org.dashfoundation.dashsdk.tokens

/**
 * How a group-gatable token action is authorized — the Kotlin analog of the
 * Swift `GroupActionMode`
 * (`packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/Tokens`).
 *
 * The native FFI takes a flattened
 * `(kind: u8, position: u16, actionId: ByteArray?, actionIsProposer: Bool)`
 * tuple (decoded Rust-side by `decode_group_info` in
 * `rs-platform-wallet-ffi/src/tokens/group_info.rs`). This sealed type is the
 * typed surface; [flatten] produces the tuple the [TokensNative] calls take.
 *
 * Kind tags (must stay in sync with `decode_group_info`):
 * - `0` [None] — single-signer, not a group action.
 * - `1` [Propose] — the caller proposes a new action at [Propose.position].
 * - `2` [SignExisting] — the caller co-signs an existing proposal
 *   identified by [SignExisting.actionId] at [SignExisting.position].
 */
sealed class GroupAction {

    /** Not a group action (single-signer). */
    object None : GroupAction()

    /** Propose a new group action at group contract [position]. */
    data class Propose(val position: Int) : GroupAction()

    /**
     * Co-sign an existing proposal. [actionId] is the 32-byte proposal id
     * (from the pending-actions list); [position] is the group contract
     * position; [actionIsProposer] marks whether the co-signed proposal's
     * `action_is_proposer` flag is set (mirrors the Swift
     * `.signExisting(position:actionId:actionIsProposer:)` payload).
     */
    data class SignExisting(
        val position: Int,
        val actionId: ByteArray,
        val actionIsProposer: Boolean,
    ) : GroupAction() {
        override fun equals(other: Any?): Boolean =
            other is SignExisting &&
                position == other.position &&
                actionIsProposer == other.actionIsProposer &&
                actionId.contentEquals(other.actionId)

        override fun hashCode(): Int {
            var result = position
            result = 31 * result + actionId.contentHashCode()
            result = 31 * result + actionIsProposer.hashCode()
            return result
        }
    }

    /**
     * Flatten into the `(kind, position, actionId, actionIsProposer)` tuple
     * the native layer expects. [SignExisting] validates that [actionId] is
     * exactly 32 bytes before it reaches the FFI (the native side rejects a
     * wrong length, but failing here gives a clearer, earlier error).
     */
    fun flatten(): Flat = when (this) {
        is None -> Flat(kind = 0, position = 0, actionId = null, actionIsProposer = false)
        is Propose -> {
            require(position in 0..UShort.MAX_VALUE.toInt()) {
                "group position $position out of u16 range"
            }
            Flat(kind = 1, position = position, actionId = null, actionIsProposer = false)
        }
        is SignExisting -> {
            require(position in 0..UShort.MAX_VALUE.toInt()) {
                "group position $position out of u16 range"
            }
            require(actionId.size == 32) {
                "group actionId must be 32 bytes, got ${actionId.size}"
            }
            Flat(kind = 2, position = position, actionId = actionId, actionIsProposer = actionIsProposer)
        }
    }

    /** The flattened tuple passed straight through to the [TokensNative] calls. */
    data class Flat(
        val kind: Int,
        val position: Int,
        val actionId: ByteArray?,
        val actionIsProposer: Boolean,
    ) {
        override fun equals(other: Any?): Boolean {
            if (other !is Flat) return false
            if (kind != other.kind) return false
            if (position != other.position) return false
            if (actionIsProposer != other.actionIsProposer) return false
            val a = actionId
            val b = other.actionId
            return when {
                a == null -> b == null
                b == null -> false
                else -> a.contentEquals(b)
            }
        }

        override fun hashCode(): Int {
            var result = kind
            result = 31 * result + position
            result = 31 * result + (actionId?.contentHashCode() ?: 0)
            result = 31 * result + actionIsProposer.hashCode()
            return result
        }
    }
}
