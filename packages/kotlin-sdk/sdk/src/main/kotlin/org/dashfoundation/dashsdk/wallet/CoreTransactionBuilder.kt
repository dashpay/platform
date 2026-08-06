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
 * ## Not a public API — drive only through [ManagedPlatformWallet]
 *
 * SDK consumers reach this builder through
 * [ManagedPlatformWallet.sendToAddresses] (immediate broadcast) and
 * [ManagedPlatformWallet.buildSignedPayment] (deferred BIP70/BIP270 flows and
 * MAYACHAIN-style deposits — the option parameters there thread [addOpReturn],
 * [preserveOutputOrder] and [changeToFirstInput] into the build).
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
     * deposit memo (mirror of Swift's `addOpReturn` in
     * packages/swift-sdk/.../CoreWallet/CoreTransactionBuilder.swift). Payloads over the
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
     * Swift's `preserveOutputOrder` in
     * packages/swift-sdk/.../CoreWallet/CoreTransactionBuilder.swift).
     */
    internal fun preserveOutputOrder(): CoreTransactionBuilder = apply {
        WalletManagerNative.coreTxBuilderPreserveOutputOrder(handle)
    }

    /**
     * Route change to the first selected input's address (VIN0) for a
     * MAYACHAIN-style deposit — MAYAChain identifies the depositor by VIN0
     * and pays refunds there (mirror of Swift's `changeToFirstInput` in
     * packages/swift-sdk/.../CoreWallet/CoreTransactionBuilder.swift).
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

    /**
     * Consume this configured builder and, in ONE atomic native operation,
     * select + reserve + sign the inputs and register the built transaction for
     * deferred (BIP70/BIP270) submission. Selection and reservation commit as
     * a single unit under the wallet-manager lock, so concurrent deferred
     * builds cannot double-select an input. Returns the decoded
     * [ManagedPlatformWallet.SignedCoreTransaction].
     */
    internal fun finalizeSignedPayment(
        wallet: ManagedPlatformWallet,
        accountType: AccountType,
        accountIndex: Int,
        coreSignerHandle: Long,
    ): ManagedPlatformWallet.SignedCoreTransaction {
        require(accountIndex >= 0) { "accountIndex must be non-negative" }
        require(coreSignerHandle != 0L) { "coreSignerHandle must be non-zero" }
        // Validate every borrowed dependency before transferring builder
        // ownership. Once getAndSet(0) runs, JNI consumes the native builder.
        val walletHandle = wallet.handle
        val builderPtr = handleRef.getAndSet(0)
        check(builderPtr != 0L) { "CoreTransactionBuilder has been consumed or closed" }
        val blob = WalletManagerNative.coreWalletFinalizeSignedPayment(
            builderPtr,
            walletHandle,
            accountType.ffiValue,
            accountIndex,
            coreSignerHandle,
        )
        // Native finalization has ALREADY inserted the payment and committed its
        // reservation by the time this blob returns; the token only gains its
        // owning NativeCleaner once fromRegisterBlob finishes constructing the
        // SignedCoreTransaction. So if construction throws (allocation failure, a
        // malformed blob from an ABI mismatch, or Cleaner-registration failure)
        // the native token would be registered with no JVM owner able to release
        // it, leaking the reservation until key-wallet's TTL. Parse the token
        // first (its 8 big-endian bytes lead the blob) and release it defensively
        // if ownership construction fails, mirroring the owner-guarded release on
        // the rest of the deferred path (dashpay/platform#4185).
        var token: Long? = null
        return try {
            token = java.nio.ByteBuffer.wrap(blob).long
            ManagedPlatformWallet.SignedCoreTransaction.fromRegisterBlob(blob)
        } catch (error: Throwable) {
            token?.let { value ->
                runCatching { WalletManagerNative.coreWalletReleaseSignedPayment(value) }
            }
            throw error
        }
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
     * consuming the ownership token — mirror of Swift's `serializedData()` in
     * packages/swift-sdk/.../CoreWallet/CoreTransactionBuilder.swift.
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
