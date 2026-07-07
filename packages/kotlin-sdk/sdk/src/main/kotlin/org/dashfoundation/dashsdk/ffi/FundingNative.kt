package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for shielded-funding support helpers — the Halo 2
 * prover warm-up / readiness probe and the shielded fee estimator —
 * mirrors `rs-unified-sdk-jni/src/funding.rs`.
 *
 * Internal: the public API is
 * [org.dashfoundation.dashsdk.funding.ShieldedProver]. These entry points
 * are process-global (no wallet handle) and back the shielded funding
 * screens' prover-status indicator and fee preview. Available only when
 * the native library was built with shielded support
 * ([org.dashfoundation.dashsdk.Sdk.hasShielded]); calling them on a
 * non-shielded build throws [UnsatisfiedLinkError].
 */
internal object FundingNative {

    /** Kick the ~30s Halo 2 proving-key build onto a background thread. Idempotent. */
    external fun warmUpProver()

    /** Whether the Halo 2 proving key is already built (UI "preparing prover…" gate). */
    external fun proverIsReady(): Boolean

    /**
     * The flat shielded fee in credits for a transition of [kind]
     * (0 = ShieldedTransfer/Shield, 1 = Unshield, 2 = ShieldedWithdrawal)
     * and Orchard action count [numActions]. Pure computation; throws on
     * an unknown kind or overflow.
     */
    external fun estimateShieldedFee(kind: Int, numActions: Int): Long

    // ── Shielded funding submits (manager-handle calls) ──────────────

    /**
     * The 43-byte raw default Orchard payment address for [account] on the
     * wallet's bound shielded sub-wallet, or null when unbound (bridges
     * `platform_wallet_manager_shielded_default_address`). The "shield to
     * self" default recipient for [shieldedFundFromAssetLock].
     */
    external fun shieldedDefaultAddress(
        managerHandle: Long,
        walletId: ByteArray,
        account: Int,
    ): ByteArray?

    /**
     * Fund the wallet's shielded pool from a fresh Core L1 asset lock
     * built from the wallet balance (bridges
     * `platform_wallet_manager_shielded_fund_from_asset_lock`).
     * [recipientRaw43] is the 43-byte raw Orchard address; [surplusOutput]
     * is the optional 21-byte remainder platform address (null = none);
     * [coreSignerHandle] is the manager's `MnemonicResolverHandle`. Blocks
     * for the ~30s Halo 2 proof; the note arrives on the next shielded sync.
     */
    external fun shieldedFundFromAssetLock(
        managerHandle: Long,
        walletId: ByteArray,
        fundingAccountIndex: Int,
        amountDuffs: Long,
        recipientRaw43: ByteArray,
        surplusOutput: ByteArray?,
        coreSignerHandle: Long,
    )

    /**
     * Resume a stuck shielded fund-from-asset-lock by outpoint (bridges
     * `platform_wallet_manager_shielded_resume_fund_from_asset_lock`).
     * [outPointTxid] is the 32-byte raw txid (little-endian wire order).
     */
    external fun shieldedResumeFundFromAssetLock(
        managerHandle: Long,
        walletId: ByteArray,
        outPointTxid: ByteArray,
        outPointVout: Int,
        recipientRaw43: ByteArray,
        surplusOutput: ByteArray?,
        coreSignerHandle: Long,
    )

    /**
     * Seed the wallet's shielded note pool toward [targetTotalNotes] in
     * batches (bridges `platform_wallet_manager_shielded_seed_pool_notes`).
     * [account] is the shielded account (usually 0); [fundingAccountIndex]
     * the Core account funding the asset locks; [coreSignerHandle] the
     * manager's `MnemonicResolverHandle`. [progressBridge] is an optional
     * [SeedPoolProgressBridge] fired per batch on a worker thread (null = no
     * progress). Blocks for one ~30s Halo 2 proof per batch.
     */
    external fun shieldedSeedPoolNotes(
        managerHandle: Long,
        walletId: ByteArray,
        account: Int,
        targetTotalNotes: Long,
        fundingAccountIndex: Int,
        coreSignerHandle: Long,
        progressBridge: SeedPoolProgressBridge?,
    )

    /**
     * Shield from Platform balance, Type 15 (bridges
     * `platform_wallet_manager_shielded_shield`). Spends [amount] credits
     * (1 DASH = 1e11) from the wallet's [paymentAccount] Platform-Payment
     * addresses into its own bound shielded pool ([shieldedAccount]).
     * Unlike the pool-internal spends below, the shield touches the
     * transparent Platform-address side, so it takes the Keystore address
     * signer [signerAddressHandle] (the manager's `signerHandle`), not a
     * resolver. Self-shield only (Rust always targets this wallet's own
     * default Orchard address). Blocks for the ~30s Halo 2 proof.
     */
    external fun shieldedShield(
        managerHandle: Long,
        walletId: ByteArray,
        shieldedAccount: Int,
        paymentAccount: Int,
        amount: Long,
        signerAddressHandle: Long,
    )

    // ── Shielded outgoing spends (types 16/17/19) ─────────────────────
    //
    // Manager-handle calls like the funding submits above; each signs with
    // the bound shielded sub-wallet's own Orchard spend key Rust-side, so no
    // signer/resolver handle crosses JNI. Each blocks for the ~30s Halo 2
    // proof and returns nothing on success.

    /**
     * Shielded → shielded transfer, Type 16 (bridges
     * `platform_wallet_manager_shielded_transfer`). [recipientRaw43] is the
     * recipient's raw 43-byte Orchard payment address; [amount] is in
     * credits (1 DASH = 1e11). [memoText] is an optional UTF-8 memo (null /
     * empty = none; Rust enforces the 32-byte UTF-8 limit).
     */
    external fun shieldedTransfer(
        managerHandle: Long,
        walletId: ByteArray,
        account: Int,
        recipientRaw43: ByteArray,
        amount: Long,
        memoText: String?,
    )

    /**
     * Shielded → Platform unshield, Type 17 (bridges
     * `platform_wallet_manager_shielded_unshield`). [toPlatformAddress] is a
     * bech32m string (`dash1…` / `tdash1…`) parsed and network-checked on
     * the Rust side; [amount] is in credits.
     */
    external fun shieldedUnshield(
        managerHandle: Long,
        walletId: ByteArray,
        account: Int,
        toPlatformAddress: String,
        amount: Long,
    )

    /**
     * Shielded → Core L1 withdrawal, Type 19 (bridges
     * `platform_wallet_manager_shielded_withdraw`). [toCoreAddress] is a
     * Base58Check string; [amount] is in credits (converted to duffs at
     * 1000:1 by the network); [coreFeePerByte] is the L1 fee rate in
     * duffs/byte (1 = the dashmate default).
     */
    external fun shieldedWithdraw(
        managerHandle: Long,
        walletId: ByteArray,
        account: Int,
        toCoreAddress: String,
        amount: Long,
        coreFeePerByte: Int,
    )
}

/**
 * Native → Kotlin bridge for shielded seed-pool progress. The native
 * `shieldedSeedPoolNotes` call invokes [onProgress] once per batch on a
 * Tokio worker thread (the JNI trampoline attaches it as a daemon), so
 * implementations must be thread-safe and non-blocking.
 */
abstract class SeedPoolProgressBridge {
    /**
     * @param batchIndex the batch just completed (1-based as it advances).
     * @param batchesTotalEstimate the estimated total batch count.
     * @param poolNotesNow the pool note count after this batch.
     * @param target the target note count.
     */
    abstract fun onProgress(
        batchIndex: Long,
        batchesTotalEstimate: Long,
        poolNotesNow: Long,
        target: Long,
    )
}
