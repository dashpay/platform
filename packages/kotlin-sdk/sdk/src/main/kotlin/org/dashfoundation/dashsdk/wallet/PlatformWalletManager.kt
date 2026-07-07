package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.FundingNative
import org.dashfoundation.dashsdk.ffi.NativeWalletEventBridge
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.PlatformWalletPersistenceHandler
import org.dashfoundation.dashsdk.persistence.toHex
import org.dashfoundation.dashsdk.security.BiometricGate
import org.dashfoundation.dashsdk.security.IdentityKeyPrivateKeyDeriver
import org.dashfoundation.dashsdk.security.KeystoreSigner
import org.dashfoundation.dashsdk.security.MnemonicResolverAndPersister
import org.dashfoundation.dashsdk.security.WalletStorage
import java.util.concurrent.atomic.AtomicLong

/**
 * The one type SwiftUI's Android counterpart needs for all wallet
 * operations — port of `PlatformWalletManager.swift`.
 *
 * Owns the Rust-side `PlatformWalletManager` (via a native bundle handle)
 * which drives wallet creation from mnemonic, watch-only restore, and the
 * platform-address / identity / shielded sync loops. Persistence flows
 * through [PlatformWalletPersistenceHandler] (Room); mnemonic resolution
 * and signing flow through [MnemonicResolverAndPersister] / [KeystoreSigner]
 * (Keystore-wrapped secrets), which attach to the SDK per-call rather than
 * to the manager — matching how the Swift manager holds only a persistence
 * handler + event handler.
 *
 * ## Network lock
 *
 * The Rust manager is network-locked at construction (`WalletManager::new`
 * binds the SDK's network). This wrapper enforces the same invariant:
 * [network] is immutable and [require]d to match [sdk]'s network. Network
 * switching means closing this instance and creating a new one — see
 * [WalletManagerStore]; there is no reconfiguration path.
 *
 * ## Lifecycle
 *
 * [AutoCloseable]. [close] stops the sync loops, destroys the native
 * bundle (which runs the Rust `shutdown()` — quiescing every callback
 * before its context `GlobalRef`s are freed), then closes the resolver +
 * signer children. Order matters: the native manager must be torn down
 * before the children whose bridges its callbacks reference.
 *
 * Blocking natives are wrapped `suspend` / [withContext] `(Dispatchers.IO)`
 * and errors pass through `mapNativeErrors` at the public boundary.
 *
 * @param sdk the network-locked SDK the manager drives.
 * @param network the network; MUST equal `sdk.network`.
 * @param database the Room database persistence writes into.
 * @param walletStorage Keystore-wrapped secret store (mnemonics + keys).
 * @param biometricGate optional auth gate for out-of-window key access.
 */
class PlatformWalletManager(
    val sdk: Sdk,
    val network: Network,
    private val database: DashDatabase,
    private val walletStorage: WalletStorage,
    biometricGate: BiometricGate? = null,
) : AutoCloseable {

    init {
        require(network == sdk.network) {
            "PlatformWalletManager is network-locked: network=$network but sdk.network=${sdk.network}"
        }
    }

    // ── Reactive plumbing ─────────────────────────────────────────────

    /**
     * Scope for the SPV progress poll loop and event fan-out. A
     * [SupervisorJob] so one failing collector never tears the loop down;
     * cancelled in [close].
     */
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    /**
     * Typed sync events fanned from the native `EventHandlerCallbacks`
     * vtable. `extraBufferCapacity` absorbs bursty completion arrays without
     * blocking the Tokio worker threads the trampolines run on (a
     * `tryEmit`-friendly buffer, matching the Swift handler's
     * fire-and-republish). Replay 0 — services see events from subscription
     * onward and back-fill startup state from Room, exactly as Swift does.
     */
    private val _syncEvents = MutableSharedFlow<WalletSyncEvent>(
        replay = 0,
        extraBufferCapacity = 256,
    )

    /** Hot stream of Rust-owned sync events for the services to reduce. */
    val syncEvents: SharedFlow<WalletSyncEvent> = _syncEvents.asSharedFlow()

    // ── Children (retained for the manager's lifetime) ────────────────
    //
    // NOTE: [persistenceHandler] is declared below, after [mnemonicResolver],
    // because it needs the resolver handle to derive + persist identity
    // private keys (item 1). Property initializers run top-to-bottom, so the
    // resolver must exist first. It is still constructed before the native
    // bundle create at the bottom of the init region.

    /**
     * Event receiver that republishes every native callback onto
     * [_syncEvents] — the Kotlin analogue of Swift's
     * `PlatformWalletEventHandler`. `tryEmit` is non-blocking (safe on the
     * Tokio worker threads the trampolines attach); a full buffer drops the
     * event, in which case the services' poll fallback (`is*SyncRunning`)
     * and the Room watermark still converge liveness.
     */
    private val eventBridge: NativeWalletEventBridge = object : NativeWalletEventBridge() {
        override fun onWalletEvent(eventDebug: String) {
            _syncEvents.tryEmit(WalletSyncEvent.Generic(eventDebug))
        }

        override fun onError(message: String) {
            _syncEvents.tryEmit(WalletSyncEvent.Error(message))
        }

        override fun onPlatformAddressSyncCompleted(
            walletId: ByteArray,
            success: Boolean,
            foundCount: Long,
            absentCount: Long,
            checkpointHeight: Long,
            newSyncHeight: Long,
            newSyncTimestamp: Long,
            lastKnownRecentBlock: Long,
            errorMessage: String?,
        ) {
            _syncEvents.tryEmit(
                WalletSyncEvent.PlatformAddressResult(
                    walletId = walletId,
                    success = success,
                    foundCount = foundCount,
                    absentCount = absentCount,
                    checkpointHeight = checkpointHeight,
                    newSyncHeight = newSyncHeight,
                    newSyncTimestamp = newSyncTimestamp,
                    lastKnownRecentBlock = lastKnownRecentBlock,
                    errorMessage = errorMessage,
                ),
            )
        }

        override fun onPlatformAddressSyncPassCompleted(syncUnixSeconds: Long, walletCount: Int) {
            _syncEvents.tryEmit(
                WalletSyncEvent.PlatformAddressPassCompleted(syncUnixSeconds, walletCount),
            )
        }

        override fun onShieldedSyncCompleted(
            walletId: ByteArray,
            success: Boolean,
            skipped: Boolean,
            cooldownSkip: Boolean,
            newNotes: Int,
            totalScanned: Long,
            newlySpent: Int,
            balance: Long,
            errorMessage: String?,
        ) {
            _syncEvents.tryEmit(
                WalletSyncEvent.ShieldedResult(
                    walletId = walletId,
                    success = success,
                    skipped = skipped,
                    cooldownSkip = cooldownSkip,
                    newNotes = newNotes,
                    totalScanned = totalScanned,
                    newlySpent = newlySpent,
                    balance = balance,
                    errorMessage = errorMessage,
                ),
            )
        }

        override fun onShieldedSyncPassCompleted(syncUnixSeconds: Long, walletCount: Int) {
            _syncEvents.tryEmit(
                WalletSyncEvent.ShieldedPassCompleted(syncUnixSeconds, walletCount),
            )
        }

        override fun onShieldedSyncProgress(cumulativeScanned: Long, blockHeight: Long) {
            _syncEvents.tryEmit(WalletSyncEvent.ShieldedProgress(cumulativeScanned, blockHeight))
        }

        override fun onShieldedTreeProgress(leavesCommitted: Long, totalTarget: Long) {
            _syncEvents.tryEmit(WalletSyncEvent.ShieldedTreeProgress(leavesCommitted, totalTarget))
        }
    }

    /**
     * Mnemonic resolver + signer, backed by [walletStorage]. Exposed so
     * callers can pass [mnemonicResolverHandle] / [signerHandle] into the
     * per-call FFI entry points that derive-from-mnemonic or sign. They
     * attach to the SDK per-call, NOT to the manager.
     */
    private val mnemonicResolver = MnemonicResolverAndPersister(walletStorage)
    private val signer =
        KeystoreSigner(walletStorage, network, biometricGate, database.platformAddressDao())

    /**
     * Persistence handler (Room writer). Constructed here — after
     * [mnemonicResolver] — so it can be handed an
     * [IdentityKeyPrivateKeyDeriver] backed by the resolver handle: the
     * identity-key persist callback derives the private half via a single
     * (deadlock-safe, resolver-keyed) Rust FFI call and encrypts it into
     * [walletStorage] (item 1 — the CLAUDE.md "one allowed exception").
     */
    private val identityKeyDeriver = IdentityKeyPrivateKeyDeriver(
        network = network,
        mnemonicResolverHandle = mnemonicResolver.nativeHandle,
        walletStorage = walletStorage,
    )

    private val persistenceHandler = PlatformWalletPersistenceHandler(
        database = database,
        privateKeyDeriver = identityKeyDeriver,
        network = network,
    )

    /** `MnemonicResolverHandle` for FFI calls that derive from a stored mnemonic. */
    val mnemonicResolverHandle: Long get() = mnemonicResolver.nativeHandle

    /** `SignerHandle` for FFI calls that need signatures. */
    val signerHandle: Long get() = signer.nativeHandle

    /**
     * Re-derive the canonical identity-authentication private key at
     * `(identityIndex, keyIndex)` from this wallet's mnemonic and re-encrypt
     * it into [walletStorage] under [publicKeyData]'s hex — the repair action
     * behind `WalletKeyHealthSheet` (port of the iOS re-derive path in
     * `WalletKeyHealthSheet.swift`, which calls
     * `deriveIdentityAuthKeyAtSlot`).
     *
     * The whole `mnemonic → seed → path → key` derivation runs in Rust via
     * the resolver-keyed FFI ([IdentityKeyPrivateKeyDeriver], the CLAUDE.md
     * "one allowed exception"); Kotlin only encrypts the returned scalar.
     * Returns the recorded storage identifier (e.g. `privkey.<pubkeyHex>`),
     * or throws on a derivation / storage failure.
     */
    fun repairIdentityKey(
        walletId: ByteArray,
        publicKeyData: ByteArray,
        identityIndex: Int,
        keyIndex: Int,
    ): String? {
        require(identityIndex >= 0) { "identityIndex must be non-negative, got $identityIndex" }
        require(keyIndex >= 0) { "keyIndex must be non-negative, got $keyIndex" }
        return identityKeyDeriver.deriveAndStore(
            walletId = walletId,
            publicKeyData = publicKeyData,
            identityIndex = identityIndex,
            keyIndex = keyIndex,
        )
    }

    /**
     * Derive the full keypair at an identity key slot — the add-key path
     * (AddIdentityKeyView parity): IdentityUpdateTransition rows need the
     * public half, which the scalar-only derive discards. Lock-free
     * (resolver-keyed), suspend on IO. Caller zeroes the private half.
     */
    suspend fun deriveIdentityKeyPair(
        walletId: ByteArray,
        identityIndex: Int,
        keyIndex: Int,
    ): Pair<ByteArray, ByteArray> = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
        require(identityIndex >= 0) { "identityIndex must be non-negative, got $identityIndex" }
        require(keyIndex >= 0) { "keyIndex must be non-negative, got $keyIndex" }
        val pair = org.dashfoundation.dashsdk.errors.mapNativeErrors {
            org.dashfoundation.dashsdk.ffi.IdentityNative.deriveIdentityKeyPairWithResolver(
                network.ffiValue,
                walletId,
                mnemonicResolver.nativeHandle,
                identityIndex,
                keyIndex,
            )
        }
        check(pair.size == 2) { "keypair derive returned ${pair.size} elements" }
        pair[0] to pair[1]
    }

    /**
     * Identity registration / discovery / DPNS-name bridge. Stateless
     * wrapper over the identity JNI surface; callers thread the wallet
     * handle + [signerHandle] / [mnemonicResolverHandle] into each call.
     * ← the identity slice of `ManagedPlatformWallet.swift`.
     */
    val identityRegistration: org.dashfoundation.dashsdk.identity.IdentityRegistration =
        org.dashfoundation.dashsdk.identity.IdentityRegistration()

    /**
     * Identity credit-movement bridge — transfer / withdraw / top-up.
     * Stateless wrapper over the credits JNI surface; callers thread the
     * wallet handle + [signerHandle] into each call. ← the credit-movement
     * slice of `ManagedPlatformWallet.swift`.
     */
    val identityCredits: org.dashfoundation.dashsdk.credits.IdentityCredits =
        org.dashfoundation.dashsdk.credits.IdentityCredits()

    /**
     * Identity add/disable-keys bridge — the identity-update slice of
     * `ManagedPlatformWallet.swift` (`updateIdentity(addPublicKeys:...)`,
     * driven by Swift `AddIdentityKeyView`). Stateless; callers thread the
     * wallet handle + [signerHandle] into each call.
     */
    val identityUpdates: org.dashfoundation.dashsdk.identity.IdentityUpdates =
        org.dashfoundation.dashsdk.identity.IdentityUpdates()

    /**
     * Document purchase + set-price bridge — the document state-transition
     * slice of `ManagedPlatformWallet.swift` (driven by Swift
     * `DocumentWithPriceView`). Stateless; callers thread the wallet handle +
     * [signerHandle] into each call.
     */
    val documentTransactions: org.dashfoundation.dashsdk.documents.DocumentTransactions =
        org.dashfoundation.dashsdk.documents.DocumentTransactions()

    /**
     * Masternode contested-resource vote bridge — port of
     * `SDK.castContestedResourceVote` (driven by Swift `ContestDetailView`).
     * Stateless; callers thread the SDK handle into each call.
     */
    val voteCasting: org.dashfoundation.dashsdk.voting.VoteCasting =
        org.dashfoundation.dashsdk.voting.VoteCasting()

    // ── Native manager bundle ─────────────────────────────────────────

    private val bundleRef: AtomicLong = AtomicLong(
        try {
            WalletManagerNative.nativeCreate(sdk.handle, persistenceHandler, eventBridge)
        } catch (t: Throwable) {
            // The resolver/signer fields above already hold native
            // handles; without this, a failed manager construction
            // (closed SDK, GlobalRef failure) leaks them for the
            // process lifetime — no user code ever gets to call close().
            runCatching { mnemonicResolver.close() }
            runCatching { signer.close() }
            throw t
        },
    )

    /** Raw native manager `Handle` (for the sync / wallet-accessor calls). */
    private val managerHandle: Long =
        WalletManagerNative.nativeManagerHandle(bundleRef.get())

    // ── Published wallet map ──────────────────────────────────────────

    private val _wallets = MutableStateFlow<Map<String, ManagedPlatformWallet>>(emptyMap())

    /**
     * All wallets currently held by the Rust manager, keyed by walletId
     * hex. Mirror of the Swift `wallets` map — the Rust manager holds N
     * wallets concurrently; look up a specific wallet by its hex id.
     */
    val wallets: StateFlow<Map<String, ManagedPlatformWallet>> = _wallets.asStateFlow()

    // Per-wallet-id Core-send locks. Every ManagedPlatformWallet this manager
    // builds is handed the SAME Mutex for its wallet id, so all wrappers for
    // one wallet id serialize their split setFunding/buildSigned sequence
    // together — closing the double-select window even when loadPersistedWallets
    // hands out a fresh wrapper for a wallet id whose earlier wrapper is still
    // held (see ManagedPlatformWallet.sendToAddresses). computeIfAbsent (NOT
    // getOrPut, which is non-atomic on ConcurrentHashMap) guarantees a single
    // shared instance. Lives on the manager and dies with it.
    private val coreSendMutexes =
        java.util.concurrent.ConcurrentHashMap<String, kotlinx.coroutines.sync.Mutex>()

    private fun coreSendMutex(walletIdHex: String): kotlinx.coroutines.sync.Mutex =
        coreSendMutexes.computeIfAbsent(walletIdHex) { kotlinx.coroutines.sync.Mutex() }

    // ── Wallet creation ───────────────────────────────────────────────

    /**
     * Create a wallet from a BIP39 mnemonic — port of Swift
     * `createWallet(mnemonic:network:name:)`.
     *
     * Ordering (matches Swift `CreateWalletView`): the FFI derives the
     * wallet from the phrase and returns the 32-byte id FIRST; only then
     * do we [WalletStorage.storeMnemonic] keyed by that id (the id is not
     * known until the FFI returns). The mnemonic never crosses to Rust
     * here beyond the derivation input — Kotlin only persists it.
     *
     * @param mnemonic the BIP39 phrase (English).
     * @param name optional label, persisted onto the Room wallet row —
     *   the Rust persister doesn't know user-facing labels, so Kotlin
     *   stamps it after the FFI returns the scoped id, exactly as Swift
     *   `CreateWalletView` writes `walletLabel` onto the persisted wallet
     *   (name via the manager + keychain metadata blob).
     * @param createDefaultAccounts whether to seed default accounts.
     */
    suspend fun createWallet(
        mnemonic: String,
        name: String? = null,
        createDefaultAccounts: Boolean = true,
    ): ManagedPlatformWallet = withContext(Dispatchers.IO) {
        val outHandle = LongArray(1)
        val walletId = mapNativeErrors {
            WalletManagerNative.createWalletFromMnemonic(
                managerHandle,
                mnemonic,
                network.ffiValue,
                createDefaultAccounts,
                outHandle,
            )
        }
        // Adopt the native handle into its AutoCloseable owner BEFORE the
        // fallible Keystore/Room steps — a raw jlong has no owner, so a
        // throw below would otherwise leak the Rust registry entry.
        val managed = ManagedPlatformWallet(
            handle = outHandle[0],
            walletId = walletId,
            coreSendMutex = coreSendMutex(walletId.toHex()),
        )
        try {
            // Store the mnemonic keyed by the id the FFI just derived.
            walletStorage.storeMnemonic(walletId, mnemonic)

            // Persist the display name onto the Room row the persistence
            // callbacks just wrote (a persist step, not orchestration —
            // ← CreateWalletView.swift stamping the label per created wallet).
            name?.trim()?.takeIf { it.isNotEmpty() }?.let { label ->
                database.walletDao().updateName(walletId, label, System.currentTimeMillis())
            }
        } catch (t: Throwable) {
            // Full rollback, not just the wrapper's Arc clone: the wallet is
            // already REGISTERED in the native manager (and its persistence
            // callbacks may have written Room rows), so it must be
            // unregistered — removeWallet runs the persistence cascade —
            // or the next loadPersistedWallets resurrects it as an orphan
            // with no Keystore mnemonic. Best-effort scrub of a partially
            // written mnemonic keeps the Keystore from drifting.
            runCatching {
                mapNativeErrors { WalletManagerNative.removeWallet(managerHandle, walletId) }
            }
            runCatching { walletStorage.deleteMnemonic(walletId) }
            managed.close()
            throw t
        }
        _wallets.update { it + (walletId.toHex() to managed) }
        managed
    }

    // ── Watch-only restore ────────────────────────────────────────────

    /**
     * Rehydrate wallets from Room on app launch — port of Swift
     * `loadFromPersistor()`.
     *
     * Calls `platform_wallet_manager_load_from_persistor`, which fires the
     * `onLoadWalletList` persistence callback; Rust reconstructs each
     * persisted wallet as watch-only. We then [WalletManagerNative.getWallet]
     * per restorable id to obtain a [ManagedPlatformWallet] handle.
     *
     * Idempotent: with no persisted state, leaves [wallets] untouched.
     * Signing fails on watch-only wallets until a future unlock flow.
     */
    suspend fun loadPersistedWallets(): List<ManagedPlatformWallet> = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.loadFromPersistor(managerHandle) }

        // Room is the source of truth for the restorable id list — the same
        // rows the load callback just fed to Rust. Scope to THIS network so
        // a per-network manager only restores its own wallets (matching the
        // network lock and the Swift persistence handler's network scoping).
        val ids = database.walletDao().getByNetwork(network.ffiValue)
            .map { it.walletId }
            .filter { it.size == 32 }

        val restored = ArrayList<ManagedPlatformWallet>(ids.size)
        val additions = LinkedHashMap<String, ManagedPlatformWallet>()
        for (walletId in ids) {
            val handle = try {
                mapNativeErrors { WalletManagerNative.getWallet(managerHandle, walletId) }
            } catch (_: Exception) {
                // One wallet failing (id/xpub drift) doesn't fail the batch.
                continue
            }
            val managed = ManagedPlatformWallet(
                handle = handle,
                walletId = walletId,
                coreSendMutex = coreSendMutex(walletId.toHex()),
            )
            restored.add(managed)
            additions[walletId.toHex()] = managed
        }
        if (additions.isNotEmpty()) {
            _wallets.update { it + additions }
        }
        restored
    }

    /** The managed wallet with the given 32-byte id, or null if not loaded. */
    fun wallet(forWalletId: ByteArray): ManagedPlatformWallet? =
        _wallets.value[forWalletId.toHex()]

    // ── Sync lifecycle (mirror of Swift start/stop) ───────────────────

    suspend fun startPlatformAddressSync() = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.platformAddressSyncStart(managerHandle) }
    }

    suspend fun stopPlatformAddressSync() = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.platformAddressSyncStop(managerHandle) }
    }

    suspend fun isPlatformAddressSyncRunning(): Boolean = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.platformAddressSyncIsRunning(managerHandle) }
    }

    /**
     * Reset the platform-address (BLAST) sync state — the native side of the
     * Sync tab's "Clear" action (#3959), port of Swift
     * `resetPlatformAddressSyncState()`. Quiesces the loop (leaves it
     * restartable, does NOT auto-restart), then clears each wallet's credit
     * balances and the provider watermark/seed so the next start rescans from
     * scratch; the durable address derivation state is preserved. The
     * caller (`PlatformBalanceSyncService.clearLocalState`) runs this FIRST,
     * fail-closed, before clearing the Room mirror.
     */
    suspend fun resetPlatformAddressSyncState() = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.platformAddressSyncReset(managerHandle) }
    }

    suspend fun startIdentitySync() = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.identitySyncStart(managerHandle) }
    }

    suspend fun stopIdentitySync() = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.identitySyncStop(managerHandle) }
    }

    suspend fun isIdentitySyncRunning(): Boolean = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.identitySyncIsRunning(managerHandle) }
    }

    /**
     * Start the shielded sync loop. Only meaningful when the native
     * library was built with shielded support ([Sdk.hasShielded]); the
     * Rust entry point is absent otherwise and this throws.
     */
    suspend fun startShieldedSync() = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.shieldedSyncStart(managerHandle) }
    }

    suspend fun stopShieldedSync() = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.shieldedSyncStop(managerHandle) }
    }

    suspend fun isShieldedSyncRunning(): Boolean = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.shieldedSyncIsRunning(managerHandle) }
    }

    /**
     * Whether a shielded sync **pass is currently in flight** — port of
     * Swift's `PlatformWalletManager.isShieldedSyncing()` (polled into
     * `shieldedSyncIsSyncing`). Distinct from [isShieldedSyncRunning], which
     * reports whether the background *loop* is alive and stays `true` for its
     * whole lifetime (including between passes). Use this — not the loop-alive
     * flag — for the UI "syncing…" indicator and for gating actions (like the
     * shielded Clear button) that only need to avoid a pass mutating the store
     * underneath them.
     */
    suspend fun isShieldedSyncing(): Boolean = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.shieldedSyncIsSyncing(managerHandle) }
    }

    /**
     * Configure the network-scoped shielded coordinator — port of Swift's
     * `PlatformWalletManager.configureShielded(dbPath:)`
     * (`PlatformWalletManagerShieldedSync.swift`). Opens (or creates) the
     * per-network commitment-tree SQLite file at [dbPath]; every subsequent
     * [bindShielded] on this manager reuses the one handle. Idempotent at
     * the path level: the same path no-ops, a different path throws (the
     * SQLite handle can't be repointed mid-flight). Must run before any
     * [bindShielded]. Only meaningful on a shielded build ([Sdk.hasShielded]);
     * the native entry point is absent otherwise and this throws.
     */
    suspend fun configureShielded(dbPath: String) = withContext(Dispatchers.IO) {
        require(dbPath.isNotBlank()) { "dbPath must not be blank" }
        mapNativeErrors { WalletManagerNative.shieldedConfigure(managerHandle, dbPath) }
    }

    /**
     * Derive Orchard keys for [walletId] from this manager's mnemonic
     * resolver and register the ZIP-32 account indices in [accounts] on the
     * shielded coordinator — port of Swift's
     * `PlatformWalletManager.bindShielded(walletId:resolver:accounts:)`
     * (`PlatformWalletManagerShieldedSync.swift`); the resolver is the
     * manager-owned one rather than a per-call parameter, matching the
     * other resolver-keyed wrappers here. The resolver fires exactly once;
     * mnemonic and seed are zeroized Rust-side before this returns.
     * Requires a prior [configureShielded]; idempotent — a second call
     * replaces the previous binding for the same wallet. Only meaningful on
     * a shielded build ([Sdk.hasShielded]); the native entry point is
     * absent otherwise and this throws.
     */
    suspend fun bindShielded(
        walletId: ByteArray,
        accounts: List<Int> = listOf(0),
    ) = withContext(Dispatchers.IO) {
        require(walletId.size == 32) {
            "walletId must be exactly 32 bytes, got ${walletId.size}"
        }
        require(accounts.isNotEmpty()) { "accounts must be non-empty" }
        require(accounts.size <= 64) {
            "accounts must contain at most 64 entries, got ${accounts.size}"
        }
        require(accounts.all { it >= 0 }) { "accounts must be non-negative" }
        mapNativeErrors {
            WalletManagerNative.shieldedBind(
                managerHandle,
                walletId,
                mnemonicResolver.nativeHandle,
                accounts.toIntArray(),
            )
        }
    }

    /**
     * Set the background shielded sync interval — port of Swift's
     * `PlatformWalletManager.setShieldedSyncInterval(seconds:)`
     * (`PlatformWalletManagerShieldedSync.swift`). Only meaningful on a
     * shielded build ([Sdk.hasShielded]); the native entry point is absent
     * otherwise and this throws.
     */
    suspend fun setShieldedSyncInterval(seconds: Long) = withContext(Dispatchers.IO) {
        require(seconds > 0) { "seconds must be positive, got $seconds" }
        mapNativeErrors { WalletManagerNative.shieldedSyncSetInterval(managerHandle, seconds) }
    }

    /**
     * Run one forced shielded sync pass across all registered wallets —
     * port of Swift's `PlatformWalletManager.syncShieldedNow()`
     * (`PlatformWalletManagerShieldedSync.swift`), the user-initiated
     * "Sync Now" entry point (bypasses the caught-up cooldown). Blocks
     * for the pass on `Dispatchers.IO`. Only meaningful on a shielded
     * build ([Sdk.hasShielded]); the native entry point is absent
     * otherwise and this throws.
     */
    suspend fun syncShieldedNow() = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.shieldedSyncNow(managerHandle) }
    }

    /**
     * Reset the Rust-side shielded state on this manager — port of Swift's
     * `PlatformWalletManager.clearShielded()`
     * (`PlatformWalletManagerShieldedSync.swift`). Quiesces the background
     * sync loop, drops every wallet registration from the network-scoped
     * coordinator, empties the shared commitment tree, and resets the
     * caught-up cooldown, so the next [bindShielded] + sync cold-rebuilds
     * from index 0.
     *
     * The per-network SQLite file stays on disk (its contents are reset);
     * the host must wipe its own Room rows AFTER this succeeds
     * ([org.dashfoundation.dashsdk.services.ShieldedService.clearLocalState]
     * orders the two). Throws on a store-reset failure so the caller can
     * fail closed and keep its rows rather than orphan a still-populated
     * tree. Only meaningful on a shielded build ([Sdk.hasShielded]); the
     * native entry point is absent otherwise and this throws.
     */
    suspend fun clearShieldedStorage() = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.shieldedClear(managerHandle) }
    }

    // ── Shielded funding submits ──────────────────────────────────────
    //
    // These are manager-handle FFI calls (they resolve the wallet + its
    // shielded coordinator Rust-side from the walletId), matching the Swift
    // `PlatformWalletManager.shieldedFundFromAssetLock` / `seedShieldedPoolNotes`
    // shapes — hence they live on the manager, not `ManagedPlatformWallet`.
    // Only meaningful on a shielded build ([Sdk.hasShielded]); the native
    // entry points are absent otherwise and these throw.

    /**
     * The 43-byte raw default Orchard payment address for [account] on the
     * wallet's bound shielded sub-wallet, or null when the wallet has no
     * bound shielded sub-wallet — port of Swift's `shieldedDefaultAddress`.
     * The natural "shield to self" default recipient for
     * [shieldedFundFromAssetLock].
     */
    suspend fun shieldedDefaultAddress(
        walletId: ByteArray,
        account: Int = 0,
    ): ByteArray? = withContext(Dispatchers.IO) {
        require(account >= 0) { "account must be non-negative, got $account" }
        mapNativeErrors {
            FundingNative.shieldedDefaultAddress(managerHandle, walletId, account)
        }
    }

    /**
     * Fund a wallet's shielded (Orchard) pool from a fresh Core L1 asset
     * lock — port of Swift's `shieldedFundFromAssetLock`. Blocks for the
     * ~30s Halo 2 proof; the shielded note itself arrives on the next
     * shielded sync pass, so nothing is returned.
     *
     * @param walletId the 32-byte wallet id.
     * @param recipientRaw43 the 43-byte raw Orchard payment address
     *   (11-byte diversifier + 32-byte pk_d).
     * @param fundingAccountIndex the Core BIP44 account funding the lock.
     * @param amountDuffs the L1 lock amount in duffs.
     * @param surplusOutput optional 21-byte remainder platform address.
     */
    suspend fun shieldedFundFromAssetLock(
        walletId: ByteArray,
        recipientRaw43: ByteArray,
        amountDuffs: Long,
        fundingAccountIndex: Int = 0,
        surplusOutput: ByteArray? = null,
    ): Unit = withContext(Dispatchers.IO) {
        require(amountDuffs > 0) { "amountDuffs must be positive, got $amountDuffs" }
        require(fundingAccountIndex >= 0) {
            "fundingAccountIndex must be non-negative, got $fundingAccountIndex"
        }
        mapNativeErrors {
            FundingNative.shieldedFundFromAssetLock(
                managerHandle,
                walletId,
                fundingAccountIndex,
                amountDuffs,
                recipientRaw43,
                surplusOutput,
                mnemonicResolverHandle,
            )
        }
    }

    /**
     * Resume a stuck shielded fund-from-asset-lock from an already-tracked
     * lock — port of Swift's `shieldedResumeFundFromAssetLock`.
     *
     * @param outPointTxid the 32-byte raw txid (little-endian wire order).
     */
    suspend fun shieldedResumeFundFromAssetLock(
        walletId: ByteArray,
        outPointTxid: ByteArray,
        outPointVout: Int,
        recipientRaw43: ByteArray,
        surplusOutput: ByteArray? = null,
    ): Unit = withContext(Dispatchers.IO) {
        require(outPointTxid.size == 32) {
            "outPointTxid must be exactly 32 bytes, got ${outPointTxid.size}"
        }
        require(outPointVout >= 0) { "outPointVout must be non-negative, got $outPointVout" }
        mapNativeErrors {
            FundingNative.shieldedResumeFundFromAssetLock(
                managerHandle,
                walletId,
                outPointTxid,
                outPointVout,
                recipientRaw43,
                surplusOutput,
                mnemonicResolverHandle,
            )
        }
    }

    /**
     * Seed a wallet's shielded note pool toward [targetTotalNotes] in
     * batches — port of Swift's `seedShieldedPoolNotes`. Each batch builds a
     * real + zero-value filler note set behind one ~30s Halo 2 proof, run
     * serially. [onProgress] (optional) fires once per batch; it is invoked
     * on a background thread and marshalled onto the caller via the
     * [FundingNative.SeedPoolProgressBridge].
     *
     * @param walletId the 32-byte wallet id.
     * @param account the shielded account (usually 0).
     * @param fundingAccountIndex the Core BIP44 account funding the locks.
     */
    suspend fun seedShieldedPoolNotes(
        walletId: ByteArray,
        targetTotalNotes: Long,
        account: Int = 0,
        fundingAccountIndex: Int = 0,
        onProgress: ((batchIndex: Long, batchesTotalEstimate: Long, poolNotesNow: Long, target: Long) -> Unit)? = null,
    ): Unit = withContext(Dispatchers.IO) {
        require(targetTotalNotes > 0) {
            "targetTotalNotes must be positive, got $targetTotalNotes"
        }
        require(account >= 0) { "account must be non-negative, got $account" }
        require(fundingAccountIndex >= 0) {
            "fundingAccountIndex must be non-negative, got $fundingAccountIndex"
        }
        val bridge = onProgress?.let { cb ->
            object : org.dashfoundation.dashsdk.ffi.SeedPoolProgressBridge() {
                override fun onProgress(
                    batchIndex: Long,
                    batchesTotalEstimate: Long,
                    poolNotesNow: Long,
                    target: Long,
                ) = cb(batchIndex, batchesTotalEstimate, poolNotesNow, target)
            }
        }
        mapNativeErrors {
            FundingNative.shieldedSeedPoolNotes(
                managerHandle,
                walletId,
                account,
                targetTotalNotes,
                fundingAccountIndex,
                mnemonicResolverHandle,
                bridge,
            )
        }
    }

    // ── Shielded outgoing spends (types 16/17/19) ──────────────────────
    //
    // Manager-handle FFI calls like the funding submits above; each signs
    // with the bound shielded sub-wallet's own Orchard spend key Rust-side
    // (cached by [bindShielded]), so no signer or resolver handle crosses
    // JNI — matching the Swift counterparts, which take no signer either.
    // Each blocks for the ~30s Halo 2 proof on Dispatchers.IO. On the
    // AMBIGUOUS outcome (broadcast accepted, execution unconfirmed) they
    // throw [org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed]
    // — the caller must NOT retry (the spent notes stay reserved Rust-side;
    // the next shielded sync reconciles the outcome).

    /**
     * Shielded → shielded transfer (Type 16) — port of Swift's
     * `PlatformWalletManager.shieldedTransfer(walletId:account:recipientRaw43:amount:memo:)`
     * (`PlatformWalletManagerShieldedSync.swift`). Spends notes from
     * [account] on [walletId] and creates a new note for [recipientRaw43].
     *
     * @param walletId the 32-byte wallet id.
     * @param recipientRaw43 the recipient's raw 43-byte Orchard payment
     *   address (11-byte diversifier + 32-byte pk_d) — same shape
     *   [shieldedDefaultAddress] returns.
     * @param amount credits to transfer (1 DASH = 1e11 credits).
     * @param account the ZIP-32 shielded account to spend from (usually 0).
     * @param memo optional UTF-8 memo attached to the recipient's note
     *   (null / empty = no memo; the UTF-8 byte length must be at most 32
     *   or Rust rejects it — the 36-byte on-chain encoding is Rust-side).
     */
    suspend fun shieldedTransfer(
        walletId: ByteArray,
        recipientRaw43: ByteArray,
        amount: Long,
        account: Int = 0,
        memo: String? = null,
    ): Unit = withContext(Dispatchers.IO) {
        require(amount > 0) { "amount must be positive, got $amount" }
        require(account >= 0) { "account must be non-negative, got $account" }
        require(recipientRaw43.size == 43) {
            "recipientRaw43 must be exactly 43 bytes, got ${recipientRaw43.size}"
        }
        mapNativeErrors {
            FundingNative.shieldedTransfer(
                managerHandle,
                walletId,
                account,
                recipientRaw43,
                amount,
                memo?.takeIf { it.isNotEmpty() },
            )
        }
    }

    /**
     * Shielded → Platform unshield (Type 17) — port of Swift's
     * `PlatformWalletManager.shieldedUnshield(walletId:account:toPlatformAddress:amount:)`
     * (`PlatformWalletManagerShieldedSync.swift`). Spends notes from
     * [account] on [walletId] and credits [toPlatformAddress].
     *
     * @param toPlatformAddress the recipient as a bech32m string
     *   (`dash1…` mainnet / `tdash1…` testnet). Forwarded as-is — Rust
     *   parses it via `PlatformAddress::from_bech32m_string` and verifies
     *   the network, so hosts never hand-roll the storage variant tag.
     * @param amount credits to unshield (1 DASH = 1e11 credits).
     */
    suspend fun shieldedUnshield(
        walletId: ByteArray,
        toPlatformAddress: String,
        amount: Long,
        account: Int = 0,
    ): Unit = withContext(Dispatchers.IO) {
        require(amount > 0) { "amount must be positive, got $amount" }
        require(account >= 0) { "account must be non-negative, got $account" }
        require(toPlatformAddress.isNotBlank()) { "toPlatformAddress is empty" }
        mapNativeErrors {
            FundingNative.shieldedUnshield(
                managerHandle,
                walletId,
                account,
                toPlatformAddress,
                amount,
            )
        }
    }

    /**
     * Shielded → Core L1 withdrawal (Type 19) — port of Swift's
     * `PlatformWalletManager.shieldedWithdraw(walletId:account:toCoreAddress:amount:coreFeePerByte:)`
     * (`PlatformWalletManagerShieldedSync.swift`). Spends notes from
     * [account] on [walletId] and creates an L1 withdrawal to
     * [toCoreAddress].
     *
     * @param toCoreAddress the L1 recipient as a Base58Check string; Rust
     *   parses it and verifies it matches the wallet's network.
     * @param amount credits to withdraw (1 DASH = 1e11 credits); the
     *   network converts to L1 duffs at the 1000:1 rate.
     * @param coreFeePerByte the L1 fee rate in duffs/byte (1 is the
     *   dashmate default, matching the Swift default).
     */
    suspend fun shieldedWithdraw(
        walletId: ByteArray,
        toCoreAddress: String,
        amount: Long,
        coreFeePerByte: Int = 1,
        account: Int = 0,
    ): Unit = withContext(Dispatchers.IO) {
        require(amount > 0) { "amount must be positive, got $amount" }
        require(account >= 0) { "account must be non-negative, got $account" }
        require(coreFeePerByte > 0) {
            "coreFeePerByte must be positive, got $coreFeePerByte"
        }
        require(toCoreAddress.isNotBlank()) { "toCoreAddress is empty" }
        mapNativeErrors {
            FundingNative.shieldedWithdraw(
                managerHandle,
                walletId,
                account,
                toCoreAddress,
                amount,
                coreFeePerByte,
            )
        }
    }

    /**
     * Start the Core SPV client — port of Swift `startSpv(...)`. Flattened
     * `platform_wallet_manager_spv_start`; the native side owns the sync
     * loop from here (this wrapper only launches it). Starts the 1 Hz
     * progress poll ([spvProgress]) mirroring Swift's `startProgressPolling`.
     *
     * @param dataDir SPV storage directory (required).
     * @param peers `host:port` seeds; empty for the network defaults.
     * @param userAgent optional; null → the FFI default.
     * @param devnetName required iff [network] is [Network.DEVNET].
     */
    suspend fun startSpv(
        dataDir: String,
        peers: List<String> = emptyList(),
        userAgent: String? = null,
        restrictToConfiguredPeers: Boolean = false,
        startFromHeight: Int = 0,
        devnetName: String? = null,
        llmqDevnetSize: Int = 0,
        llmqDevnetThreshold: Int = 0,
    ) {
        withContext(Dispatchers.IO) {
            mapNativeErrors {
                WalletManagerNative.spvStart(
                    managerHandle,
                    dataDir,
                    network.ffiValue,
                    userAgent,
                    peers.toTypedArray(),
                    restrictToConfiguredPeers,
                    startFromHeight,
                    devnetName,
                    llmqDevnetSize,
                    llmqDevnetThreshold,
                )
            }
        }
        startSpvProgressPolling()
    }

    /** Whether the Core SPV client is running. */
    suspend fun isSpvRunning(): Boolean = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.spvIsRunning(managerHandle) }
    }

    suspend fun stopSpv() = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.spvStop(managerHandle) }
    }

    /** Clear all persisted SPV storage (headers, filters, state). */
    suspend fun clearSpvStorage() = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.spvClearStorage(managerHandle) }
    }

    /** One-shot SPV progress poll (the same call [spvProgress] loops on). */
    suspend fun spvSyncProgress(): SpvSyncProgressData = withContext(Dispatchers.IO) {
        val longs = LongArray(17)
        val percentages = DoubleArray(5)
        mapNativeErrors { WalletManagerNative.spvSyncProgress(managerHandle, longs, percentages) }
        SpvSyncProgressData.fromNative(longs, percentages)
    }

    /** Unix seconds of the SPV header tip, or 0 if not running / no headers. */
    suspend fun spvTipUnixSeconds(): Long = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.spvTipUnixSeconds(managerHandle) }
    }

    // ── DAPI address ban list (Wave-1B) ───────────────────────────────

    /**
     * Snapshot of every DAPI address' ban state as a JSON array string, or
     * null when the list is empty — port of the manager-level ban query
     * behind Swift `BannedAddressesView`. Bridges
     * `platform_wallet_manager_address_ban_info`. Each element:
     * `{"address","banned","banCount","bannedUntilMs","reason"}`.
     *
     * Synchronous by design (the native side takes a cheap `blocking_read`);
     * callers that need to stay off the main thread wrap it themselves.
     */
    fun addressBanInfo(): String? =
        mapNativeErrors { WalletManagerNative.managerAddressBanInfo(managerHandle) }

    // ── SPV progress (fast-cadence StateFlow) ─────────────────────────

    private val _spvProgress = MutableStateFlow(SpvSyncProgressData.EMPTY)

    /**
     * SPV sync progress, refreshed on a 1 Hz loop while [startSpv] is
     * active — port of Swift `PlatformWalletManager.spvProgress`. The
     * StateFlow REFLECTS the Rust-owned SPV loop's state (via
     * `platform_wallet_manager_sync_progress`); Kotlin makes no sync
     * decisions. The app maps this into its own overlay shape.
     */
    val spvProgress: StateFlow<SpvSyncProgressData> = _spvProgress.asStateFlow()

    private val _spvTipUnixSeconds = MutableStateFlow(0L)

    /** SPV header-tip block time (unix seconds), refreshed with [spvProgress]. */
    val spvTipUnixSecondsFlow: StateFlow<Long> = _spvTipUnixSeconds.asStateFlow()

    private var progressPollJob: Job? = null

    /**
     * Launch the 1 Hz SPV progress poll (idempotent — a running loop is
     * reused). Mirrors Swift's `startProgressPolling`: poll
     * `sync_progress` + `tip_unix_seconds`, publish only on change to avoid
     * StateFlow churn, and stop once SPV is no longer running.
     */
    private fun startSpvProgressPolling() {
        if (progressPollJob?.isActive == true) return
        progressPollJob = scope.launch {
            while (isActive && !isClosed) {
                val running = runCatching { isSpvRunning() }.getOrDefault(false)
                if (running) {
                    runCatching { spvSyncProgress() }.getOrNull()?.let { next ->
                        if (next != _spvProgress.value) _spvProgress.value = next
                    }
                    runCatching { spvTipUnixSeconds() }.getOrNull()?.let { tip ->
                        if (tip != _spvTipUnixSeconds.value) _spvTipUnixSeconds.value = tip
                    }
                } else if (_spvProgress.value != SpvSyncProgressData.EMPTY) {
                    // SPV stopped — reset so the overlay clears.
                    _spvProgress.value = SpvSyncProgressData.EMPTY
                }
                delay(POLL_INTERVAL_MS)
            }
        }
    }

    // ── Lifecycle ─────────────────────────────────────────────────────

    val isClosed: Boolean get() = bundleRef.get() == 0L

    /**
     * Tear down in dependency order:
     * 1. Stop the sync loops (best-effort — the native destroy also runs
     *    `shutdown()`, so failures here are non-fatal).
     * 2. Destroy the native bundle — runs Rust `shutdown()` to quiesce
     *    every callback-firing task, THEN frees the persistence + event
     *    context `GlobalRef`s. Nothing may fire a callback after this.
     * 3. Close the resolver + signer children (their bridge `GlobalRef`s
     *    are released) — safe only after the native manager can no longer
     *    invoke them.
     *
     * Idempotent: the [AtomicLong] guard destroys the bundle exactly once.
     * Closing the wallet wrappers is left to their owners (each is its own
     * `AutoCloseable`); the map is cleared so no stale handle leaks.
     */
    override fun close() {
        val bundle = bundleRef.getAndSet(0)
        if (bundle == 0L) return

        // Stop the progress poll loop + event fan-out scope before teardown
        // so no collector touches a destroyed handle.
        progressPollJob?.cancel()
        scope.cancel()

        // Best-effort stop; ignore failures (destroy shuts everything down).
        runCatching { WalletManagerNative.platformAddressSyncStop(managerHandle) }
        runCatching { WalletManagerNative.identitySyncStop(managerHandle) }
        runCatching { WalletManagerNative.shieldedSyncStop(managerHandle) }
        runCatching { WalletManagerNative.spvStop(managerHandle) }

        // Destroy the native manager (shutdown + free contexts).
        WalletManagerNative.nativeDestroy(bundle)

        // Now safe to release the resolver / signer bridges.
        runCatching { mnemonicResolver.close() }
        runCatching { signer.close() }

        // Drop wallet wrappers we handed out; each still self-destructs via
        // its own Cleaner, but clearing avoids surfacing stale handles.
        _wallets.value.values.forEach { runCatching { it.close() } }
        _wallets.value = emptyMap()
    }

    private companion object {
        /** SPV progress poll cadence — matches Swift's 1 Hz `startProgressPolling`. */
        const val POLL_INTERVAL_MS = 1_000L
    }
}
