import Foundation
import SwiftData
import Combine
import DashSDKFFI
import os.log

/// Lock-guarded monotonic generation counter, safe to read and bump from
/// any thread. Used to drop sync completion events that belong to a
/// generation already superseded by a `stop`/`clear`/`reset`, even when a
/// restart happens in the same `@MainActor` turn (a plain boolean gate
/// can't, because the restart re-opens the gate before the stale,
/// previously-enqueued completion task runs). Shared by the shielded,
/// platform-address, and DPNS sync paths.
final class SyncGenerationCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var value: UInt64 = 0
    func current() -> UInt64 { lock.withLock { value } }
    @discardableResult func bump() -> UInt64 { lock.withLock { value &+= 1; return value } }
}

/// Per-wallet DashPay "needs unlock / verify failed" status, surfaced for the
/// UI. One coherent snapshot per wallet (not parallel dictionaries) so a banner
/// is a pure function of one `Equatable` value.
public struct DashPayUnlockStatus: Equatable {
    /// Count of deferred account-build contact-crypto ops waiting for a signer
    /// unlock to finish payment-account setup. A wallet-scoped upper bound
    /// (aggregates the wallet's identities; may include ops that resolve to
    /// channel-broken on the next drain) — phrase it as "waiting," not "will
    /// succeed." Polled from `platform_wallet_pending_contact_crypto_count`.
    public var pendingAccountBuilds: UInt32 = 0
    /// The Keychain-resolved seed does not bind to this wallet (Rust
    /// `SeedMismatch`): DashPay signing is disabled until the mapping is fixed.
    /// Set from the verify FFI result at unlock.
    public var seedMismatch: Bool = false
    /// A deferred-crypto drain is in flight. Drives a "finishing…" state and
    /// disables a second Unlock tap so the banner can't kick a concurrent drain.
    public var draining: Bool = false

    public init(pendingAccountBuilds: UInt32 = 0, seedMismatch: Bool = false, draining: Bool = false) {
        self.pendingAccountBuilds = pendingAccountBuilds
        self.seedMismatch = seedMismatch
        self.draining = draining
    }

    /// Whether anything is worth showing the user (a banner host can early-out).
    public var hasSignal: Bool { seedMismatch || draining || pendingAccountBuilds > 0 }
}

/// Effective persistence contract reported by the native manager after it
/// intersects the host declaration with the callbacks actually installed.
public struct PlatformWalletPersistenceCapabilities: Equatable, Sendable {
    public static let version1: UInt32 = 1
    public static let atomicChangesets: UInt64 = 1 << 0
    public static let invitations: UInt64 = 1 << 1
    public static let assetLockFundingIndices: UInt64 = 1 << 2
    public static let shieldedViewingKeys: UInt64 = 1 << 3
    public static let providerTransactions: UInt64 = 1 << 4
    public static let unsignedTokenStorage: UInt64 = 1 << 5
    public static let pendingContactCrypto: UInt64 = 1 << 6
    public static let walletRestore: UInt64 = 1 << 7
    /// DPNS username-marketplace name-state rows (price / sale status /
    /// counterparty) are mirrored durably. Mirrors
    /// `PersistenceCapabilities::DPNS_NAME_STATES`.
    public static let dpnsNameStates: UInt64 = 1 << 8
    /// Tracked asset-lock rows, including status and proof updates, can be
    /// persisted. Restart hydration is separately attested by `walletRestore`.
    public static let trackedAssetLocks: UInt64 = 1 << 9
    /// Tracked (wallet-independent) masternodes are persisted and restored
    /// across restarts. Mirrors
    /// `PersistenceCapabilities::TRACKED_MASTERNODES`.
    public static let trackedMasternodes: UInt64 = 1 << 10

    public let version: UInt32
    public let bits: UInt64

    public init(version: UInt32 = 0, bits: UInt64 = 0) {
        self.version = version
        self.bits = bits
    }

    public func contains(_ capability: UInt64) -> Bool {
        bits & capability == capability
    }
}

/// Timing + FFI-result record of one native manager teardown, returned by
/// [`PlatformWalletManager/shutdown()`] so the host can log it (the SDK has
/// no dependency on any app-side logger).
///
/// Deliberately NOT a worker-level shutdown report: the Rust FFI returns
/// `Success` for a live handle even when a worker missed its join budget
/// (that outcome is a Rust-side WARN, not an error), so Swift cannot know
/// clean-vs-timed-out per worker. These metrics report only what Swift
/// observes — the result code and wall time of each FFI call, and the
/// thread the teardown ran on.
public struct PlatformWalletShutdownMetrics: Sendable {
    public struct Step: Sendable {
        /// FFI entry point, e.g. "spv_stop", "destroy".
        public let name: String
        /// Raw `PlatformWalletResultCode` of the call (0 = success).
        public let ffiCode: Int32
        public let milliseconds: Int

        public init(name: String, ffiCode: Int32, milliseconds: Int) {
            self.name = name
            self.ffiCode = ffiCode
            self.milliseconds = milliseconds
        }
    }

    /// The six teardown calls in execution order (5× sync stop + destroy).
    public let steps: [Step]
    public let totalMilliseconds: Int
    /// Whether the blocking native teardown ran off the main thread. The
    /// whole point of `shutdown()` is that this is `true`.
    public let ranOffMainThread: Bool

    public init(steps: [Step], totalMilliseconds: Int, ranOffMainThread: Bool) {
        self.steps = steps
        self.totalMilliseconds = totalMilliseconds
        self.ranOffMainThread = ranOffMainThread
    }
}

/// Native entry points used by the production teardown orchestration.
///
/// Keeping the functions injectable as a group lets tests exercise the real
/// ordering, timing, result mapping, and logging in
/// [`PlatformWalletManager.performNativeTeardown`] without calling Rust. The
/// unchecked conformance is intentional: these closures are immutable C-entry
/// wrappers, and the whole value is copied onto the dedicated destroy queue.
struct PlatformWalletNativeTeardownCalls: @unchecked Sendable {
    typealias Call = @Sendable (Handle) -> PlatformWalletFFIResult

    let spvStop: Call
    let platformAddressSyncStop: Call
    let shieldedSyncStop: Call
    let dashPaySyncStop: Call
    let dpnsSyncStop: Call
    let destroy: Call

    static let live = PlatformWalletNativeTeardownCalls(
        spvStop: platform_wallet_manager_spv_stop,
        platformAddressSyncStop: platform_wallet_manager_platform_address_sync_stop,
        shieldedSyncStop: platform_wallet_manager_shielded_sync_stop,
        dashPaySyncStop: platform_wallet_manager_dashpay_sync_stop,
        dpnsSyncStop: platform_wallet_manager_dpns_sync_stop,
        destroy: platform_wallet_manager_destroy
    )
}

/// The one thing SwiftUI needs for all wallet operations.
///
/// Owns the Rust-side `PlatformWalletManager` handle which drives:
/// - Wallet creation from mnemonic/seed
/// - SPV sync (core chain: headers, filters, masternodes)
/// - BLAST address balance sync
/// - Identity, DashPay, asset lock, token-balance tracking
/// - Persistence via SwiftData callbacks
///
/// Use as a root `@StateObject` and pass via `.environmentObject(_:)`.
/// Views observe `@Published` properties directly — no coordinator
/// class in the middle.
@MainActor
public class PlatformWalletManager: ObservableObject {
    fileprivate nonisolated static let log = Logger(
        subsystem: "dashpay.SwiftDashSDK",
        category: "PlatformWallet"
    )

    // MARK: - Published observables

    /// Whether [`configure`] has been called successfully.
    @Published public private(set) var isConfigured: Bool = false

    /// Initialization diagnostic for the effective persistence backend.
    @Published public private(set) var persistenceCapabilities =
        PlatformWalletPersistenceCapabilities()

    /// The current SPV sync progress. Updated by the polling task
    /// started in [`configure`].
    @Published public private(set) var spvProgress: PlatformSpvSyncProgress = .empty

    @Published public private(set) var spvIsRunning: Bool = false

    /// The peers the SPV client is currently connected to, each
    /// classified against the masternode list. Empty while SPV isn't
    /// running. Updated by the polling task started in [`configure`].
    @Published public private(set) var spvPeers: [PlatformSpvPeerInfo] = []

    /// Block time of the SPV header storage's current tip (if any).
    /// `nil` while SPV isn't running or hasn't stored a header yet.
    /// Useful as a "is core producing blocks?" indicator — when this
    /// stamp stops advancing, the chain is stalled even though the
    /// local SPV client is healthy.
    @Published public private(set) var spvTipBlockTime: Date?

    /// Whether the Rust-owned platform-address sync manager is currently in flight.
    @Published public private(set) var platformAddressSyncIsSyncing: Bool = false

    /// Last completed platform-address sync event emitted by Rust.
    @Published public internal(set) var lastPlatformAddressSyncEvent: PlatformAddressSyncEvent?

    /// Whether the Rust-owned shielded sync coordinator currently has
    /// a pass in flight.
    @Published public private(set) var shieldedSyncIsSyncing: Bool = false

    /// Whether the Rust-owned DashPay sync coordinator currently has
    /// a pass in flight. The single sync-in-progress signal: all
    /// three DashPay sync callers (`.task`, pull-to-refresh, the
    /// background loop) observe this one flag, and a pull-to-refresh
    /// during an in-flight sync attaches to it instead of
    /// double-firing. Updated by the polling task started in
    /// [`configure`]. Named after the `shieldedSyncIsSyncing` /
    /// `platformAddressSyncIsSyncing` mirrors (the natural
    /// `isDashPaySyncing` would collide with the wrapper method of
    /// that name).
    @Published public private(set) var dashPaySyncIsSyncing: Bool = false

    /// Last completed shielded sync event emitted by Rust.
    @Published public internal(set) var lastShieldedSyncEvent: ShieldedSyncEvent?

    /// Last completed cross-wallet DPNS marketplace sync event emitted by
    /// Rust. Every native pointer has been copied before publication.
    @Published public internal(set) var lastDpnsSyncEvent: DpnsSyncEvent?

    /// Cumulative number of encrypted notes scanned in the **current**
    /// in-flight shielded sync pass, published once per chunk (~every
    /// 2048 notes) via the Rust-side progress callback. Nil between
    /// passes. Lets UI render a live counter / `ProgressView` during
    /// the cold sync of a large pool (1M notes can take 20+ min in a
    /// single SDK call; without this there's no signal between start
    /// and end).
    ///
    /// Paired with `currentShieldedSyncBlockHeight` — emitted in the
    /// same callback. They update together; the chain-tip number lets
    /// the UI estimate "still N blocks behind".
    @Published public internal(set) var currentShieldedSyncScanned: UInt64?
    @Published public internal(set) var currentShieldedSyncBlockHeight: UInt64?

    /// Cumulative count of note commitments appended to the local
    /// Orchard commitment tree in the **current** in-flight shielded
    /// sync pass — the "checked / committed-to-tree" signal, distinct
    /// from `currentShieldedSyncScanned` (which counts *downloaded*
    /// notes). Published once per committed batch via the Rust-side
    /// tree-progress callback. Nil between passes.
    ///
    /// Paired with `currentShieldedTreeTotal` — emitted in the same
    /// callback. `currentShieldedTreeTotal == 0` (or nil) means the
    /// total is indeterminate; the UI should render a spinner rather
    /// than a determinate bar.
    @Published public internal(set) var currentShieldedTreeCommitted: UInt64?
    @Published public internal(set) var currentShieldedTreeTotal: UInt64?

    /// Monotonic generation for shielded sync passes. Each `stop`/`clear`
    /// bumps it; the FFI completion callback snapshots the generation at
    /// enqueue time and `handleShieldedSyncCompleted` drops any event whose
    /// snapshot no longer matches the current generation.
    ///
    /// The Rust quiesce barrier guarantees no persistence after stop/clear,
    /// but the completion callback is re-dispatched onto this `@MainActor`,
    /// so a final, already-dispatched event can land just after stop/clear
    /// returns. A plain boolean gate is bypassable: a caller can stop (set
    /// the flag) and restart (clear the flag) in the same actor turn, which
    /// re-opens the gate before the stale, previously-enqueued completion
    /// task runs — so the old event leaks into the new run. Tying
    /// suppression to a generation closes that race: the stale task carries
    /// the pre-stop generation, the restart does not reset the counter, so
    /// the snapshot mismatches and the event is dropped even on a same-turn
    /// restart.
    ///
    /// `nonisolated` + lock-guarded so the FFI callback thread can snapshot
    /// it without hopping onto the main actor first.
    nonisolated let shieldedSyncGeneration = SyncGenerationCounter()

    /// Generation guard for platform-address (BLAST/DIP-17) sync
    /// completion events, mirroring [`shieldedSyncGeneration`]. The FFI
    /// completion callback snapshots this on its own thread before the
    /// main-actor hop; `stopPlatformAddressSync` / `resetPlatformAddressSyncState`
    /// bump it so a trailing completion the main actor delivers *after*
    /// the stop/reset is dropped instead of repainting the just-cleared
    /// sync-status UI.
    nonisolated let platformAddressSyncGeneration = SyncGenerationCounter()

    /// Generation guard for DPNS marketplace completion events. A native
    /// completion can already be queued for the main actor when shutdown
    /// begins; bumping this generation when the handle is consumed prevents
    /// that trailing callback from publishing after the manager is stopped.
    nonisolated let dpnsSyncGeneration = SyncGenerationCounter()

    /// All wallets currently held by the Rust-side
    /// `PlatformWalletManager`, keyed by the 32-byte wallet id.
    ///
    /// The Rust manager holds N wallets concurrently — BLAST sync
    /// iterates every wallet in this map. Views should look up the
    /// wallet they care about via [`wallet(for:)`] rather than
    /// assuming a single "active" wallet.
    @Published public private(set) var wallets: [Data: ManagedPlatformWallet] = [:]

    /// Per-wallet DashPay needs-unlock / verify-failed status, keyed by the
    /// 32-byte wallet id. `pendingAccountBuilds` is refreshed by the progress
    /// poller; `seedMismatch` and `draining` are set at the unlock / drain call
    /// sites. Keys are pruned when a wallet leaves [`wallets`] (and explicitly
    /// on [`deleteWallet`]) so a re-created wallet with the same deterministic
    /// id can't inherit a stale banner.
    @Published public private(set) var dashPayUnlockStatus: [Data: DashPayUnlockStatus] = [:]

    /// Last error from a wallet operation, if any. Cleared on successful op.
    @Published public private(set) var lastError: Error?

    /// Internal seam so manager extensions in other files can record a
    /// failure (`lastError`'s setter is file-private).
    func recordLastError(_ error: Error) {
        lastError = error
    }

    // MARK: - Internals

    /// FFI handle; `NULL_HANDLE` until [`configure`] is called.
    internal private(set) var handle: Handle = NULL_HANDLE

    /// Convenience access for Swift-side callers (e.g. `persistence`).
    /// Lifetime for the FFI callback context is NOT this reference's job:
    /// Rust holds its own retained reference (transferred at `configure`)
    /// and releases it when its last worker drops.
    private var persistenceHandler: PlatformWalletPersistenceHandler?

    /// SwiftData container + network captured at `configure`, used to build a
    /// `KeychainSigner` (the identity document signer) for the unlock-time
    /// auto-accept drain. Nil when configured without persistence (no Keychain
    /// signing possible → the drain runs provider-only).
    /// `internal`, not `private`: the ordered bring-up in
    /// `PlatformWalletManagerStartup` builds the same signer for the same
    /// drain, and reusing these beats duplicating the construction there.
    var modelContainer: ModelContainer?
    var signerNetwork: Network?

    /// Convenience reference; the FFI callback context's lifetime is
    /// owned by Rust (retained reference transferred at `configure`,
    /// released when its last worker drops), not by this property.
    private var eventHandler: PlatformWalletEventHandler?

    /// Background task that polls SPV progress.
    private var progressPollTask: Task<Void, Never>?

    /// The single in-flight (or completed) [`shutdown()`] operation. Set
    /// exactly once by the first caller that takes a live handle; later
    /// callers await the same task and receive the same metrics. MainActor
    /// isolation serializes the check-and-set (no suspension point between
    /// them), so no lock is needed. A shutdown before configuration remains
    /// an uncached no-op because the manager can still be configured later.
    private var shutdownTask: Task<PlatformWalletShutdownMetrics, Never>?

    /// Test seam for the individual native calls. Production keeps `.live`;
    /// tests replace the function table while still running the production
    /// teardown orchestration end-to-end.
    internal var nativeTeardownCalls = PlatformWalletNativeTeardownCalls.live

    /// Dedicated serial queue for the blocking native teardown. The Rust
    /// `destroy` runs `block_on(shutdown())` on the calling thread and can
    /// legitimately take tens of seconds when an in-flight sync pass ignores
    /// cancellation, so it must park a plain GCD thread — never the main
    /// thread, and never a Swift Concurrency cooperative-pool thread (which
    /// is why this is a DispatchQueue and not `Task.detached`).
    nonisolated static let destroyQueue = DispatchQueue(
        label: "org.dash.platform-wallet.destroy",
        qos: .userInitiated
    )

    // MARK: - Init

    /// Empty init for `@StateObject` usage. Call [`configure`] before
    /// any wallet operations.
    public init() {}

    /// Convenience: create and configure in one call.
    public convenience init(sdk: SDK, modelContainer: ModelContainer? = nil) throws {
        self.init()
        try self.configure(sdk: sdk, modelContainer: modelContainer)
    }

    deinit {
        progressPollTask?.cancel()
        // Emergency fallback ONLY. The supported teardown path is an explicit
        // `await shutdown()` before dropping the last reference — it takes the
        // handle exactly once and runs the blocking native teardown off-main
        // deterministically. Reaching this branch means a code path dropped a
        // configured manager without shutting it down; schedule the same
        // teardown fire-and-forget on the dedicated queue rather than
        // blocking whatever thread ARC happens to release on (the destroy's
        // Rust-side `block_on(shutdown())` can take tens of seconds when a
        // sync pass is wedged — running it inline here is the historical
        // network-switch UI freeze).
        //
        // Safe without `self`: `Handle` is a registry key from a monotonic
        // counter (never reused), destroying an already-removed handle is an
        // Ok no-op, and Rust owns the persistence/event callback contexts
        // (handed over retained at `configure`, with a `release_fn`) — a
        // straggling worker keeps its handler alive through that retain and
        // Rust releases it when the worker exits.
        if handle != NULL_HANDLE {
            let h = handle
            let calls = nativeTeardownCalls
            Self.log.warning(
                "PlatformWalletManager deallocated without shutdown(); scheduling fallback native teardown off-main for handle \(h, privacy: .public)"
            )
            Self.destroyQueue.async {
                _ = Self.performNativeTeardown(h, calls: calls)
            }
        }
    }

    // MARK: - Shutdown

    /// Tear down the native manager without blocking the main thread.
    ///
    /// Takes ownership of the FFI handle exactly once on the main actor
    /// (zeroing [`handle`] and flipping [`isConfigured`] so every later
    /// operation fails fast through `ensureConfigured()`), then runs the
    /// full native teardown — the same five sync stops plus
    /// `platform_wallet_manager_destroy` the old `deinit` performed, in the
    /// same order — on [`destroyQueue`]. The Rust destroy `block_on`s its
    /// bounded lifecycle shutdown on that queue's thread, which can take
    /// tens of seconds when an in-flight sync pass ignores cancellation;
    /// the caller awaits a continuation instead of blocking.
    ///
    /// Idempotent: the first caller starts the teardown, every later caller
    /// awaits the same task and receives the same metrics. Cancellation of
    /// a calling task does not interrupt the teardown (`Task<_, Never>.value`
    /// is a non-throwing await), so the native teardown always runs to
    /// completion once started.
    ///
    /// Never throws by design: `platform_wallet_manager_destroy` returns
    /// `Success` for a live handle even when a worker misses its join budget
    /// (a Rust-side WARN, not an error), so a thrown error would carry no
    /// actionable signal. Per-step FFI codes travel in the returned metrics
    /// for the host to log.
    @discardableResult
    public func shutdown() async -> PlatformWalletShutdownMetrics {
        if let task = shutdownTask {
            return await task.value
        }
        guard handle != NULL_HANDLE else {
            // Never configured (or a test double without a handle): nothing
            // to tear down. Do not cache this no-op: a manager may still be
            // configured later, and that live handle must then be torn down.
            return PlatformWalletShutdownMetrics(
                steps: [],
                totalMilliseconds: 0,
                ranOffMainThread: false)
        }

        // Take-once: from this point every FFI entry gated on
        // `ensureConfigured()` / `handle != NULL_HANDLE` rejects cleanly,
        // and the generation bumps drop any trailing sync event the main
        // actor delivers after this turn.
        let h = handle
        handle = NULL_HANDLE
        isConfigured = false
        progressPollTask?.cancel()
        shieldedSyncGeneration.bump()
        platformAddressSyncGeneration.bump()
        dpnsSyncGeneration.bump()

        let calls = nativeTeardownCalls
        let task = Task {
            await withCheckedContinuation { (continuation: CheckedContinuation<PlatformWalletShutdownMetrics, Never>) in
                Self.destroyQueue.async {
                    continuation.resume(returning: Self.performNativeTeardown(h, calls: calls))
                }
            }
        }
        shutdownTask = task
        return await task.value
    }

    /// The blocking native teardown body, shared by [`shutdown()`] and the
    /// `deinit` fallback: exactly the five sync stops plus destroy the old
    /// synchronous `deinit` ran, in the same order, each timed and its FFI
    /// code recorded.
    ///
    /// The stops are kept even though Rust's `shutdown()` inside destroy is
    /// a superset — they preserve the historical teardown order as defense
    /// in depth (`spv_stop` is itself a blocking, abort-escalating join, so
    /// it too must run on this queue, never the main thread). Rust's destroy
    /// path provides the authoritative join barrier.
    nonisolated static func performNativeTeardown(
        _ handle: Handle,
        calls: PlatformWalletNativeTeardownCalls = .live
    ) -> PlatformWalletShutdownMetrics {
        let offMain = !Thread.isMainThread
        let totalStart = CFAbsoluteTimeGetCurrent()
        var steps: [PlatformWalletShutdownMetrics.Step] = []
        steps.reserveCapacity(6)

        func run(_ name: String, _ call: (Handle) -> PlatformWalletFFIResult) {
            let start = CFAbsoluteTimeGetCurrent()
            let result = PlatformWalletResult(call(handle))
            let ms = Int((CFAbsoluteTimeGetCurrent() - start) * 1000)
            steps.append(.init(name: name, ffiCode: result.code.rawValue, milliseconds: ms))
            if !result.isSuccess {
                Self.log.error(
                    "native teardown step \(name, privacy: .public) failed with \(String(describing: result.code), privacy: .public): \(result.message ?? "<no detail from Rust>", privacy: .public)"
                )
            }
        }

        // Stop the network event source first as defense in depth for the
        // teardown order; Rust's destroy path provides the authoritative
        // join barrier.
        run("spv_stop", calls.spvStop)
        run("platform_address_sync_stop", calls.platformAddressSyncStop)
        run("shielded_sync_stop", calls.shieldedSyncStop)
        run("dashpay_sync_stop", calls.dashPaySyncStop)
        run("dpns_sync_stop", calls.dpnsSyncStop)
        // Rust OWNS the persistence/event callback handlers (handed over
        // retained at `configure`, with a `release_fn`): any worker that
        // outlives destroy keeps its handler alive through that retain and
        // Rust releases it when the worker exits. Nothing to leak, retain,
        // or gate on here.
        run("destroy", calls.destroy)

        let metrics = PlatformWalletShutdownMetrics(
            steps: steps,
            totalMilliseconds: Int((CFAbsoluteTimeGetCurrent() - totalStart) * 1000),
            ranOffMainThread: offMain
        )
        let stepSummary = steps
            .map { "\($0.name)=\($0.milliseconds)ms(code \($0.ffiCode))" }
            .joined(separator: " ")
        Self.log.info(
            "native teardown finished in \(metrics.totalMilliseconds, privacy: .public)ms offMain=\(offMain, privacy: .public): \(stepSummary, privacy: .public)"
        )
        return metrics
    }

    /// Test-only factory: a manager carrying a fake non-null handle and an
    /// injected native-call table, so shutdown tests exercise the real
    /// take-once / exactly-once / idempotency and teardown-orchestration paths
    /// without calling FFI.
    /// Internal on purpose — never call from production code (`configure`
    /// is the only production path that assigns a handle).
    static func makeForTesting(
        handle: Handle,
        calls: PlatformWalletNativeTeardownCalls
    ) -> PlatformWalletManager {
        let manager = PlatformWalletManager()
        try! manager.configureForTesting(handle: handle, calls: calls)
        return manager
    }

    /// Test-only equivalent of a successful native configuration. Keeping it
    /// separate from the factory lets tests cover shutdown-before-configure.
    func configureForTesting(
        handle: Handle,
        calls: PlatformWalletNativeTeardownCalls
    ) throws {
        try ensureConfigurationAllowed()
        precondition(handle != NULL_HANDLE)
        self.handle = handle
        isConfigured = true
        nativeTeardownCalls = calls
    }

    // MARK: - Configuration

    /// Configure the manager with an SDK and an optional SwiftData
    /// container. Must be called before any wallet operations.
    ///
    /// Spawns a background task that polls SPV sync progress every
    /// second and publishes it to [`spvProgress`].
    public func configure(sdk: SDK, modelContainer: ModelContainer? = nil) throws {
        try ensureConfigurationAllowed()
        guard let sdkHandle = sdk.handle else {
            throw PlatformWalletError.invalidParameter("SDK has no handle")
        }
        guard let innerSdkPtr = dash_sdk_get_inner_sdk_ptr(sdkHandle) else {
            throw PlatformWalletError.invalidParameter(
                "dash_sdk_get_inner_sdk_ptr returned NULL for the supplied SDK"
            )
        }
        // The Rust manager is network-locked at construction
        // (`WalletManager::new(sdk.network)`); thread that same
        // network through to the persistence handler so its
        // `loadWalletList` only restores wallets bound to this
        // network, matching the per-network manager design.
        try configure(
            sdkPointer: UnsafeRawPointer(innerSdkPtr),
            modelContainer: modelContainer,
            network: sdk.network
        )
    }

    /// Configure with a raw Sdk pointer (advanced usage).
    public func configure(
        sdkPointer: UnsafeRawPointer,
        modelContainer: ModelContainer? = nil,
        network: Network? = nil
    ) throws {
        try ensureConfigurationAllowed()
        var handle: Handle = NULL_HANDLE

        let handler: PlatformWalletPersistenceHandler?
        var persistence: PersistenceCallbacks
        var declaredCapabilities: PersistenceCapabilitiesFFI
        var persistenceExtension: PersistenceCallbacksExtension
        if let container = modelContainer {
            let h = PlatformWalletPersistenceHandler(
                modelContainer: container,
                network: network
            )
            persistence = h.makeCallbacks()
            declaredCapabilities = h.makePersistenceCapabilities()
            persistenceExtension = h.makePersistenceCallbacksExtension()
            handler = h
        } else {
            persistence = PersistenceCallbacks()
            declaredCapabilities = PersistenceCapabilitiesFFI(
                version: 0,
                reserved: 0,
                bits: 0
            )
            persistenceExtension = PersistenceCallbacksExtension()
            persistenceExtension.struct_size = UInt(MemoryLayout<PersistenceCallbacksExtension>.size)
            persistenceExtension.version = UInt32(PLATFORM_WALLET_PERSISTENCE_CALLBACKS_EXTENSION_VERSION)
            persistenceExtension.reserved = 0
            handler = nil
        }

        let eventHandler = PlatformWalletEventHandler(manager: self)
        var eventHandlerCallbacks = eventHandler.makeCallbacks()
        var eventHandlerExtension = eventHandler.makeCallbacksExtension()

        do {
            try platform_wallet_manager_create_with_extensions(
                sdkPointer,
                &persistence,
                &eventHandlerCallbacks,
                &declaredCapabilities,
                &persistenceExtension,
                &eventHandlerExtension,
                &handle
            ).check()
        } catch {
            // A failed create never took ownership of the retained callback
            // contexts (`makeCallbacks` pre-retains for the transfer), so
            // balance the retains here or the handlers leak.
            if let context = persistence.context {
                Unmanaged<PlatformWalletPersistenceHandler>.fromOpaque(context).release()
            }
            if let context = eventHandlerCallbacks.context {
                Unmanaged<PlatformWalletEventHandler>.fromOpaque(context).release()
            }
            throw error
        }

        var effectiveCapabilities = PersistenceCapabilitiesFFI(
            version: 0,
            reserved: 0,
            bits: 0
        )
        do {
            try platform_wallet_manager_persistence_capabilities(
                handle,
                &effectiveCapabilities
            ).check()
        } catch {
            _ = platform_wallet_manager_destroy(handle)
            throw error
        }

        self.handle = handle
        self.persistenceHandler = handler
        self.eventHandler = eventHandler
        self.modelContainer = modelContainer
        self.persistenceCapabilities = PlatformWalletPersistenceCapabilities(
            version: effectiveCapabilities.version,
            bits: effectiveCapabilities.bits
        )
        self.signerNetwork = network
        self.isConfigured = true

        startProgressPolling()
    }

    /// A manager owns at most one configured native lifetime. A no-op shutdown
    /// before first configuration is allowed, but once a live handle has been
    /// consumed its cached shutdown result makes the instance terminal; a new
    /// native handle must be owned by a new manager.
    private func ensureConfigurationAllowed() throws {
        precondition(!isConfigured, "PlatformWalletManager already configured")
        guard shutdownTask == nil else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager cannot be configured after shutdown"
            )
        }
    }

    /// Access the persistence handler for loading cached data.
    public var persistence: PlatformWalletPersistenceHandler? {
        persistenceHandler
    }

    // MARK: - Wallet creation

    /// Create a wallet from a BIP39 mnemonic phrase (English).
    ///
    /// Stores the returned wallet as the active [`wallet`] published
    /// property. If `name` is provided, writes it onto the persisted
    /// [`PersistentWallet`] row so the wallet detail view has a
    /// user-facing label.
    ///
    /// `birthHeight` controls the SPV historical-scan window. Pass `nil` for a
    /// freshly **generated** mnemonic — the scan starts at the current chain tip
    /// (nothing was funded before now). Pass `0` when **importing/restoring an
    /// existing** mnemonic, so the wallet scans from genesis and sees funds
    /// (including DashPay payments) received before this device knew the wallet;
    /// without it, history — and the coreHeight rescan backfill — is clamped to
    /// the tip. `Some(h)` pins a known funding height.
    @discardableResult
    public func createWallet(
        mnemonic: String,
        network: Network,
        name: String? = nil,
        createDefaultAccounts: Bool = true,
        birthHeight: UInt32? = nil
    ) throws -> ManagedPlatformWallet {
        try ensureConfigured()
        var walletHandle: Handle = NULL_HANDLE
        var walletId: FFIByteTuple32 =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)

        let accountOptions: UInt32 = createDefaultAccounts ? 1 : 0

        try mnemonic.withCString { mnemonicPtr in
            try platform_wallet_manager_create_wallet_from_mnemonic_with_birth_height(
                handle,
                mnemonicPtr,
                network.ffiValue,
                accountOptions,
                birthHeight != nil,
                birthHeight ?? 0,
                &walletHandle,
                &walletId
            ).check()
        }

        let idData = withUnsafeBytes(of: &walletId) { Data($0) }
        if let name = name, !name.isEmpty {
            persistenceHandler?.setWalletName(walletId: idData, name: name)
        }
        let w = ManagedPlatformWallet(handle: walletHandle, walletId: idData)
        self.wallets[idData] = w
        return w
    }

    /// Create a wallet from raw 64-byte seed bytes.
    ///
    /// See `createWallet(mnemonic:...)` for the `birthHeight` semantics: `nil`
    /// scans from the current tip (fresh wallet), `0` scans from genesis
    /// (imported/restored wallet that may have prior on-chain history).
    @discardableResult
    public func createWallet(
        seed: Data,
        network: Network,
        name: String? = nil,
        createDefaultAccounts: Bool = true,
        birthHeight: UInt32? = nil
    ) throws -> ManagedPlatformWallet {
        try ensureConfigured()
        guard seed.count == 64 else {
            throw PlatformWalletError.invalidParameter(
                "seed must be 64 bytes, got \(seed.count)"
            )
        }

        var walletHandle: Handle = NULL_HANDLE
        var walletId: FFIByteTuple32 =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)

        let accountOptions: UInt32 = createDefaultAccounts ? 1 : 0

        try seed.withUnsafeBytes { seedPtr in
            try platform_wallet_manager_create_wallet_from_seed_with_birth_height(
                handle,
                network.ffiValue,
                seedPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                UInt(seed.count),
                accountOptions,
                birthHeight != nil,
                birthHeight ?? 0,
                &walletHandle,
                &walletId
            ).check()
        }

        let idData = withUnsafeBytes(of: &walletId) { Data($0) }
        if let name = name, !name.isEmpty {
            persistenceHandler?.setWalletName(walletId: idData, name: name)
        }
        let w = ManagedPlatformWallet(handle: walletHandle, walletId: idData)
        self.wallets[idData] = w
        return w
    }

    // MARK: - Watch-only restore from persister

    /// Rehydrate wallets from SwiftData on app launch.
    ///
    /// Calls `platform_wallet_manager_load_from_persistor` which fires
    /// the Swift-side `on_load_wallet_list_fn` callback. For each
    /// persisted wallet, Rust reconstructs an **external-signable**
    /// (watch-only, no key material) `Wallet` plus the wallet's
    /// persisted platform-address sync snapshot. After the FFI returns,
    /// we call `platform_wallet_manager_get_wallet` for each restored id
    /// so Swift gets a `ManagedPlatformWallet` handle.
    ///
    /// Each restored wallet then runs the seedless unlock via
    /// [`unlockWalletFromKeychain`](Self/unlockWalletFromKeychain(_:)): it
    /// verifies the Keychain-resolved seed binds to the wallet (refusing a
    /// mis-mapped slot) and drains any contact-crypto deferred while the
    /// wallet was seedless — the seed never becomes resident; signing runs
    /// through the resolver. Wallets with no stored mnemonic (genuine
    /// watch-only) stay watch-only — the unlock is best-effort per
    /// wallet and never fails the restore.
    ///
    /// Idempotent: if there's no persisted state, does nothing and
    /// leaves `self.wallets` untouched. Safe to call before any
    /// `createWallet` flow.
    @discardableResult
    public func loadFromPersistor() throws -> [ManagedPlatformWallet] {
        try ensureConfigured()

        try platform_wallet_manager_load_from_persistor(handle).check()

        // Ask SwiftData for the list of wallet ids we just told Rust
        // to load. We reuse the same container rather than shipping a
        // separate FFI "list ids" entry, because SwiftData already is
        // the source of truth.
        guard let persistenceHandler = persistenceHandler else {
            return []
        }
        let walletIds = persistenceHandler.restorableWalletIds()
        var restored: [ManagedPlatformWallet] = []
        restored.reserveCapacity(walletIds.count)

        for walletId in walletIds {
            guard walletId.count == 32 else { continue }
            var walletHandle: Handle = NULL_HANDLE
            do {
                try walletId.withUnsafeBytes { idPtr in
                    // C signature is `const uint8_t (*wallet_id)[32]`, which Swift
                    // imports as `UnsafePointer<FFIByteTuple32>?`. Rebind the raw
                    // 32-byte buffer to that 32-tuple shape so the call type-checks.
                    guard let base = idPtr.baseAddress?.assumingMemoryBound(to: FFIByteTuple32.self) else {
                        throw PlatformWalletError.nullPointer(
                            "wallet_id buffer base address was nil"
                        )
                    }
                    try platform_wallet_manager_get_wallet(
                        handle,
                        base,
                        &walletHandle
                    ).check()
                }
                let managedWallet = ManagedPlatformWallet(handle: walletHandle, walletId: walletId)
                restored.append(managedWallet)
                self.wallets[walletId] = managedWallet

                // Seedless unlock of the just-restored external-signable
                // (watch-only) wallet: verify the Keychain-resolved seed binds
                // to this wallet and drain any deferred contact-crypto.
                // Best-effort, per wallet: a wallet with no stored mnemonic
                // (genuine watch-only) stays watch-only, and any unlock error
                // (e.g. a mis-mapped Keychain slot) is logged-and-continued so
                // one wallet can't fail the whole restore.
                do {
                    let unlocked = try unlockWalletFromKeychain(managedWallet)
                    // NSLog (not print) so the unlock outcome is observable
                    // off-Xcode — it pairs with the resolver audit line in
                    // MnemonicResolver.resolve for "what touched the seed".
                    NSLog(
                        "🔓 wallet unlock %@: %@",
                        String(walletId.toHexString().prefix(8)),
                        unlocked ? "seed verified" : "no mnemonic — stays watch-only"
                    )
                } catch let error as PlatformWalletError {
                    // Distinguish a wrong-seed binding (Rust `SeedMismatch` →
                    // `ErrorInvalidParameter` → `.invalidParameter`) from a
                    // transient failure. The verify FFI is the only `.check()` on
                    // this path and `walletId` is already 32 bytes here, so
                    // `.invalidParameter` ≡ the seed-binding rejection — a
                    // security-relevant Keychain slot mis-mapping, not a hiccup.
                    // Either way the wallet stays external-signable (cannot sign),
                    // so no wrong-seed signing can occur.
                    if case .invalidParameter = error {
                        print(
                            "🚫 wallet unlock REJECTED \(walletId.toHexString().prefix(8)): "
                                + "seed does not bind (mis-mapped Keychain slot?) — stays watch-only"
                        )
                    } else {
                        // Transient (resolver/Keychain unavailable, …) — not
                        // retried this pass; a later signer-present action re-tries.
                        print("⚠️ wallet unlock failed \(walletId.toHexString().prefix(8)) (transient): \(error)")
                    }
                    self.lastError = error
                } catch {
                    print("❌ wallet unlock failed \(walletId.toHexString().prefix(8)): \(error)")
                    self.lastError = error
                }
            } catch {
                // Log and skip — one wallet failing doesn't fail the
                // whole restore. Usually means wallet_id / xpub
                // disagreement (SwiftData drift vs. Rust recompute).
                self.lastError = error
            }
        }

        // Kick off a background catch-up pass for every persisted
        // asset lock at `statusRaw < 2`. Closes the SPV-restart gap:
        // the wallet's in-memory transactions map was just
        // selectively repopulated by the load path (Rust-side
        // `restore_unresolved_asset_lock_tx_records`), so the next
        // chain-lock event picked up by SPV will cascade through
        // `apply_chain_lock` and promote each funding tx; the
        // catch-up `Task` parks on `wait_for_proof` until that
        // happens and on success the Rust changeset writes
        // `statusRaw = 3 + proofBytes` back to SwiftData. UI updates
        // reactively via `@Query`.
        catchUpStuckAssetLocks(wallets: restored)

        return restored
    }

    // MARK: - Keychain seed unlock

    /// Seedless unlock of a restored external-signable wallet.
    ///
    /// The persisted-restore path (`loadFromPersistor`) rehydrates every
    /// wallet **external-signable** — per-account xpubs only, no key
    /// material. Rather than grafting a resident seed back on, signing runs
    /// through the Keychain-backed resolver per-operation. This unlock does
    /// two things, both through a resolver (the seed never becomes resident):
    ///
    /// 1. **Verify** the resolved seed binds to this wallet — delegated in
    ///    full to [`verifySeedBinding`](Self/verifySeedBinding(_:)), which is
    ///    also the entry point for callers that need the gate WITHOUT the
    ///    drain below.
    /// 2. **Drain** (in the background) any contact-crypto deferred while
    ///    the wallet was seedless — `platform_wallet_drain_pending_contact_crypto`.
    ///    The drain re-fetches + decrypts over the network, so it runs in a
    ///    detached task off the caller's thread. Scheduled only when the
    ///    signerless `platform_wallet_pending_contact_crypto_count` probe
    ///    reports queued work.
    ///
    /// Per the Swift-SDK FFI boundary rules, the mnemonic → seed conversion
    /// happens entirely inside the resolver vtable in Rust; Swift only
    /// checks the Keychain entry's existence (`hasMnemonic`) and never pulls
    /// the plaintext across.
    ///
    /// - Parameter wallet: the restored `ManagedPlatformWallet`.
    /// - Returns: `true` if the wallet's seed verified (drain scheduled when
    ///   the pending-ops probe reports queued work); `false` if no mnemonic
    ///   is stored for this wallet (a genuine watch-only wallet), without
    ///   throwing.
    /// - Throws: `PlatformWalletError` if the verify FFI fails (e.g. the
    ///   resolved seed does not bind — a mis-mapped Keychain slot).
    /// Whether a wallet has a Keychain seed at all, once the binding holds.
    public enum SeedBindingCheck: Sendable, Equatable {
        /// A mnemonic is stored for this wallet and it derives the wallet's
        /// persisted BIP44 account-0 xpub. Signing for this wallet is sound.
        case verified
        /// No mnemonic is stored — a genuine watch-only wallet, imported by
        /// xpub. Nothing to contradict and nothing that can sign.
        case watchOnly
    }

    /// Verify that the Keychain seed for `wallet` actually owns it — and do
    /// nothing else.
    ///
    /// This is step 1 of [`unlockWalletFromKeychain`](Self/unlockWalletFromKeychain(_:))
    /// on its own. It exists because that method is not a verification
    /// primitive: it also schedules a background drain of the deferred
    /// contact crypto. A caller that needs the gate and not the drain — one
    /// about to run its own mnemonic-derived work, such as
    /// [`startWalletSubsystems`](Self/startWalletSubsystems(wallet:budget:gapLimit:storage:))
    /// — would otherwise have to launch a competing drain to ask the
    /// question. Two drains over two snapshots duplicate the network and
    /// ECDH work and race each other's channel-broken and auto-accept
    /// writes, which is why the unlock path already refuses to stack them.
    ///
    /// Marker-cached exactly as the unlock is: the outcome is a pure function
    /// of (mnemonic, network) against a fixed persisted xpub, so a match
    /// costs a string comparison and never touches the Keychain. A rewritten
    /// mnemonic item changes its stamp and forces the full check again.
    ///
    /// Publishes `dashPayUnlockStatus[walletId].seedMismatch` from the
    /// result, so a caller may read it afterwards — but callers that must not
    /// race the publisher should use the return value, which is ordered with
    /// respect to the work it guards.
    ///
    /// - Returns: `.verified` when the stored seed binds, `.watchOnly` when
    ///   there is no stored mnemonic to bind.
    /// - Throws: `PlatformWalletError` when the seed does not bind (a
    ///   mis-mapped Keychain slot) or the verify FFI otherwise fails.
    @discardableResult
    public func verifySeedBinding(_ wallet: ManagedPlatformWallet) throws -> SeedBindingCheck {
        try verifySeedBinding(wallet, storage: WalletStorage())
    }

    /// [`verifySeedBinding`](Self/verifySeedBinding(_:)) against a specific
    /// `WalletStorage`.
    ///
    /// A caller that resolves the mnemonic from one store must be verified
    /// against that same store, or the check answers a question about a
    /// different Keychain than the one the work will read — approving one
    /// mnemonic while the derivation uses another. `startWalletSubsystems`
    /// takes a `storage` parameter for exactly this reason and passes it here.
    @discardableResult
    func verifySeedBinding(
        _ wallet: ManagedPlatformWallet,
        storage walletStorage: WalletStorage
    ) throws -> SeedBindingCheck {
        try ensureConfigured()
        let walletId = wallet.walletId
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be 32 bytes, got \(walletId.count)"
            )
        }

        // A genuine watch-only wallet (imported by xpub, never holding a
        // seed) has no Keychain mnemonic — stays watch-only. Existence-only
        // check; the plaintext is never materialized in Swift.
        guard walletStorage.hasMnemonic(for: walletId) else {
            // No mnemonic is no longer a mismatch: a wallet whose seed failed
            // to bind and whose Keychain item was then removed must not keep
            // publishing the banner for a seed that is no longer there.
            setDashPaySeedMismatch(walletId, false)
            return .watchOnly
        }

        let walletHandle = wallet.handle
        // Resolver-backed signer, over the SAME store the check above read and
        // the caller's work will read: the mnemonic is fetched from the
        // Keychain inside the resolver vtable Rust-side; no resident seed.
        let coreSigner = MnemonicResolver(storage: walletStorage)

        // Wrong-seed / wrong-wallet gate, marker-cached: the check is a pure
        // function of (mnemonic, network) against the wallet's persisted
        // account-0 xpub, so after one successful verify Rust hands back a
        // marker — the verified xpub bound to the mnemonic Keychain item's
        // identity stamp — that Swift persists on the wallet row. Later
        // launches pass both back and Rust skips the resolver entirely while
        // the marker still matches; any rewrite of the Keychain item changes
        // the stamp and forces the full check again, so a replaced mnemonic
        // can never coast on an old verification. Swift only loads/stores
        // the marker and reads the stamp; match-vs-verify is decided in
        // Rust. `withExtendedLifetime` keeps the resolver alive across the
        // synchronous FFI call (its vtable callback fires during it, when a
        // full verify runs). Throws if the resolved seed derives a different
        // BIP44 account-0 xpub than the wallet's persisted one.
        //
        // Publish the per-wallet `seedMismatch` from the verify result itself,
        // scoped to JUST this call: the verify FFI maps Rust `SeedMismatch` →
        // `.invalidParameter`, and scoping the catch here keeps the earlier
        // 32-byte `walletId` precondition (also `.invalidParameter`) from being
        // mistaken for a seed mismatch. Rethrow so the existing caller handling
        // (loadFromPersistor's log-and-continue) is unchanged.
        do {
            let storedMarker = persistenceHandler?.seedBindingMarker(walletId: walletId)
            // Attribute-only stamp of the mnemonic Keychain item (secret never
            // materialized). Rust binds the marker to it, so any rewrite of
            // the item invalidates the cached verification. `nil` (attributes
            // unreadable) disables the cache for this launch — Rust then
            // always runs the full check and hands back no marker.
            let keychainStamp = walletStorage.mnemonicKeychainStamp(for: walletId)
            // Set only when a full verification ran and bound — the signal to
            // persist the fresh marker. Freed unconditionally below.
            var newMarkerPtr: UnsafeMutablePointer<CChar>? = nil
            defer {
                if let ptr = newMarkerPtr { platform_wallet_string_free(ptr) }
            }
            try withExtendedLifetime(coreSigner) {
                // Nested withCString over the two optional inputs; nil maps to
                // a null pointer (the FFI treats both as "absent").
                func callVerify(
                    _ markerPtr: UnsafePointer<CChar>?,
                    _ stampPtr: UnsafePointer<CChar>?
                ) -> PlatformWalletFFIResult {
                    platform_wallet_verify_seed_binds_to_wallet_cached(
                        walletHandle,
                        coreSigner.handle,
                        markerPtr,
                        stampPtr,
                        &newMarkerPtr
                    )
                }
                let result: PlatformWalletFFIResult
                switch (storedMarker, keychainStamp) {
                case let (marker?, stamp?):
                    result = marker.withCString { m in
                        stamp.withCString { s in callVerify(m, s) }
                    }
                case let (marker?, nil):
                    result = marker.withCString { m in callVerify(m, nil) }
                case let (nil, stamp?):
                    result = stamp.withCString { s in callVerify(nil, s) }
                case (nil, nil):
                    result = callVerify(nil, nil)
                }
                try result.check()
            }
            if let ptr = newMarkerPtr {
                persistenceHandler?.setSeedBindingMarker(
                    walletId: walletId,
                    marker: String(cString: ptr)
                )
                NSLog(
                    "🔐 seed binding verified via resolver for %@ — marker "
                        + "persisted; later launches skip the derivation",
                    String(walletId.toHexString().prefix(8))
                )
            }
            setDashPaySeedMismatch(walletId, false)
        } catch let error as PlatformWalletError {
            if case .invalidParameter = error {
                setDashPaySeedMismatch(walletId, true)
            }
            throw error
        }

        return .verified
    }

    @discardableResult
    public func unlockWalletFromKeychain(_ wallet: ManagedPlatformWallet) throws -> Bool {
        // Step 1 in full, side-effect-free. A watch-only wallet has nothing
        // to unlock and nothing to drain for.
        guard try verifySeedBinding(wallet) == .verified else { return false }

        let walletId = wallet.walletId
        let walletHandle = wallet.handle
        // Resolver-backed signer for the drain: the mnemonic is fetched from
        // the Keychain inside the resolver vtable Rust-side; no resident seed.
        let coreSigner = MnemonicResolver()

        // Heal pre-breadcrumb identity keys so they sign via the resolver
        // (derive-sign-destroy) rather than the stored scalar. Idempotent and
        // Keychain-sourced; runs once the seed is confirmed present for this
        // wallet, which is exactly when its identity keys become signable.
        // Fire-and-forget off the main actor — signing falls back to the stored
        // scalar until this heals, so it never needs to block unlock.
        persistenceHandler?.scheduleBackfillIdentityKeyBreadcrumbs(walletId: walletId)

        // Only schedule the background drain when the signerless probe
        // reports queued work. This is the TOTAL drainable count — every op
        // kind, including the ContactInfoDecrypt refreshes that the
        // user-facing banner count excludes — because the unlock drain is
        // the launch-time application point for cross-device contact
        // metadata; gating on the filtered count would strand
        // ContactInfoDecrypt-only queues until some other signer-present
        // action ran a drain. On a probe failure fall through and schedule
        // the drain (old behavior): it is always safe to run.
        var pendingOps: UInt32 = 0
        let countResult = platform_wallet_drainable_contact_crypto_count(
            walletHandle, &pendingOps
        )
        if PlatformWalletResultCode(ffi: countResult.code) == .success && pendingOps == 0 {
            NSLog(
                "🧵 contact-crypto drain skipped for %@ — no pending ops",
                String(walletId.toHexString().prefix(8))
            )
            return true
        }
        NSLog(
            "🧵 contact-crypto drain scheduling for %@ — %u pending op(s)",
            String(walletId.toHexString().prefix(8)), pendingOps
        )

        // Don't stack a second drain on an in-flight one: a banner Unlock tap
        // (or a second unlock) while a drain runs would duplicate the network
        // re-fetch + ECDH work and race the channel-broken writes. The banner
        // also disables Unlock while `draining`, but guard here too.
        if dashPayUnlockStatus[walletId]?.draining == true {
            return true
        }
        setDashPayDraining(walletId, true)

        // Identity document signer for the DIP-15 auto-accept pass (which sends
        // the reciprocal contact request). Nil when no SwiftData container is
        // configured → the drain runs provider-only (account build / contactInfo)
        // and skips auto-accept. `KeychainSigner` is `@unchecked Sendable`;
        // captured (with the resolver) in the detached task below.
        let identitySigner: KeychainSigner? = self.modelContainer.map {
            KeychainSigner(modelContainer: $0, network: self.signerNetwork ?? .testnet)
        }

        // Drain deferred contact-crypto in the background — it re-fetches and
        // decrypts over the network, so it must not block the caller. The
        // detached task retains `coreSigner` (+ the identity signer), keeping
        // them alive for the drain's vtable callbacks. It captures the raw
        // `walletHandle` (a `UInt64`), not the `ManagedPlatformWallet`: if the
        // wallet is destroyed before the drain runs, `with_item` Rust-side simply
        // misses the handle and the drain no-ops (NotFound) — no use-after-free.
        // Fire-and-forget: a failure here is not fatal (the next signer-present
        // DashPay action re-attempts the drain via its own provider), but it is
        // no longer swallowed silently — the failure lands on `lastError` and
        // the `draining` flag is cleared, both on the main actor.
        Task.detached(priority: .utility) { [weak self] in
            var drained: UInt32 = 0
            let result = withExtendedLifetime((coreSigner, identitySigner)) {
                platform_wallet_drain_pending_contact_crypto(
                    walletHandle,
                    identitySigner?.handle,
                    coreSigner.handle,
                    &drained
                )
            }
            let drainError: Error? = {
                do { try result.check(); return nil } catch { return error }
            }()
            // Hop to the main actor via a direct call, not a `MainActor.run`
            // closure: capturing the task-isolated `self` into a main-actor
            // closure is what Swift 6 region isolation rejects. `self` is
            // `@MainActor` (Sendable) and the locally-built `drainError` /
            // `drained` are region-transferable, so the call is race-free.
            await self?.finishContactCryptoDrain(
                walletId: walletId, drained: drained, drainError: drainError)
        }
        return true
    }

    /// Main-actor tail of the deferred contact-crypto drain: clears the
    /// `draining` flag and surfaces any failure on `lastError`. Split out of
    /// the `Task.detached` body so it hops back via a direct `@MainActor`
    /// call instead of capturing `self` into a `MainActor.run` closure.
    @MainActor
    private func finishContactCryptoDrain(
        walletId: Data, drained: UInt32, drainError: Error?
    ) {
        setDashPayDraining(walletId, false)
        if let drainError {
            lastError = drainError
            print(
                "⚠️ contact-crypto drain failed for "
                    + "\(walletId.toHexString().prefix(8)): \(drainError)"
            )
        } else if drained > 0 {
            // `drained` counts cleared queue entries — both completed
            // and permanently-failed (channel-broken) ops — so report
            // it neutrally rather than implying all succeeded.
            print(
                "🔑 processed \(drained) deferred contact-crypto op(s) for "
                    + "\(walletId.toHexString().prefix(8))"
            )
        }
    }

    /// For every persisted asset lock at `statusRaw < 2` (Built /
    /// Broadcast), kick off a background `Task` that drives
    /// `asset_lock_manager_catch_up_blocking` to completion or
    /// timeout. Fire-and-forget — the proof reaches the UI via the
    /// `AssetLockChangeSet` that the catch-up call queues internally.
    ///
    /// Called from `loadFromPersistor` after every wallet is
    /// inserted. App-foreground / network-reconnect callers can
    /// invoke this directly to retry whatever was still pending.
    public func catchUpStuckAssetLocks(wallets: [ManagedPlatformWallet]) {
        guard let persistenceHandler = persistenceHandler else { return }
        for wallet in wallets {
            let walletId = wallet.walletId
            let locks = persistenceHandler.loadCachedAssetLocks(walletId: walletId)
            let pending = locks.filter { $0.statusRaw < 2 }
            if pending.isEmpty { continue }

            // Snapshot the asset-lock manager wrapper ON the main
            // actor (where `wallet` lives), then hand the wrapper
            // itself — not just its bare `Handle` — to the detached
            // tasks. Each task's capture is a retain on the wrapper,
            // so `deinit` (which calls `asset_lock_manager_destroy`
            // and invalidates the FFI handle) can't fire until the
            // last in-flight catch-up task drops its retain. That
            // closes the lifetime race where a follow-up
            // `catchUpStuckAssetLocks` call (e.g. on
            // app-foreground / network-reconnect) used to destroy
            // the previous batch's handles mid-FFI-call.
            //
            // `ManagedAssetLockManager` is `@unchecked Sendable`
            // (immutable `let handle`, no shared mutable state, deinit
            // runs exactly once via ARC), so capturing it across
            // task boundaries is safe.
            let assetLockManager: ManagedAssetLockManager
            do {
                assetLockManager = try wallet.assetLockManager()
            } catch {
                self.lastError = error
                continue
            }

            // Cap concurrency to avoid saturating iOS's cooperative
            // thread pool. Each catch-up `block_on` parks a worker
            // thread for up to 300s; N stuck locks at launch (after a
            // multi-identity registration interrupted by an app kill)
            // would otherwise spawn N parallel parked threads,
            // starving every other `Task` in the app (UI updates,
            // SwiftData writes, network calls).
            //
            // `MAX_CONCURRENT_CATCH_UPS = 4` is conservative for a
            // 4-8 worker pool typical on iPhones. The real bottleneck
            // is per-lock SPV chainlock arrival, not the catch-up
            // throughput — running 4 in parallel vs 50 changes nothing
            // about how fast each individual lock resolves.
            let outpoints: [(txid: Data, vout: UInt32)] = pending.compactMap {
                PlatformWalletManager.decodeOutPointForCatchUp($0.outPointHex)
            }
            guard !outpoints.isEmpty else { continue }
            Task.detached(priority: .background) {
                await withTaskGroup(of: Void.self) { group in
                    let maxConcurrent = 4
                    var nextIndex = 0
                    // Seed the group with up to `maxConcurrent` tasks.
                    // Each `group.addTask` closure captures
                    // `assetLockManager` — that retain keeps the
                    // wrapper alive for the duration of the FFI call,
                    // independently of the outer detached task and
                    // independently of any future
                    // `catchUpStuckAssetLocks` invocation.
                    while nextIndex < outpoints.count && nextIndex < maxConcurrent {
                        let (txid, vout) = outpoints[nextIndex]
                        group.addTask {
                            Self.runCatchUp(assetLockManager: assetLockManager, txid: txid, vout: vout)
                        }
                        nextIndex += 1
                    }
                    // As each finishes, queue the next pending entry.
                    while await group.next() != nil {
                        if nextIndex < outpoints.count {
                            let (txid, vout) = outpoints[nextIndex]
                            group.addTask {
                                Self.runCatchUp(assetLockManager: assetLockManager, txid: txid, vout: vout)
                            }
                            nextIndex += 1
                        }
                    }
                }
            }
        }
    }

    /// Single catch-up invocation body. Extracted from the inline
    /// `Task.detached` so the `withTaskGroup` coordinator can call it
    /// directly. The `assetLockManager` parameter is captured by each
    /// task closure — the task's retain on the wrapper guarantees the
    /// FFI handle (read via `assetLockManager.handle`) stays valid
    /// for the entire `asset_lock_manager_catch_up_blocking` call,
    /// even if a follow-up `catchUpStuckAssetLocks` invocation
    /// replaces the manager-wide reference midway through.
    ///
    /// `nonisolated` because `PlatformWalletManager` is
    /// `@MainActor`-isolated by default and the detached task body
    /// runs off the main actor — the FFI call is synchronous and
    /// reads no `PlatformWalletManager` state.
    nonisolated private static func runCatchUp(assetLockManager: ManagedAssetLockManager, txid: Data, vout: UInt32) {
        // Build the txid tuple inline so the Task body captures only
        // Sendable values.
        var txidTuple: FFIByteTuple32 =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
             0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
        txid.withUnsafeBytes { buf in
            withUnsafeMutableBytes(of: &txidTuple) { dst in
                dst.copyBytes(from: buf.prefix(32))
            }
        }
        // Five-minute ceiling matches the `wait_for_proof` deadline
        // the production resume path uses.
        let result = asset_lock_manager_catch_up_blocking(
            assetLockManager.handle, &txidTuple, vout, 300
        )
        // Timeouts and proof-wait failures (catch-up
        // `errorWalletOperation`) are expected during normal
        // operation (SPV not yet caught up to the funding block,
        // chain-lock hasn't fired yet) — discard.
        // `errorInvalidHandle` is NOT expected: each task retains its
        // own `assetLockManager` wrapper, so the handle is guaranteed
        // valid for the duration of this call. If it surfaces, log it
        // loudly via NSLog so an operator running without `tracing`
        // capture still sees the programmer error.
        let code = PlatformWalletResultCode(ffi: result.code)
        if code == .errorInvalidHandle {
            NSLog(
                "[catch-up] asset_lock_manager_catch_up_blocking returned errorInvalidHandle for outpoint %@:%u — handle invalid despite task-owned wrapper retain",
                txid.map { String(format: "%02x", $0) }.joined(),
                vout
            )
        }
    }

    /// Parse `<txid_hex (display order)>:<vout>` into the wire-order
    /// 32-byte `txid` + `vout` pair the FFI expects. Internal to
    /// `catchUpStuckAssetLocks`; mirrors the Swift-side
    /// `decodeOutPointHex` helper without taking a dependency on the
    /// private one in the persistence handler.
    private static func decodeOutPointForCatchUp(
        _ hex: String
    ) -> (txid: Data, vout: UInt32)? {
        let parts = hex.split(separator: ":", maxSplits: 1, omittingEmptySubsequences: false)
        guard parts.count == 2 else { return nil }
        let txidHex = String(parts[0])
        guard let vout = UInt32(parts[1]) else { return nil }
        guard txidHex.count == 64 else { return nil }
        var txid = Data(capacity: 32)
        var idx = txidHex.startIndex
        for _ in 0..<32 {
            let end = txidHex.index(idx, offsetBy: 2)
            guard let byte = UInt8(txidHex[idx..<end], radix: 16) else { return nil }
            txid.append(byte)
            idx = end
        }
        // outPointHex is display-order; the FFI expects wire-order
        // (the same orientation `PersistentTransaction.txid` is
        // stored in).
        return (txid: Data(txid.reversed()), vout: vout)
    }

    // MARK: - Wallet deletion

    /// Fully wipe a wallet's Rust, SwiftData, and Keychain footprint.
    ///
    /// Requires the manager to have been `configure`d with a
    /// `ModelContainer` — the per-identity Keychain sweep needs the
    /// wallet's identity ids, which only the persistence handler can
    /// resolve. The no-persistence configuration mode is rejected
    /// here rather than silently leaving identity key material behind.
    ///
    /// Deleting an already-removed wallet succeeds unless an
    /// operation fails.
    public func deleteWallet(walletId: Data) throws {
        try ensureConfigured()
        guard walletId.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "walletId must be 32 bytes, got \(walletId.count)"
            )
        }
        guard let persistenceHandler = persistenceHandler else {
            throw PlatformWalletError.invalidHandle(
                "deleteWallet requires a persistence handler — configure the manager with a ModelContainer"
            )
        }

        let identityIds = try persistenceHandler.identityIdsForWallet(walletId: walletId)

        // Wipe Keychain BEFORE the SwiftData identity deletion runs.
        // Order matters for retry-safety: if `deleteWalletData`
        // commits identity rows and then throws partway, a retry
        // would see `identityIdsForWallet == []` and the
        // `deleteAllKeychainItems(forIdentityId:)` sweep below
        // could no longer find the keys to purge. Doing the
        // keychain side first leaves at worst stale SwiftData
        // rows on a retry — repeating the wipe is harmless, and
        // every keychain call here is idempotent (no-op on "not
        // found"). Mnemonic / metadata stay in `WalletStorage`
        // for now so a retry can still derive any missed key.
        for identityId in identityIds {
            try KeychainManager.shared.deleteAllKeychainItems(forIdentityId: identityId)
        }
        try KeychainManager.shared.deleteAllIdentityPrivateKeys(forWalletId: walletId)

        try walletId.withUnsafeBytes { raw in
            guard let base = raw.baseAddress?.assumingMemoryBound(to: FFIByteTuple32.self) else {
                throw PlatformWalletError.nullPointer(
                    "wallet_id buffer base address was nil"
                )
            }
            try platform_wallet_manager_remove_wallet(handle, base).check()
        }

        wallets.removeValue(forKey: walletId)
        // Drop the needs-unlock banner state immediately so a re-created wallet
        // with the same deterministic id doesn't inherit a stale banner (the
        // poller would also prune it, but not until the next tick).
        dashPayUnlockStatus.removeValue(forKey: walletId)

        try persistenceHandler.deleteWalletData(walletId: walletId)

        // The mnemonic + metadata blobs in the Keychain are keyed by
        // `walletId`. With network-scoped wallet ids the same mnemonic
        // maps to a DIFFERENT id per network, so a given id is owned by
        // exactly one network's wallet and carries its own mnemonic
        // copy — purging it can't orphan a sibling network (those live
        // under their own distinct ids). The `walletRowCountAcrossNetworks
        // == 0` check is therefore expected to be true right after
        // `deleteWalletData` removes this id's lone row; it is retained
        // as a defensive guard (and to stay correct should the id model
        // ever change) so we never delete the phrase while any row for
        // this exact id still exists.
        let remaining = try persistenceHandler.walletRowCountAcrossNetworks(walletId: walletId)
        if remaining == 0 {
            let storage = WalletStorage()
            // Delete metadata first so the mnemonic remains available for retry.
            try storage.deleteMetadata(for: walletId)
            try storage.deleteMnemonic(for: walletId)
        }
    }

    // MARK: - Per-wallet lookup

    /// Return the managed wallet with the given 32-byte id, or `nil`
    /// if it is not loaded.
    ///
    /// The Rust manager can hold multiple wallets at once (BLAST
    /// sync operates on all of them); UI surfaces that act against a
    /// specific wallet should route through this lookup rather than
    /// assuming a single active wallet exists.
    public func wallet(for walletId: Data) -> ManagedPlatformWallet? {
        wallets[walletId]
    }

    /// Convenience for bootstrap / single-wallet UI surfaces: an
    /// arbitrary-but-deterministic wallet from the managed set
    /// (sorted by walletId). Returns `nil` when no wallets are
    /// loaded.
    public var firstWallet: ManagedPlatformWallet? {
        guard !wallets.isEmpty else { return nil }
        let key = wallets.keys.min(by: { $0.lexicographicallyPrecedes($1) })
        return key.flatMap { wallets[$0] }
    }

    // MARK: - DashPay needs-unlock signal

    /// Count of deferred **account-build** contact-crypto ops queued for the
    /// wallet (the contacts waiting for a signer unlock to finish payment-account
    /// setup). Thin bridge over `platform_wallet_pending_contact_crypto_count`;
    /// the Rust side decides what counts (account-build ops only). Signerless —
    /// safe to poll.
    public func pendingAccountBuildCount(for walletId: Data) throws -> UInt32 {
        guard let wallet = wallets[walletId] else {
            throw PlatformWalletError.invalidParameter("unknown wallet")
        }
        var count: UInt32 = 0
        try platform_wallet_pending_contact_crypto_count(wallet.handle, &count).check()
        return count
    }

    /// Update `seedMismatch` for a wallet, gated on change to avoid needless
    /// `@Published` churn.
    private func setDashPaySeedMismatch(_ walletId: Data, _ value: Bool) {
        var status = dashPayUnlockStatus[walletId] ?? .init()
        guard status.seedMismatch != value else { return }
        status.seedMismatch = value
        dashPayUnlockStatus[walletId] = status
    }

    /// Update `draining` for a wallet, gated on change.
    private func setDashPayDraining(_ walletId: Data, _ value: Bool) {
        var status = dashPayUnlockStatus[walletId] ?? .init()
        guard status.draining != value else { return }
        status.draining = value
        dashPayUnlockStatus[walletId] = status
    }

    // MARK: - Xpub rendering

    /// Render a bincode-encoded per-account `ExtendedPubKey` (as
    /// stored on `PersistentAccount.accountExtendedPubKeyBytes`) as a
    /// BIP32 base58check string. The encoded key carries its own
    /// network, so `xpub…`/`tpub…` is produced automatically.
    ///
    /// Returns `nil` if the bytes are empty or the decode fails.
    public static func accountExtendedPubKeyString(bytes: Data) -> String? {
        guard !bytes.isEmpty else { return nil }
        var outPtr: UnsafeMutablePointer<CChar>? = nil
        do {
            try bytes.withUnsafeBytes { raw in
                guard let base = raw.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                    throw PlatformWalletError.nullPointer(
                        "xpub bytes buffer base address was nil"
                    )
                }
                try platform_wallet_account_xpub_to_string(
                    base,
                    UInt(bytes.count),
                    &outPtr
                ).check()
            }
        } catch {
            return nil
        }
        guard let cStr = outPtr else {
            return nil
        }
        let str = String(cString: cStr)
        platform_wallet_free_string(cStr)
        return str
    }

    // MARK: - Per-account balances

    /// Per-account balance snapshot read from Rust's in-memory state.
    ///
    /// `keysUsed` / `keysTotal` are the number of derived addresses
    /// across every pool on the account, with `keysUsed` further
    /// filtered by `AddressInfo.used`. The fields are populated for
    /// both funds and keys variants — the explorer surfaces them as
    /// the headline number on keys-only rows where balance is zero by
    /// construction.
    public struct AccountBalance {
        public let typeTag: UInt8
        public let standardTag: UInt8
        public let index: UInt32
        public let registrationIndex: UInt32
        public let keyClass: UInt32
        public let userIdentityId: Data
        public let friendIdentityId: Data
        public let confirmed: UInt64
        public let unconfirmed: UInt64
        public let immature: UInt64
        public let locked: UInt64
        public let keysUsed: UInt32
        public let keysTotal: UInt32
    }

    /// Query per-account balances directly from the Rust-side
    /// `WalletManager`'s in-memory state. No disk I/O — reads the
    /// live `ManagedCoreAccount.balance` values maintained during SPV
    /// processing.
    public func accountBalances(for walletId: Data) -> [AccountBalance] {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32 else {
            return []
        }

        var outEntries: UnsafePointer<AccountBalanceEntryFFI>?
        var outCount: UInt = 0

        let ffiResult = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            let base = raw.baseAddress?.assumingMemoryBound(to: UInt8.self)
            return platform_wallet_manager_get_account_balances(
                handle,
                base,
                &outEntries,
                &outCount
            )
        }

        let result = PlatformWalletResult(ffiResult)


        guard result.isSuccess else {
            self.lastError = PlatformWalletError(result: result)
            return []
        }

        guard let entries = outEntries, outCount > 0 else {
            return []
        }

        defer {
            platform_wallet_manager_free_account_balances(
                UnsafeMutablePointer(mutating: entries),
                outCount
            )
        }

        return (0..<Int(outCount)).map { i in
            var entry = entries[i]
            let uid = withUnsafeBytes(of: &entry.user_identity_id) { Data($0) }
            let fid = withUnsafeBytes(of: &entry.friend_identity_id) { Data($0) }
            return AccountBalance(
                typeTag: entry.type_tag,
                standardTag: entry.standard_tag,
                index: entry.index,
                registrationIndex: entry.registration_index,
                keyClass: entry.key_class,
                userIdentityId: uid,
                friendIdentityId: fid,
                confirmed: entry.confirmed,
                unconfirmed: entry.unconfirmed,
                immature: entry.immature,
                locked: entry.locked,
                keysUsed: entry.keys_used,
                keysTotal: entry.keys_total
            )
        }
    }

    // MARK: - Internals

    /// `internal` so the extensions in sibling files gate on the same check
    /// rather than re-implementing it.
    func ensureConfigured() throws {
        if !isConfigured || handle == NULL_HANDLE {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
    }

    /// Starts the SPV progress polling loop. Cancelled on deinit.
    ///
    /// `@Published` assignments are gated on inequality so that identical
    /// snapshots don't trigger SwiftUI re-evaluation. A naive 1 Hz reassignment
    /// of a non-Equatable struct caused every observer (sync screens, memory
    /// explorer, global indicator) to re-evaluate every second, accreting
    /// SwiftUI attribute-graph state and burning CPU long after sync settled.
    private func startProgressPolling() {
        progressPollTask?.cancel()
        progressPollTask = Task { [weak self] in
            while !Task.isCancelled {
                guard let self = self else { return }
                if let progress = try? self.syncProgress(), progress != self.spvProgress {
                    self.spvProgress = progress
                }
                if let running = try? self.isSpvRunning(), running != self.spvIsRunning {
                    self.spvIsRunning = running
                }
                if let peers = try? self.connectedSpvPeers(), peers != self.spvPeers {
                    self.spvPeers = peers
                }
                if let isSyncing = try? self.isPlatformAddressSyncing(),
                   isSyncing != self.platformAddressSyncIsSyncing {
                    self.platformAddressSyncIsSyncing = isSyncing
                }
                if let isSyncing = try? self.isShieldedSyncing(),
                   isSyncing != self.shieldedSyncIsSyncing {
                    self.shieldedSyncIsSyncing = isSyncing
                }
                if let isSyncing = try? self.isDashPaySyncing(),
                   isSyncing != self.dashPaySyncIsSyncing {
                    self.dashPaySyncIsSyncing = isSyncing
                }
                let tip = (try? self.currentSpvTipBlockTime()) ?? nil
                if tip != self.spvTipBlockTime {
                    self.spvTipBlockTime = tip
                }
                // Refresh the per-wallet needs-unlock count (account-build ops).
                // Per-wallet, so O(wallets)/tick; gated on change per key.
                for walletId in self.wallets.keys {
                    if let n = try? self.pendingAccountBuildCount(for: walletId),
                       n != self.dashPayUnlockStatus[walletId]?.pendingAccountBuilds {
                        var status = self.dashPayUnlockStatus[walletId] ?? .init()
                        status.pendingAccountBuilds = n
                        self.dashPayUnlockStatus[walletId] = status
                    }
                }
                // Prune status for wallets no longer loaded (e.g. removed by a
                // wipe) so a re-created wallet with the same id starts clean.
                let stale = self.dashPayUnlockStatus.keys.filter { self.wallets[$0] == nil }
                for walletId in stale {
                    self.dashPayUnlockStatus.removeValue(forKey: walletId)
                }
                try? await Task.sleep(for: .seconds(1))
            }
        }
    }

    /// Drop the platform-address published mirror after a reset/clear so a
    /// later `configure()` re-subscribe — which Combine replays the current
    /// `@Published` value to a fresh subscriber — can't repaint stale sync
    /// state over a just-cleared UI.
    ///
    /// Lives here (not in the `…AddressSync` extension) because
    /// `platformAddressSyncIsSyncing` is `private(set)`; the generation guard
    /// only blocks *future* stale callbacks and can't un-publish a value
    /// already held on these `@Published` properties. Called by
    /// `resetPlatformAddressSyncState` after the Rust drain returns.
    func resetPlatformAddressPublishedMirror() {
        lastPlatformAddressSyncEvent = nil
        platformAddressSyncIsSyncing = false
    }
}
