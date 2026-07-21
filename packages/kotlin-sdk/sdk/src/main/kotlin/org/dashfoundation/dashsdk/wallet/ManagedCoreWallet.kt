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

    /** Consume and broadcast a V2 finalized transaction. */
    fun broadcastTransaction(tx: FinalizedCoreTransaction): String =
        WalletManagerNative.coreWalletBroadcastSignedTransactionV2(
            handle,
            tx.takeForBroadcast(),
        )

    /** Consume without sending and release the selected inputs immediately. */
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
