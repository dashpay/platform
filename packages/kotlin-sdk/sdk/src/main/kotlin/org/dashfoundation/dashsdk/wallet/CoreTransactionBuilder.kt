package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.ffi.NativeCleaner
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import java.util.concurrent.atomic.AtomicLong

/**
 * key-wallet transaction builder over FFI — Android port of Swift's
 * `CoreTransactionBuilder` (packages/swift-sdk/.../CoreWallet/CoreTransactionBuilder.swift).
 *
 * Configure outputs step by step, then call [finalizeAtomic]. Rust performs
 * funding selection and ReservationSet insertion in one indivisible operation,
 * drops the wallet-manager lock, and only then invokes the mnemonic resolver.
 *
 * Owns the native `FFITransactionBuilder` pointer. Like the Swift type it is
 * freed on [close] / [finalizeAtomic] (which consumes it) or a [NativeCleaner]
 * backstop; the class is NOT thread-safe (the FFI builder must be used from
 * one thread at a time).
 *
 * ## Not a public API — drive only through [ManagedPlatformWallet.sendToAddresses]
 *
 * The old [setFunding] + [buildSigned] sequence remains only as a deprecated ABI
 * compatibility path and is not used by SDK convenience sends.
 *
 * @param network the wallet network — output and change addresses are
 *   validated against it Rust-side.
 */
class CoreTransactionBuilder internal constructor(network: Network) : AutoCloseable {

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
    internal fun addOutput(address: String, amountDuffs: Long): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderAddOutput(handle, address, amountDuffs)
    }

    /**
     * Add a zero-value OP_RETURN output carrying [data] for a MAYACHAIN-style
     * deposit memo (mirror of Swift's `addOpReturn`). Payloads over the
     * 80-byte standardness limit are rejected Rust-side without disturbing
     * outputs already added.
     * See https://docs.mayaprotocol.com/mayachain-dev-docs/concepts/sending-transactions
     */
    internal fun addOpReturn(data: ByteArray): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderAddOpReturn(handle, data)
    }

    /** Override the change address (network-checked Rust-side). */
    internal fun setChangeAddress(address: String): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderSetChangeAddress(handle, address)
    }

    /**
     * Preserve outputs in insertion order (skip BIP-69 sorting) for a
     * MAYACHAIN-style deposit — vault must stay VOUT0, memo VOUT1 (mirror of
     * Swift's `preserveOutputOrder`).
     */
    internal fun preserveOutputOrder(): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderPreserveOutputOrder(handle)
    }

    /**
     * Route change to the first selected input's address (VIN0) for a
     * MAYACHAIN-style deposit — MAYAChain identifies the depositor by VIN0
     * and pays refunds there (mirror of Swift's `changeToFirstInput`).
     */
    internal fun changeToFirstInput(): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderChangeToFirstInput(handle)
    }

    /** Set the fee rate in duffs/kB (> 0). */
    internal fun setFeeRate(satPerKb: Long): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderSetFeeRate(handle, satPerKb)
    }

    /** Set the coin-selection strategy. */
    internal fun setSelectionStrategy(strategy: SelectionStrategy): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderSetSelectionStrategy(handle, strategy.ffiValue)
    }

    /** Set the advisory chain-tip height (non-negative). */
    internal fun setCurrentHeight(height: Int): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderSetCurrentHeight(handle, height)
    }

    /** Fund from the account's UTXOs and set its change address. */
    @Deprecated("Use finalizeAtomic; split funding/signing is not concurrency-safe")
    internal fun setFunding(
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
    @Deprecated("Use finalizeAtomic; split funding/signing is not concurrency-safe")
    internal fun buildSigned(
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

    /**
     * Consume this configured builder, atomically select and reserve inputs,
     * then sign after Rust has released its wallet-manager lock.
     */
    internal fun finalizeAtomic(
        wallet: ManagedPlatformWallet,
        accountType: AccountType,
        accountIndex: Int,
        coreSignerHandle: Long,
    ): FinalizedCoreTransaction {
        require(accountIndex >= 0) { "accountIndex must be non-negative" }
        require(coreSignerHandle != 0L) { "coreSignerHandle must be non-zero" }
        // Validate every borrowed dependency before transferring builder
        // ownership. Once getAndSet(0) runs, JNI consumes the native builder.
        val walletHandle = wallet.handle
        val builderPtr = handleRef.getAndSet(0)
        check(builderPtr != 0L) { "CoreTransactionBuilder has been consumed or closed" }
        val transaction = WalletManagerNative.coreTxBuilderFinalize(
            builderPtr,
            walletHandle,
            accountType.ffiValue,
            accountIndex,
            coreSignerHandle,
        )
        val fee = try {
            WalletManagerNative.coreSignedTransactionV2Fee(transaction)
        } catch (error: Throwable) {
            WalletManagerNative.coreSignedTransactionV2Free(transaction)
            throw error
        }
        return FinalizedCoreTransaction(transaction, fee)
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

/** Ownership token for an atomically finalized Core transaction. */
class FinalizedCoreTransaction internal constructor(handle: Long, val fee: Long) : AutoCloseable {
    private val handleRef = AtomicLong(handle)
    private val cleanable = NativeCleaner.register(this, Cleanup(handleRef))

    internal fun takeForBroadcast(): Long = handleRef.getAndSet(0).also {
        check(it != 0L) { "FinalizedCoreTransaction has already been consumed" }
    }

    internal fun takeForAbandon(): Long = takeForBroadcast()

    /**
     * Consensus-serialized signed transaction bytes (copied out) WITHOUT
     * consuming the ownership token — mirror of Swift's `serializedData()`.
     * Lets the caller assert the deposit shape (e.g. MAYACHAIN's
     * vault/OP_RETURN/change output order) before deciding to broadcast.
     */
    fun serializedData(): ByteArray {
        val handle = handleRef.get()
        check(handle != 0L) { "FinalizedCoreTransaction has already been consumed" }
        return WalletManagerNative.coreSignedTransactionV2Bytes(handle)
    }

    override fun close() = cleanable.clean()

    private class Cleanup(private val handleRef: AtomicLong) : Runnable {
        override fun run() {
            val handle = handleRef.getAndSet(0)
            if (handle != 0L) WalletManagerNative.coreSignedTransactionV2Free(handle)
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
