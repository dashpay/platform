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
import org.dashfoundation.dashsdk.ffi.AddressPrivateKeyNative
import org.dashfoundation.dashsdk.ffi.FundingNative
import org.dashfoundation.dashsdk.ffi.IdentityRemovalNative
import org.dashfoundation.dashsdk.ffi.ShieldedTokenPoolNative
import org.dashfoundation.dashsdk.ffi.ShieldedTokenPurchaseNative
import org.dashfoundation.dashsdk.ffi.NativePersistenceBridge
import org.dashfoundation.dashsdk.ffi.NativeWalletEventBridge
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import org.dashfoundation.dashsdk.funding.ShieldedProver
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
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyNotStoredException
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
import org.dashfoundation.dashsdk.security.PendingIdentityKeyDelete
import org.dashfoundation.dashsdk.security.MnemonicResolverAndPersister
import org.dashfoundation.dashsdk.security.WalletStorage
import org.dashfoundation.dashsdk.tokens.ContactRequestRef
import org.dashfoundation.dashsdk.tokens.EstablishedContactRef
import org.dashfoundation.dashsdk.voting.MasternodeVotingSigner
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
         * A stored core changeset's non-empty sweeps are durably applied
         * in order: each swept transaction and its outputs are deleted,
         * each released outpoint of the wallet's own is freed unless a
         * stored network-final spender still claims it, and each
         * non-released input RETAINS a durable spend claim — a stamp on
         * the materialised coin, or a tombstone where the funding TXO has
         * not materialised yet — that outlives the loser's deletion, or a
         * post-restart funding delivery would credit a coin the network
         * already consumed. Mirrors `PersistenceCapabilities::CORE_SWEEP_REMOVAL`;
         * aliased to the bridge's declaration so the mirror cannot drift
         * from the bit the handler attests (bit 10, `TRACKED_MASTERNODES`,
         * is deliberately absent: Android never attests it).
         */
        const val CORE_SWEEP_REMOVAL: Long = NativePersistenceBridge.CAPABILITY_CORE_SWEEP_REMOVAL
        /**
         * The private contact request sync's scan state (each identity's
         * read position in DashPay's private contact request store, and its
         * private contacts' shielded addresses) is stored on its round and
         * restored at load. Mirrors
         * `PersistenceCapabilities::PRIVATE_CONTACT_REQUEST_SCAN_STATE`.
         */
        const val PRIVATE_CONTACT_REQUEST_SCAN_STATE: Long =
            NativePersistenceBridge.CAPABILITY_PRIVATE_CONTACT_REQUEST_SCAN_STATE
        /**
         * The Orchard recipient of a shielded asset-lock funding survives
         * store and load with its row, so a resume after a restart pays the
         * recipient the user confirmed. Mirrors
         * `PersistenceCapabilities::ASSET_LOCK_SHIELDED_RECIPIENTS`.
         */
        const val ASSET_LOCK_SHIELDED_RECIPIENTS: Long =
            NativePersistenceBridge.CAPABILITY_ASSET_LOCK_SHIELDED_RECIPIENTS
        /**
         * A tracked asset lock's operation id survives store and load, so a
         * wallet drain ("Max") can be found again after a restart. Mirrors
         * `PersistenceCapabilities::ASSET_LOCK_OPERATION_IDS`.
         */
        const val ASSET_LOCK_OPERATION_IDS: Long =
            NativePersistenceBridge.CAPABILITY_ASSET_LOCK_OPERATION_IDS
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
 * ## SPV quorum keys
 *
 * On an adaptive SDK ([Sdk.ContextProviderKind.ADAPTIVE], the default) the
 * manager serves the SDK the quorum keys of its own SPV client, which the
 * SDK uses to verify proofs once that client has synced. Only one manager
 * serves an SDK: constructing a second on the same SDK while the first is
 * open throws, so close a manager before constructing another on its SDK.
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
        //
        // The OTHER degradation — MO-972's lock-gate drop, where DEVICE_BOUND
        // keys lose setUnlockedDeviceRequired on a device with the
        // false-locked defect on record — is deliberately NOT reported here:
        // effectiveKeySecurityPolicy() cannot express it (see KeySecurityPolicy,
        // "Lock-gate degradation") and the record is a suspending DataStore
        // read with no scope available yet. It is logged loudly at the moment
        // it is recorded (WalletStorage.healFalseLockedMnemonicStore /
        // recordLockBindingDefectFromDeniedRead), and hosts can read it any
        // time via WalletStorage.isMasterKeyLockBindingDefectObserved().
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
     *
     * Internal, with [managerHandle], for the wrappers kept in their own
     * files (`PlatformWalletManagerWalletDrain.kt`).
     */
    internal val teardownGate = TeardownGate()

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

    private val _shieldedBalances =
        MutableStateFlow<Map<String, ShieldedBalanceUpdate>>(emptyMap())

    /**
     * The latest shielded balance update of each wallet, by hex wallet id,
     * as Rust announced it (`on_shielded_balance_changed_fn`). Rust sends an
     * update whenever a wallet's balance state changes, so this is the
     * balance to show: nothing reads or adds one up on the Kotlin side. A
     * wallet without an entry has not been announced yet. A state, not an
     * event stream, so a collector that starts late still sees the latest.
     * Each update is the latest state Rust read, not one per ledger change.
     * Emptied when the manager closes: a closed manager's balances are no
     * longer anyone's.
     */
    val shieldedBalances: StateFlow<Map<String, ShieldedBalanceUpdate>> =
        _shieldedBalances.asStateFlow()

    /**
     * Guards [shieldedBalances] together with [shieldedBalancesClosed]: an
     * update is published and teardown empties the map under it, so an
     * update that arrives while the manager closes cannot refill the map,
     * also when the map was empty before and after.
     */
    private val shieldedBalancesLock = Any()

    /** Set when teardown starts. Read and written under [shieldedBalancesLock]. */
    private var shieldedBalancesClosed = false

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

        override fun onShieldedBalanceChanged(
            walletId: ByteArray,
            revision: Long,
            status: Int,
            ordinarySpendableCredits: Long,
            ordinaryBalanceEstablished: Boolean,
            accounts: LongArray,
        ) {
            // Runs on a native thread: an exception must not travel back
            // into Rust. An update that cannot be read is dropped, and the
            // wallet keeps the last one that could.
            val update = try {
                decodeShieldedBalanceUpdate(
                    walletId,
                    revision,
                    status,
                    ordinarySpendableCredits,
                    ordinaryBalanceEstablished,
                    accounts,
                )
            } catch (e: IllegalStateException) {
                android.util.Log.w("PlatformWalletManager", "Dropped a shielded balance update: ${e.message}")
                return
            }
            synchronized(shieldedBalancesLock) {
                if (!shieldedBalancesClosed) {
                    _shieldedBalances.value = latestShieldedBalances(_shieldedBalances.value, update)
                }
            }
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
                recordRemovedIdentityKeys = { deletes ->
                    walletStorage.withPrivateKeyExclusion { recordPendingIdentityKeyDeletes(deletes) }
                },
                settlePendingKeyDeletesAfterRound = { inFlight ->
                    walletStorage.withPrivateKeyExclusion {
                        settlePendingIdentityKeyDeletes(database, this, inFlight)
                    }
                },
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
     * Runs [block] with a signer that can sign with the masternode voting key stored under
     * [identifier], and with nothing else. Port of Swift `MasternodeVoteCall.run`
     * (`MasternodeVoterSection.swift`), which builds a `KeychainSigner` for each vote, and of
     * `KeychainSigner.withStoredMasternodeVotingKey(identityId:isolation:perform:)`
     * (`FFI/KeychainSigner.swift`).
     *
     * A masternode vote is signed with the voting key Rust builds from the voting address
     * (ECDSA_HASH160, data = the address), so the signer is asked for exactly those 20
     * bytes. This creates a [KeystoreSigner] for this call only, on this manager's
     * [walletStorage], bound to exactly that request: the voting address with key type
     * ECDSA_HASH160 reads the identity's own voting entry ([MasternodeVotingKeyStore]), and
     * every other key, wallet keys and platform addresses included, is refused. This
     * manager's own signer never resolves the address to the voting key, so a vote cannot
     * change what a concurrent wallet operation signs with. Pass the [MasternodeVotingSigner] to a signer vote, for example
     * `sdk.decisionProposals.castVote(proposalId, choice, proTxHash, voter)`. When [block]
     * returns or throws, votes still in flight are awaited and the dedicated signer is
     * freed; the voter cannot be used afterwards.
     *
     * The manager's teardown waits for the whole call.
     *
     * @param identifier the identity's `IdentityEntity.votingPrivateKeyIdentifier`, as
     *   [MasternodeVotingKeyStore.store] returned it.
     * @throws MasternodeVotingKeyNotStoredException when no identity row names [identifier],
     *   no readable voting key is stored under it, or it is not a voting key identifier.
     * @throws IllegalStateException once the manager is closing; and whatever [block] throws.
     */
    suspend fun <T> withStoredMasternodeVotingKey(
        identifier: String,
        block: suspend (voter: MasternodeVotingSigner) -> T,
    ): T = teardownGate.withOp {
        val keys = MasternodeVotingKeyStore(walletStorage)
        val votingAddress = try {
            keys.votingAddressOf(identifier)
        } catch (_: IllegalArgumentException) {
            throw MasternodeVotingKeyNotStoredException()
        }
        // An entry no identity row names is not a loaded voter (a clear or an interrupted
        // write left it), however readable it is.
        if (!database.identityDao().existsByVotingPrivateKeyIdentifier(identifier) ||
            !keys.isStored(identifier)
        ) {
            throw MasternodeVotingKeyNotStoredException()
        }
        // Bound to the one voting key: no fallback to wallet keys, no platform address
        // derivation, and no invalidation hook (that bookkeeping is for wallet keys).
        val dedicated = KeystoreSigner.forBoundKey(
            walletStorage, network, biometricGate, database.platformAddressDao(),
            KeystoreSigner.BoundKey(votingAddress, KeystoreSigner.KEY_TYPE_ECDSA_HASH160, identifier),
        )
        try {
            val voter = MasternodeVotingSigner.create(votingAddress, dedicated)
            try {
                block(voter)
            } finally {
                withContext(NonCancellable) { voter.end() }
            }
        } finally {
            dedicated.close()
        }
    }

    /** This manager's own signer, for tests that check what it can sign. */
    internal val signerForTesting: KeystoreSigner get() = signer

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
            // The identity this key was repaired for was removed meanwhile:
            // record the key, then settle it like an identity removal would
            // (kept while another identity uses it, this wallet's claim
            // released), so a failed delete is retried by the next removal.
            // Under the callback exclusion first, as settling requires: an
            // in-flight round that stores the alias for this wallet commits its
            // row before or after the settle, never between its read and its
            // release.
            abandonStoredKey = { pubkeyHex ->
                persistenceHandler.withCallbackExclusion {
                    walletStorage.withPrivateKeyExclusion {
                        recordPendingIdentityKeyDeletes(listOf(PendingIdentityKeyDelete(pubkeyHex, walletId)))
                        settlePendingIdentityKeyDeletes(
                            database,
                            this,
                            persistenceHandler.aliasesStagedInOpenRounds(),
                        )
                    }
                }
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
     * DIP-13 sub-features DashPay Connect v2 keys live under
     * (`m/9'/coin'/5'/<subFeature>'/0'/identityId'/leaf'[/purpose'/rotation']`),
     * registered by the DIP-13 amendment dashpay/dips#191. Mirror of
     * Swift's `ManagedPlatformWallet.ConnectSubFeature`.
     */
    enum class ConnectSubFeature(val value: Int) {
        /** Session authentication key: the leaf is the connect request id; no purpose level. */
        SESSION_AUTHENTICATION(6),
        /**
         * App encryption key pair: the leaf is the bound data contract's id;
         * the purpose level is a [ConnectKeyPurpose], followed by a rotation level.
         */
        APP_ENCRYPTION(7),
    }

    /**
     * The `purpose'` level of the app-encryption path: the DPP
     * purpose discriminant of the half being derived. Only these two exist
     * in the DIP-13 amendment, so the type makes any other value (in
     * particular 0, the FFI's "no purpose level") unrepresentable. Mirror
     * of Swift's `ManagedPlatformWallet.ConnectKeyPurpose`.
     */
    enum class ConnectKeyPurpose(val value: Int) {
        ENCRYPTION(1),
        DECRYPTION(2),
    }

    /**
     * Derive the DashPay Connect session authentication key at
     * `m/9'/coin'/5'/6'/0'/<identityId>'/<leaf>'` from the wallet's seed,
     * resolved through the mnemonic resolver — the Android analog of Swift's
     * `deriveConnectKey(subFeature:identityId:leaf:network:storage:)`. [identityId]
     * and [leaf] (the connect request id) are DIP-14 256-bit hardened
     * children, so no wallet-local counter is an input. Only
     * [ConnectSubFeature.SESSION_AUTHENTICATION] derives here; app
     * encryption keys need a purpose and rotation, so passing
     * [ConnectSubFeature.APP_ENCRYPTION] throws.
     *
     * @return `(privateKey(32), publicKey(33))`. Caller zeroes the private half.
     */
    suspend fun deriveConnectKey(
        walletId: ByteArray,
        subFeature: ConnectSubFeature,
        identityId: ByteArray,
        leaf: ByteArray,
    ): Pair<ByteArray, ByteArray> = connectKey(identityId, leaf, "leaf") {
        org.dashfoundation.dashsdk.ffi.IdentityNative.deriveConnectKeyV2WithResolver(
            network.ffiValue,
            walletId,
            mnemonicResolver.nativeHandle,
            subFeature.value,
            identityId,
            leaf,
            0,
            0,
        )
    }

    /**
     * Derive one half of a DashPay Connect app encryption pair at
     * `m/9'/coin'/5'/7'/0'/<identityId>'/<leaf>'/<purpose>'/<rotation>'` —
     * the Android analog of Swift's
     * `deriveConnectKey(subFeature:identityId:leaf:purpose:rotation:network:storage:)`.
     * [leaf] is the bound contract's id; [rotation] is the non-negative
     * rotation level, 0 for the first pair. Only
     * [ConnectSubFeature.APP_ENCRYPTION] derives here; anything else throws.
     * Keys registered before DIP-13 gained the rotation level come from
     * [deriveLegacyAppEncryptionKey].
     *
     * @return `(privateKey(32), publicKey(33))`. Caller zeroes the private half.
     */
    suspend fun deriveConnectKey(
        walletId: ByteArray,
        subFeature: ConnectSubFeature,
        identityId: ByteArray,
        leaf: ByteArray,
        purpose: ConnectKeyPurpose,
        rotation: Int,
    ): Pair<ByteArray, ByteArray> = connectKey(identityId, leaf, "leaf") {
        org.dashfoundation.dashsdk.ffi.IdentityNative.deriveConnectKeyV2WithResolver(
            network.ffiValue,
            walletId,
            mnemonicResolver.nativeHandle,
            subFeature.value,
            identityId,
            leaf,
            purpose.value,
            rotation,
        )
    }

    /**
     * Derive one half of a DashPay Connect app encryption pair at the
     * pre-rotation layout `m/9'/coin'/5'/7'/0'/<identityId>'/<contractId>'/<purpose>'`.
     * Only for recognising keys registered on testnet/devnet before DIP-13
     * gained the rotation level; derive new keys with [deriveConnectKey].
     * Mirror of Swift's `deriveLegacyAppEncryptionKey`.
     *
     * @return `(privateKey(32), publicKey(33))`. Caller zeroes the private half.
     */
    suspend fun deriveLegacyAppEncryptionKey(
        walletId: ByteArray,
        identityId: ByteArray,
        contractId: ByteArray,
        purpose: ConnectKeyPurpose,
    ): Pair<ByteArray, ByteArray> = connectKey(identityId, contractId, "contractId") {
        org.dashfoundation.dashsdk.ffi.IdentityNative.deriveLegacyAppEncryptionKeyWithResolver(
            network.ffiValue,
            walletId,
            mnemonicResolver.nativeHandle,
            identityId,
            contractId,
            purpose.value,
        )
    }

    private suspend fun connectKey(
        identityId: ByteArray,
        leaf: ByteArray,
        leafName: String,
        derive: () -> Array<ByteArray>,
    ): Pair<ByteArray, ByteArray> = teardownGate.opWithCleanupOnCancellation(
        // withContext's prompt cancellation can discard a completed derive
        // before the caller receives the pair it is documented to wipe; scrub
        // the private half on that handoff failure.
        cleanup = { pair -> pair.first.fill(0) },
    ) {
        require(identityId.size == 32) { "identityId must be 32 bytes, got ${identityId.size}" }
        require(leaf.size == 32) { "$leafName must be 32 bytes, got ${leaf.size}" }
        val pair = org.dashfoundation.dashsdk.errors.mapNativeErrors(derive)
        check(pair.size == 2) { "connect key derive returned ${pair.size} elements" }
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
        org.dashfoundation.dashsdk.identity.IdentityUpdates(teardownGate) { signerHandle }

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

    /**
     * Decision proposal create and edit (protocol version 15): the decision proposal
     * slice of `ManagedPlatformWallet` (`ManagedPlatformWalletDecisionProposals.swift`).
     * Stateless; callers thread the wallet handle and [signerHandle] into each call. Reads
     * and the masternode vote are on `Sdk.decisionProposals`.
     */
    val decisionProposals: org.dashfoundation.dashsdk.voting.DecisionProposalTransactions =
        org.dashfoundation.dashsdk.voting.DecisionProposalTransactions(teardownGate)

    /**
     * Multiple choice proposal create and edit, and entering a choice (protocol version
     * 15): the multiple choice proposal slice of `ManagedPlatformWallet`
     * (`ManagedPlatformWalletMultipleChoiceProposals.swift`). Stateless; callers thread the
     * wallet handle and [signerHandle] into each call. Reads and the masternode vote are on
     * `Sdk.multipleChoiceProposals`.
     */
    val multipleChoiceProposals: org.dashfoundation.dashsdk.voting.MultipleChoiceProposalTransactions =
        org.dashfoundation.dashsdk.voting.MultipleChoiceProposalTransactions(teardownGate)

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
    internal val managerHandle: Long = nativeInitialization.managerHandle

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
     * @throws org.dashfoundation.dashsdk.security.KeystoreDeviceLockedException
     *   (RETRYABLE after unlock) if the device is locked at entry AND the
     *   master key is actually lock-bound — decided by the Keystore itself
     *   via [WalletStorage.ensureMasterKeyNotLockBlocked]'s preflight
     *   encrypt, and thrown BEFORE the native create, so nothing was
     *   created and nothing needs rolling back — or if the Keystore denies
     *   the mnemonic store as device-locked after the false-locked bounded
     *   retry in [WalletStorage.storeMnemonic] is exhausted AND its
     *   last-rung degradation (re-encrypting under the never-lock-bound
     *   master alias) also failed (that path runs the full rollback below
     *   first). A locked device whose master key is NOT lock-bound
     *   (generated before a PIN was enrolled) proceeds normally, as does a
     *   device whose false-locked Keystore defect is already on record
     *   (mnemonic writes target the never-lock-bound alias, which no lock
     *   state can deny).
     */
    suspend fun createWallet(
        mnemonic: String,
        name: String? = null,
        createDefaultAccounts: Boolean = true,
        birthHeight: UInt? = null,
    ): ManagedPlatformWallet = withContext(Dispatchers.IO) {
        // Fail-fast pre-check BEFORE the native create: on a locked device
        // whose MASTER_ALIAS key is lock-bound (setUnlockedDeviceRequired)
        // the storeMnemonic step below is guaranteed to be denied, which
        // would force the full rollback dance. The pre-check preflights one
        // master-alias encrypt so the Keystore itself renders that verdict
        // (a key generated before any PIN existed carries no lock binding
        // and keeps working while locked — creation must proceed then).
        // Failing here instead means no native wallet was created, no Room
        // rows were written, and there is nothing to roll back — the typed
        // KeystoreDeviceLockedException tells the caller to retry after
        // unlock.
        walletStorage.ensureMasterKeyNotLockBlocked(operation = "createWallet")
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

    /**
     * Remove one identity from this device. Port of SwiftExampleApp's
     * `IdentitiesContentView.removeIdentityLocally` (#6084), with the live
     * wallet removal of dashpay/platform#6098 done first. Nothing is
     * broadcast and the identity stays on Platform; it can be loaded again.
     *
     * Rust removes the identity from every running wallet that manages it
     * (`platform_wallet_manager_remove_identity_from_device`), so no later
     * sync stores it again, or refuses while its DashPay contact request
     * channel is held. A refusal throws the platform-wallet
     * [org.dashfoundation.dashsdk.errors.DashSdkError] and nothing on the
     * device is deleted. Then the identity's Room rows go in one transaction,
     * then its stored private keys and masternode voting key; a secret the
     * secure storage refuses to delete after the rows are gone is reported in
     * [IdentityDeviceRemovalResult.keysLeft] instead of thrown. A key another
     * identity row references, or another wallet's owner index claims, is
     * kept. See [deleteIdentityFromDevice] for the order.
     *
     * @param identityId the 32-byte identity id.
     */
    suspend fun removeIdentityFromDevice(identityId: ByteArray): IdentityDeviceRemovalResult =
        teardownGate.op {
            deleteIdentityFromDevice(
                identityId,
                persistenceHandler,
                database,
                walletStorage,
                removeFromLiveWallets = { id ->
                    mapNativeErrors { IdentityRemovalNative.removeIdentityFromDevice(managerHandle, id) }
                },
            )
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
     *
     * On failure the manager is unchanged and still usable — fix the store
     * and call again, or destroy the manager and rebuild it.
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

        // Finish identity-key deletes an identity removal recorded but could
        // not settle before (a failed round-end settle, a process death):
        // best effort, retried on the next launch or removal.
        try {
            persistenceHandler.settlePendingIdentityKeyDeletesNow()
        } catch (cancellation: kotlin.coroutines.cancellation.CancellationException) {
            throw cancellation
        } catch (t: Throwable) {
            android.util.Log.w(
                "PlatformWalletManager",
                "pending identity-key deletes not settled on load; retried later",
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

    /**
     * Reveal the private key of [address], one of the tracked core addresses
     * of the wallet [walletId], as hex and WIF. Port of Swift's
     * `ManagedPlatformWallet.coreAddressPrivateKey(address:)`
     * (`ManagedPlatformWallet.swift`), bridging
     * `platform_wallet_address_private_key`.
     *
     * Rust finds the address in the wallet's account pools, reads the mnemonic
     * through this manager's resolver and derives the key; Kotlin only decodes
     * the result. Which addresses have a key to reveal is Rust's answer: an
     * empty or malformed address, one the wallet does not track, or a DashPay
     * contact's external address whose key the wallet never held, throws the
     * [org.dashfoundation.dashsdk.errors.DashSdkError] Rust reports.
     *
     * Swift builds a resolver per call; here the call borrows this manager's
     * resolver under this manager's teardown gate, so the wallet and the
     * resolver always come from the same live manager and no caller-supplied
     * handle reaches native code.
     *
     * Gate this behind device authentication before calling it. The caller
     * owns the result and should [CoreAddressPrivateKey.clear] it when the key
     * leaves the screen. If the calling coroutine is cancelled after the native
     * call returns, the key is cleared here instead of being handed back.
     *
     * @throws org.dashfoundation.dashsdk.errors.DashSdkError.NotFound when no
     *   wallet with [walletId] is loaded in this manager.
     */
    suspend fun coreAddressPrivateKey(
        walletId: ByteArray,
        address: String,
    ): CoreAddressPrivateKey = teardownGate.opWithCleanupOnCancellation(cleanup = { it.clear() }) {
        val wallet = wallet(forWalletId = walletId)
            ?: throw DashSdkError.NotFound("wallet ${walletId.toHex()} is not loaded")
        val blob = mapNativeErrors {
            AddressPrivateKeyNative.revealCoreAddressPrivateKey(
                wallet.handle,
                mnemonicResolverHandle,
                address,
            )
        }
        CoreAddressPrivateKey.decodeAndWipe(blob)
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
     *
     * Does not wait for a shielded operation in flight (a bind, an added
     * account, a token pool transfer or unshield, a profile write, a tip
     * address preparation): while one runs this throws
     * [DashSdkError.PlatformWallet.ShieldedOperationInProgress] at once,
     * with nothing changed and the sync loop still running. Keep the Room
     * rows and call it again once the operation has finished.
     */
    suspend fun clearShieldedStorage() = withContext(Dispatchers.IO) {
        mapNativeErrors { WalletManagerNative.shieldedClear(managerHandle) }
    }

    /**
     * The ordinary ZIP-32 shielded accounts Rust holds for [walletId], in
     * ascending order: the accounts of its last [bindShielded], without the
     * identity accounts Rust adds to a bind by itself. Empty for a wallet
     * that was never bound on this manager.
     *
     * The list survives [clearShieldedStorage], which drops the wallet's
     * registration on the coordinator but not the keys Rust holds for it. So
     * it is what to pass to [bindShielded] to register the wallet again
     * with the accounts it had. Rust is the source: the host keeps no copy
     * of its own.
     *
     * Swift has no counterpart: its Clear registers nothing again, and its
     * next Sync Now registers the other wallets with the default account.
     * Only meaningful on a shielded build ([Sdk.hasShielded]); the native
     * entry point is absent otherwise and this throws.
     */
    suspend fun shieldedOrdinaryAccounts(walletId: ByteArray): List<Int> =
        withContext(Dispatchers.IO) {
            require(walletId.size == 32) {
                "walletId must be exactly 32 bytes, got ${walletId.size}"
            }
            mapNativeErrors {
                WalletManagerNative.shieldedOrdinaryAccounts(managerHandle, walletId)
            }.toList()
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
     * Consensus-pinned flat shielded fee (in credits) for a pool-paid
     * shielded transition of [kind] with [numActions] Orchard actions —
     * port of Swift's `PlatformWalletManager.estimateShieldedFee`
     * (`PlatformWalletManagerShieldedSync.swift`). Computed at this
     * manager's network-tracked platform version (the same version the
     * shielded builders carve fees with), so the preview can't drift from
     * the fee the consensus gate validates even when the connected network
     * hasn't activated the client's latest protocol version yet. No
     * network round-trip. A single-note spend with change is
     * `numActions = 2`.
     */
    suspend fun estimateShieldedFee(
        kind: ShieldedProver.FeeKind,
        numActions: Int = 2,
    ): Long = withContext(Dispatchers.IO) {
        mapNativeErrors {
            FundingNative.estimateShieldedFee(managerHandle, kind.raw, numActions)
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
     * Fund a wallet's shielded (Orchard) pool by DRAINING its CoinJoin account
     * (`m/9'/coinType'/4'/coinJoinAccountIndex'`) into a single asset lock —
     * port of Swift's `shieldedFundFromCoinJoinDrain`.
     *
     * Sibling of [shieldedFundFromAssetLock] with drain funding, which is what
     * makes this the CoinJoin → Shielded migration path: every final mixed-coin
     * UTXO is consumed and the lock value is `Σ inputs − L1 fee`, computed
     * Rust-side, so the mixed coins never hop through a transparent BIP44
     * address on the way in. Hence no amount parameter, and no surplus output
     * (the single-recipient remainder flow pins the consensus surplus to zero).
     *
     * The recipient receives `lockValue − poolFee` credits. The Rust preflight
     * rejects a drain whose balance could not clear the Type 18 pool fee, so an
     * unrecoverable dust lock is never broadcast; a drain of an empty account
     * fails with the typed asset-lock shortfall
     * ([org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.AssetLockInsufficientFunds]).
     * A stuck lock resumes via [shieldedResumeFundFromAssetLock] exactly like a
     * BIP44-funded one.
     *
     * Blocks for the ~30s Halo 2 proof; the shielded note itself arrives on the
     * next shielded sync pass, so nothing is returned.
     *
     * @param walletId the 32-byte wallet id.
     * @param recipientRaw43 the 43-byte raw Orchard payment address
     *   (11-byte diversifier + 32-byte pk_d).
     * @param coinJoinAccountIndex the CoinJoin account whose whole balance
     *   funds the asset lock (account 0 for every current wallet).
     */
    suspend fun shieldedFundFromCoinJoinDrain(
        walletId: ByteArray,
        recipientRaw43: ByteArray,
        coinJoinAccountIndex: Int = 0,
    ): Unit = teardownGate.op {
        require(coinJoinAccountIndex >= 0) {
            "coinJoinAccountIndex must be non-negative, got $coinJoinAccountIndex"
        }
        mapNativeErrors {
            FundingNative.shieldedFundFromCoinJoinDrain(
                managerHandle,
                walletId,
                coinJoinAccountIndex,
                recipientRaw43,
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
     * Shield from a Platform IDENTITY's balance (Type 21). Sibling of
     * [shieldedShield] with the identity, rather than the transparent
     * Platform-Payment addresses, as the funding side: [amount] credits move
     * straight out of [identityId]'s balance into this wallet's own bound
     * shielded pool ([shieldedAccount]), and the identity is debited [amount]
     * plus the metered fee plus the shielded compute fee
     * ([ShieldedProver.FeeKind.ShieldFromIdentity]). The identity must be
     * managed by this wallet. Signed by the Keystore identity signer
     * ([signerHandle]) with the identity's TRANSFER key, the same handle
     * [org.dashfoundation.dashsdk.credits.IdentityCredits.transferToAddresses]
     * threads through. Swift counterpart:
     * `PlatformWalletManager.shieldedShieldFromIdentity` in
     * packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PlatformWalletManagerShieldedSync.swift. Self-shield only (Rust always targets this wallet's own
     * default Orchard address, so there is no recipient parameter). Blocks for
     * the ~30s Halo 2 proof; the note arrives on the next shielded sync pass.
     *
     * @param walletId the 32-byte wallet id.
     * @param identityId the 32-byte funding identity id.
     * @param amount credits to shield (1 DASH = 1e11).
     * @return the identity's proven post-debit credit balance; the wallet only
     *   confirms on the identity's own balance proof, any other result throws
     *   the shielded-spend-unconfirmed error (do not resubmit).
     */
    suspend fun shieldedShieldFromIdentity(
        walletId: ByteArray,
        identityId: ByteArray,
        amount: Long,
        shieldedAccount: Int = 0,
    ): Long = teardownGate.op {
        require(identityId.size == 32) {
            "identityId must be exactly 32 bytes, got ${identityId.size}"
        }
        require(amount > 0) { "amount must be positive, got $amount" }
        require(shieldedAccount >= 0) {
            "shieldedAccount must be non-negative, got $shieldedAccount"
        }
        mapNativeErrors {
            FundingNative.shieldedShieldFromIdentity(
                managerHandle,
                walletId,
                shieldedAccount,
                identityId,
                amount,
                signerHandle,
            )
        }
    }

    /** List active and archived durable identity-funded shields for this manager's wallet. */
    suspend fun shieldedIdentityDebitRecoveryRecords(
        walletId: ByteArray,
    ): List<ShieldedIdentityDebitRecoveryRecord> = teardownGate.op {
        require(walletId.size == 32) { "walletId must be exactly 32 bytes, got ${walletId.size}" }
        mapNativeErrors {
            FundingNative.shieldedIdentityDebitRecoveryRecords(managerHandle, walletId)
                .map { ShieldedIdentityDebitRecoveryRecord.fromNative(it) }
        }
    }

    /**
     * Stop automatic retry of exactly `(walletId, accountIndex, activityId)`.
     * Archives the complete signed record for later scan confirmation and audit,
     * preserves an Unknown outcome, and permits a new identity debit after saving.
     *
     * This does not cancel an already relayed or in-flight payment. A new payment
     * may cause an additional debit even if the old nonce is still usable. Obtain
     * the user's informed acknowledgement before passing true. Acknowledgement
     * is mandatory, has no default, and false is rejected by the wallet.
     */
    suspend fun abandonShieldedIdentityDebit(
        walletId: ByteArray,
        accountIndex: UInt,
        activityId: ByteArray,
        acknowledgePossibleExecution: Boolean,
    ) = teardownGate.op {
        require(walletId.size == 32) { "walletId must be exactly 32 bytes, got ${walletId.size}" }
        require(activityId.size == 32) { "activityId must be exactly 32 bytes, got ${activityId.size}" }
        mapNativeErrors {
            FundingNative.abandonShieldedIdentityDebit(
                managerHandle, walletId, accountIndex.toInt(), activityId,
                acknowledgePossibleExecution,
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

    // ── Identity shielded accounts ─────────────────────────────────────

    /**
     * Prepare the dedicated shielded account of the wallet-owned identity
     * [identityId] on [walletId] and return the account's default 43-byte raw
     * Orchard address. Port of Swift's
     * `PlatformWalletManager.prepareShieldedIdentityAddress(walletId:identityId:resolver:)`
     * (`PlatformWalletManagerShieldedSync.swift`). Publishes nothing and
     * moves no funds.
     *
     * Rust proves the identity's slot with the seed this manager's mnemonic
     * resolver supplies, binds the identity's account next to the wallet's
     * bound accounts and records the preparation for this session. A contact
     * request sent afterwards carries the sender's per-contact shielded
     * address only once this preparation stands. Requires a prior
     * [bindShielded] of [walletId]. Repeated calls are idempotent. Only
     * meaningful on a shielded build ([Sdk.hasShielded]); the native entry
     * point is absent otherwise and this throws.
     *
     * Throws [DashSdkError.NotFound] when the wallet holds no identity
     * [identityId], [DashSdkError.PlatformWallet.ShieldedRecoveryCorrupted] or
     * [DashSdkError.PlatformWallet.ShieldedRecoveryKeysRequired] when the bind
     * meets unusable persisted shielded state, and
     * [DashSdkError.PlatformWallet.WalletOperation] for every other failure
     * (an unbound wallet, a seed that does not derive the identity's key),
     * with the wallet's reason in the message.
     */
    suspend fun prepareShieldedIdentityAddress(
        walletId: ByteArray,
        identityId: ByteArray,
    ): ByteArray = teardownGate.op {
        mapNativeErrors {
            FundingNative.prepareShieldedIdentityAddress(
                managerHandle,
                walletId,
                mnemonicResolver.nativeHandle,
                identityId,
            )
        }
    }

    // ── Shielded tips to a DashPay username ────────────────────────────
    //
    // A tip is an ordinary shielded transfer to the shielded address a
    // name's DashPay profile publishes. Rust
    // (`platform-wallet`'s `wallet/shielded/tip_payment.rs`) decides
    // everything a payment screen would otherwise decide for itself: what
    // the typed amount is, whom the name pays, whether that changed, whether
    // an earlier payment is unresolved, which account pays, whether the tip
    // may be sent now and whether an identity's balance is known. These
    // wrappers pass the arguments in and the results out, as the Swift
    // ones in `PlatformWalletManagerShieldedTips.swift` do.

    /**
     * Parse a tip amount the user typed in DASH into exact credits. Port of
     * Swift's `PlatformWalletManager.parseShieldedTipAmount`. A pure parse:
     * no wallet, no network, and it does not wait.
     *
     * Rust accepts ASCII digits with one optional decimal separator, `.` or
     * `,`, ignoring surrounding whitespace, and refuses everything else (a
     * sign, digit grouping, an exponent, zero, digits finer than one
     * credit). Show the returned amount in the confirmation, not the typed
     * text: `1,000` is one DASH. Do not parse the text on the host.
     *
     * Throws [DashSdkError.PlatformWallet.Generic] with native code 2
     * (`ErrorInvalidParameter`) and Rust's reason for a text it refuses, and
     * [DashSdkError.InvalidParameter] for an amount past what a `Long` of
     * credits carries.
     */
    fun parseShieldedTipAmount(text: String): Long =
        mapNativeErrors { FundingNative.parseShieldedTipAmount(text) }

    /**
     * Resolve [username] for the user to confirm a shielded tip from
     * [walletId]: whom it pays now (the identity the DPNS name resolves to
     * and the shielded address its DashPay profile publishes, both fetched
     * with proof), how that compares with the recipient this wallet last
     * confirmed for the name, and whether an earlier shielded payment to
     * that recipient is still unresolved. Port of Swift's
     * `PlatformWalletManager.reviewShieldedTip`. Changes nothing and moves
     * no funds. Accepts `alice` and `alice.dash`.
     *
     * Show [ShieldedTipReview.recipient] and [ShieldedTipReview.change],
     * then pass the recipient back to [sendShieldedTip] unchanged. While
     * [ShieldedTipReview.unresolved] is set a send is refused. Requires a
     * prior [bindShielded] of [walletId].
     *
     * When the tip starts from a saved contact, pass the contact's 32-byte
     * identity id as [contactIdentityId]: Rust refuses the review with
     * [DashSdkError.PlatformWallet.ShieldedTipRecipientMismatch] when the
     * name now belongs to another identity, instead of showing that
     * identity as a first tip to the contact. Leave it null when the user
     * typed the name. Do not compare the ids on the host.
     *
     * Throws [DashSdkError.PlatformWallet.ShieldedRecoveryCorrupted] or
     * [DashSdkError.PlatformWallet.ShieldedRecoveryKeysRequired] when the
     * wallet cannot tell whom a saved shielded spend pays, and
     * [DashSdkError.PlatformWallet.WalletOperation] for every other failure
     * (a name that does not exist, a profile that publishes no valid
     * shielded address, an unbound wallet), with the wallet's reason in the
     * message.
     */
    suspend fun reviewShieldedTip(
        walletId: ByteArray,
        username: String,
        contactIdentityId: ByteArray? = null,
    ): ShieldedTipReview = teardownGate.op {
        decodeShieldedTipReview(
            mapNativeErrors {
                FundingNative.reviewShieldedTip(
                    managerHandle,
                    walletId,
                    username,
                    contactIdentityId,
                )
            },
        )
    }

    /**
     * Send a shielded tip of [amount] credits from [walletId] to [username],
     * paid from [source], with [memo] on the recipient's note. Port of
     * Swift's `PlatformWalletManager.sendShieldedTip`. Blocks for the Halo 2
     * proof.
     *
     * The tip pays [expectedRecipient], the [ShieldedTipReview.recipient]
     * the user confirmed, and nothing else: Rust resolves the name and its
     * profile again and refuses a recipient that differs, for the user to
     * review again. There is no fallback to a transparent address.
     *
     * Throws:
     * - [DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed]: the tip may
     *   have left the wallet and may still execute. Never send it again; a
     *   shielded sync resolves it. Read
     *   [DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed] and
     *   [DashSdkError.StateTransitionResultUnconfirmed] the same way, as
     *   for [shieldedTransfer].
     * - [DashSdkError.PlatformWallet.ShieldedTipPending]: this call sent
     *   nothing. Another tip from this wallet is being sent, or an earlier
     *   shielded payment to the recipient may still execute. That earlier
     *   payment may be this same tip and may pay the recipient: a shielded
     *   sync resolves it as executed or as unable to execute, and the
     *   refusal ends either way. Do not send again because the refusal
     *   ended; send again only once the earlier payment is known to have
     *   failed (its shielded activity entry reads failed, not confirmed).
     *   This call returns no id for that entry, so when the exact attempt
     *   cannot be identified, treat its outcome as unknown and do not send
     *   again.
     * - [DashSdkError.NotFound]: the wallet holds no identity
     *   [ShieldedTipSource.Identity.identityId]. Nothing was sent.
     * - [DashSdkError.PlatformWallet.ShieldedTipRecipientMismatch]:
     *   [contactIdentityId] was given and the reviewed recipient is another
     *   identity. Nothing was sent.
     * - any other error: this call sent nothing (for example
     *   [DashSdkError.PlatformWallet.WalletOperation] for a recipient that
     *   changed since the review).
     *
     * A caller cancelled while the native call runs still gets its outcome
     * when the call completes ([opReturningCompletedResult]): native code
     * cannot see the cancellation, so the tip can be sent after it, and a
     * sent tip must never read as a cancelled one. Handle the result before
     * suspending again.
     *
     * @param amount credits to send, as [parseShieldedTipAmount] returned
     *   them.
     * @param memo optional UTF-8 memo on the recipient's note (null or
     *   empty: no memo; Rust refuses more than 32 UTF-8 bytes).
     * @param contactIdentityId the saved contact's 32-byte identity id when
     *   the tip starts from a contact, as for [reviewShieldedTip]; null when
     *   the user typed the name.
     */
    suspend fun sendShieldedTip(
        walletId: ByteArray,
        source: ShieldedTipSource,
        username: String,
        expectedRecipient: ShieldedTipRecipient,
        amount: Long,
        memo: String? = null,
        contactIdentityId: ByteArray? = null,
    ): Unit = teardownGate.opReturningCompletedResult {
        // The C call takes the source as a nullable identity id beside an
        // account number that it ignores when the id is present.
        val sourceIdentityId: ByteArray?
        val sourceAccount: Int
        when (source) {
            is ShieldedTipSource.Identity -> {
                sourceIdentityId = source.identityId
                sourceAccount = 0
            }
            is ShieldedTipSource.Account -> {
                sourceIdentityId = null
                sourceAccount = source.account
            }
        }
        mapNativeErrors {
            FundingNative.sendShieldedTip(
                managerHandle,
                walletId,
                mnemonicResolver.nativeHandle,
                sourceIdentityId,
                sourceAccount,
                username,
                expectedRecipient.identityId,
                expectedRecipient.address,
                contactIdentityId,
                amount,
                memo,
            )
        }
    }

    /**
     * The shielded balance of the wallet identity [identityId]'s own account
     * on [walletId], with what the wallet knows about it. Port of Swift's
     * `PlatformWalletManager.shieldedIdentityBalance`. Reads local state
     * only.
     *
     * Show the returned state; do not add up stored notes, which cannot
     * tell an empty account from one whose ledger was cleared.
     * [ShieldedIdentityBalance.Unverified] stands until
     * [prepareShieldedIdentityAddress] has proved the identity's account in
     * this session.
     *
     * Throws [DashSdkError.NotFound] when the wallet holds no identity
     * [identityId], and [DashSdkError.PlatformWallet.WalletOperation] when
     * the read was busy (a scan holds the store: keep the last state and
     * read again later) or failed otherwise, with the reason in the message.
     */
    suspend fun shieldedIdentityBalance(
        walletId: ByteArray,
        identityId: ByteArray,
    ): ShieldedIdentityBalance = teardownGate.op {
        decodeShieldedIdentityBalance(
            mapNativeErrors {
                FundingNative.shieldedIdentityBalance(managerHandle, walletId, identityId)
            },
        )
    }

    // ── Shielded outgoing spends (types 16/17/19) ──────────────────────
    //
    // Manager-handle FFI calls like the funding submits above; each signs
    // with the bound shielded sub-wallet's own Orchard spend key Rust-side
    // (cached by [bindShielded]), so no signer or resolver handle crosses
    // JNI — matching the Swift counterparts, which take no signer either.
    // Each blocks for the ~30s Halo 2 proof on Dispatchers.IO. On the
    // AMBIGUOUS outcome (the spend may have reached a node and its result is
    // not proved, a node's refusal or verdict included) they
    // throw [org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed]:
    // the caller must NOT retry (the spent notes stay reserved Rust-side
    // until a later shielded sync resolves them).

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
     * Shielded to existing-identity top-up (Type 22), a port of Swift's
     * `PlatformWalletManager.shieldedIdentityTopUpFromPool`
     * (`PlatformWalletManagerShieldedSync.swift`). Spends notes from
     * [account] on [walletId] and credits [identityId]'s Platform balance.
     *
     * The identity only has to exist on Platform; it does not have to be
     * one this wallet manages. The flat pool-paid fee
     * ([ShieldedProver.FeeKind.IdentityTopUpFromPool], i.e.
     * [estimateShieldedFee] kind 4) is spent from the notes on top of
     * [amount].
     *
     * @param walletId the 32-byte wallet id.
     * @param identityId the 32-byte id of the identity being topped up.
     * @param amount credits the identity receives (1 DASH = 1e11 credits).
     * @param account the ZIP-32 shielded account to spend from (usually 0).
     */
    suspend fun shieldedIdentityTopUpFromPool(
        walletId: ByteArray,
        identityId: ByteArray,
        amount: Long,
        account: Int = 0,
    ): Unit = teardownGate.op {
        require(amount > 0) { "amount must be positive, got $amount" }
        require(account >= 0) { "account must be non-negative, got $account" }
        require(identityId.size == 32) {
            "identityId must be exactly 32 bytes, got ${identityId.size}"
        }
        mapNativeErrors {
            FundingNative.shieldedIdentityTopUpFromPool(
                managerHandle,
                walletId,
                mnemonicResolver.nativeHandle,
                account,
                identityId,
                amount,
            )
        }
    }

    /**
     * Buy [tokenCount] tokens at their direct purchase price from this
     * wallet's shielded notes: the Type 28 `TokenPurchaseFromShieldedPool`
     * transition. Port of Swift's
     * `PlatformWalletManager.shieldedTokenPurchaseFromPool`
     * (`PlatformWalletManagerShieldedSync.swift`).
     *
     * The notes of [account] pay [totalAgreedPrice] plus the pool-paid fee
     * ([ShieldedProver.FeeKind.TokenPurchaseFromPool]), and the tokens are
     * minted as a note to the account's own address in the token's shielded
     * pool. No identity is involved. Rust checks the token count, the
     * network's protocol version, the token's shielded pool and the note
     * count, selects the notes, builds, broadcasts, records the activity
     * and requests the shielded sync that looks for it.
     *
     * Returning means SUBMITTED, not confirmed: Platform accepted the
     * purchase, but its result proof does not show that it executed. It
     * stays pending, with its notes reserved, until a shielded sync finds
     * it. Do not show it as complete and do not retry it.
     *
     * Throws [DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed] once the
     * purchase may have reached a node and its result is not known: it may
     * already be on chain or may still execute, so the caller must NOT
     * retry. That is the error Rust reports this case with. A caller should
     * read [DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed] and
     * [DashSdkError.StateTransitionResultUnconfirmed] the same way, as the
     * example app does for every shielded operation: they also say a result
     * is not known. Any other error means nothing was sent.
     *
     * @param walletId the 32-byte wallet id.
     * @param contractId the 32-byte id of the token's data contract.
     * @param tokenPosition the token's position in that contract.
     * @param tokenCount tokens to buy, in the token's base units.
     * @param totalAgreedPrice credits the buyer agrees to pay; Platform
     *   refuses the purchase unless it covers the token's configured price
     *   for [tokenCount].
     * @param account the ZIP-32 shielded account to spend from (usually 0).
     */
    suspend fun shieldedTokenPurchaseFromPool(
        walletId: ByteArray,
        contractId: ByteArray,
        tokenPosition: Int,
        tokenCount: ULong,
        totalAgreedPrice: ULong,
        account: Int = 0,
    ): Unit = teardownGate.op {
        require(account >= 0) { "account must be non-negative, got $account" }
        require(walletId.size == 32) { "walletId must be exactly 32 bytes, got ${walletId.size}" }
        require(contractId.size == 32) {
            "contractId must be exactly 32 bytes, got ${contractId.size}"
        }
        requireTokenPosition(tokenPosition)
        mapNativeErrors {
            ShieldedTokenPurchaseNative.purchaseFromPool(
                managerHandle,
                walletId,
                mnemonicResolver.nativeHandle,
                account,
                contractId,
                tokenPosition,
                tokenCount.toLong(),
                totalAgreedPrice.toLong(),
            )
        }
    }

    /**
     * A token position is a u16 on the Rust side: refuse one that does not
     * fit before the native call, with the same error `Tokens` raises.
     */
    private fun requireTokenPosition(tokenPosition: Int) {
        require(tokenPosition in 0..0xFFFF) {
            "tokenPosition must be in 0..65535, got $tokenPosition"
        }
    }

    /**
     * Whether the token at [tokenPosition] of [contractId] can be bought
     * with [shieldedTokenPurchaseFromPool], from the same verified read of
     * its configuration that purchase refuses on. Port of Swift's
     * `PlatformWalletManager.shieldedTokenPurchaseAvailability` (same file).
     * A purchase form offers the shielded payment on
     * [ShieldedTokenPurchaseAvailability.Available] only.
     *
     * Throws when the question itself could not be asked (a closed
     * manager); a configuration that could not be read is the
     * [ShieldedTokenPurchaseAvailability.ConfigurationUnreadable] answer.
     */
    suspend fun shieldedTokenPurchaseAvailability(
        contractId: ByteArray,
        tokenPosition: Int,
    ): ShieldedTokenPurchaseAvailability = teardownGate.op {
        require(contractId.size == 32) {
            "contractId must be exactly 32 bytes, got ${contractId.size}"
        }
        requireTokenPosition(tokenPosition)
        ShieldedTokenPurchaseAvailability.fromNative(
            mapNativeErrors {
                ShieldedTokenPurchaseNative.availability(managerHandle, contractId, tokenPosition)
            },
        )
    }

    // ── Token shielded pools (types 26/27) ─────────────────────────────
    //
    // The wallet's notes in token shielded pools, kept by Rust in the
    // shielded store's own SQLite file (no Room table). The watched pools
    // are scanned by the ordinary shielded sync ([syncShieldedNow] and the
    // background loop); a type 28 purchase watches its token's pool by
    // itself. Ports of the same-named Swift methods in
    // `PlatformWalletManagerShieldedSync.swift`.

    /**
     * Watch the shielded pool of the token at [tokenPosition] of
     * [contractId] for [walletId], so every later shielded sync scans it for
     * the wallet's notes. Returns the token's 32-byte id. Rust reads the
     * token's configuration from Platform with proofs first and refuses a
     * token without a shielded pool, or one whose configuration cannot be
     * read (asking again may succeed), with its reason in the error message.
     * Watching a watched pool again changes nothing.
     */
    suspend fun shieldedTokenPoolWatch(
        walletId: ByteArray,
        contractId: ByteArray,
        tokenPosition: Int,
    ): ByteArray = teardownGate.op {
        require(walletId.size == 32) { "walletId must be exactly 32 bytes, got ${walletId.size}" }
        require(contractId.size == 32) {
            "contractId must be exactly 32 bytes, got ${contractId.size}"
        }
        requireTokenPosition(tokenPosition)
        mapNativeErrors {
            ShieldedTokenPoolNative.watch(managerHandle, walletId, contractId, tokenPosition)
        }
    }

    /**
     * Stop watching [tokenId]'s shielded pool for [walletId], forgetting the
     * wallet's notes and scan state there. Unwatching a pool not watched
     * changes nothing.
     */
    suspend fun shieldedTokenPoolUnwatch(walletId: ByteArray, tokenId: ByteArray): Unit =
        teardownGate.op {
            require(walletId.size == 32) { "walletId must be exactly 32 bytes, got ${walletId.size}" }
            require(tokenId.size == 32) { "tokenId must be exactly 32 bytes, got ${tokenId.size}" }
            mapNativeErrors { ShieldedTokenPoolNative.unwatch(managerHandle, walletId, tokenId) }
        }

    /**
     * [walletId]'s tokens in every shielded pool it watches, one row per
     * watched pool and bound account, in Rust's order, from the local scan
     * alone (no network read). Empty when the wallet watches no pool.
     */
    suspend fun shieldedTokenPoolBalances(walletId: ByteArray): List<ShieldedTokenPoolBalance> =
        teardownGate.op {
            require(walletId.size == 32) { "walletId must be exactly 32 bytes, got ${walletId.size}" }
            mapNativeErrors {
                ShieldedTokenPoolNative.balances(managerHandle, walletId)
                    .map { ShieldedTokenPoolBalance.fromNative(it) }
            }
        }

    /**
     * Send [amount] of the token at [tokenPosition] of [contractId] from the
     * notes of [account] in that token's shielded pool to the Orchard
     * address [recipientRaw43] in the same pool: the Type 26
     * `TokenShieldedTransferWithShieldedFee` transition. The change goes
     * back to the account and the fee is paid from the account's notes in
     * the credit pool. Rust checks the amount, the token, the network and
     * the note counts, selects both note sets, builds, broadcasts, records a
     * Token Transfer activity row (kind tag 11, amount in the token's
     * smallest unit) and requests a shielded sync.
     *
     * Returning means SUBMITTED, not confirmed: the transfer stays pending,
     * its notes reserved ([ShieldedTokenPoolBalance.reservedBalance]), until
     * a shielded sync confirms it. Do not show it as complete and do not
     * retry it. A caller cancelled while the native call runs still gets
     * this return once the call finished ([opReturningCompletedResult]), so
     * a submitted transfer is never reported as a cancellation. Errors are
     * read as for [shieldedTokenPurchaseFromPool]:
     * [DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed] (and the other
     * "result not known" errors) mean it may have gone through.
     *
     * @param amount tokens to send, in the token's smallest unit, exactly as
     *   the user typed them.
     * @param recipientRaw43 the recipient's raw 43-byte Orchard payment
     *   address, the shape [shieldedDefaultAddress] returns.
     */
    @Suppress("LongParameterList")
    suspend fun shieldedTokenTransfer(
        walletId: ByteArray,
        contractId: ByteArray,
        tokenPosition: Int,
        recipientRaw43: ByteArray,
        amount: ULong,
        account: Int = 0,
    ): Unit = teardownGate.opReturningCompletedResult {
        require(account >= 0) { "account must be non-negative, got $account" }
        require(walletId.size == 32) { "walletId must be exactly 32 bytes, got ${walletId.size}" }
        require(contractId.size == 32) {
            "contractId must be exactly 32 bytes, got ${contractId.size}"
        }
        require(recipientRaw43.size == 43) {
            "recipientRaw43 must be exactly 43 bytes, got ${recipientRaw43.size}"
        }
        requireTokenPosition(tokenPosition)
        mapNativeErrors {
            ShieldedTokenPoolNative.transfer(
                managerHandle,
                walletId,
                mnemonicResolver.nativeHandle,
                account,
                contractId,
                tokenPosition,
                recipientRaw43,
                amount.toLong(),
            )
        }
    }

    /**
     * Move [amount] of the token at [tokenPosition] of [contractId] from the
     * notes of [account] in that token's shielded pool into the token
     * balance of the existing identity [recipientIdentityId]: the Type 27
     * `TokenUnshieldWithShieldedFee` transition. The change goes back to the
     * account and the fee is paid from the account's credit pool notes.
     * Rust checks and selects as for [shieldedTokenTransfer] and records a
     * Token Unshield activity row (kind tag 12).
     *
     * Submitted, never confirmed, exactly as for [shieldedTokenTransfer]
     * (including its cancellation rule): do not retry it.
     */
    @Suppress("LongParameterList")
    suspend fun shieldedTokenUnshield(
        walletId: ByteArray,
        contractId: ByteArray,
        tokenPosition: Int,
        recipientIdentityId: ByteArray,
        amount: ULong,
        account: Int = 0,
    ): Unit = teardownGate.opReturningCompletedResult {
        require(account >= 0) { "account must be non-negative, got $account" }
        require(walletId.size == 32) { "walletId must be exactly 32 bytes, got ${walletId.size}" }
        require(contractId.size == 32) {
            "contractId must be exactly 32 bytes, got ${contractId.size}"
        }
        require(recipientIdentityId.size == 32) {
            "recipientIdentityId must be exactly 32 bytes, got ${recipientIdentityId.size}"
        }
        requireTokenPosition(tokenPosition)
        mapNativeErrors {
            ShieldedTokenPoolNative.unshield(
                managerHandle,
                walletId,
                mnemonicResolver.nativeHandle,
                account,
                contractId,
                tokenPosition,
                recipientIdentityId,
                amount.toLong(),
            )
        }
    }

    /**
     * What a token pool spend of [kind] moving [amount] would take from
     * [account]'s notes, chosen as the spend chooses them: the fee in
     * credits (paid from the credit pool) and the token and credit note
     * counts. Local state only: no network read, nothing reserved, no seed.
     * Throws Rust's refusal when the spend would be refused on these notes
     * (a zero amount, too few tokens, too few credits for the fee, too many
     * notes); a form shows that message in place of the fee.
     */
    suspend fun shieldedTokenSpendPreview(
        walletId: ByteArray,
        contractId: ByteArray,
        tokenPosition: Int,
        kind: ShieldedTokenPoolSpendKind,
        amount: ULong,
        account: Int = 0,
    ): ShieldedTokenSpendPreview = teardownGate.op {
        require(account >= 0) { "account must be non-negative, got $account" }
        require(walletId.size == 32) { "walletId must be exactly 32 bytes, got ${walletId.size}" }
        require(contractId.size == 32) {
            "contractId must be exactly 32 bytes, got ${contractId.size}"
        }
        requireTokenPosition(tokenPosition)
        ShieldedTokenSpendPreview.fromNative(
            mapNativeErrors {
                ShieldedTokenPoolNative.spendPreview(
                    managerHandle,
                    walletId,
                    account,
                    contractId,
                    tokenPosition,
                    kind.raw,
                    amount.toLong(),
                )
            },
        )
    }

    /**
     * Send a DashPay private contact request (protocol version 15) from
     * [senderIdentityId] to [recipientIdentityId], paying for it from
     * [account]'s shielded notes on [walletId], so nothing on chain names
     * either identity. Port of Swift's
     * `PlatformWalletManager.sendPrivateContactRequest`
     * (`packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PrivateContactRequests.swift`).
     *
     * The request is signed by this manager's identity signer
     * ([signerHandle], the handle the public `Dashpay.sendContactRequest`
     * is given) and sealed with DashPay keys derived through this manager's
     * resolver, which also supplies the Orchard spend authority. The
     * request carries an Orchard address the contact may pay us at
     * privately, which the wallet derives for this contact alone. Blocks
     * for the Halo 2 proof.
     *
     * Before anything is paid the wallet reads DashPay's private store for
     * the sender to its end. Throws
     * [DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed] when the payment
     * may already be on chain,
     * [DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed] when the
     * SDK could not confirm a transition it may have accepted, and
     * [DashSdkError.PlatformWallet.PrivateContactRequestPending] when the
     * pair is held for a private request: in each case do not send again.
     * [DashSdkError.PlatformWallet.PrivateContactStoreNotRead] sent nothing;
     * calling again continues the read.
     * [DashSdkError.PlatformWallet.PrivateContactPaymentsNotRestored] and
     * [DashSdkError.PlatformWallet.PrivateContactRecoveryUnsupported] refuse
     * before the read.
     *
     * @return the created request; close it (or `use {}`) when done.
     */
    suspend fun sendPrivateContactRequest(
        walletId: ByteArray,
        senderIdentityId: ByteArray,
        recipientIdentityId: ByteArray,
        accountLabel: String? = null,
        autoAcceptProof: ByteArray? = null,
        account: Int = 0,
    ): ContactRequestRef = teardownGate.op {
        require(account >= 0) { "account must be non-negative, got $account" }
        val handle = mapNativeErrors {
            DashpayNative.sendPrivateContactRequest(
                managerHandle,
                walletId,
                mnemonicResolver.nativeHandle,
                account,
                senderIdentityId,
                recipientIdentityId,
                accountLabel,
                autoAcceptProof,
                signerHandle,
            )
        }
        ContactRequestRef(handle)
    }

    /**
     * Accept a DashPay contact request that came privately, replying
     * privately and paying for the reply from [account]'s shielded notes on
     * [walletId]. Port of Swift's
     * `PlatformWalletManager.acceptPrivateContactRequest`
     * (`packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PrivateContactRequests.swift`).
     *
     * A host accepts with [acceptContactRequestReplyingInKind], which
     * picks this reply itself; this explicit accept stays for a host that
     * wants a private reply. The reply is signed by this manager's identity
     * signer ([signerHandle]). It carries an Orchard address the contact
     * may pay us at privately, which the wallet derives for this contact
     * alone. Blocks for the Halo 2 proof.
     *
     * @return the established contact; close it (or `use {}`) when done.
     */
    suspend fun acceptPrivateContactRequest(
        walletId: ByteArray,
        request: ContactRequestRef,
        account: Int = 0,
    ): EstablishedContactRef = teardownGate.op {
        require(account >= 0) { "account must be non-negative, got $account" }
        val handle = mapNativeErrors {
            DashpayNative.acceptPrivateContactRequest(
                managerHandle,
                walletId,
                mnemonicResolver.nativeHandle,
                account,
                request.value,
                signerHandle,
            )
        }
        EstablishedContactRef(handle)
    }

    /**
     * Accept a DashPay contact request replying the way it came: privately,
     * paid from [account]'s shielded notes on [walletId], when it came
     * privately and no reply exists yet; otherwise with a `contactRequest`
     * document, or by adopting the reply already sent. Which reply a request
     * takes is the wallet's decision, so this is the accept a host calls.
     * Port of Swift's `PlatformWalletManager.acceptContactRequestReplyingInKind`
     * (`packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PrivateContactRequests.swift`).
     *
     * [request] is the incoming request (`Dashpay.incomingContactRequest`).
     * The reply is signed by this manager's identity signer ([signerHandle]);
     * the Orchard spend authority comes from this manager's resolver and is
     * derived only for a private reply. A private reply carries an Orchard
     * address the contact may pay us at privately, which the wallet derives
     * for this contact alone.
     *
     * A public or adopted reply needs neither shielded support nor a
     * mnemonic read for the payment. Throws
     * [DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed] (or
     * [DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed]) when a
     * private reply may already be on chain, or
     * [DashSdkError.PlatformWallet.PrivateContactRequestPending] when the pair
     * is held for one: do not accept again;
     * [DashSdkError.PlatformWallet.PrivateContactStoreNotRead] when the
     * private store read every accept runs first did not finish (nothing
     * sent; calling again continues it); and
     * [DashSdkError.PlatformWallet.ShieldedBuildRequired] when the request
     * needs a private reply and the native library has no shielded support.
     *
     * @return the established contact; close it (or `use {}`) when done.
     */
    suspend fun acceptContactRequestReplyingInKind(
        walletId: ByteArray,
        request: ContactRequestRef,
        account: Int = 0,
    ): EstablishedContactRef = teardownGate.op {
        require(account >= 0) { "account must be non-negative, got $account" }
        val handle = mapNativeErrors {
            DashpayNative.acceptContactRequestReplyingInKind(
                managerHandle,
                walletId,
                mnemonicResolver.nativeHandle,
                account,
                request.value,
                signerHandle,
            )
        }
        EstablishedContactRef(handle)
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

        val key = walletId.toHex()
        // Pure delegation — the WHOLE guard sequence (storage read, status-key
        // derivation, stale-mismatch clear, early-return decision) lives in
        // [isGenuineWatchOnly] so `WatchOnlySeedMismatchTest` pins it without
        // the native library. Don't inline any of it back here: logic at this
        // call site is exactly what the unit tests cannot see.
        if (isGenuineWatchOnly(
                walletId = walletId,
                hasMnemonic = walletStorage::hasMnemonic,
                updateUnlockStatus = { statusKey, transform ->
                    updateUnlockStatus(statusKey, transform)
                },
            )
        ) {
            return false
        }
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

    // ── Ordered wallet bring-up ───────────────────────────────────────

    /**
     * Bring one wallet's DashPay state up in dependency order — identity →
     * contacts → contact-account drain — then return so the caller can
     * start Core SPV. Port of Swift's `startWalletSubsystems`
     * (`PlatformWalletManagerStartup.swift`); the ordering, the retry
     * policy and the budget all live Rust-side
     * (`platform_wallet::manager::startup`) — this is a thin bridge.
     *
     * A contact's DIP-15 payment addresses are derived from its contact
     * account, and an address the wallet is not watching when the
     * compact-filter scan passes its funding height produces no
     * transaction at all. Call this once per wallet load, immediately
     * before [startSpv], so the first filter set already covers them —
     * a restored wallet then needs no receival-payment rescan at all.
     *
     * Budget expiry is reported in the outcome, never thrown: Core sync is
     * the wallet's primary function and must not be held hostage to
     * Platform being slow. Start SPV regardless of the returned status;
     * inspect [WalletStartupOutcome.contactAccountsPending] for
     * diagnostics.
     *
     * Key material follows the drain's per-call contract: the mnemonic
     * resolver and identity signer are built for this call and closed when
     * it returns — Rust borrows and never retains them. An auth-gated
     * signing failure (identity keys are biometric-gated on Android)
     * leaves the affected entries queued; the recurring sweep self-heals.
     *
     * Throws only for a malformed request (bad wallet id, negative
     * arguments, unknown wallet, torn-down manager).
     *
     * @param walletId the 32-byte wallet id.
     * @param budgetSecs ceiling for the whole sequence in seconds; 0 = SDK
     *   default (20s). Never unbounded — this call gates Core SPV.
     * @param gapLimit identity-discovery gap limit; 0 = SDK default.
     */
    suspend fun startWalletSubsystems(
        walletId: ByteArray,
        budgetSecs: Long = 0,
        gapLimit: Int = 0,
    ): WalletStartupOutcome = teardownGate.op {
        require(walletId.size == 32) { "walletId must be 32 bytes, got ${walletId.size}" }
        require(budgetSecs >= 0) { "budgetSecs must be non-negative, got $budgetSecs" }
        require(gapLimit >= 0) { "gapLimit must be non-negative, got $gapLimit" }
        withContext(Dispatchers.IO) {
            // Each handle is guarded from the moment it exists: the signer's
            // constructor can throw (Keystore unlock, DAO access), and a single
            // try covering both would leak the resolver's native handle when it
            // does. Closed in reverse construction order.
            val startupResolver = MnemonicResolverAndPersister(walletStorage)
            try {
                val startupSigner =
                    KeystoreSigner(
                        walletStorage,
                        network,
                        biometricGate,
                        database.platformAddressDao(),
                    )
                try {
                    val blob = mapNativeErrors {
                        WalletManagerNative.startWalletSubsystems(
                            managerHandle,
                            walletId,
                            startupResolver.nativeHandle,
                            startupSigner.nativeHandle,
                            budgetSecs,
                            gapLimit,
                        )
                    }
                    val rawStatus = blob.firstOrNull()?.toInt()?.and(0xFF)
                    if (rawStatus != null && !WalletStartupStatus.isKnownRaw(rawStatus)) {
                        // Append-only ABI: a newer native library reported a status
                        // this build predates. decode() maps it to PARTIAL_NO_IDENTITY
                        // (Swift parity); say so once so the mismatch is visible.
                        android.util.Log.w(
                            "PlatformWalletManager",
                            "startWalletSubsystems: unknown WalletStartupStatus discriminant " +
                                "$rawStatus from the native library; treating as PARTIAL_NO_IDENTITY " +
                                "(update the Kotlin SDK to match the native build)",
                        )
                    }
                    WalletStartupOutcome.decode(blob)
                } finally {
                    runCatching { startupSigner.close() }
                }
            } finally {
                runCatching { startupResolver.close() }
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
        // The native emitter stops with the manager, so what it last sent
        // would otherwise stay on screen for good.
        synchronized(shieldedBalancesLock) {
            shieldedBalancesClosed = true
            _shieldedBalances.value = emptyMap()
        }
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
 * The genuine-watch-only guard of
 * [PlatformWalletManager.unlockWalletFromKeystore] — the COMPLETE sequence
 * the unlock runs ahead of the binding verify: probe mnemonic existence
 * through [hasMnemonic] ([WalletStorage.hasMnemonic] in production — an
 * existence check, no decrypt), and when nothing is stored clear any stale
 * `seedMismatch` under the wallet's hex status key BEFORE reporting
 * watch-only.
 *
 * A wallet with no stored mnemonic (imported by xpub, or one whose Keystore
 * entry was removed) is genuine watch-only. Reporting that MUST first clear
 * any stale `seedMismatch`: no mnemonic is not a mismatch, and a wallet whose
 * seed once failed to bind and whose Keystore entry was then deleted would
 * otherwise keep publishing the unlock banner forever for a seed that is no
 * longer there — nothing downstream of the early return can clear it.
 *
 * The ordering is the whole contract, which is why the WHOLE guard — the
 * storage read, the status-key derivation, the clearing transform, and the
 * early-return decision — lives here rather than inline. The call site in
 * [PlatformWalletManager.unlockWalletFromKeystore] is pure delegation, so
 * this function IS the call-site shape and `WatchOnlySeedMismatchTest` pins
 * it without the native library. (An earlier cut took a pre-computed
 * `hasMnemonic: Boolean` and a bare clear lambda, which left the real
 * read/clear/return sequence living untested at the call site.) Mirror of
 * the Swift `verifySeedBinding` watch-only arm
 * (PlatformWalletManager.swift:764-770).
 *
 * @param hasMnemonic the storage existence probe, invoked with [walletId].
 * @param updateUnlockStatus the manager's status-map updater, invoked with
 *   the wallet's hex status key and the `seedMismatch = false` transform.
 * @return true when the wallet is watch-only and the caller must report it as
 *   such; false when a mnemonic exists and the binding verify must run —
 *   this path must not touch `seedMismatch` (the verify publishes the real
 *   result).
 */
internal suspend fun isGenuineWatchOnly(
    walletId: ByteArray,
    hasMnemonic: suspend (walletId: ByteArray) -> Boolean,
    updateUnlockStatus: (key: String, transform: (DashPayUnlockStatus) -> DashPayUnlockStatus) -> Unit,
): Boolean {
    if (hasMnemonic(walletId)) return false
    updateUnlockStatus(walletId.toHex()) { it.copy(seedMismatch = false) }
    return true
}

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
