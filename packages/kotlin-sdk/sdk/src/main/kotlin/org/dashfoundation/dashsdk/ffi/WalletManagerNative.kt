package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for the platform-wallet `PlatformWalletManager`
 * lifecycle and per-wallet accessors — mirrors
 * `rs-unified-sdk-jni/src/wallet_manager.rs`.
 *
 * [nativeCreate] takes the SDK handle plus the two Kotlin bridge objects
 * (persistence + event), builds the native persistence / event vtables
 * with boxed `GlobalRef` contexts, hands them — **with ownership** (the
 * vtables carry a `release_fn`) — to
 * `platform_wallet_manager_create_with_persistence_capabilities`, and
 * returns a boxed **bundle** pointer as a `jlong`. The native manager
 * frees each context box exactly once, when its last worker reference
 * drops; [nativeDestroy] shuts the manager down (bounded quiesce + join
 * of every callback-firing task) and a worker that straggles past it
 * keeps its bridge `GlobalRef` alive until that worker exits.
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

    /** Effective, post-intersection persistence contract for diagnostics. */
    external fun nativePersistenceCapabilitiesVersion(bundle: Long): Int

    external fun nativePersistenceCapabilitiesBits(bundle: Long): Long

    /** Shut down + free a bundle from [nativeCreate]. Safe on 0. */
    external fun nativeDestroy(bundle: Long)

    /**
     * Resolve every (name, descriptor) pair the Rust persistence
     * trampolines dispatch against [NativePersistenceBridge]. Returns
     * `null` when all slots resolve; otherwise the first unresolvable
     * `"name descriptor"` pair. Virtual dispatch defers each resolution to
     * the slot's first live call, so without this check a drifted
     * descriptor would surface only mid-flow at runtime — the instrumented
     * suite calls it to pin the Rust↔Kotlin lockstep up front.
     */
    external fun nativeVerifyPersistenceBridgeDescriptors(
        bridge: NativePersistenceBridge,
    ): String?

    // ── Wallet creation / restore ─────────────────────────────────────

    /**
     * Create a wallet from a BIP39 mnemonic.
     *
     * Both out-buffers are caller-allocated and validated by the native
     * side BEFORE the wallet is created: creation synchronously commits
     * Room rows through the persistence callbacks, so no fallible JNI
     * allocation may sit between that commit and the handle/id reaching
     * Kotlin (a post-commit publish failure would strand rows Kotlin has
     * no id to clean up).
     *
     * @param outWalletHandle a `LongArray(1)` receiving the created
     *   `PlatformWallet` handle.
     * @param outWalletId a `ByteArray(32)` receiving the wallet id.
     */
    external fun createWalletFromMnemonic(
        managerHandle: Long,
        mnemonic: String,
        network: Int,
        createDefaultAccounts: Boolean,
        outWalletHandle: LongArray,
        outWalletId: ByteArray,
    )

    /**
     * Create a wallet from a BIP39 mnemonic with an explicit SPV
     * birth-height override.
     *
     * @param hasBirthHeightOverride when `false`, behaves exactly like
     *   [createWalletFromMnemonic] (birth height from the confirmed header
     *   tip — the fresh-wallet default).
     * @param birthHeightOverride the SPV compact-filter scan start height
     *   when [hasBirthHeightOverride] is `true` (reinterpreted as `u32`);
     *   imported/restored mnemonics pass `0` for a full historical scan.
     * @param outWalletHandle a `LongArray(1)` receiving the created handle.
     * @param outWalletId a `ByteArray(32)` receiving the wallet id —
     *   caller-allocated and pre-validated for the same reason as
     *   [createWalletFromMnemonic].
     */
    external fun createWalletFromMnemonicWithBirthHeight(
        managerHandle: Long,
        mnemonic: String,
        network: Int,
        createDefaultAccounts: Boolean,
        hasBirthHeightOverride: Boolean,
        birthHeightOverride: Int,
        outWalletHandle: LongArray,
        outWalletId: ByteArray,
    )

    /** Rehydrate the manager from its persister (fires `onLoadWalletList`). */
    external fun loadFromPersistor(managerHandle: Long)

    /** `PlatformWallet` handle for a registered wallet id; throws NotFound. */
    external fun getWallet(managerHandle: Long, walletId: ByteArray): Long

    /** Remove one wallet from the manager (idempotent on missing). */
    external fun removeWallet(managerHandle: Long, walletId: ByteArray)

    // ── Per-wallet accessors ──────────────────────────────────────────

    /** Balance as `long[4]` = {confirmed, unconfirmed, immature, locked}. */
    external fun walletGetBalance(walletHandle: Long): LongArray

    // ── Core transaction builder (1:1 over `core_wallet_tx_builder_*`) ─
    //
    // Each step is a thin extern (one export = one FFI call, per
    // `packages/kotlin-sdk/CLAUDE.md`); the `CoreTransactionBuilder` Kotlin
    // class orchestrates the sequence, mirroring the Swift
    // `CoreTransactionBuilder` + the `.coreToCore` flow in
    // `SendViewModel.swift`. `builder` and `tx` are opaque native pointers
    // carried as `Long` (the builder's `FFITransactionBuilder`, and a
    // heap-boxed `FFICoreTransaction` from [coreTxBuilderBuildSigned]).

    /**
     * `core_wallet_tx_builder_new` — create a builder for [network]
     * (`Network.ffiValue`). Returns the builder pointer (0 after throwing);
     * free with [coreTxBuilderDestroy] or [coreTxBuilderBuildSigned] (which
     * consumes it).
     */
    external fun coreTxBuilderNew(network: Int): Long

    /**
     * `core_wallet_tx_builder_add_output` — append a recipient output.
     * [address] is network-checked Rust-side; [amount] (duffs) must be
     * positive.
     */
    external fun coreTxBuilderAddOutput(builder: Long, address: String, amount: Long)

    /**
     * `core_wallet_tx_builder_set_change_address` — override the change
     * address (network-checked). Optional; the Core→Core send relies on
     * [coreTxBuilderSetFunding], which also sets a change address.
     */
    external fun coreTxBuilderSetChangeAddress(builder: Long, address: String)

    /** `core_wallet_tx_builder_set_fee_rate` — fee rate in duffs/kB (> 0). */
    external fun coreTxBuilderSetFeeRate(builder: Long, satPerKb: Long)

    /**
     * `core_wallet_tx_builder_set_selection_strategy` — coin-selection
     * strategy by discriminant (0 SmallestFirst, 1 LargestFirst,
     * 2 BranchAndBound, 3 OptimalConsolidation, 4 Random, 5 All).
     */
    external fun coreTxBuilderSetSelectionStrategy(builder: Long, strategy: Int)

    /**
     * `core_wallet_tx_builder_set_current_height` — advisory chain-tip
     * height (funding / build override it with the wallet's last processed
     * height). Non-negative.
     */
    external fun coreTxBuilderSetCurrentHeight(builder: Long, height: Int)

    /**
     * `core_wallet_tx_builder_set_funding` — fund from a wallet account,
     * setting inputs AND the change address. [accountType]: 0 BIP44,
     * 1 BIP32, 2 CoinJoin.
     */
    external fun coreTxBuilderSetFunding(
        builder: Long,
        walletHandle: Long,
        accountType: Int,
        accountIndex: Int,
    )

    /**
     * `core_wallet_tx_builder_build_signed` — build + sign against the wallet
     * account, resolving Core ECDSA signatures via [coreSignerHandle] (a
     * `MnemonicResolverHandle`). CONSUMES the builder (do not reuse the
     * builder handle afterwards). Returns an opaque built-transaction pointer
     * (0 after throwing) for [coreWalletBroadcastTransaction] /
     * [coreTransactionFree].
     */
    external fun coreTxBuilderBuildSigned(
        builder: Long,
        walletHandle: Long,
        accountType: Int,
        accountIndex: Int,
        coreSignerHandle: Long,
    ): Long

    /** Atomic V2 finalizer; consumes [builder] and returns an opaque registry handle. */
    external fun coreTxBuilderFinalize(
        builder: Long,
        walletHandle: Long,
        accountType: Int,
        accountIndex: Int,
        coreSignerHandle: Long,
    ): Long

    /**
     * `core_wallet_tx_builder_destroy` — free a builder from [coreTxBuilderNew]
     * that was NOT consumed by [coreTxBuilderBuildSigned]. Safe on 0.
     */
    external fun coreTxBuilderDestroy(builder: Long)

    /**
     * `platform_wallet_get_core` — resolve the transient core-wallet handle
     * from a `PlatformWallet` handle, for [coreWalletBroadcastTransaction].
     * Free with [coreWalletDestroy]. Returns 0 after throwing.
     */
    external fun platformWalletGetCore(walletHandle: Long): Long

    /**
     * `core_wallet_build_signed_payment` — build + sign a standard L1 payment
     * funded from ONE of the wallet's signable funds accounts, WITHOUT
     * broadcasting.
     *
     * [coreHandle] is a core-wallet handle from [platformWalletGetCore].
     * [outputsBlob] encodes the recipients big-endian as `u32 count` then per
     * row `u32 addrLen, addr utf8, u64 amount`. [feePerKb] is duffs/kB (0 =
     * default). [coreSignerHandle] is the manager's `MnemonicResolverHandle`.
     * [fundingPath] is an optional UTF-8 BIP32 derivation-path string
     * (dashpay/platform#4184) naming the single funds account whose UTXOs fund
     * the payment: null (the default) funds from the unmixed BIP44 account; an
     * explicit account-level path (e.g. the DIP-9 CoinJoin account path) funds
     * strictly from that one account, with no union across accounts and no
     * consent gate.
     *
     * Returns a `byte[]` packed big-endian as
     * `u64 fee, u64 change, u64 reservationHandle,` then the
     * consensus-serialized signed transaction bytes (0-length / null after
     * throwing). Does NOT broadcast and does NOT persist a debit.
     *
     * `reservationHandle` is the opaque stand-in for the key-wallet reservation
     * token this build stamped on its inputs (0 = it reserved nothing); it must
     * be passed back to [coreWalletReleasePaymentReservation] to abandon the
     * build. The token itself cannot cross the ABI — it has no public
     * constructor, by design, so that it cannot be forged.
     */
    external fun coreWalletBuildSignedPayment(
        coreHandle: Long,
        outputsBlob: ByteArray,
        feePerKb: Long,
        coreSignerHandle: Long,
        fundingPath: String?,
    ): ByteArray

    /**
     * `core_wallet_release_payment_reservation` — release the UTXO reservation
     * a [coreWalletBuildSignedPayment] call took, for a build that will NOT be
     * broadcast.
     *
     * `build_signed_payment` leaves its selected inputs reserved on success,
     * expecting a broadcast to follow. A caller that abandons the build must
     * call this or the coins stay unselectable until key-wallet's 24-block TTL
     * backstop reclaims them — and that backstop never fires before the first
     * sync completes (`ReservationSet::sweep` early-returns at height 0), so an
     * abandoned build on a freshly restored wallet can otherwise strand the
     * whole balance for the life of the process (dashpay/platform#4247 review).
     * This call consults no height.
     *
     * [coreHandle] is a core-wallet handle from [platformWalletGetCore].
     * [txBytes] is the consensus-serialized signed transaction exactly as
     * [coreWalletBuildSignedPayment] returned it. [fundingPath] must be the SAME
     * optional path the build was given (null = the unmixed BIP44 account).
     * [reservationHandle] must be the SAME handle that build returned: it is the
     * ownership proof that stops this release from freeing a concurrent build's
     * inputs after key-wallet's TTL swept this build's reservation and that other
     * build re-reserved the same outpoint (dashpay/platform#4247 review). An
     * unrecognised non-zero handle throws rather than releasing unguarded.
     *
     * FOR ABANDONED BUILDS ONLY — never after a successful broadcast. Repeated
     * releases of the same abandoned build are idempotent; that is the only sense
     * in which this is safe to call twice.
     */
    external fun coreWalletReleasePaymentReservation(
        coreHandle: Long,
        txBytes: ByteArray,
        fundingPath: String?,
        reservationHandle: Long,
    )

    /**
     * `core_wallet_build_signed_payment_with_token` — build + sign a standard L1
     * payment funded from ONE of the wallet's signable funds accounts AND
     * register it for deferred submission, returning a reservation token.
     *
     * The bridge between [coreWalletBuildSignedPayment] (selects by derivation
     * path, so it can reach a **DashPay receiving-funds** account, but returns
     * raw bytes with no token and never broadcasts) and
     * [coreWalletFinalizeSignedPayment] (mints a token and can broadcast, but
     * selects only BIP44 / BIP32 / CoinJoin). Parameters are exactly
     * [coreWalletBuildSignedPayment]'s.
     *
     * On success the funding UTXOs are RESERVED and owned by the returned token:
     * consume it with [coreWalletBroadcastSignedPayment] or
     * [coreWalletReleaseSignedPayment].
     *
     * Returns a big-endian BLOB: `u64 token, u64 feeDuffs, u64 changeDuffs,
     * u32 txidLen, txid utf8, u32 txBytesLen, txBytes`.
     */
    external fun coreWalletBuildSignedPaymentWithToken(
        coreHandle: Long,
        outputsBlob: ByteArray,
        feePerKb: Long,
        coreSignerHandle: Long,
        fundingPath: String?,
    ): ByteArray

    /**
     * `core_wallet_broadcast_transaction` — broadcast a transaction built by
     * [coreTxBuilderBuildSigned]. [accountType]/[accountIndex] identify the
     * funding account so a definitive rejection releases its UTXO
     * reservation. Returns the txid as a lowercase hex string.
     */
    external fun coreWalletBroadcastTransaction(
        coreHandle: Long,
        tx: Long,
        accountType: Int,
        accountIndex: Int,
    ): String

    /** Consume and broadcast an atomically finalized V2 transaction. */
    external fun coreWalletBroadcastSignedTransactionV2(coreHandle: Long, transaction: Long): String

    /** Consume without sending and immediately release the reservation. */
    external fun coreWalletAbandonSignedTransactionV2(coreHandle: Long, transaction: Long)

    /** Idempotent cleaner fallback; abandons and releases the reservation. */
    external fun coreSignedTransactionV2Free(transaction: Long)

    /** Read the finalized transaction's fee before consumption. */
    external fun coreSignedTransactionV2Fee(transaction: Long): Long

    /** `core_wallet_destroy` — release a core handle from [platformWalletGetCore]. Safe on 0. */
    external fun coreWalletDestroy(coreHandle: Long)

    /**
     * `core_wallet_transaction_free` — free a transaction from
     * [coreTxBuilderBuildSigned] (its box AND the tx bytes it owns). Safe on
     * 0; call exactly once per built transaction.
     */
    external fun coreTransactionFree(tx: Long)

    /**
     * `core_wallet_signed_payment_finalize` — atomically fund, reserve, sign,
     * AND register a builder for deferred (BIP70/BIP270) submission in one
     * native call. Selection and reservation commit as a single unit under the
     * wallet-manager lock, closing the double-selection window. CONSUMES
     * [builder]. [accountType]/[accountIndex] identify the funding account
     * (0 BIP44, 1 BIP32, 2 CoinJoin); [coreSignerHandle] is a
     * `MnemonicResolverHandle`.
     *
     * Returns a big-endian BLOB decoded into a `SignedCoreTransaction`:
     * `u64 token, u64 feeDuffs, u32 txidLen, txid utf8, u32 txBytesLen, txBytes`.
     */
    external fun coreWalletFinalizeSignedPayment(
        builder: Long,
        walletHandle: Long,
        accountType: Int,
        accountIndex: Int,
        coreSignerHandle: Long,
    ): ByteArray

    /**
     * `core_wallet_signed_payment_broadcast` — broadcast the payment behind
     * [token], reconciling its reservation on failure and consuming the token.
     * Rather than double-broadcasting, an unusable token throws one of three
     * sibling codes — `ErrorStaleReservationToken` (34, aged out),
     * `ErrorReservationTokenConsumed` (35, already consumed/unknown), or
     * `ErrorReservationWalletMismatch` (36, different wallet generation).
     * [coreHandle] must resolve to the wallet the token was minted against.
     * Returns the txid as a lowercase hex string.
     */
    external fun coreWalletBroadcastSignedPayment(coreHandle: Long, token: Long): String

    /**
     * `core_wallet_signed_payment_release` — release the funding reservation
     * behind [token] and drop it. Idempotent: releasing an unknown /
     * already-consumed token is a silent no-op.
     */
    external fun coreWalletReleaseSignedPayment(token: Long)

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
     * Whether a shielded sync **pass is in flight right now** — distinct from
     * [shieldedSyncIsRunning], which reports whether the background *loop* is
     * alive (and stays `true` for the loop's whole lifetime, even while it
     * sleeps between passes). Drives the UI "isSyncing" mirror; gating the
     * shielded "Clear" button on the loop-alive flag would pin it disabled
     * forever. Only present when the native library is built with shielded.
     */
    external fun shieldedSyncIsSyncing(managerHandle: Long): Boolean

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
     * Reset the Rust-side shielded state — stop the background sync loop,
     * drop every wallet registration from the network-scoped coordinator,
     * empty the shared commitment tree, and reset the caught-up cooldown.
     * The per-network SQLite file stays on disk (contents reset). Throws on
     * a store-reset failure. Only present when the native library is built
     * with shielded.
     */
    external fun shieldedClear(managerHandle: Long)

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

    /**
     * Rewind one loaded wallet's in-memory compact-filter checkpoint.
     * JNI symbol:
     * `Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_spvRescanFilters`;
     * descriptor `(J[BI)V`; marshals
     * `platform_wallet_manager_spv_rescan_filters`.
     */
    external fun spvRescanFilters(
        managerHandle: Long,
        walletId: ByteArray,
        fromHeight: Int,
    )

    /** Clear all persisted SPV storage (headers, filters, state). */
    external fun spvClearStorage(managerHandle: Long)

    /** SPV `is_running`. */
    external fun spvIsRunning(managerHandle: Long): Boolean
    external fun spvStop(managerHandle: Long)

    /**
     * The proTxHashes of every masternode in the current-tip deterministic
     * masternode list whose voting-key hash matches the 20-byte [votingKeyId]
     * (hash160 of a voting public key), as a flat `byte[]` of concatenated
     * 32-byte proTxHashes (internal byte order) — the caller splits into
     * 32-byte rows. Replaces dashj's
     * `MasternodeListManager.getMasternodesByVotingKey(votingKeyId)` used by
     * contested-username voting. Returns an EMPTY (non-null) `byte[]` when the
     * masternode list hasn't synced (SPV client not running / DML unavailable)
     * or no masternode uses the key; throws only on a structural FFI error.
     * JNI symbol:
     * `Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_masternodesByVotingKey`;
     * bridges `platform_wallet_manager_masternodes_by_voting_key`.
     */
    external fun masternodesByVotingKey(
        managerHandle: Long,
        votingKeyId: ByteArray,
    ): ByteArray

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
     * Copy one manager-owned tracked-lock snapshot into a single Kotlin
     * result. JNI symbol:
     * `Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_trackedAssetLocks`;
     * descriptor:
     * `(J[B)Lorg/dashfoundation/dashsdk/ffi/TrackedAssetLocksNativeResult;`.
     *
     * JNI owns the C list/free pairing; see [TrackedAssetLocksNativeResult].
     */
    external fun trackedAssetLocks(
        managerHandle: Long,
        walletId: ByteArray,
    ): TrackedAssetLocksNativeResult

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
