package org.dashfoundation.dashsdk.ffi

import android.util.Log

/**
 * Receiving side of the platform-wallet-ffi `EventHandlerCallbacks`
 * vtable, reached from Rust via the trampolines in
 * `rs-unified-sdk-jni/src/events.rs`.
 *
 * Every slot of the vtable is wired: the two ABI-simple slots
 * ([onWalletEvent] / [onError]) plus the platform-address and shielded
 * completion / progress slots. The completion callbacks fan the Rust-owned
 * `results` arrays out into one flat per-entry call, with a trailing
 * `…PassCompleted` boundary call carrying the pass's unix timestamp and
 * entry count. This mirrors `PlatformWalletManager.swift`'s
 * `PlatformWalletEventHandler`, which republishes each result as a Combine
 * `@Published` event; the Kotlin manager republishes onto a `SharedFlow`
 * that the sync services collect (see `PlatformWalletManager.kt`).
 *
 * ## Threading
 *
 * Callbacks fire on Rust Tokio worker threads (attached as JNI daemons).
 * Keep the bodies non-blocking and never throw across the boundary — the
 * Rust trampolines clear any pending JNI exception but cannot recover
 * work. The base implementations are inert; subclasses override to fan
 * events into their own reactive state.
 */
abstract class NativeWalletEventBridge {

    /** `on_wallet_event_fn` — descriptor `(Ljava/lang/String;)V`. */
    open fun onWalletEvent(eventDebug: String) {
        Log.v(TAG, "wallet event: $eventDebug")
    }

    /** `on_error_fn` — descriptor `(Ljava/lang/String;)V`. */
    open fun onError(message: String) {
        Log.w(TAG, "wallet manager error: $message")
    }

    /**
     * `on_platform_address_sync_completed_fn`, once per wallet result —
     * descriptor `([BZJJJJJJLjava/lang/String;)V`.
     *
     * @param walletId 32-byte wallet id.
     * @param success whether this wallet synced without error.
     * @param foundCount addresses found this pass.
     * @param absentCount addresses confirmed absent this pass.
     * @param checkpointHeight platform checkpoint height used.
     * @param newSyncHeight the new incremental-sync watermark height.
     * @param newSyncTimestamp unix seconds of [newSyncHeight]'s block.
     * @param lastKnownRecentBlock compaction marker.
     * @param errorMessage failure message, or null on success.
     */
    open fun onPlatformAddressSyncCompleted(
        walletId: ByteArray,
        success: Boolean,
        foundCount: Long,
        absentCount: Long,
        checkpointHeight: Long,
        newSyncHeight: Long,
        newSyncTimestamp: Long,
        lastKnownRecentBlock: Long,
        errorMessage: String?,
    ) {
    }

    /**
     * `on_platform_address_sync_completed_fn` pass boundary — descriptor
     * `(JI)V`. Fires once after all per-wallet [onPlatformAddressSyncCompleted]
     * calls for the pass (including when [walletCount] is 0).
     *
     * @param syncUnixSeconds unix seconds the pass completed.
     * @param walletCount number of per-wallet results that preceded this.
     */
    open fun onPlatformAddressSyncPassCompleted(syncUnixSeconds: Long, walletCount: Int) {
    }

    /**
     * `on_shielded_sync_completed_fn`, once per wallet result — descriptor
     * `([BZZZIJIJLjava/lang/String;)V`.
     *
     * @param walletId 32-byte wallet id.
     * @param success sync succeeded; numeric fields are meaningful.
     * @param skipped wallet had no bound shielded sub-wallet (mutually
     *   exclusive with [success]).
     * @param cooldownSkip success but short-circuited by the caught-up
     *   cooldown — every numeric field is zero; preserve the prior cache.
     * @param newNotes new decrypted notes this pass.
     * @param totalScanned total encrypted notes scanned (cumulative in pass).
     * @param newlySpent notes newly detected as spent this pass.
     * @param balance current unspent shielded balance after the pass.
     * @param errorMessage failure message, or null on success / skipped.
     */
    open fun onShieldedSyncCompleted(
        walletId: ByteArray,
        success: Boolean,
        skipped: Boolean,
        cooldownSkip: Boolean,
        newNotes: Int,
        totalScanned: Long,
        newlySpent: Int,
        balance: Long,
        errorMessage: String?,
    ) {
    }

    /**
     * `on_shielded_sync_completed_fn` pass boundary — descriptor `(JI)V`.
     * Fires once after all per-wallet [onShieldedSyncCompleted] calls.
     */
    open fun onShieldedSyncPassCompleted(syncUnixSeconds: Long, walletCount: Int) {
    }

    /**
     * `on_shielded_sync_progress_fn` — descriptor `(JJ)V`. Fires ~every
     * 2048 notes during a shielded pass with the cumulative downloaded-note
     * count and the latest observed block height.
     */
    open fun onShieldedSyncProgress(cumulativeScanned: Long, blockHeight: Long) {
    }

    /**
     * `on_shielded_tree_progress_fn` — descriptor `(JJ)V`. Fires per
     * committed batch as decrypted commitments append to the local Orchard
     * tree. [totalTarget] `== 0` means the on-chain total is indeterminate.
     */
    open fun onShieldedTreeProgress(leavesCommitted: Long, totalTarget: Long) {
    }

    private companion object {
        const val TAG = "DashWalletEvent"
    }
}
