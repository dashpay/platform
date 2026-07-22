package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.ffi.NativeCleaner
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import java.util.concurrent.atomic.AtomicLong

/**
 * Core wallet handle for transaction broadcasting — Android port of the
 * transaction surface of Swift's `ManagedCoreWallet`
 * (packages/swift-sdk/.../CoreWallet/ManagedCoreWallet.swift).
 *
 * Obtained via [ManagedPlatformWallet.coreWallet]. Owns the transient
 * core-wallet handle and destroys it (`core_wallet_destroy`) on [close] or a
 * [NativeCleaner] backstop, exactly like the Swift type's `deinit`. Balance /
 * address reads live on other Kotlin paths; this port carries only the
 * broadcast entry point the Core→Core send needs.
 */
class ManagedCoreWallet internal constructor(handle: Long) : AutoCloseable {

    private val handleRef = AtomicLong(handle)
    private val cleanable = NativeCleaner.register(this, HandleCleanup(handleRef))

    private val handle: Long
        get() = handleRef.get().also {
            check(it != 0L) { "ManagedCoreWallet has been closed" }
        }

    /**
     * Broadcast a transaction built by [CoreTransactionBuilder.buildSigned].
     * The funding account captured at build time is forwarded so a definitive
     * broadcast rejection releases the UTXO reservation `buildSigned` took.
     * Returns the txid as a lowercase hex string.
     */
    @Deprecated("Use the atomic FinalizedCoreTransaction send path")
    fun broadcastTransaction(tx: CoreTransaction): String =
        WalletManagerNative.coreWalletBroadcastTransaction(
            handle,
            tx.handle,
            tx.accountType.ffiValue,
            tx.accountIndex,
        )

    /**
     * Consume and broadcast a V2 finalized transaction. A handle held past the
     * reservation age bound throws the typed
     * [StaleReservationToken][org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.StaleReservationToken]
     * (native code 34, shared with the deferred-token surface) instead of
     * broadcasting against inputs key-wallet's TTL may have re-selected.
     *
     * On that refusal the handle has **already been consumed** by this call, so
     * a follow-up [abandonTransaction] is an invalid-handle error, not a recovery
     * path — there is nothing left to release, and the aged reservation is left
     * for key-wallet's TTL to reclaim (releasing it by outpoint could free a
     * newer build's reservation). Recover by rebuilding the transaction.
     */
    fun broadcastTransaction(tx: FinalizedCoreTransaction): String =
        WalletManagerNative.coreWalletBroadcastSignedTransactionV2(
            handle,
            tx.takeForBroadcast(),
        )

    /**
     * Consume a finalized transaction without sending. Below the reservation age
     * bound this releases the selected inputs immediately so a rebuild can
     * reselect them. If the handle has aged past the bound the by-outpoint
     * release is skipped — key-wallet's TTL may already have swept and
     * re-reserved the outpoint, so releasing it could free a newer build's
     * reservation — and the aged reservation is left for the TTL to reclaim; the
     * handle is torn down either way.
     */
    fun abandonTransaction(tx: FinalizedCoreTransaction) {
        WalletManagerNative.coreWalletAbandonSignedTransactionV2(
            handle,
            tx.takeForAbandon(),
        )
    }

    /**
     * Broadcast the deferred payment behind [token] and return its txid. An
     * unusable token surfaces as one of the three sibling deferred-token
     * errors — aged out
     * ([org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.StaleReservationToken]),
     * already consumed / unknown
     * ([org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.ReservationTokenConsumed]),
     * or a different wallet generation
     * ([org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.ReservationWalletMismatch]).
     */
    internal fun broadcastSignedPayment(token: Long): String =
        WalletManagerNative.coreWalletBroadcastSignedPayment(handle, token)

    override fun close() {
        cleanable.clean()
    }

    /** Destroys the transient core handle exactly once. */
    private class HandleCleanup(private val handleRef: AtomicLong) : Runnable {
        override fun run() {
            val handle = handleRef.getAndSet(0)
            if (handle != 0L) {
                WalletManagerNative.coreWalletDestroy(handle)
            }
        }
    }
}
