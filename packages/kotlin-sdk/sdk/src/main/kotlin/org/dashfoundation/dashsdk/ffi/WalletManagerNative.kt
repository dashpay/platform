package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for the platform-wallet `PlatformWalletManager`
 * lifecycle and per-wallet accessors — mirrors
 * `rs-unified-sdk-jni/src/wallet_manager.rs`.
 *
 * [nativeCreate] takes the SDK handle plus the two Kotlin bridge objects
 * (persistence + event), builds the native persistence / event vtables
 * with boxed `GlobalRef` contexts, hands them to
 * `platform_wallet_manager_create`, and returns a boxed **bundle** pointer
 * as a `jlong`. The bundle owns the two context boxes for the manager's
 * lifetime; [nativeDestroy] shuts the manager down (quiescing every
 * callback-firing task) and only then frees them.
 *
 * The raw manager `Handle` used by the sync / wallet-accessor calls is
 * read from the bundle via [nativeManagerHandle].
 *
 * All calls throw [DashSDKException] on a non-`Success`
 * `PlatformWalletFFIResult`; the public wrappers convert via
 * `mapNativeErrors`.
 */
internal object WalletManagerNative {

    /**
     * Create a manager.
     *
     * @param sdkHandle the `SDKHandle` from [SdkNative.createTrusted].
     * @param persistenceBridge the persistence callback receiver.
     * @param eventBridge the wallet-event / error receiver.
     * @return non-zero bundle handle, or 0 after a thrown exception.
     */
    external fun nativeCreate(
        sdkHandle: Long,
        persistenceBridge: NativePersistenceBridge,
        eventBridge: NativeWalletEventBridge,
    ): Long

    /** The raw manager `Handle` for a bundle (for sync / accessor calls). */
    external fun nativeManagerHandle(bundle: Long): Long

    /** Shut down + free a bundle from [nativeCreate]. Safe on 0. */
    external fun nativeDestroy(bundle: Long)

    // ── Wallet creation / restore ─────────────────────────────────────

    /**
     * Create a wallet from a BIP39 mnemonic.
     *
     * @param outWalletHandle a `LongArray(1)` receiving the created
     *   `PlatformWallet` handle.
     * @return the 32-byte wallet id.
     */
    external fun createWalletFromMnemonic(
        managerHandle: Long,
        mnemonic: String,
        network: Int,
        createDefaultAccounts: Boolean,
        outWalletHandle: LongArray,
    ): ByteArray

    /** Rehydrate the manager from its persister (fires `onLoadWalletList`). */
    external fun loadFromPersistor(managerHandle: Long)

    /** `PlatformWallet` handle for a registered wallet id; throws NotFound. */
    external fun getWallet(managerHandle: Long, walletId: ByteArray): Long

    /** Remove one wallet from the manager (idempotent on missing). */
    external fun removeWallet(managerHandle: Long, walletId: ByteArray)

    // ── Per-wallet accessors ──────────────────────────────────────────

    /** 32-byte wallet id of a `PlatformWallet` handle. */
    external fun walletGetId(walletHandle: Long): ByteArray

    /** Balance as `long[4]` = {confirmed, unconfirmed, immature, locked}. */
    external fun walletGetBalance(walletHandle: Long): LongArray

    /**
     * Build, sign, and broadcast a Core payment from this platform wallet
     * to [addresses]/[amounts], returning the serialized signed
     * transaction bytes.
     *
     * Single composite (item 2): the Rust side acquires the core-wallet
     * handle, invokes `core_wallet_send_to_addresses` (build + sign via the
     * resolver-backed core signer AND broadcast), then releases the
     * transient handle — no orchestration crosses this boundary.
     *
     * @param accountType 0 = BIP44, 1 = BIP32.
     * @param accountIndex account index (0 for the default account).
     * @param addresses recipient Dash addresses (base58).
     * @param amounts matching duff amounts (same length as [addresses]).
     * @param coreSignerHandle a `MnemonicResolverHandle` (the manager's
     *   resolver) used for the Core ECDSA signatures.
     * @return the serialized signed transaction bytes.
     */
    external fun walletCoreSendToAddresses(
        walletHandle: Long,
        accountType: Int,
        accountIndex: Int,
        addresses: Array<String>,
        amounts: LongArray,
        coreSignerHandle: Long,
    ): ByteArray

    /**
     * Enumerate the wallet's Platform-payment addresses with cached credit
     * balances, as a big-endian blob: `u32 rowCount` then per row
     * `u8 addressType (0 P2PKH / 1 P2SH), u8[20] hash, u64 balance`. Backs
     * the top-up funding-input builder. Composite Rust call (get-platform →
     * enumerate → free → destroy-handle); no orchestration crosses here.
     */
    external fun walletAddressesWithBalances(walletHandle: Long): ByteArray

    /**
     * Fund Platform addresses from a Core L1 asset lock built from the
     * wallet balance. Composite Rust call (get-platform →
     * `platform_address_wallet_fund_from_asset_lock_signer` → free-changeset
     * → destroy-handle). Returns the changeset blob (`u32 rowCount` then per
     * row `u8 addressType, u8[20] hash, u64 balance`).
     *
     * @param recipientsBlob big-endian: `u32 rowCount` then per row
     *   `u8 addressType, u8[20] hash, u8 hasBalance (0/1), u64 balance` —
     *   exactly one row must have `hasBalance = 0` (the fee-absorbing
     *   remainder recipient).
     * @param signerHandle the platform-address per-input `SignerHandle`.
     * @param coreSignerHandle the manager's `MnemonicResolverHandle` for the
     *   asset-lock's outer state-transition signature.
     */
    external fun walletFundFromAssetLock(
        walletHandle: Long,
        amountDuffs: Long,
        accountIndex: Int,
        platformAccountIndex: Int,
        recipientsBlob: ByteArray,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): ByteArray

    /**
     * Resume a stuck Platform-address asset-lock funding from an already-
     * tracked lock. Sibling of [walletFundFromAssetLock] — same changeset
     * blob return, but keyed by the 32-byte little-endian [outPointTxid] +
     * [outPointVout] instead of a fresh amount/account.
     */
    external fun walletResumeFundFromAssetLock(
        walletHandle: Long,
        outPointTxid: ByteArray,
        outPointVout: Int,
        platformAccountIndex: Int,
        recipientsBlob: ByteArray,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): ByteArray

    // ── Wallet-signed Platform-address credit movement (ADDR-02/04) ───

    /**
     * Transfer platform-address credits to recipients, wallet-signed (AUTO
     * input selection; Rust owns selection/balancing/nonces/signing).
     * Composite Rust call (get-platform → `platform_address_wallet_transfer`
     * → free-changeset → destroy-handle). Returns the changeset blob (`u32
     * rowCount` then per row `u8 addressType, u8[20] hash, u64 balance`).
     *
     * @param outputsBlob big-endian: `u32 rowCount` then per row
     *   `u8 addressType (0 P2PKH only), u8[20] hash, u64 credits`.
     * @param signerHandle the platform-address per-input `SignerHandle`.
     */
    external fun walletPlatformAddressTransfer(
        walletHandle: Long,
        accountIndex: Int,
        outputsBlob: ByteArray,
        signerHandle: Long,
    ): ByteArray

    /**
     * Withdraw platform-address credits (full account balance, AUTO input
     * selection) to a Core L1 address, wallet-signed. The address is
     * network-checked Rust-side. Composite Rust call (get-platform →
     * `platform_address_wallet_withdraw_to_address` → free-changeset →
     * destroy-handle). Returns the changeset blob (same layout as
     * [walletPlatformAddressTransfer]).
     *
     * @param coreAddress base58 Core address.
     * @param coreFeePerByte must be a Fibonacci-sequence value (DPP rejects
     *   non-Fibonacci rates).
     * @param signerHandle the platform-address per-input `SignerHandle`.
     */
    external fun walletPlatformAddressWithdraw(
        walletHandle: Long,
        accountIndex: Int,
        coreAddress: String,
        coreFeePerByte: Int,
        signerHandle: Long,
    ): ByteArray

    /**
     * Preflight an AUTO withdrawal without signing / broadcasting / consuming
     * a Core address. Returns `long[3]` = `[canWithdraw (0/1),
     * netWithdrawable, estimatedFee]` (figures 0 unless canWithdraw). The UI
     * gates on the canWithdraw flag (the authoritative signal).
     */
    external fun walletPlatformAddressPreflightWithdrawal(
        walletHandle: Long,
        accountIndex: Int,
        coreFeePerByte: Int,
    ): LongArray

    /**
     * The version-locked minimum input / output amounts (credits) that gate
     * the transfer/withdraw UI, as `long[2]` = `[minInput, minOutput]`.
     * Composite (get-platform → read both → destroy-handle).
     */
    external fun walletPlatformAddressMinAmounts(walletHandle: Long): LongArray

    /** Destroy a `PlatformWallet` handle. */
    external fun walletDestroy(walletHandle: Long)

    // ── Sync lifecycle (start / stop / isRunning per loop) ────────────

    external fun platformAddressSyncStart(managerHandle: Long)
    external fun platformAddressSyncStop(managerHandle: Long)
    external fun platformAddressSyncIsRunning(managerHandle: Long): Boolean

    /**
     * Reset the platform-address (BLAST) sync state — the native half of the
     * Sync tab's "Clear" action (#3959). Quiesces the loop (leaves it
     * restartable, does NOT auto-restart), then clears each wallet's credit
     * balances + the provider watermark/seed so the next start is a full
     * rescan; the durable address bijection is preserved.
     */
    external fun platformAddressSyncReset(managerHandle: Long)

    external fun identitySyncStart(managerHandle: Long)
    external fun identitySyncStop(managerHandle: Long)
    external fun identitySyncIsRunning(managerHandle: Long): Boolean

    /** Shielded loop — only present when the native library is built with shielded. */
    external fun shieldedSyncStart(managerHandle: Long)
    external fun shieldedSyncStop(managerHandle: Long)
    external fun shieldedSyncIsRunning(managerHandle: Long): Boolean

    /**
     * Configure the network-scoped shielded coordinator: open (or create)
     * the per-network commitment-tree SQLite file at [dbPath], reused by
     * every subsequent [shieldedBind] on this manager. Idempotent at the
     * path level (same path no-ops; a different path throws). Only present
     * when the native library is built with shielded.
     */
    external fun shieldedConfigure(managerHandle: Long, dbPath: String)

    /**
     * Derive Orchard keys for [walletId] (32 bytes) via the SDK-level
     * mnemonic resolver [resolverHandle] and register the ZIP-32 account
     * indices in [accounts] (1..=64 non-negative entries) on the shielded
     * coordinator. Requires a prior [shieldedConfigure]; idempotent per
     * wallet. Only present when the native library is built with shielded.
     */
    external fun shieldedBind(
        managerHandle: Long,
        walletId: ByteArray,
        resolverHandle: Long,
        accounts: IntArray,
    )

    /**
     * Set the background shielded sync interval in seconds (must be
     * positive). Only present when the native library is built with
     * shielded.
     */
    external fun shieldedSyncSetInterval(managerHandle: Long, intervalSeconds: Long)

    /**
     * Run one forced shielded sync pass across all registered wallets —
     * the "Sync Now" entry point (bypasses the caught-up cooldown). Blocks
     * for the pass; call on `Dispatchers.IO`. Only present when the native
     * library is built with shielded.
     */
    external fun shieldedSyncNow(managerHandle: Long)

    /**
     * Start the Core SPV client — flattened form of
     * `platform_wallet_manager_spv_start` (11 discrete params, no config
     * struct). [userAgent] / [devnetName] pass JVM null → FFI null;
     * [devnetName] is required iff `network == Devnet.ffiValue`. The native
     * side validates the devnet / LLMQ pairing and throws on a mismatch.
     */
    external fun spvStart(
        managerHandle: Long,
        dataDir: String,
        network: Int,
        userAgent: String?,
        peers: Array<String>,
        restrictToConfiguredPeers: Boolean,
        startFromHeight: Int,
        devnetName: String?,
        llmqDevnetSize: Int,
        llmqDevnetThreshold: Int,
    )

    /**
     * Poll SPV sync progress. Fills [outLongs] (`LongArray(17)`) with the
     * integer / bool fields and [outPercentages] (`DoubleArray(5)`) with the
     * overall + per-phase percentages of the flattened `FFISpvSyncProgress`.
     * See `SpvProgressData.fromNative` for the field order.
     */
    external fun spvSyncProgress(
        managerHandle: Long,
        outLongs: LongArray,
        outPercentages: DoubleArray,
    )

    /** Unix seconds of the SPV header tip, or 0 if not running / no headers. */
    external fun spvTipUnixSeconds(managerHandle: Long): Long

    /** Clear all persisted SPV storage (headers, filters, state). */
    external fun spvClearStorage(managerHandle: Long)

    /** SPV `is_running`. */
    external fun spvIsRunning(managerHandle: Long): Boolean
    external fun spvStop(managerHandle: Long)

    // ── Wallet-memory snapshots (Wave-1B) ─────────────────────────────

    /**
     * In-memory summary of a wallet's Rust-side state as `long[4]` =
     * `[identitiesCount, watchedCount, lastScannedIndex,
     * trackedAssetLocksCount]`. Bridges `platform_wallet_get_in_memory_summary`
     * (Swift `wallet.inMemorySummary()`).
     */
    external fun walletInMemorySummary(walletHandle: Long): LongArray

    /**
     * The ids of every identity the wallet currently manages, as a flat
     * `byte[]` (concatenated 32-byte ids). Bridges
     * `platform_wallet_list_in_memory_identity_ids`.
     */
    external fun walletInMemoryIdentityIds(walletHandle: Long): ByteArray

    /**
     * The ids of every out-of-wallet / observed identity, as a flat
     * `byte[]`. Bridges `platform_wallet_list_in_memory_watched_identity_ids`.
     */
    external fun walletInMemoryWatchedIdentityIds(walletHandle: Long): ByteArray

    /**
     * The BIP-9 identity index recorded on a managed-identity snapshot
     * handle (from `TokensNative.getManagedIdentity`), or `-1` when the
     * identity is out-of-wallet. Bridges `managed_identity_get_identity_index`.
     */
    external fun managedIdentityGetIdentityIndex(identityHandle: Long): Long

    /**
     * The lifecycle status of a managed-identity snapshot handle as its
     * `IdentityStatusFFI` discriminant (0 Unknown, 1 PendingCreation,
     * 2 Active, 3 FailedCreation, 4 NotFound), or `-1` after throwing.
     * Bridges `managed_identity_get_status`.
     */
    external fun managedIdentityGetStatus(identityHandle: Long): Int

    // ── DAPI address ban list (Wave-1B) ───────────────────────────────

    /**
     * Snapshot of every DAPI address' ban state as a JSON array string, or
     * null when the list is empty. Bridges
     * `platform_wallet_manager_address_ban_info` (Swift `BannedAddressesView`).
     * Each element: `{"address","banned","banCount","bannedUntilMs","reason"}`.
     */
    external fun managerAddressBanInfo(managerHandle: Long): String?

    // ── Withdrawal preflight reason (Wave-1B micro-gap) ───────────────

    /**
     * The advisory `success_with_message` reason string surfaced when an
     * AUTO withdrawal can't be funded — the message the triple-returning
     * [walletPlatformAddressPreflightWithdrawal] discards. Returns null when
     * the withdrawal CAN proceed or no message was recorded; throws only on a
     * structural FFI error.
     */
    external fun walletPlatformAddressPreflightWithdrawalReason(
        walletHandle: Long,
        accountIndex: Int,
        coreFeePerByte: Int,
    ): String?
}
