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
     * caveat. Builds pick change themselves (`setFunding`); this
     * accessor exists for callers that must NAME a change address
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
