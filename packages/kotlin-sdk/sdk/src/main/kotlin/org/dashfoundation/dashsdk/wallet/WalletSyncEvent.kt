package org.dashfoundation.dashsdk.wallet

/**
 * Typed sync events fanned out by [PlatformWalletManager] from the native
 * `EventHandlerCallbacks` vtable (bridged in `rs-unified-sdk-jni/src/events.rs`).
 *
 * These are the Kotlin analogue of the `@Published` events
 * `PlatformWalletManager.swift`'s `PlatformWalletEventHandler` republishes
 * (`lastPlatformAddressSyncEvent`, `lastShieldedSyncEvent`,
 * `currentShieldedSyncScanned`, …). The manager exposes them on a
 * `SharedFlow`; the sync services collect and reduce them into their own
 * `StateFlow`s. Services REFLECT these Rust-owned events — they never drive
 * the sync loops from Kotlin.
 */
sealed interface WalletSyncEvent {

    /** Generic Debug-formatted `WalletEvent` string (`on_wallet_event_fn`). */
    data class Generic(val debug: String) : WalletSyncEvent

    /** Fatal error string (`on_error_fn`). */
    data class Error(val message: String) : WalletSyncEvent

    /**
     * One wallet's platform-address sync result
     * (`on_platform_address_sync_completed_fn`, per entry).
     */
    data class PlatformAddressResult(
        val walletId: ByteArray,
        val success: Boolean,
        val foundCount: Long,
        val absentCount: Long,
        val checkpointHeight: Long,
        val newSyncHeight: Long,
        val newSyncTimestamp: Long,
        val lastKnownRecentBlock: Long,
        val errorMessage: String?,
    ) : WalletSyncEvent {
        override fun equals(other: Any?): Boolean =
            other is PlatformAddressResult &&
                walletId.contentEquals(other.walletId) &&
                success == other.success &&
                foundCount == other.foundCount &&
                absentCount == other.absentCount &&
                checkpointHeight == other.checkpointHeight &&
                newSyncHeight == other.newSyncHeight &&
                newSyncTimestamp == other.newSyncTimestamp &&
                lastKnownRecentBlock == other.lastKnownRecentBlock &&
                errorMessage == other.errorMessage

        override fun hashCode(): Int {
            var result = walletId.contentHashCode()
            result = 31 * result + success.hashCode()
            result = 31 * result + newSyncHeight.hashCode()
            result = 31 * result + newSyncTimestamp.hashCode()
            return result
        }
    }

    /** Platform-address sync pass boundary (all wallets done). */
    data class PlatformAddressPassCompleted(
        val syncUnixSeconds: Long,
        val walletCount: Int,
    ) : WalletSyncEvent

    /**
     * One wallet's shielded sync result (`on_shielded_sync_completed_fn`,
     * per entry).
     */
    data class ShieldedResult(
        val walletId: ByteArray,
        val success: Boolean,
        val skipped: Boolean,
        val cooldownSkip: Boolean,
        val newNotes: Int,
        val totalScanned: Long,
        val newlySpent: Int,
        val balance: Long,
        val errorMessage: String?,
    ) : WalletSyncEvent {
        override fun equals(other: Any?): Boolean =
            other is ShieldedResult &&
                walletId.contentEquals(other.walletId) &&
                success == other.success &&
                skipped == other.skipped &&
                cooldownSkip == other.cooldownSkip &&
                newNotes == other.newNotes &&
                totalScanned == other.totalScanned &&
                newlySpent == other.newlySpent &&
                balance == other.balance &&
                errorMessage == other.errorMessage

        override fun hashCode(): Int {
            var result = walletId.contentHashCode()
            result = 31 * result + balance.hashCode()
            result = 31 * result + totalScanned.hashCode()
            result = 31 * result + newNotes
            return result
        }
    }

    /** Shielded sync pass boundary (all wallets done). */
    data class ShieldedPassCompleted(
        val syncUnixSeconds: Long,
        val walletCount: Int,
    ) : WalletSyncEvent

    /** Live shielded download progress (`on_shielded_sync_progress_fn`). */
    data class ShieldedProgress(
        val cumulativeScanned: Long,
        val blockHeight: Long,
    ) : WalletSyncEvent

    /** Live shielded tree-commit progress (`on_shielded_tree_progress_fn`). */
    data class ShieldedTreeProgress(
        val leavesCommitted: Long,
        val totalTarget: Long,
    ) : WalletSyncEvent
}
