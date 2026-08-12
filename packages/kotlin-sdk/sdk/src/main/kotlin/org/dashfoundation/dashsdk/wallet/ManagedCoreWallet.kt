package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.errors.mapNativeErrors
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
 * [NativeCleaner] backstop, exactly like the Swift type's `deinit`.
 * Balance reads live on other Kotlin paths; this port carries the
 * broadcast entry points the Core→Core send needs plus the engine's
 * next-unused address accessors ([nextReceiveAddress] / [nextChangeAddress]).
 */
class ManagedCoreWallet internal constructor(handle: Long) : AutoCloseable {

    private val handleRef = AtomicLong(handle)
    private val cleanable = NativeCleaner.register(this, HandleCleanup(handleRef))

    private val handle: Long
        get() = handleRef.get().also {
            check(it != 0L) { "ManagedCoreWallet has been closed" }
        }

    /**
     * Consume and broadcast a finalized transaction. A handle held past the
     * reservation age bound throws the typed
     * [StaleReservationToken][org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.StaleReservationToken]
     * (native code 34, shared with the deferred-token surface) instead of
     * broadcasting against inputs key-wallet's TTL may have re-selected.
     *
     * On that refusal the handle has **already been consumed** by this call and
     * its funding reservation released owner-guarded (freed only while this
     * build still owned it; a no-op once a TTL sweep or re-reservation
     * transferred ownership). This call consumes the Kotlin-side handle up
     * front (on EVERY outcome, success included), so a follow-up
     * [abandonTransaction] fails locally with [IllegalStateException] because
     * [FinalizedCoreTransaction] has already been consumed; it never re-enters
     * native code and is not a recovery path — there is nothing left to
     * release. Recover by rebuilding the transaction, which can reselect the
     * freed inputs immediately.
     */
    fun broadcastTransaction(tx: FinalizedCoreTransaction): String = mapNativeErrors {
        WalletManagerNative.coreWalletBroadcastSignedTransaction(
            handle,
            tx.takeForBroadcast(),
        )
    }

    /**
     * Consume a finalized transaction without sending. With the build's owner
     * token present (the normal funded-finalize case) the release is
     * owner-guarded and safe at any age: it frees the selected inputs while
     * this build still owns them — so a rebuild can reselect them immediately —
     * and no-ops once key-wallet's TTL sweep or a re-reservation transferred
     * ownership. Only a token-less handle honours the reservation age bound and
     * skips its unguarded by-outpoint release past it (releasing by outpoint
     * could free a newer build's reservation), leaving the aged reservation for
     * the TTL to reclaim. The handle is torn down either way.
     *
     * Consumes the Kotlin-side handle: calling this (or [broadcastTransaction])
     * on an already-consumed [FinalizedCoreTransaction] fails locally with
     * [IllegalStateException] before any native code runs.
     */
    fun abandonTransaction(tx: FinalizedCoreTransaction) {
        WalletManagerNative.coreWalletAbandonSignedTransaction(
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

    /**
     * The engine's next unused BIP-44 EXTERNAL (receive) address for
     * [accountIndex], base58-encoded — Android port of Swift's
     * `ManagedCoreWallet.nextReceiveAddress(accountIndex:)`
     * (`SwiftDashSDKReceiveAddressReader`'s source on iOS).
     *
     * Answered from the engine's in-memory used-set, so it is
     * authoritative over the Room `core_addresses` mirror and needs no
     * persistence read. "Unused" means never seen on-chain: the engine
     * keeps no issued-marker, so repeated calls return the SAME address
     * until it receives funds (current-address semantics, not
     * per-invoice handout). Cold-start caveat (same as iOS documents):
     * on a fresh install/post-wipe the used-set starts empty, so this
     * answers index 0 until SPV replay catches the used-set up.
     */
    fun nextReceiveAddress(accountIndex: Int = 0): String {
        require(accountIndex >= 0) { "accountIndex must be non-negative, got $accountIndex" }
        // Public boundary: map the JNI layer's DashSDKException into the
        // typed DashSdkError hierarchy (the DashSdkError.kt contract).
        return mapNativeErrors {
            WalletManagerNative.coreWalletNextReceiveAddress(handle, accountIndex)
        }
    }

    /**
     * The engine's next unused BIP-44 INTERNAL (change) address for
     * [accountIndex], base58-encoded — the change-side twin of
     * [nextReceiveAddress]; same used-set semantics and cold-start
     * caveat. Builds pick change themselves during funding selection;
     * this accessor exists for callers that must NAME a change address
     * up front (e.g. `CoreTransactionBuilder.setChangeAddress`).
     */
    fun nextChangeAddress(accountIndex: Int = 0): String {
        require(accountIndex >= 0) { "accountIndex must be non-negative, got $accountIndex" }
        return mapNativeErrors {
            WalletManagerNative.coreWalletNextChangeAddress(handle, accountIndex)
        }
    }

    /**
     * Sign [message] with the private key behind [address] and return the base64
     * signature — a classic Dash signed message. See
     * [ManagedPlatformWallet.signMessage] for the full contract; drive this
     * through it, not directly.
     */
    internal fun signMessage(
        address: String,
        message: String,
        coreSignerHandle: Long,
    ): String =
        WalletManagerNative.coreWalletSignMessage(
            handle,
            address,
            message,
            coreSignerHandle,
        )

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
