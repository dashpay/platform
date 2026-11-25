package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for the pooled Core to shielded drain ("Max"), mirrors
 * `rs-unified-sdk-jni/src/wallet_drain.rs`. Each function is one call into
 * `platform-wallet-ffi`'s `shielded_wallet_drain` module.
 *
 * Internal: the public API is
 * [org.dashfoundation.dashsdk.wallet.shieldedWalletDrainPreview],
 * [org.dashfoundation.dashsdk.wallet.shieldedFundFromWalletDrain],
 * [org.dashfoundation.dashsdk.wallet.shieldedWalletDrainStatus] and
 * [org.dashfoundation.dashsdk.wallet.WalletDrainRules]. Available only when
 * the native library was built with shielded support
 * ([org.dashfoundation.dashsdk.Sdk.hasShielded]); calling these on a
 * non-shielded build throws [UnsatisfiedLinkError].
 */
internal object WalletDrainNative {
    // The rules below take no handle, so nothing else has loaded the library
    // for a caller that reaches them first.
    init { NativeLoader.ensureLoaded() }

    /**
     * Estimate a drain of [accountIndex] (bridges
     * `platform_wallet_manager_shielded_wallet_drain_preview`). Returns eight
     * longs, decoded by
     * [org.dashfoundation.dashsdk.wallet.WalletDrainPreview.decode]: status,
     * candidate input count, candidate balance (duffs), pool fee (credits),
     * has-L1-fee flag, L1 fee (duffs), has-recipient flag, recipient credits.
     */
    external fun preview(managerHandle: Long, walletId: ByteArray, accountIndex: Int): LongArray

    /**
     * Run the drain under the 16-byte [operationId] the caller already
     * stored (bridges `platform_wallet_manager_shielded_fund_from_wallet_drain`).
     * [coreSignerHandle] is the manager's `MnemonicResolverHandle`.
     *
     * Returns the outcome AND the error beside it in one packed array,
     * decoded by [org.dashfoundation.dashsdk.wallet.WalletDrainOutcome.decode]:
     * state (1 byte), has-outpoint flag (1), txid in wire order (32), then
     * vout, error code, consensus code and consensus kind as little-endian
     * 32-bit values, then the error message as UTF-8. A failed drain is NOT
     * thrown, because its error does not erase the outpoint. Throws only for
     * an argument that could not be read, before any wallet code runs.
     */
    external fun fundFromWalletDrain(
        managerHandle: Long,
        walletId: ByteArray,
        operationId: ByteArray,
        accountIndex: Int,
        recipientRaw43: ByteArray,
        coreSignerHandle: Long,
    ): ByteArray

    /**
     * Read one operation by its id (bridges
     * `platform_wallet_manager_shielded_wallet_drain_status`). Throws when
     * the read fails; otherwise returns the outcome packed as
     * [fundFromWalletDrain] does, with no error.
     */
    external fun status(managerHandle: Long, walletId: ByteArray, operationId: ByteArray): ByteArray

    /** Bridges `platform_wallet_wallet_drain_marker_decision`; true keeps the stored id. */
    external fun keepsStoredId(
        readSucceeded: Boolean,
        reportedState: Int,
        attemptOverlappedRead: Boolean,
    ): Boolean

    /**
     * Bridges `platform_wallet_wallet_drain_recovery_action`: 0 = in
     * progress, 1 = resume the named outpoint, 2 = check again.
     */
    external fun recoveryAction(
        attemptInFlight: Boolean,
        hasRead: Boolean,
        readSucceeded: Boolean,
        reportedState: Int,
        readNamedAssetLock: Boolean,
    ): Int

    /**
     * Bridges `platform_wallet_wallet_drain_status_during_host_call`: the
     * state to report (0 not sent, 1 pending, 2 completed) for a status read
     * that returned [reportedState] while a drain call of this host for the
     * same operation id is active ([hostCallActive]).
     */
    external fun statusDuringHostCall(reportedState: Int, hostCallActive: Boolean): Int

    /** Bridges `platform_wallet_wallet_drain_attempt_finished`. */
    external fun attemptFinished(state: Int): Boolean

    /** Bridges `platform_wallet_wallet_drain_preview_allows_start`. */
    external fun previewAllowsStart(previewStatus: Int): Boolean

    /** Bridges `platform_wallet_wallet_drain_start_offered`; [network] is `Network.ffiValue`. */
    external fun startOffered(network: Int): Boolean

    /**
     * Bridges `platform_wallet_wallet_drain_lock_listing`: 0 = not decided
     * here, 1 = hidden, 2 = offered with the resumable locks.
     */
    external fun lockListing(
        fundingType: Int,
        status: Int,
        hasOperationId: Boolean,
        hostHoldsStoredId: Boolean,
    ): Int
}
