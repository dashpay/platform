package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for shielded-funding support helpers — the Halo 2
 * prover warm-up / readiness probe and the shielded fee estimator —
 * mirrors `rs-unified-sdk-jni/src/funding.rs`.
 *
 * Internal: the public API is
 * [org.dashfoundation.dashsdk.funding.ShieldedProver] for the
 * process-global prover probes and
 * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.estimateShieldedFee]
 * for the (manager-handle) fee estimator. They back the shielded funding
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
     * and Orchard action count [numActions], computed at [managerHandle]'s
     * network-tracked platform version. No network round-trip; throws on
     * an unknown kind, an invalid manager handle, or overflow.
     */
    external fun estimateShieldedFee(managerHandle: Long, kind: Int, numActions: Int): Long

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

    /**
     * Create an identity funded from the shielded pool, Type 20 (bridges
     * `platform_wallet_manager_shielded_identity_create_from_pool`). Spends a
     * note of the fixed exit [denomination] (credits — a member of the
     * ACTIVE protocol version's on-chain set: 0.1/0.3/0.5/1.0 DASH through
     * PV12, 0.03/0.1/0.25/0.5/1.0 DASH from PV13) from the wallet's bound Orchard pool
     * ([account]) to fund a new identity at [identityIndex]. [pubkeysBlob] is
     * the same flat registration-key blob ID-08 uses; [fallbackAddress] is
     * the REQUIRED 21-byte PlatformAddress that receives the value (minus a
     * penalty) if creation fails a stateful check. [signerAddressHandle] is
     * the Keystore identity signer. Blocks for the ~30s Halo 2 proof; returns
     * the new 32-byte identity id.
     */
    external fun shieldedIdentityCreateFromPool(
        managerHandle: Long,
        walletId: ByteArray,
        resolverHandle: Long,
        account: Int,
        identityIndex: Int,
        pubkeysBlob: ByteArray,
        denomination: Long,
        fallbackAddress: ByteArray,
        signerAddressHandle: Long,
    ): ByteArray

    /**
     * Create an identity funded from a ONE-TIME Orchard key, Type 20 (bridges
     * `platform_wallet_manager_shielded_identity_create_from_one_time_key`) —
     * the L2-invitation *claim* side. Like [shieldedIdentityCreateFromPool],
     * but the Orchard spend authority is the invitation's single-use 32-byte
     * spending key [oneTimeSk] rather than the wallet's own bound pool. The
     * wallet derives the key's viewing keys, transiently scans the network for
     * the note(s) funded to it, and spends them. [changeAddressRaw43] is the
     * claimer's OWN 43-byte default Orchard address that receives any
     * over-funding change note (zero for a well-formed invitation).
     * [fundingBirthHeight] is an advisory hint: a negative value means "no
     * hint". [pubkeysBlob] / [denomination] / [fallbackAddress] /
     * [identityIndex] / [signerAddressHandle] match the pool variant. Blocks
     * for the ~30s Halo 2 proof; returns the new 32-byte identity id.
     */
    external fun shieldedIdentityCreateFromOneTimeKey(
        managerHandle: Long,
        walletId: ByteArray,
        oneTimeSk: ByteArray,
        fundingBirthHeight: Int,
        changeAddressRaw43: ByteArray,
        identityIndex: Int,
        pubkeysBlob: ByteArray,
        denomination: Long,
        fallbackAddress: ByteArray,
        signerAddressHandle: Long,
    ): ByteArray

    /**
     * Generate a fresh one-time Orchard spending key + its default payment
     * address (bridges `platform_wallet_generate_one_time_orchard_key`) — the
     * *inviter* side of an L2 shielded invitation. Handle-less: a one-time key
     * is process-local Orchard crypto, not bound to any wallet.
     *
     * Returns a single 75-byte blob: bytes `[0, 32)` are the 32-byte one-time
     * spending key and bytes `[32, 75)` are the 43-byte raw default Orchard
     * address to fund. The inviter funds a note to the address; a claimer given
     * the spending key spends it via [shieldedIdentityCreateFromOneTimeKey].
     */
    external fun generateOneTimeOrchardKey(): ByteArray

    /**
     * Derive the default 43-byte raw Orchard address from a 32-byte one-time
     * spending key (bridges `platform_wallet_orchard_address_from_spending_key`)
     * — the RNG-free counterpart of [generateOneTimeOrchardKey]. Handle-less;
     * throws if [spendingKey] is not a valid Orchard spending key.
     */
    external fun orchardAddressFromSpendingKey(spendingKey: ByteArray): ByteArray

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
        resolverHandle: Long,
        account: Int,
        recipientRaw43: ByteArray,
        amount: Long,
        memoText: String?,
    )

    /**
     * Multi-output shielded → shielded transfer, Type 16 (bridges
     * `platform_wallet_manager_shielded_transfer_multi`).
     *
     * [recipientsRaw43] holds `amounts.size` raw 43-byte Orchard addresses
     * laid out back to back (length must be `43 * amounts.size`), and
     * [amounts] the matching credit values. Each pair becomes its own note;
     * repeating the same address funds it with several independent notes.
     * [memoText] is attached to every recipient note.
     */
    external fun shieldedTransferMulti(
        managerHandle: Long,
        walletId: ByteArray,
        resolverHandle: Long,
        account: Int,
        recipientsRaw43: ByteArray,
        amounts: LongArray,
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
        resolverHandle: Long,
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
        resolverHandle: Long,
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
