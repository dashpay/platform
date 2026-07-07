package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.ffi.NativeCleaner
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import java.util.concurrent.atomic.AtomicLong

/**
 * key-wallet transaction builder over FFI — Android port of Swift's
 * `CoreTransactionBuilder` (packages/swift-sdk/.../CoreWallet/CoreTransactionBuilder.swift).
 *
 * Build step by step, [buildSigned], then broadcast separately via
 * [ManagedCoreWallet.broadcastTransaction]. Per `packages/kotlin-sdk/CLAUDE.md`
 * each native step is a thin `WalletManagerNative` extern (one call = one FFI
 * fn); THIS class does the orchestration, exactly as the Swift builder does.
 *
 * Owns the native `FFITransactionBuilder` pointer. Like the Swift type it is
 * freed on [close] / [buildSigned] (which consumes it) or a [NativeCleaner]
 * backstop; the class is NOT thread-safe (the FFI builder must be used from
 * one thread at a time).
 *
 * @param network the wallet network — output and change addresses are
 *   validated against it Rust-side.
 */
class CoreTransactionBuilder(network: Network) : AutoCloseable {

    /** Standard account derivation shape (mirror of `CoreAccountTypeFFI`). */
    enum class AccountType(val ffiValue: Int) {
        BIP44(0),
        BIP32(1),
        COIN_JOIN(2),
    }

    /**
     * Coin-selection strategy — mirror of key-wallet's `SelectionStrategy`
     * (`CoreSelectionStrategyFFI`). [ALL] drains the account.
     */
    enum class SelectionStrategy(val ffiValue: Int) {
        SMALLEST_FIRST(0),
        LARGEST_FIRST(1),
        BRANCH_AND_BOUND(2),
        OPTIMAL_CONSOLIDATION(3),
        RANDOM(4),
        ALL(5),
    }

    private val handleRef = AtomicLong(WalletManagerNative.coreTxBuilderNew(network.ffiValue))
    private val cleanable = NativeCleaner.register(this, BuilderCleanup(handleRef))

    private val handle: Long
        get() = handleRef.get().also {
            check(it != 0L) { "CoreTransactionBuilder has been consumed or closed" }
        }

    /** Append a recipient output ([amountDuffs] > 0). */
    fun addOutput(address: String, amountDuffs: Long): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderAddOutput(handle, address, amountDuffs)
    }

    /** Override the change address (network-checked Rust-side). */
    fun setChangeAddress(address: String): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderSetChangeAddress(handle, address)
    }

    /** Set the fee rate in duffs/kB (> 0). */
    fun setFeeRate(satPerKb: Long): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderSetFeeRate(handle, satPerKb)
    }

    /** Set the coin-selection strategy. */
    fun setSelectionStrategy(strategy: SelectionStrategy): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderSetSelectionStrategy(handle, strategy.ffiValue)
    }

    /** Set the advisory chain-tip height (non-negative). */
    fun setCurrentHeight(height: Int): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderSetCurrentHeight(handle, height)
    }

    /** Fund from the account's UTXOs and set its change address. */
    fun setFunding(
        wallet: ManagedPlatformWallet,
        accountType: AccountType,
        accountIndex: Int,
    ): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderSetFunding(
            handle,
            wallet.handle,
            accountType.ffiValue,
            accountIndex,
        )
    }

    /**
     * Build + sign against the account; returns the signed transaction
     * WITHOUT broadcasting. CONSUMES the builder (it is freed Rust-side and
     * this instance must not be reused afterwards).
     *
     * @param coreSignerHandle a `MnemonicResolverHandle` (the manager's
     *   resolver) used for the Core ECDSA signatures.
     */
    fun buildSigned(
        wallet: ManagedPlatformWallet,
        accountType: AccountType,
        accountIndex: Int,
        coreSignerHandle: Long,
    ): CoreTransaction {
        val builderPtr = handleRef.getAndSet(0)
        check(builderPtr != 0L) { "CoreTransactionBuilder has been consumed or closed" }
        // The FFI frees the builder on every path — mark it consumed (the
        // AtomicLong swap above) so [close] / the cleaner never double-free
        // it, matching Swift's `consumed = true` before the check.
        val txPtr = WalletManagerNative.coreTxBuilderBuildSigned(
            builderPtr,
            wallet.handle,
            accountType.ffiValue,
            accountIndex,
            coreSignerHandle,
        )
        return CoreTransaction(txPtr, accountType, accountIndex)
    }

    override fun close() {
        cleanable.clean()
    }

    /** Destroys an unconsumed builder exactly once. */
    private class BuilderCleanup(private val handleRef: AtomicLong) : Runnable {
        override fun run() {
            val handle = handleRef.getAndSet(0)
            if (handle != 0L) {
                WalletManagerNative.coreTxBuilderDestroy(handle)
            }
        }
    }
}

/**
 * A built, signed core transaction — Android port of Swift's `CoreTransaction`.
 * Broadcast it via [ManagedCoreWallet.broadcastTransaction]; its native bytes
 * are freed on [close] or a [NativeCleaner] backstop.
 *
 * @property accountType the funding account captured at build time —
 *   [ManagedCoreWallet.broadcastTransaction] forwards it so a failed
 *   broadcast releases the UTXO reservation `buildSigned` took.
 */
class CoreTransaction internal constructor(
    handle: Long,
    val accountType: CoreTransactionBuilder.AccountType,
    val accountIndex: Int,
) : AutoCloseable {

    private val handleRef = AtomicLong(handle)
    private val cleanable = NativeCleaner.register(this, TransactionCleanup(handleRef))

    /** Opaque native `FFICoreTransaction` pointer; throws if already freed. */
    internal val handle: Long
        get() = handleRef.get().also {
            check(it != 0L) { "CoreTransaction has been freed" }
        }

    override fun close() {
        cleanable.clean()
    }

    /** Frees the native transaction (box + tx bytes) exactly once. */
    private class TransactionCleanup(private val handleRef: AtomicLong) : Runnable {
        override fun run() {
            val handle = handleRef.getAndSet(0)
            if (handle != 0L) {
                WalletManagerNative.coreTransactionFree(handle)
            }
        }
    }
}
