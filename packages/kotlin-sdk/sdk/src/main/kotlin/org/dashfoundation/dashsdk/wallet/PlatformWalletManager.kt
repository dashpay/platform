package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
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
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.DashpayNative
import org.dashfoundation.dashsdk.ffi.DpnsMarketplaceNative
import org.dashfoundation.dashsdk.ffi.FundingNative
import org.dashfoundation.dashsdk.ffi.NativeWalletEventBridge
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.PlatformWalletPersistenceHandler
import org.dashfoundation.dashsdk.persistence.entities.DashpayPaymentEntity
import org.dashfoundation.dashsdk.persistence.hexToByteArray
import org.dashfoundation.dashsdk.persistence.toBase58String
import org.dashfoundation.dashsdk.persistence.toHex
import org.json.JSONArray
import org.dashfoundation.dashsdk.security.BiometricGate
import org.dashfoundation.dashsdk.security.IdentityKeyPrivateKeyDeriver
import org.dashfoundation.dashsdk.security.KeystoreSigner
import org.dashfoundation.dashsdk.security.MnemonicResolverAndPersister
import org.dashfoundation.dashsdk.security.WalletStorage
import java.util.concurrent.atomic.AtomicLong

/** Effective native persistence contract exposed for initialization diagnostics. */
data class PlatformWalletPersistenceCapabilities(
    val version: Int,
    val bits: Long,
) {
    fun contains(capability: Long): Boolean = bits and capability == capability

    companion object {
        const val VERSION_1: Int = 1
        const val ATOMIC_CHANGESETS: Long = 1L shl 0
        const val INVITATIONS: Long = 1L shl 1
        const val ASSET_LOCK_FUNDING_INDICES: Long = 1L shl 2
        const val SHIELDED_VIEWING_KEYS: Long = 1L shl 3
        const val PROVIDER_TRANSACTIONS: Long = 1L shl 4
        const val UNSIGNED_TOKEN_STORAGE: Long = 1L shl 5
        const val PENDING_CONTACT_CRYPTO: Long = 1L shl 6
        const val WALLET_RESTORE: Long = 1L shl 7
        const val DPNS_NAME_STATES: Long = 1L shl 8
        const val TRACKED_ASSET_LOCKS: Long = 1L shl 9
        /**
         * A stored core changeset's swept transactions are durably removed:
         * the loser's row (and any tombstoned pending-input claim standing
         * in for a not-yet-materialized UTXO) actually leaves Room. Mirrors
         * `PersistenceCapabilities::CORE_SWEEP_REMOVAL`.
         */
        const val CORE_SWEEP_REMOVAL: Long = 1L shl 10
    }
}

/** Fully validated native state adopted by [PlatformWalletManager]. */
internal data class PlatformWalletNativeInitialization(
    val bundle: Long,
    val managerHandle: Long,
    val persistenceCapabilities: PlatformWalletPersistenceCapabilities,
)

/**
 * Construct the native manager as one transaction.
 *
 * No partially-constructed [PlatformWalletManager] exists for callers to
 * close, so every failure path must release the resources created by property
 * initializers above the native bundle. Cleanup mirrors the successful close
 * order: stop Kotlin work, destroy the callback-owning native bundle, then
 * release its resolver/signer dependencies and the persistence executor.
 * Cleanup failures are suppressed onto the initialization failure so all
 * resources still receive one close attempt.
 */
internal fun initializePlatformWalletNativeManager(
    nativeCreate: () -> Long,
    nativeManagerHandle: (Long) -> Long,
    nativePersistenceCapabilitiesVersion: (Long) -> Int,
    nativePersistenceCapabilitiesBits: (Long) -> Long,
    nativeDestroy: (Long) -> Unit,
    cancelScope: () -> Unit,
    closeMnemonicResolver: () -> Unit,
    closeSigner: () -> Unit,
    closePersistenceHandler: () -> Unit,
): PlatformWalletNativeInitialization {
    var bundle = 0L
    try {
        bundle = nativeCreate()
        check(bundle != 0L) { "nativeCreate returned a zero bundle handle" }

        val managerHandle = nativeManagerHandle(bundle)
        check(managerHandle != 0L) { "nativeManagerHandle returned a zero manager handle" }

        return PlatformWalletNativeInitialization(
            bundle = bundle,
            managerHandle = managerHandle,
            persistenceCapabilities = PlatformWalletPersistenceCapabilities(
                version = nativePersistenceCapabilitiesVersion(bundle),
                bits = nativePersistenceCapabilitiesBits(bundle),
            ),
        )
    } catch (initializationFailure: Throwable) {
        fun cleanup(action: () -> Unit) {
            try {
                action()
            } catch (cleanupFailure: Throwable) {
                if (cleanupFailure !== initializationFailure) {
                    initializationFailure.addSuppressed(cleanupFailure)
                }
            }
        }

        cleanup(cancelScope)
        if (bundle != 0L) {
            // The local owns the bundle until this function returns, so this
            // is the sole destroy attempt on failed initialization.
            cleanup { nativeDestroy(bundle) }
            bundle = 0L
        }
        cleanup(closeMnemonicResolver)
        cleanup(closeSigner)
        cleanup(closePersistenceHandler)
        throw initializationFailure
    }
}

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
 * bundle (which runs the Rust `shutdown()` — a bounded quiesce + join of
 * every callback-firing task; the context `GlobalRef`s themselves are
 * owned and freed by the native manager when its last worker reference
 * drops), then closes the resolver + signer children. Order matters: the
 * native manager must be torn down before the children whose bridges its
 * callbacks reference.
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
    private val biometricGate: BiometricGate? = null,
) : AutoCloseable {

    init {
        require(network == sdk.network) {
            "PlatformWalletManager is network-locked: network=$network but sdk.network=${sdk.network}"
        }
        // One-time honesty log for the lockless-device identity-key policy
        // degradation (dashpay/platform#4060): if the requested AUTH_GATED
        // policy is effectively DEVICE_BOUND (no secure lock screen), say so
        // once, loudly, at manager construction. Best-effort — the probe
        // touches KeyguardManager/AndroidKeyStore, which may be absent in
        // JVM test fixtures.
        runCatching {
            val requested = walletStorage.keySecurityPolicy
            val effective = walletStorage.effectiveKeySecurityPolicy()
            if (effective != requested) {
                android.util.Log.w(
                    "PlatformWalletManager",
                    "identity-key security policy degraded: requested=$requested " +
                        "effective=$effective (no secure lock screen; new keys use the " +
                        "device-bound alias — dashpay/platform#4060)",
                )
            }
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
     * Fence for operations that borrow this manager's raw native handles
     * ([signerHandle] / [mnemonicResolverHandle]) from THEIR OWN coroutine
     * scopes — the manager-scope join in teardown cannot cover them.
     * Threaded into every wallet wrapper and stateless facade this manager
     * builds; [closeSuspending] awaits it before freeing the signer /
     * resolver boxes (raw, non-refcounted `Box::from_raw` frees on the
     * Rust side).
     */
    private val teardownGate = TeardownGate()

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

        override fun onDpnsMarketplaceSyncCompleted(
            walletId: ByteArray,
            success: Boolean,
            namesTracked: Int,
            namesAdded: Int,
            namesDeparted: Int,
            pricesChanged: Int,
            errorMessage: String?,
        ) {
            _syncEvents.tryEmit(
                WalletSyncEvent.DpnsMarketplaceResult(
                    walletId = walletId,
                    success = success,
                    namesTracked = namesTracked,
                    namesAdded = namesAdded,
                    namesDeparted = namesDeparted,
                    pricesChanged = pricesChanged,
                    errorMessage = errorMessage,
                ),
            )
        }

        override fun onDpnsMarketplaceSyncPassCompleted(
            syncUnixSeconds: Long,
            walletCount: Int,
        ) {
            _syncEvents.tryEmit(
                WalletSyncEvent.DpnsMarketplacePassCompleted(syncUnixSeconds, walletCount),
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
     * Holder for the four children below, built together so a fallible
     * constructor partway through the group can't strand the ones built
     * before it — see [CoreChildren] and the `run { }` block that builds it.
     */
    private class CoreChildren(
        val mnemonicResolver: MnemonicResolverAndPersister,
        val signer: KeystoreSigner,
        val identityKeyDeriver: IdentityKeyPrivateKeyDeriver,
        val persistenceHandler: PlatformWalletPersistenceHandler,
    )

    /**
     * Mnemonic resolver + signer, backed by [walletStorage]; persistence
     * handler (Room writer), constructed after the resolver so it can be
     * handed an [IdentityKeyPrivateKeyDeriver] backed by the resolver
     * handle — the identity-key persist callback derives the private half
     * via a single (deadlock-safe, resolver-keyed) Rust FFI call and
     * encrypts it into [walletStorage] (item 1 — the CLAUDE.md "one allowed
     * exception").
     *
     * [mnemonicResolver] and [signer] each own a JNI handle (freed by their
     * own [AutoCloseable.close]); [persistenceHandler] owns a single-thread
     * `Executor` when constructed without an injected dispatcher, as here.
     * Building all four as locals inside one guarded block — rather than as
     * four independent property initializers — means a throw partway
     * through (e.g. [KeystoreSigner]'s native `createSigner` failing) closes
     * whatever was already built instead of leaking it: with independent
     * initializers, a later one throwing aborts the whole constructor with
     * no [PlatformWalletManager] instance left to call `close()` on the
     * earlier ones.
     */
    private val coreChildren: CoreChildren = run {
        var mnemonicResolver: MnemonicResolverAndPersister? = null
        var signer: KeystoreSigner? = null
        try {
            val resolver = MnemonicResolverAndPersister(walletStorage)
                .also { mnemonicResolver = it }
            val keySigner = KeystoreSigner(
                walletStorage, network, biometricGate, database.platformAddressDao(),
                // Durable invalidation bookkeeping (#4060 round-2 finding 3):
                // a sign-time KeyPermanentlyInvalidatedException nulls the
                // Room rows' keychain identifier and re-seeds
                // pendingIdentityKeys, making repair reachable even for
                // legacy-alias keys the cheap capability check can never see
                // as broken. `persistenceHandler` resolves through
                // coreChildren AFTER construction completes — the lambda only
                // runs on later sign attempts, never during this block.
                onSigningKeyInvalidated = { pubkeyHex ->
                    try {
                        persistenceHandler.recordSigningKeyInvalidated(pubkeyHex) {
                            walletStorage.isPrivateKeyDecryptable(it)
                        }
                    } catch (cancellation: kotlin.coroutines.cancellation.CancellationException) {
                        // NEVER swallow structured-concurrency cancellation —
                        // rethrow so a teardown of the signer's IO scope
                        // propagates instead of being masked as a benign
                        // bookkeeping miss.
                        throw cancellation
                    } catch (t: Throwable) {
                        // Do NOT fail open (dashpay/platform#4183 review): the
                        // durable invalidation bookkeeping (null the row's
                        // keychain identifier + re-seed pendingIdentityKeys)
                        // did NOT complete, so the repair signal is not yet
                        // persisted. The sign still fails with the typed code
                        // 31, but swallowing this silently would leave the key
                        // looking healthy on the next launch. Surface it loudly
                        // and rethrow so the signer's own best-effort guard —
                        // not this bookkeeping lambda — is the single place
                        // that decides bookkeeping failure is non-fatal to the
                        // completion; the repair stays retryable (the durable
                        // rows were not cleared, and the next sign attempt / the
                        // next loadPersistedWallets reconstruction re-runs it).
                        android.util.Log.e(
                            "PlatformWalletManager",
                            "durable sign-time invalidation bookkeeping FAILED for key " +
                                "${pubkeyHex.take(16)}… — the pending-repair signal is not yet " +
                                "persisted; it will be retried on the next sign attempt or the " +
                                "next launch's pending-key reconstruction",
                            t,
                        )
                        throw t
                    }
                },
            ).also { signer = it }
            val deriver = IdentityKeyPrivateKeyDeriver(
                network = network,
                mnemonicResolverHandle = resolver.nativeHandle,
                walletStorage = walletStorage,
            )
            val handler = PlatformWalletPersistenceHandler(
                database = database,
                privateKeyDeriver = deriver,
                network = network,
            )
            CoreChildren(resolver, keySigner, deriver, handler)
        } catch (constructionFailure: Throwable) {
            // No `handler`-close branch: PlatformWalletPersistenceHandler is
            // built LAST, so it either finishes constructing (assigned to
            // `handler`, adopted by CoreChildren, this catch never runs for
            // it) or its own constructor throws before returning — nothing
            // is ever captured here to close. This assumes that
            // constructor's own resource acquisition (its owned Executor,
            // when no dispatcher is injected, as here) either fully
            // succeeds or fully fails atomically; if it ever grows a
            // fallible step of its own AFTER allocating that executor,
            // this block would need a matching local + cleanup branch.
            fun cleanup(action: () -> Unit) {
                try {
                    action()
                } catch (cleanupFailure: Throwable) {
                    if (cleanupFailure !== constructionFailure) {
                        constructionFailure.addSuppressed(cleanupFailure)
                    }
                }
            }

            cleanup { signer?.close() }
            cleanup { mnemonicResolver?.close() }
            cleanup { scope.cancel() }
            throw constructionFailure
        }
    }
    private val mnemonicResolver get() = coreChildren.mnemonicResolver
    private val signer get() = coreChildren.signer
    private val identityKeyDeriver get() = coreChildren.identityKeyDeriver
    private val persistenceHandler get() = coreChildren.persistenceHandler

    /**
     * Identity keys whose private half could not be derived/stored during
     * persistence (keyed by public-key hex) — the queryable "keys pending"
     * state of dashpay/platform#4053. Such keys were persisted watch-only
     * and cannot sign; repair via [repairIdentityKey]. Empty in the healthy
     * case.
     */
    val pendingIdentityKeys:
        kotlinx.coroutines.flow.StateFlow<Map<String, PlatformWalletPersistenceHandler.PendingIdentityKey>>
        get() = persistenceHandler.pendingIdentityKeys

    /** `MnemonicResolverHandle` for FFI calls that derive from a stored mnemonic. */
    val mnemonicResolverHandle: Long get() = mnemonicResolver.nativeHandle

    /** `SignerHandle` for FFI calls that need signatures. */
    val signerHandle: Long get() = signer.nativeHandle

    /**
     * Re-derive the canonical identity-authentication private key for
     * [publicKeyData] from this wallet's mnemonic and re-encrypt it into
     * [walletStorage] under its hex — the repair action behind
     * `WalletKeyHealthSheet` (port of the iOS re-derive path in
     * `WalletKeyHealthSheet.swift`, which calls `deriveIdentityAuthKeyAtSlot`).
     *
     * The derivation slot is read from the PERSISTED `public_keys` row's
     * derivation breadcrumbs — NEVER from a caller-supplied key id
     * (dashpay/platform#4060 blocker 1): a wrong index derives a DIFFERENT
     * valid scalar that round-trips through encrypt/decrypt fine and would
     * persist an unusable key. The whole `mnemonic → seed → path → key`
     * derivation runs in Rust via the resolver-keyed FFI
     * ([IdentityKeyPrivateKeyDeriver], the CLAUDE.md "one allowed exception");
     * Kotlin only encrypts the returned scalar. Returns the recorded storage
     * identifier (e.g. `privkey.<pubkeyHex>`), or throws on a
     * derivation / verification failure.
     *
     * FORCE-replaces the stored entry (never trusts the shape+fingerprint
     * usability short-circuit), but first VERIFIES the derived PUBLIC key
     * equals [publicKeyData] BEFORE persisting — a mismatched slot fails the
     * repair without storing anything or clearing pending. After the store it
     * VERIFIES the blob with the real-decrypt probe
     * ([WalletStorage.probeIdentityKeyRecoverability]); a blob that does not
     * actually decrypt fails the repair with a typed
     * [DashSdkError.PlatformWallet.SigningKeyUnavailable].
     *
     * Only after both verifications is the key dropped from
     * [pendingIdentityKeys] and the Room rows' `privateKeyKeychainIdentifier`
     * updated so the durable pending-repair reconstruction does not resurrect
     * the key. The full orchestration lives in
     * [PlatformWalletPersistenceHandler.repairIdentityKeyDurably] (this
     * manager cannot be constructed on the JVM; the handler is unit-testable).
     */
    suspend fun repairIdentityKey(
        walletId: ByteArray,
        publicKeyData: ByteArray,
    ): String? = teardownGate.op {
        // The whole repair — read the PERSISTED derivation breadcrumbs,
        // force-re-derive, verify the derived public key matches
        // [publicKeyData] before persisting, verify the stored blob decrypts,
        // durably record the identifier, and only then clear pending — lives
        // in the persistence handler ([repairIdentityKeyDurably]) so it is
        // unit-testable (this manager cannot be constructed on the JVM) and
        // shares the handler's authoritative pending-key state. The manager
        // only supplies the wallet-scoped collaborators: the resolver-keyed
        // deriver (already wired into the handler) and the real-decrypt probe.
        //
        // deriveAndStore is a synchronous JNI call keyed on the manager's
        // resolver handle — the teardownGate keeps teardown from freeing it
        // mid-derive (callers run on their own Compose scopes).
        //
        // NB (dashpay/platform#4060 blocker 1): the derivation indices are NOT
        // caller-supplied — a caller passing the DPP key id would derive a
        // different valid scalar that round-trips fine and persists an
        // unusable key. They come from the row's derivation breadcrumbs, and
        // the derived public key is checked against [publicKeyData] before any
        // store.
        persistenceHandler.repairIdentityKeyDurably(
            walletId = walletId,
            publicKeyData = publicKeyData,
            verifyRecoverable = { pubkeyHex ->
                // UserNotAuthenticatedException counts as verified inside the
                // probe (key present, opens after auth — this manager holds no
                // BiometricGate on this path, and the just-written fingerprint
                // rules out the wrong-key-behind-locked-gate ambiguity because
                // the blob was written under the captured public key).
                walletStorage.probeIdentityKeyRecoverability(pubkeyHex)
            },
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
    ): Pair<ByteArray, ByteArray> = teardownGate.op {
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
        org.dashfoundation.dashsdk.identity.IdentityRegistration(teardownGate)

    /**
     * Identity credit-movement bridge — transfer / withdraw / top-up.
     * Stateless wrapper over the credits JNI surface; callers thread the
     * wallet handle + [signerHandle] into each call. ← the credit-movement
     * slice of `ManagedPlatformWallet.swift`.
     */
    val identityCredits: org.dashfoundation.dashsdk.credits.IdentityCredits =
        org.dashfoundation.dashsdk.credits.IdentityCredits(teardownGate)

    /**
     * Rust-authoritative tracked locks eligible for generic identity
     * registration/top-up recovery. The JNI call copies and frees the native
     * list as one snapshot; Kotlin filters to funding types 0/1/2 and statuses
     * 0…3. Invitations, address/shielded locks, consumed rows, and malformed
     * rows are never offered by this generic surface.
     */
    suspend fun trackedIdentityRecoveryAssetLocks(
        walletId: ByteArray,
    ): List<TrackedAssetLock> = teardownGate.op {
        require(walletId.size == 32) { "walletId must be exactly 32 bytes" }
        val native = mapNativeErrors {
            WalletManagerNative.trackedAssetLocks(managerHandle, walletId)
        }
        TrackedAssetLock.eligibleFromNative(native)
    }

    /**
     * Identity add/disable-keys bridge — the identity-update slice of
     * `ManagedPlatformWallet.swift` (`updateIdentity(addPublicKeys:...)`,
     * driven by Swift `AddIdentityKeyView`). Stateless; callers thread the
     * wallet handle + [signerHandle] into each call.
     */
    val identityUpdates: org.dashfoundation.dashsdk.identity.IdentityUpdates =
        org.dashfoundation.dashsdk.identity.IdentityUpdates(teardownGate)

    /**
     * Document purchase + set-price bridge — the document state-transition
     * slice of `ManagedPlatformWallet.swift` (driven by Swift
     * `DocumentWithPriceView`). Stateless; callers thread the wallet handle +
     * [signerHandle] into each call.
     */
    val documentTransactions: org.dashfoundation.dashsdk.documents.DocumentTransactions =
        org.dashfoundation.dashsdk.documents.DocumentTransactions(teardownGate)

    /** DPNS marketplace queries, trades, history and per-wallet sync. */
    val dpnsMarketplace: org.dashfoundation.dashsdk.dpns.DpnsMarketplace =
        org.dashfoundation.dashsdk.dpns.DpnsMarketplace(teardownGate)

    /**
     * Masternode contested-resource vote bridge — port of
     * `SDK.castContestedResourceVote` (driven by Swift `ContestDetailView`).
     * Stateless; callers thread the SDK handle into each call.
     */
    val voteCasting: org.dashfoundation.dashsdk.voting.VoteCasting =
        org.dashfoundation.dashsdk.voting.VoteCasting()

    // ── Native manager bundle ─────────────────────────────────────────

    private val nativeInitialization = initializePlatformWalletNativeManager(
        nativeCreate = {
            WalletManagerNative.nativeCreate(sdk.handle, persistenceHandler, eventBridge)
        },
        nativeManagerHandle = WalletManagerNative::nativeManagerHandle,
        nativePersistenceCapabilitiesVersion =
            WalletManagerNative::nativePersistenceCapabilitiesVersion,
        nativePersistenceCapabilitiesBits = WalletManagerNative::nativePersistenceCapabilitiesBits,
        nativeDestroy = WalletManagerNative::nativeDestroy,
        cancelScope = scope::cancel,
        closeMnemonicResolver = mnemonicResolver::close,
        closeSigner = signer::close,
        closePersistenceHandler = persistenceHandler::close,
    )

    private val bundleRef: AtomicLong = AtomicLong(nativeInitialization.bundle)

    /** Raw native manager `Handle` (for the sync / wallet-accessor calls). */
    private val managerHandle: Long = nativeInitialization.managerHandle

    /** Effective persistence contract captured during native initialization. */
    val persistenceCapabilities: PlatformWalletPersistenceCapabilities =
        nativeInitialization.persistenceCapabilities

    // ── Published wallet map ──────────────────────────────────────────

    private val _wallets = MutableStateFlow<Map<String, ManagedPlatformWallet>>(emptyMap())

    /**
     * All wallets currently held by the Rust manager, keyed by walletId
     * hex. Mirror of the Swift `wallets` map — the Rust manager holds N
     * wallets concurrently; look up a specific wallet by its hex id.
     */
    val wallets: StateFlow<Map<String, ManagedPlatformWallet>> = _wallets.asStateFlow()

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
     * @param birthHeight the SPV compact-filter scan-start height.
     *   `null` (fresh wallet) resolves the birth height from SPV's
     *   confirmed header tip, so nothing funded before init is scanned.
     *   `0u` requests a full historical scan from genesis — pass it when
     *   **importing/restoring** an existing mnemonic so Core funds and
     *   payments received before this device registered the wallet are
     *   seen (`Some(h)` pins a specific height). Mirror of Swift
     *   `PlatformWalletManager.createWallet(..., birthHeight:)`
     *   (`birthHeight: showImportOption ? 0 : nil`).
     */
    suspend fun createWallet(
        mnemonic: String,
        name: String? = null,
        createDefaultAccounts: Boolean = true,
        birthHeight: UInt? = null,
    ): ManagedPlatformWallet = withContext(Dispatchers.IO) {
        // Caller-allocated out-buffers: the JNI side validates both BEFORE
        // the native create so no fallible allocation follows the
        // persistence commit (a post-commit publish failure would strand
        // Room rows Kotlin has no id to clean up).
        val outHandle = LongArray(1)
        val walletId = ByteArray(32)
        mapNativeErrors {
            WalletManagerNative.createWalletFromMnemonicWithBirthHeight(
                managerHandle,
                mnemonic,
                network.ffiValue,
                createDefaultAccounts,
                birthHeight != null,
                (birthHeight ?: 0u).toInt(),
                outHandle,
                walletId,
            )
        }
        // Adopt the native handle into its AutoCloseable owner BEFORE the
        // fallible Keystore/Room steps — a raw jlong has no owner, so a
        // throw below would otherwise leak the Rust registry entry.
        val managed = ManagedPlatformWallet(
            handle = outHandle[0],
            walletId = walletId,
            gate = teardownGate,
        )
        var mnemonicStored = false
        try {
            // Store the mnemonic keyed by the id the FFI just derived.
            walletStorage.storeMnemonic(walletId, mnemonic)
            mnemonicStored = true

            // Persist the display name onto the Room row the persistence
            // callbacks just wrote (a persist step, not orchestration —
            // ← CreateWalletView.swift stamping the label per created wallet).
            name?.trim()?.takeIf { it.isNotEmpty() }?.let { label ->
                database.walletDao().updateName(walletId, label, System.currentTimeMillis())
            }

            // Un-tombstone LAST, once every fallible step above has
            // succeeded: wallet ids are deterministic (seed + network), so
            // a delete-then-reimport of the same phrase reuses this id and
            // needs any stale rejection from a prior removeWallet cleared —
            // but clearing it any earlier would open a window (this
            // function's own remaining fallible steps) where a stale
            // in-flight identity-key store from that PRIOR wallet instance
            // could slip through and resurrect owner-index state for a
            // wallet this call might still go on to fail.
            walletStorage.clearTombstone(walletId)
        } catch (t: Throwable) {
            // Default to (re-)arming the tombstone: most paths below fully
            // roll this wallet back (native unregister + Room cascade) or
            // leave it seedless/unusable, so a stale in-flight identity-key
            // store (from a PRIOR instance of this deterministic walletId)
            // resuming after this failure must not be allowed to write.
            // The ONE exception — the wallet actually SURVIVES as a valid,
            // loadable entity (Room rollback itself failed, so the rows
            // the original native create already wrote are still intact,
            // and the mnemonic is durable) — is un-armed again below once
            // that disposition is known. Best-effort: must not shadow the
            // real failure.
            runCatching { walletStorage.withPrivateKeyExclusion { tombstoneWallet(walletId) } }

            // Full rollback, not just the wrapper's Arc clone: the wallet is
            // already REGISTERED in the native manager and its persistence
            // callbacks may have written Room rows. The native remove only
            // unregisters — it fires no Room-cascading persistence callback
            // (same contract as removeWallet step 3) — so scrub the Room
            // footprint explicitly too, or the next loadPersistedWallets
            // resurrects the wallet as an orphan with no Keystore mnemonic.
            val nativeRollback = runCatching {
                mapNativeErrors { WalletManagerNative.removeWallet(managerHandle, walletId) }
            }
            val roomRollback = runCatching { persistenceHandler.deleteWalletData(walletId) }
            if (roomRollback.isFailure) {
                // The persisted rows could not be removed (likely the same
                // storage fault that failed the creation step). Retention
                // only helps if the phrase actually reached storage —
                // storeMnemonic itself may be what threw, and the caller
                // (CreateWalletScreen) holds the phrase only in a local
                // that dies with this failure. So make sure a copy
                // survives: retry the store now, and report the true
                // disposition either way instead of silently abandoning
                // the rows behind the creation error alone.
                if (!mnemonicStored) {
                    mnemonicStored =
                        runCatching { walletStorage.storeMnemonic(walletId, mnemonic) }.isSuccess
                }
                if (!mnemonicStored) {
                    // Neither copy of the phrase is durable. Seedless rows
                    // must not survive if they can possibly be removed —
                    // retry the cascade once before reporting.
                    if (runCatching { persistenceHandler.deleteWalletData(walletId) }.isSuccess) {
                        managed.close()
                        throw t
                    }
                }
                managed.close()
                val disposition = if (mnemonicStored) {
                    // The rows the ORIGINAL native create wrote are still
                    // intact (only this rollback's delete failed, not the
                    // create itself) and the mnemonic is durable — this
                    // wallet is a fully valid, loadable entity per
                    // loadPersistedWallets' contract (it restores whatever
                    // Room says exists, with no tombstone awareness of its
                    // own). Un-arm the tombstone the catch block's default
                    // just set, or every identity-key store against this
                    // wallet fails until process restart or a re-import.
                    runCatching { walletStorage.clearTombstone(walletId) }
                    "the mnemonic is stored, so the wallet stays recoverable — " +
                        "retry cleanup via removeWallet"
                } else {
                    "the mnemonic could NOT be stored either — this exception " +
                        "carries the phrase as the last remaining copy; back it " +
                        "up now, then re-import it or remove the wallet via " +
                        "removeWallet"
                }
                // Typed, and — in the both-writes-and-both-deletes-failed
                // corner — carrying the phrase itself: the caller's local is
                // the only other copy and it dies with this failure, which
                // would leave the surviving rows permanently seedless.
                throw WalletCreateRollbackException(
                    walletId = walletId,
                    mnemonicStored = mnemonicStored,
                    mnemonic = if (mnemonicStored) null else mnemonic,
                    message = "createWallet failed and its rollback could not delete the " +
                        "persisted rows for wallet ${walletId.toHex()}; $disposition",
                    cause = t,
                ).apply {
                    roomRollback.exceptionOrNull()?.let(::addSuppressed)
                    nativeRollback.exceptionOrNull()?.let(::addSuppressed)
                }
            }
            // The Room rows are gone. INTENTIONALLY keep the stored
            // mnemonic: deleting it here could destroy the only durable
            // copy of the phrase — the UI holds it in a local the failure
            // path discards, and on a re-import/orphan-recovery create
            // the entry predates this call entirely, so scrubbing it
            // would erase a healthy wallet's seed. A wallet-less entry is
            // exactly what the orphan-mnemonic recovery flow surfaces on
            // next launch (recover / keep / delete — the user decides).
            managed.close()
            throw t
        }
        _wallets.update { it + (walletId.toHex() to managed) }
        managed
    }

    // ── Wallet deletion ───────────────────────────────────────────────

    /**
     * Fully wipe a wallet's Rust, Room, and Keystore footprint — port of
     * Swift `PlatformWalletManager.deleteWallet(walletId:)` (drives the
     * WalletDetailScreen "Delete Wallet" action / test CORE-17).
     *
     * Ordering is chosen for retry-safety and for serialization against
     * every key-producing path:
     *  1. `platform_wallet_manager_remove_wallet` unregisters the wallet
     *     in the Rust manager FIRST (same FFI the create-rollback path
     *     calls), quiescing the wallet's own operation sources. It must
     *     run BEFORE the exclusion section below and never inside it: a
     *     persistence callback parked at the handler's callback gate can
     *     be holding the native wallet-manager write lock, so taking the
     *     gate and then calling into the manager would deadlock (ABBA).
     *  2. Close + drop the [ManagedPlatformWallet] from [_wallets], and
     *     clear any DashPay unlock-status banner so a re-created wallet with
     *     the same deterministic id can't inherit a stale banner.
     *  3. Snapshot + secret deletion + Room cascade as ONE exclusion
     *     section, serialized against every key producer: the handler's
     *     callback gate excludes persistence callbacks (so no
     *     `onPersistIdentityKeyUpsert` can write a fresh `privkey.*`
     *     alias after the snapshot, and no changeset commit can
     *     resurrect rows after the cascade), and the [WalletStorage]
     *     private-key lock excludes app-side `storePrivateKey` writers —
     *     both taken in the same order the callback path takes them
     *     (gate at entry → key lock inside storePrivateKey). Within the
     *     section: enumerate the identity keys while the `public_keys`
     *     rows still exist, refcount shared aliases (keys another
     *     wallet's identity still references are retained — secrets are
     *     keyed globally by pubkey hex), delete the whole set in ONE
     *     DataStore edit (atomic: a failure deletes nothing, everything
     *     is retryable — every step is idempotent), then run the Room
     *     cascade, which also discards any open changeset round for this
     *     wallet.
     *  4. Delete the Keystore mnemonic last, so a mid-flight retry could
     *     still re-derive any missed key before the phrase is gone. A
     *     failure here PROPAGATES (post-cascade state is still recoverable:
     *     the orphan-mnemonic flow surfaces the leftover phrase on next
     *     launch, and re-running this method is a no-op up to this step) —
     *     silently reporting success would leave the seed behind without
     *     any signal to the caller.
     *
     * The derive/store lifecycle is fenced end to end: the sweep's
     * enumeration unions [PlatformWalletPersistenceHandler.pendingAliasesFor]
     * (aliases whose rows are still buffered in an open round), a late
     * `onPersistIdentityKeyUpsert` from an operation that survived the
     * unregistration skips its derive+store when the wallet row is gone
     * (checked under the same callback gate this section holds), and a
     * rolled-back round deletes the aliases it wrote — so a successful
     * wipe leaves no identity-key ciphertext behind.
     *
     * Deleting an already-removed wallet succeeds (each step is a no-op).
     *
     * @param walletId the 32-byte network-scoped wallet id.
     */
    suspend fun removeWallet(walletId: ByteArray) = withContext(Dispatchers.IO) {
        require(walletId.size == 32) {
            "walletId must be 32 bytes, got ${walletId.size}"
        }

        // 1. Unregister in the Rust manager. Native calls are forbidden
        //    inside the exclusion section below — see the KDoc.
        mapNativeErrors { WalletManagerNative.removeWallet(managerHandle, walletId) }

        // 2. Drop the wrapper + banner state.
        val key = walletId.toHex()
        _wallets.value[key]?.let { runCatching { it.close() } }
        _wallets.update { it - key }
        _dashPayUnlockStatus.update { it - key }

        // 3. Snapshot → atomic secret delete → Room cascade, with both
        //    persistence callbacks and app-side key writers excluded.
        persistenceHandler.withCallbackExclusion {
            walletStorage.withPrivateKeyExclusion {
                val ownedIdentityIds = persistenceHandler.identityIdsForWallet(walletId)
                    .map { it.toBase58String() }
                val keysByPubkeyHex = LinkedHashMap<String, ByteArray>()
                for (identityId in ownedIdentityIds) {
                    for (dbKey in database.publicKeyDao().getByIdentityId(identityId)) {
                        keysByPubkeyHex.putIfAbsent(
                            dbKey.publicKeyData.toHex(),
                            dbKey.publicKeyData,
                        )
                    }
                }
                // Union the aliases the deriver wrote during a still-open
                // changeset round: their rows are only buffered (invisible
                // to the Room enumeration above) and the cascade below
                // discards that buffer, so without this they would survive
                // as stranded ciphertext.
                for (pendingHex in persistenceHandler.pendingAliasesFor(walletId)) {
                    keysByPubkeyHex.putIfAbsent(pendingHex, pendingHex.hexToByteArray())
                }
                // Union the DURABLE owner index: app-prestored keys whose
                // registration hasn't broadcast (no public_keys row exists
                // anywhere yet) and deriver writes orphaned by process
                // death (the in-memory fence above doesn't survive it) are
                // discoverable only here.
                for (ownedHex in walletStorage.ownedPrivateKeyAliases(walletId)) {
                    keysByPubkeyHex.putIfAbsent(ownedHex, ownedHex.hexToByteArray())
                }
                val aliasesToDelete = buildList {
                    for ((pubkeyHex, publicKeyData) in keysByPubkeyHex) {
                        val referencedElsewhere = database.publicKeyDao()
                            .countReferencesOutsideIdentities(publicKeyData, ownedIdentityIds) > 0
                        // A sibling wallet (same mnemonic, non-mainnet DIP-9
                        // path shared across Testnet/Devnet/Regtest) can
                        // already own this alias through ITS durable owner
                        // index while its own registration/add-key row is
                        // still uncommitted — a committed-`public_keys`-row
                        // check alone would miss that and delete the
                        // sibling's key out from under it.
                        val ownedElsewhere = isOwnedByAnotherWallet(pubkeyHex, walletId)
                        if (!referencedElsewhere && !ownedElsewhere) add(pubkeyHex)
                    }
                }
                // ONE atomic DataStore commit; propagates on failure with
                // nothing deleted and the cascade below not run. The same
                // commit drops the deleted hexes from every owner index;
                // this wallet's index entry is then removed outright (any
                // alias it retained as shared stays discoverable through
                // the surviving wallet's own index / Room rows).
                deletePrivateKeys(aliasesToDelete)
                deleteOwnerIndex(walletId)
                // Reject any store that was already in flight (e.g. an
                // app-level identity-key preview/derive started before this
                // deletion) and would otherwise complete AFTER this atomic
                // commit, resurrecting this wallet's owner-index entry with
                // fresh ciphertext. Cleared on the next createWallet for the
                // same (deterministic, seed-derived) id — see there.
                tombstoneWallet(walletId)

                // Room cascade (explicit, matching Swift — the native
                // remove fires no wallet-level persistence callback).
                persistenceHandler.deleteWalletDataLocked(walletId)
            }
        }

        // 4. Keystore mnemonic last (retry can still re-derive until now).
        //    Propagates on failure — the orphan-mnemonic recovery flow can
        //    still surface the leftover phrase, but the caller must not be
        //    told the wallet was fully wiped when the seed is still stored.
        walletStorage.deleteMnemonic(walletId)
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
     */
    suspend fun loadPersistedWallets(): List<ManagedPlatformWallet> = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.loadFromPersistor(managerHandle) }

        // Rebuild the durable "keys pending repair" state before the manager
        // is handed to the host (dashpay/platform#4060 finding 5): rows with
        // derivation breadcrumbs whose private half is missing or fails the
        // CHEAP capability check re-seed pendingIdentityKeys, so a repair
        // signal recorded before a process death (or a blob stranded by a
        // Keystore keypair replacement) resurfaces on every launch.
        try {
            persistenceHandler.reconstructPendingIdentityKeysFromPersistence(
                isPrivateKeyDecryptable = { walletStorage.isPrivateKeyDecryptable(it) },
            )
        } catch (cancellation: kotlin.coroutines.cancellation.CancellationException) {
            // NEVER swallow structured-concurrency cancellation from the
            // suspend reconstruction — rethrow so a cancelled load propagates
            // (dashpay/platform#4183 review). A best-effort reconstruction
            // failure is fine to absorb (the repair signal reconstructs on the
            // next launch), but cancellation must not be masked.
            throw cancellation
        } catch (t: Throwable) {
            android.util.Log.w(
                "PlatformWalletManager",
                "pending-identity-key reconstruction failed on load; repair signals will be " +
                    "rebuilt on the next launch",
                t,
            )
        }

        // Room is the source of truth for the restorable id list — the same
        // rows the load callback just fed to Rust. Scope to THIS network so
        // a per-network manager only restores its own wallets (matching the
        // network lock and the Swift persistence handler's network scoping).
        val ids = database.walletDao().getByNetwork(network.ffiValue)
            .map { it.walletId }
            .filter { it.size == 32 }

        val restored = ArrayList<ManagedPlatformWallet>(ids.size)
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
                gate = teardownGate,
            )
            restored.add(managed)
            // Publish into [wallets] BEFORE unlocking (Swift's per-wallet
            // ordering): the unlock writes onto [dashPayUnlockStatus], and
            // the 1 Hz poll prunes status keys absent from [wallets] — a
            // publish-after-unlock ordering would let the prune silently
            // drop a just-published seedMismatch.
            _wallets.update { it + (walletId.toHex() to managed) }

            // Seedless unlock of the just-restored external-signable wallet:
            // verify the Keystore-resolved seed binds to this wallet and
            // drain any deferred contact-crypto. Best-effort, per wallet —
            // this call is LOAD-BEARING for DashPay recovery: the deferred
            // contact-crypto queue is in-memory only (rebuilt by the sweep,
            // cleared by the drain), so recovery is self-healing ONLY when
            // every launch runs load → unlock → sweep. A banner-triggered
            // unlock alone would leave contacts half-established after each
            // process restart. Mirrors Swift `loadFromPersistor`.
            try {
                unlockWalletFromKeystore(managed)
            } catch (_: Exception) {
                // Outcome is published on [dashPayUnlockStatus] (seedMismatch
                // for a binding rejection); one wallet's unlock failure can't
                // fail the whole restore — the wallet simply stays
                // external-signable.
            }
        }
        // Banner state (pendingAccountBuilds) must refresh whenever wallets
        // exist, independent of whether the sweep service was started.
        if (restored.isNotEmpty()) startDashPayStatusPolling()
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
     * Whether the native manager has frozen its durable sync watermark this
     * session (dashpay/platform#4069). `true` means the wallet-event adapter
     * had a persistence `store()` rejected — the one remaining fault trigger;
     * the lossless persistence channel cannot drop or lag events —
     * so the persisted `syncedHeight` is deliberately held behind the chain
     * tip and a rescan is pending on the next launch. Poll this to surface a
     * hard "verification failed / rescan pending" state instead of leaving
     * the fault visible only in the error logs.
     *
     * The flag latches: once `true` it stays `true` for this manager
     * instance's lifetime (a destroyed-and-recreated manager — e.g. a network
     * switch through WalletManagerStore — starts unlatched).
     */
    suspend fun syncFaultDetected(): Boolean = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.syncFaultDetected(managerHandle) }
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

    /**
     * Per-account balance snapshot for [walletId] as a JSON array string
     * (see `DashpayNative.walletManagerAccountBalances` for the row
     * shape) — drives the DashPay tab's account balance display. Port of
     * Swift `PlatformWalletManager.accountBalances(for:)`.
     */
    suspend fun accountBalances(walletId: ByteArray): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { DashpayNative.walletManagerAccountBalances(managerHandle, walletId) }
    }

    /**
     * Refresh the persisted DashPay payment history for one identity:
     * one FFI read (`managed_identity_get_dashpay_payments`) + one Room
     * pass upserting [DashpayPaymentEntity] rows so the UI can observe
     * them reactively. This is the ONLY path by which payment rows
     * become durable — the recurring DashPay sweep reconciles payments
     * in-memory without persisting them (matching iOS), so callers must
     * refresh after a send and when opening a contact's payment history.
     * Upsert-only: payment history is append-only, keyed by txid.
     *
     * Returns the raw payments JSON that was persisted, or null when
     * [identityId] isn't managed by the wallet. Port of Swift
     * `PlatformWalletManager.refreshDashPayPayments(walletId:identityId:)`.
     */
    suspend fun refreshDashPayPayments(walletId: ByteArray, identityId: ByteArray): String? =
        withContext(Dispatchers.IO) {
            val managed = requireNotNull(wallet(forWalletId = walletId)) {
                "no loaded wallet with id ${walletId.toHex()}"
            }
            val json = managed.dashpay.payments(identityId) ?: return@withContext null
            // networkRaw rides on the owner identity row (the persist-path
            // convention); when the identity row hasn't landed yet the read
            // still succeeds — the rows just aren't persisted this round.
            val networkRaw = database.identityDao().getByIdentityId(identityId)?.networkRaw
                ?: return@withContext json
            // Skip-unchanged + preserve createdAt, mirroring Swift's
            // persistDashpayPayments: a Room @Upsert rewrites every column,
            // so an unconditional upsert would clobber createdAt with "now"
            // on every refresh and re-fire the payments Flow (re-rendering
            // an open payment list) even when nothing moved.
            val existingByTxid = database.dashpayDao()
                .getPaymentsByOwner(identityId)
                .associateBy { it.txid }
            val rows = JSONArray(json)
            val entities = ArrayList<DashpayPaymentEntity>(rows.length())
            for (i in 0 until rows.length()) {
                val row = rows.getJSONObject(i)
                val txid = row.optString("txid", "")
                val counterparty = row.optString("counterpartyId", "").hexToBytesOrNull()
                if (txid.isEmpty() || counterparty == null || counterparty.size != 32) continue
                val existing = existingByTxid[txid]
                val entity = DashpayPaymentEntity(
                    networkRaw = networkRaw,
                    ownerIdentityId = identityId,
                    counterpartyIdentityId = counterparty,
                    amountDuffs = row.optLong("amountDuffs"),
                    directionRaw = row.optInt("direction"),
                    statusRaw = row.optInt("status"),
                    txid = txid,
                    memo = if (row.has("memo")) row.getString("memo") else null,
                    createdAt = existing?.createdAt ?: java.util.Date(),
                )
                val unchanged = existing != null &&
                    existing.counterpartyIdentityId.contentEquals(entity.counterpartyIdentityId) &&
                    existing.amountDuffs == entity.amountDuffs &&
                    existing.directionRaw == entity.directionRaw &&
                    existing.statusRaw == entity.statusRaw &&
                    existing.memo == entity.memo
                if (!unchanged) entities.add(entity)
            }
            if (entities.isNotEmpty()) database.dashpayDao().upsertPayments(entities)
            json
        }

    /** Lower-hex decode; null on odd length, empty, or non-hex characters. */
    private fun String.hexToBytesOrNull(): ByteArray? {
        if (length % 2 != 0 || isEmpty()) return null
        val out = ByteArray(length / 2)
        for (i in out.indices) {
            val hi = Character.digit(this[2 * i], 16)
            val lo = Character.digit(this[2 * i + 1], 16)
            if (hi < 0 || lo < 0) return null
            out[i] = ((hi shl 4) or lo).toByte()
        }
        return out
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
    ) = teardownGate.op {
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
    ): Unit = teardownGate.op {
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
     * Shield from Platform balance (Type 15) — port of Swift's
     * `shieldedShield`. Spends [amount] credits from the wallet's
     * [paymentAccount] Platform-Payment addresses (auto-selected in
     * ascending derivation order) into its own bound shielded pool
     * ([shieldedAccount]). Signed by the Keystore address signer
     * ([signerHandle]); self-shield only (Rust always targets this wallet's
     * own default Orchard address, so there is no recipient parameter).
     * Blocks for the ~30s Halo 2 proof; the note arrives on the next
     * shielded sync pass.
     *
     * @param walletId the 32-byte wallet id.
     * @param amount credits to shield (1 DASH = 1e11).
     */
    suspend fun shieldedShield(
        walletId: ByteArray,
        amount: Long,
        shieldedAccount: Int = 0,
        paymentAccount: Int = 0,
    ): Unit = teardownGate.op {
        require(amount > 0) { "amount must be positive, got $amount" }
        require(shieldedAccount >= 0) {
            "shieldedAccount must be non-negative, got $shieldedAccount"
        }
        require(paymentAccount >= 0) {
            "paymentAccount must be non-negative, got $paymentAccount"
        }
        mapNativeErrors {
            FundingNative.shieldedShield(
                managerHandle,
                walletId,
                shieldedAccount,
                paymentAccount,
                amount,
                signerHandle,
            )
        }
    }

    /**
     * Create an identity funded from the shielded pool (Type 20) — port of
     * Swift's `shieldedIdentityCreateFromPool`. Spends a note of the fixed
     * exit [denomination] (credits — a member of the ACTIVE protocol
     * version's on-chain exit-denomination set: 0.1/0.3/0.5/1.0 DASH through
     * PV12, 0.03/0.1/0.25/0.5/1.0 DASH from PV13; a non-member is rejected
     * at validation) from the wallet's bound Orchard pool
     * ([account]) to fund
     * a new identity at [identityIndex]. [keys] are the rich registration rows
     * (built via `RegistrationKeys.buildRegistrationRows`), encoded to the same
     * blob every registration path uses; each row's private half must already
     * be persisted. [fallbackAddress] is the REQUIRED 21-byte PlatformAddress
     * that receives the value (minus a penalty) if creation fails a stateful
     * check. Signed by the Keystore identity signer ([signerHandle]). Blocks
     * for the ~30s Halo 2 proof.
     *
     * @return the new 32-byte identity id.
     */
    suspend fun shieldedIdentityCreateFromPool(
        walletId: ByteArray,
        identityIndex: Int,
        keys: List<org.dashfoundation.dashsdk.identity.IdentityPubkey>,
        denomination: Long,
        fallbackAddress: ByteArray,
        account: Int = 0,
    ): ByteArray = teardownGate.op {
        require(identityIndex >= 0) { "identityIndex must be non-negative, got $identityIndex" }
        require(denomination > 0) { "denomination must be positive, got $denomination" }
        require(fallbackAddress.size == 21) {
            "fallbackAddress must be 21 bytes, got ${fallbackAddress.size}"
        }
        require(keys.isNotEmpty()) { "keys must not be empty" }
        val packed = mapNativeErrors {
            FundingNative.shieldedIdentityCreateFromPool(
                managerHandle,
                walletId,
                mnemonicResolver.nativeHandle,
                account,
                identityIndex,
                org.dashfoundation.dashsdk.identity.IdentityPubkeyCodec.encode(keys),
                denomination,
                fallbackAddress,
                signerHandle,
            )
        }
        decodeShieldedCreatePayload(packed)
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
    ): Unit = teardownGate.op {
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
    ): Unit = teardownGate.op {
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
    ): Unit = teardownGate.op {
        require(amount > 0) { "amount must be positive, got $amount" }
        require(account >= 0) { "account must be non-negative, got $account" }
        require(recipientRaw43.size == 43) {
            "recipientRaw43 must be exactly 43 bytes, got ${recipientRaw43.size}"
        }
        mapNativeErrors {
            FundingNative.shieldedTransfer(
                managerHandle,
                walletId,
                mnemonicResolver.nativeHandle,
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
    ): Unit = teardownGate.op {
        require(amount > 0) { "amount must be positive, got $amount" }
        require(account >= 0) { "account must be non-negative, got $account" }
        require(toPlatformAddress.isNotBlank()) { "toPlatformAddress is empty" }
        mapNativeErrors {
            FundingNative.shieldedUnshield(
                managerHandle,
                walletId,
                mnemonicResolver.nativeHandle,
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
    ): Unit = teardownGate.op {
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
                mnemonicResolver.nativeHandle,
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

    /**
     * Arm a compact-filter rescan by rewinding [walletId]'s in-memory SPV
     * checkpoint to [fromHeight]. This call does not scan and is not a
     * cancellable/durable rescan job: a running filter loop observes it on
     * its next tick; a stopped loop observes it on next start. Equal/forward
     * heights are harmless no-ops for scan purposes. If the process dies
     * before the loop consumes and persists progress, the user must reissue
     * the request. Unknown wallets surface as typed
     * [DashSdkError.PlatformWallet.NotFound] (native code 98).
     */
    suspend fun rescanSpvFilters(walletId: ByteArray, fromHeight: Int) =
        withContext(Dispatchers.IO) {
            require(walletId.size == 32) { "walletId must be exactly 32 bytes" }
            require(fromHeight >= 0) { "fromHeight must be non-negative" }
            mapNativeErrors {
                WalletManagerNative.spvRescanFilters(managerHandle, walletId, fromHeight)
            }
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

    // ── DashPay sync + seedless unlock ────────────────────────────────
    //
    // Port of `PlatformWalletManagerDashPaySync.swift` + the unlock flow
    // in `PlatformWalletManager.swift` (563-668). The recurring sweep is
    // Rust-owned and manager-scoped; like the SPV/identity/shielded loops
    // above, its surface lives on the manager (Swift keeps it in a manager
    // extension). Status is REFLECTED via the same 1 Hz change-gated poll
    // pattern as [spvProgress] — polling, not events, is deliberate: it is
    // exactly how iOS does it, and naive re-assignment burned CPU there.

    private val _dashPaySyncIsSyncing = MutableStateFlow(false)

    /** Whether a DashPay sweep pass is executing right now (1 Hz poll). */
    val dashPaySyncIsSyncing: StateFlow<Boolean> = _dashPaySyncIsSyncing.asStateFlow()

    private val _dashPayUnlockStatus =
        MutableStateFlow<Map<String, DashPayUnlockStatus>>(emptyMap())

    /**
     * Per-wallet seedless-unlock status, keyed by wallet-id hex — Swift
     * `dashPayUnlockStatus`. `draining` while a deferred-contact-crypto
     * drain is in flight; `seedMismatch` when the stored seed failed the
     * binding verify (security-relevant: a mis-mapped Keystore slot);
     * `pendingAccountBuilds` from the 1 Hz poll (drives the unlock banner).
     */
    val dashPayUnlockStatus: StateFlow<Map<String, DashPayUnlockStatus>> =
        _dashPayUnlockStatus.asStateFlow()

    private var dashPayPollJob: Job? = null

    /** Start the recurring DashPay sweep + the 1 Hz status poll. */
    suspend fun startDashPaySync() = withContext(Dispatchers.IO) {
        mapNativeErrors { DashpayNative.dashPaySyncStart(managerHandle) }
        startDashPayStatusPolling()
    }

    /**
     * Stop the recurring sweep (restartable). The 1 Hz status poll keeps
     * running — it also serves the unlock banner (`pendingAccountBuilds`),
     * which must stay fresh while wallets exist regardless of the sweep;
     * it dies with the manager scope in [close].
     */
    suspend fun stopDashPaySync() = withContext(Dispatchers.IO) {
        mapNativeErrors { DashpayNative.dashPaySyncStop(managerHandle) }
        _dashPaySyncIsSyncing.value = false
    }

    suspend fun isDashPaySyncRunning(): Boolean = withContext(Dispatchers.IO) {
        mapNativeErrors { DashpayNative.dashPaySyncIsRunning(managerHandle) }
    }

    suspend fun isDashPaySyncing(): Boolean = withContext(Dispatchers.IO) {
        mapNativeErrors { DashpayNative.dashPaySyncIsSyncing(managerHandle) }
    }

    /** Unix seconds of the last completed sweep; 0 when never. */
    suspend fun dashPayLastSyncUnixSeconds(): Long = withContext(Dispatchers.IO) {
        mapNativeErrors { DashpayNative.dashPaySyncLastSyncUnixSeconds(managerHandle) }
    }

    /** Set the sweep interval (seconds); applies from the next tick. */
    suspend fun setDashPaySyncInterval(seconds: Long) = withContext(Dispatchers.IO) {
        mapNativeErrors { DashpayNative.dashPaySyncSetInterval(managerHandle, seconds) }
    }

    /**
     * Run one sweep pass NOW (pull-to-refresh), blocking until it
     * completes. ← Swift `dashPaySyncNow()`.
     */
    suspend fun dashPaySyncNow(): DashPaySyncSummary = withContext(Dispatchers.IO) {
        val json = mapNativeErrors { DashpayNative.dashPaySyncNow(managerHandle) }
            ?: return@withContext DashPaySyncSummary(0, 0, 0)
        val obj = org.json.JSONObject(json)
        DashPaySyncSummary(
            success = obj.optInt("success"),
            errors = obj.optInt("errors"),
            syncUnixSeconds = obj.optLong("syncUnixSeconds"),
        )
    }

    // ── DPNS marketplace sync ─────────────────────────────────────────

    /** Start the recurring cross-wallet DPNS marketplace sweep. */
    suspend fun startDpnsSync() = withContext(Dispatchers.IO) {
        mapNativeErrors { DpnsMarketplaceNative.syncStart(managerHandle) }
    }

    /** Stop the recurring DPNS marketplace sweep; it may be started again. */
    suspend fun stopDpnsSync() = withContext(Dispatchers.IO) {
        mapNativeErrors { DpnsMarketplaceNative.syncStop(managerHandle) }
    }

    suspend fun isDpnsSyncRunning(): Boolean = withContext(Dispatchers.IO) {
        mapNativeErrors { DpnsMarketplaceNative.syncIsRunning(managerHandle) }
    }

    suspend fun isDpnsSyncing(): Boolean = withContext(Dispatchers.IO) {
        mapNativeErrors { DpnsMarketplaceNative.syncIsSyncing(managerHandle) }
    }

    suspend fun dpnsLastSyncUnixSeconds(): Long = withContext(Dispatchers.IO) {
        mapNativeErrors { DpnsMarketplaceNative.syncLastUnixSeconds(managerHandle) }
    }

    suspend fun setDpnsSyncInterval(seconds: Long) = withContext(Dispatchers.IO) {
        require(seconds > 0) { "seconds must be positive" }
        mapNativeErrors { DpnsMarketplaceNative.syncSetInterval(managerHandle, seconds) }
    }

    /** Run one DPNS marketplace sweep across every registered wallet now. */
    suspend fun dpnsSyncNow(): org.dashfoundation.dashsdk.dpns.DpnsManagerSyncSummary =
        withContext(Dispatchers.IO) {
            val values = mapNativeErrors { DpnsMarketplaceNative.syncNow(managerHandle) }
            check(values.size == 3) { "DPNS sync result must contain three values" }
            org.dashfoundation.dashsdk.dpns.DpnsManagerSyncSummary(
                successCount = values[0].toInt(),
                errorCount = values[1].toInt(),
                syncUnixSeconds = values[2],
            )
        }

    /** Deferred contact-crypto entries queued on [walletId]'s wallet. */
    suspend fun contactCryptoPendingCount(walletId: ByteArray): Int =
        withContext(Dispatchers.IO) {
            val managed = wallet(forWalletId = walletId) ?: return@withContext 0
            mapNativeErrors { DashpayNative.pendingContactCryptoCount(managed.handle) }
        }

    /**
     * Seedless unlock of a restored external-signable wallet — port of
     * Swift `unlockWalletFromKeychain` (verify → drain; the identity-key
     * breadcrumb backfill step is deliberately NOT ported: the Kotlin SDK
     * has no pre-breadcrumb installs to heal).
     *
     * 1. **Verify** the Keystore-resolved seed binds to this wallet
     *    (derives the BIP44 account-0 xpub through the resolver and
     *    compares with the persisted one) — a mis-mapped Keystore slot
     *    fails here, no drain runs, and the wallet stays
     *    external-signable, so a wrong seed can never produce a
     *    network-valid signature for it (the enforcement is on-chain key
     *    ownership — the signer itself is not gated on this verify).
     * 2. **Drain** deferred contact-crypto in the background (network +
     *    ECDH per entry). Guarded against stacking on an in-flight drain.
     *
     * The seed never crosses into Kotlin: the mnemonic → seed conversion
     * happens inside the resolver vtable, and the existence check is
     * [WalletStorage.hasMnemonic] (no decrypt).
     *
     * @return true when the seed verified (drain scheduled); false when
     *   no mnemonic is stored (a genuine watch-only wallet).
     * @throws DashSdkError when the verify fails — a seed mismatch is
     *   published on [dashPayUnlockStatus] before rethrowing.
     */
    suspend fun unlockWalletFromKeystore(managed: ManagedPlatformWallet): Boolean {
        val walletId = managed.walletId
        require(walletId.size == 32) { "walletId must be 32 bytes, got ${walletId.size}" }
        if (!walletStorage.hasMnemonic(walletId)) return false

        val key = walletId.toHex()
        // Wrong-seed / wrong-wallet gate. `seedMismatch` is published from
        // the verify result itself, scoped to JUST this call: the verify
        // FFI maps Rust `SeedMismatch` → ErrorInvalidParameter (de-offset
        // native code 2), and scoping the check here keeps any other
        // invalid-parameter failure elsewhere from being mistaken for a
        // seed mismatch. Rethrown so callers keep their own handling.
        try {
            // Gated: the verify borrows the manager's resolver handle from
            // the CALLER's scope (DashPayTabScreen / loadPersistedWallets)
            // — invisible to the scope join, so it must be counted by the
            // gate. The drain below stays ungated on the manager scope: it
            // creates and owns its own resolver/signer.
            teardownGate.op {
                mapNativeErrors {
                    DashpayNative.verifySeedBindsToWallet(
                        managed.handle,
                        mnemonicResolver.nativeHandle,
                    )
                }
            }
            updateUnlockStatus(key) { it.copy(seedMismatch = false) }
        } catch (e: DashSdkError.PlatformWallet.Generic) {
            if (e.nativeCode == PWFFI_INVALID_PARAMETER) {
                updateUnlockStatus(key) { it.copy(seedMismatch = true) }
            }
            throw e
        }

        // Don't stack a second drain on an in-flight one: a banner Unlock
        // tap while a drain runs would duplicate the network re-fetch +
        // ECDH work and race the channel-broken writes. The false→true
        // transition happens inside ONE atomic flow update — a separate
        // check-then-set would let two concurrent unlocks (auto-unlock
        // racing a banner tap) both pass the check and stack drains.
        var wonDrainSlot = false
        _dashPayUnlockStatus.update { map ->
            val prev = map[key] ?: DashPayUnlockStatus()
            if (prev.draining) {
                wonDrainSlot = false
                map
            } else {
                wonDrainSlot = true
                map + (key to prev.copy(draining = true))
            }
        }
        if (!wonDrainSlot) return true

        // Drain in the background — it re-fetches and decrypts over the
        // network, so it must not block the caller. The drain gets its OWN
        // resolver + signer, owned by this coroutine (the Swift
        // Task.detached + withExtendedLifetime shape), so a manager swap
        // never races their teardown. Launched on the manager scope so
        // close() can JOIN it before nativeDestroy — the drain's store()
        // path reaches the manager-owned PersistenceCallbacks context, so
        // it must have returned before that context is freed. The raw
        // wallet handle is captured, not the wrapper: a wallet destroyed
        // before the drain runs just misses Rust-side (NotFound) — wallet
        // handles are storage-keyed, unlike the resolver/signer boxes. An
        // auth-gated signing failure inside the drain (Android-only:
        // identity keys are biometric-gated here, unlike iOS) leaves the
        // entry queued — the sweep self-heals — and surfaces like any
        // other drain error.
        val walletHandle = managed.handle
        scope.launch(Dispatchers.IO) {
            val drainResolver = MnemonicResolverAndPersister(walletStorage)
            val drainSigner =
                KeystoreSigner(walletStorage, network, biometricGate, database.platformAddressDao())
            try {
                mapNativeErrors {
                    DashpayNative.drainPendingContactCrypto(
                        walletHandle,
                        drainSigner.nativeHandle,
                        drainResolver.nativeHandle,
                    )
                }
            } catch (_: Exception) {
                // Not fatal: the next signer-present DashPay action (or the
                // next unlock) re-attempts; the queue rebuilds via the sweep.
            } finally {
                updateUnlockStatus(key) { it.copy(draining = false) }
                runCatching { drainResolver.close() }
                runCatching { drainSigner.close() }
            }
        }
        return true
    }

    private inline fun updateUnlockStatus(
        key: String,
        transform: (DashPayUnlockStatus) -> DashPayUnlockStatus,
    ) {
        _dashPayUnlockStatus.update { map ->
            val next = transform(map[key] ?: DashPayUnlockStatus())
            if (map[key] == next) map else map + (key to next)
        }
    }

    /**
     * Launch the 1 Hz DashPay status poll (idempotent). Change-gated like
     * [startSpvProgressPolling]: `isSyncing` plus per-wallet
     * `pendingAccountBuilds`, with stale wallet keys pruned — mirror of
     * the Swift `startProgressPolling` DashPay block.
     */
    private fun startDashPayStatusPolling() {
        if (dashPayPollJob?.isActive == true) return
        dashPayPollJob = scope.launch {
            while (isActive && !isClosed) {
                val syncing = runCatching {
                    DashpayNative.dashPaySyncIsSyncing(managerHandle)
                }.getOrDefault(false)
                if (syncing != _dashPaySyncIsSyncing.value) {
                    _dashPaySyncIsSyncing.value = syncing
                }

                val current = _wallets.value
                _dashPayUnlockStatus.update { map ->
                    var next = map
                    // Prune wallets that no longer exist on this manager.
                    for (staleKey in map.keys - current.keys) next = next - staleKey
                    for ((hexId, managed) in current) {
                        val pending = runCatching {
                            DashpayNative.pendingContactCryptoCount(managed.handle)
                        }.getOrDefault(0)
                        val prev = next[hexId] ?: DashPayUnlockStatus()
                        if (prev.pendingAccountBuilds != pending) {
                            next = next + (hexId to prev.copy(pendingAccountBuilds = pending))
                        }
                    }
                    next
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
    /**
     * Suspending teardown — the production path (`WalletManagerStore`'s
     * network switch runs on the Compose Main dispatcher, so teardown must
     * never block the caller's thread; joins suspend and the blocking JNI
     * teardown hops to [Dispatchers.IO]).
     */
    suspend fun closeSuspending() {
        val bundle = bundleRef.getAndSet(0)
        if (bundle == 0L) return
        // NonCancellable from the moment the bundle is claimed: bundleRef
        // is already 0, so a caller cancelled mid-teardown could never
        // retry (the guard above no-ops) — the native manager, its
        // callback GlobalRefs, the signer/resolver boxes, and the
        // persistence executor would all leak silently. Teardown must run
        // to completion once started.
        withContext(NonCancellable) { closeInternal(bundle) }
    }

    private suspend fun closeInternal(bundle: Long) {
        // Stop the progress poll loops + event fan-out scope before teardown
        // so no collector touches a destroyed handle.
        progressPollJob?.cancel()
        dashPayPollJob?.cancel()
        scope.cancel()
        // Cancellation is cooperative: a coroutine already inside a
        // synchronous JNI call — the DashPay contact-crypto drain, or a
        // poll mid-call — keeps running until that call returns, and the
        // drain can still reach persister.store(), whose
        // PersistenceCallbacks context nativeDestroy() below frees. Join
        // the scope (suspending — no thread is blocked) so every in-flight
        // native call has returned before the contexts it can reach are
        // torn down (polls exit at their next suspension; the drain when
        // its JNI call returns, bounded by the FFI's own network timeouts).
        scope.coroutineContext[Job]?.join()
        // Operations that borrow signerHandle / mnemonicResolverHandle run
        // on their CALLERS' scopes (e.g. Dashpay.sendPayment), which the
        // join above does not cover. The gate rejects new borrows and
        // awaits in-flight ones, so the raw signer/resolver boxes below
        // (non-refcounted Box::from_raw frees) are never freed mid-call.
        teardownGate.closeAndAwait()

        withContext(Dispatchers.IO) {
            // Best-effort stop; ignore failures (destroy shuts it all down).
            runCatching { DashpayNative.dashPaySyncStop(managerHandle) }
            runCatching { DpnsMarketplaceNative.syncStop(managerHandle) }
            runCatching { WalletManagerNative.platformAddressSyncStop(managerHandle) }
            runCatching { WalletManagerNative.identitySyncStop(managerHandle) }
            runCatching { WalletManagerNative.shieldedSyncStop(managerHandle) }
            runCatching { WalletManagerNative.spvStop(managerHandle) }

            // Destroy the native manager (shutdown + free contexts).
            WalletManagerNative.nativeDestroy(bundle)

            // Now safe to release the resolver / signer bridges.
            runCatching { mnemonicResolver.close() }
            runCatching { signer.close() }

            // The persistence handler's owned single-thread executor is only
            // safe to shut down once the native manager can no longer fire
            // callbacks into it (a network switch replaces the manager;
            // without this, every switch leaks a live non-daemon
            // "dash-persistence" thread).
            runCatching { persistenceHandler.close() }

            // Drop wallet wrappers we handed out; each still self-destructs
            // via its own Cleaner, but clearing avoids stale handles.
            _wallets.value.values.forEach { runCatching { it.close() } }
            _wallets.value = emptyMap()
        }
    }

    /**
     * Blocking convenience for non-suspend contexts (tests, JVM shutdown
     * paths). Blocks the calling thread until teardown completes —
     * including any in-flight network-bound drain — so NEVER call this on
     * the Android main thread; production teardown goes through
     * [closeSuspending] (see `WalletManagerStore`).
     */
    override fun close() {
        runBlocking { closeSuspending() }
    }

    private companion object {
        /** SPV progress poll cadence — matches Swift's 1 Hz `startProgressPolling`. */
        const val POLL_INTERVAL_MS = 1_000L

        /** De-offset `PlatformWalletFFIResultCode::ErrorInvalidParameter`. */
        const val PWFFI_INVALID_PARAMETER = 2
    }
}

/**
 * Per-wallet seedless-unlock status — Swift `DashPayUnlockStatus`.
 * Published on [PlatformWalletManager.dashPayUnlockStatus]; drives the
 * DashPay tab's unlock banner.
 */
data class DashPayUnlockStatus(
    /** A deferred-contact-crypto drain is in flight for this wallet. */
    val draining: Boolean = false,
    /**
     * The stored seed failed the binding verify — a mis-mapped Keystore
     * slot. Security-relevant: no drain runs and the wallet stays
     * external-signable, so the wrong seed can never produce a
     * network-valid signature for it (its identity keys derive from the
     * correct seed; a foreign-seed signature fails on-chain validation).
     */
    val seedMismatch: Boolean = false,
    /** Deferred contact-crypto entries queued (from the 1 Hz poll). */
    val pendingAccountBuilds: Int = 0,
)

/**
 * One `dashPaySyncNow` result — Swift `DashPaySyncSummary`:
 * per-identity success/error counts + the sweep's unix-seconds stamp.
 */
data class DashPaySyncSummary(
    val success: Int,
    val errors: Int,
    val syncUnixSeconds: Long,
)

/**
 * Decode the tagged shielded-create payload the JNI returns:
 * `[tag || identity_id[32] || diagnostic_utf8...]`.
 *
 * - tag 0 → returns the 32-byte identity id (success).
 * - tag != 0 → throws [DashSdkError.PlatformWallet.ShieldedCreateUnconfirmed]
 *   carrying the id AND the native diagnostic from bytes 33.. (the
 *   underlying DAPI / result-proof confirmation failure); an empty
 *   diagnostic falls back to a generic message. The C ABI writes the id
 *   on the unconfirmed outcome too — the identity may already be live,
 *   so the caller must hold the derivation slot instead of retrying.
 *
 * Extracted from [PlatformWalletManager.shieldedIdentityCreateFromPool]
 * so the codec boundary is unit-testable without the native library
 * (Rust encodes, Kotlin decodes — a silent drift would lose the id or
 * diagnostic on an ambiguous broadcast).
 */
internal fun decodeShieldedCreatePayload(packed: ByteArray): ByteArray {
    check(packed.size >= 33) { "expected >=33-byte tagged identity id, got ${packed.size}" }
    val identityId = packed.copyOfRange(1, 33)
    if (packed[0].toInt() != 0) {
        val diagnostic = if (packed.size > 33) {
            packed.copyOfRange(33, packed.size).decodeToString()
        } else {
            "shielded identity create broadcast unconfirmed"
        }
        throw DashSdkError.PlatformWallet.ShieldedCreateUnconfirmed(
            identityId = identityId,
            message = diagnostic,
        )
    }
    return identityId
}
