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

/// Arguments of one native create-wallet call, captured on the main actor
/// before hopping to the destroy queue. Every field is a plain value type.
struct PlatformWalletCreateParams: Sendable {
    let mnemonic: String
    let network: Network
    let accountOptions: UInt32
    let birthHeight: UInt32?
}

/// Native entry point used by the off-main create orchestration in
/// [`PlatformWalletManager.performCreateWallet`]. Same design as
/// [`PlatformWalletNativeTeardownCalls`]: injecting the function (rather
/// than the outcome) keeps the production timing, result mapping, and
/// logging under test without calling Rust. The unchecked conformance is
/// intentional: the closure is an immutable C-entry wrapper, and the whole
/// value is copied onto the dedicated queue.
///
/// Tests with a fake manager handle must inject this table: handles come from
/// a process-global registry, so an arbitrary non-zero test value is not
/// guaranteed to miss a live Rust entry owned by another test.
struct PlatformWalletNativeCreateCalls: @unchecked Sendable {
    /// Mirrors `platform_wallet_manager_create_wallet_from_mnemonic_with_birth_height`,
    /// folding the two out-params into the return value (the 32-byte wallet
    /// id already copied into a `Data`).
    typealias Call = @Sendable (Handle, PlatformWalletCreateParams)
        -> (result: PlatformWalletFFIResult, walletHandle: Handle, walletId: Data)

    let createFromMnemonic: Call

    static let live = PlatformWalletNativeCreateCalls(
        createFromMnemonic: { managerHandle, params in
            var walletHandle: Handle = NULL_HANDLE
            var walletId: FFIByteTuple32 =
                (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
            let result = params.mnemonic.withCString { mnemonicPtr in
                platform_wallet_manager_create_wallet_from_mnemonic_with_birth_height(
                    managerHandle,
                    mnemonicPtr,
                    params.network.ffiValue,
                    params.accountOptions,
                    params.birthHeight != nil,
                    params.birthHeight ?? 0,
                    &walletHandle,
                    &walletId
                )
            }
            let idData = withUnsafeBytes(of: &walletId) { Data($0) }
            return (result, walletHandle, idData)
        }
    )
}

/// Test seam for the native calls of the async `loadFromPersistor()`
/// overload; same contract as [`PlatformWalletNativeCreateCalls`].
struct PlatformWalletNativeLoadCalls: @unchecked Sendable {
    /// Mirrors `platform_wallet_manager_load_from_persistor`.
    typealias BulkLoad = @Sendable (Handle) -> PlatformWalletFFIResult
    /// The SwiftData id fetch between the bulk load and the per-wallet
    /// lookups. Part of the seam (not pure FFI) so unit tests can drive
    /// the lookup loop without constructing a ModelContainer.
    typealias ListRestorableIds = @Sendable (PlatformWalletPersistenceHandler?) -> [Data]
    /// Mirrors `platform_wallet_manager_get_wallet` for one 32-byte id,
    /// folding the out-param into the return value.
    typealias GetWallet = @Sendable (Handle, Data)
        -> (result: PlatformWalletFFIResult, walletHandle: Handle)

    let loadFromPersistor: BulkLoad
    let restorableWalletIds: ListRestorableIds
    let getWallet: GetWallet

    static let live = PlatformWalletNativeLoadCalls(
        loadFromPersistor: { managerHandle in
            platform_wallet_manager_load_from_persistor(managerHandle)
        },
        restorableWalletIds: { handler in
            handler?.restorableWalletIds() ?? []
        },
        getWallet: { managerHandle, walletId in
            var walletHandle: Handle = NULL_HANDLE
            let result: PlatformWalletFFIResult = walletId.withUnsafeBytes { idPtr in
                // C signature is `const uint8_t (*wallet_id)[32]`, imported
                // as `UnsafePointer<FFIByteTuple32>?` — rebind the raw
                // 32-byte buffer to that tuple shape (same dance as the
                // sync overload).
                guard let base = idPtr.baseAddress?.assumingMemoryBound(to: FFIByteTuple32.self) else {
                    return PlatformWalletFFIResult(
                        code: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_NULL_POINTER,
                        message: nil)
                }
                return platform_wallet_manager_get_wallet(
                    managerHandle,
                    base,
                    &walletHandle)
            }
            return (result, walletHandle)
        }
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

    /// Set the moment [`shutdown()`] decides to proceed, BEFORE it drains
    /// in-flight native ops: closes admission for EVERY native entrypoint —
    /// the async ones (`createWallet`, `loadFromPersistor`) and their
    /// synchronous overloads — so the drain below can terminate without a
    /// synchronous op entering while the MainActor is reentrant at an
    /// `await`.
    private var shutdownRequested = false

    /// Async native entrypoints (`createWallet`, `loadFromPersistor`)
    /// between admission and the end of their MainActor epilogue.
    /// [`shutdown()`] waits for this to reach zero before taking the
    /// handle: an admitted op must complete its FULL transaction (FFI +
    /// publish) or fail on its own terms — never be failed retroactively
    /// by a concurrent teardown after the native side already persisted
    /// data (for create, the caller would roll back its mnemonic and
    /// orphan the persisted rows).
    private var activeNativeOpCount = 0
    /// Read-only Core diagnostics have their own admission count. They must
    /// keep the manager handle alive until their FFI reads finish, but they do
    /// not make synchronous create/load/delete operations unsafe and therefore
    /// must not participate in `ensureSyncNativeOpAllowed`.
    private var activeCoreDiagnosticsNativeOpCount = 0
    private var nativeOpDrainContinuations: [CheckedContinuation<Void, Never>] = []

    /// Admission + bookkeeping shared by the async native entrypoints:
    /// rejects while a shutdown drain runs, otherwise counts the op in.
    /// MainActor-atomic (no suspension between check and increment), so the
    /// drain can never miss an admitted op. Balance with
    /// [`finishNativeOp()`] on every exit path.
    private func admitNativeOp(_ name: String) throws {
        guard !shutdownRequested else {
            throw PlatformWalletError.invalidHandle(
                "manager shutdown is in progress; \(name) rejected")
        }
        activeNativeOpCount += 1
    }

    private func finishNativeOp() {
        activeNativeOpCount -= 1
        resumeNativeOpDrainIfIdle()
    }

    /// Reserves the native manager handle for one read-only diagnostic pass.
    /// Shutdown drains this counter, while synchronous wallet operations
    /// intentionally ignore it because Rust serializes its own wallet state.
    /// Every successful admission must be balanced by
    /// ``finishCoreDiagnosticsNativeOp()``.
    func admitCoreDiagnosticsNativeOp() throws {
        guard !shutdownRequested else {
            throw PlatformWalletError.invalidHandle(
                "manager shutdown is in progress; coreWalletDiagnostics rejected")
        }
        activeCoreDiagnosticsNativeOpCount += 1
    }

    /// Releases a successful diagnostic admission. The guard keeps a future
    /// shutdown from observing a negative counter if an internal caller ever
    /// violates the admission/defer contract.
    func finishCoreDiagnosticsNativeOp() {
        guard activeCoreDiagnosticsNativeOpCount > 0 else {
            SDKLogger.event(
                "core_diagnostics_native_op_counter_underflow",
                category: .lifecycle,
                severity: .error
            )
            return
        }
        activeCoreDiagnosticsNativeOpCount -= 1
        resumeNativeOpDrainIfIdle()
    }

    private func resumeNativeOpDrainIfIdle() {
        guard activeNativeOpCount == 0,
              activeCoreDiagnosticsNativeOpCount == 0,
              !nativeOpDrainContinuations.isEmpty
        else { return }
        let waiters = nativeOpDrainContinuations
        nativeOpDrainContinuations.removeAll()
        waiters.forEach { $0.resume() }
    }

    /// Test seam for the individual native calls. Production keeps `.live`;
    /// tests replace the function table while still running the production
    /// teardown orchestration end-to-end.
    internal var nativeTeardownCalls = PlatformWalletNativeTeardownCalls.live

    /// Test seam for the native create call used by the async
    /// `createWallet(mnemonic:)` overload; same contract as
    /// [`nativeTeardownCalls`].
    internal var nativeCreateCalls = PlatformWalletNativeCreateCalls.live

    /// Test seam for the native calls of the async `loadFromPersistor()`
    /// overload; same contract as [`nativeTeardownCalls`].
    internal var nativeLoadCalls = PlatformWalletNativeLoadCalls.live

    /// Dedicated serial queue for the blocking native teardown AND the
    /// blocking native create (async `createWallet(mnemonic:)` overload).
    /// Both park the calling thread — the Rust `destroy` runs
    /// `block_on(shutdown())` and can legitimately take tens of seconds when
    /// an in-flight sync pass ignores cancellation; create derives hundreds
    /// of keys and flushes persistence synchronously — so they must park a
    /// plain GCD thread: never the main thread, and never a Swift
    /// Concurrency cooperative-pool thread (which is why this is a
    /// DispatchQueue and not `Task.detached`).
    ///
    /// Sharing ONE queue is deliberate: memory safety already comes from the
    /// Rust-side registry (the create call holds a read guard for its whole
    /// duration; destroy's map removal takes the write lock), so the queue's
    /// job is deterministic FIFO — a teardown enqueued after an admitted
    /// create runs after it, never concurrently with it. Accepted trade-off:
    /// the queue is process-wide, so a create on one manager can queue
    /// behind another manager's slow destroy (rare — hosts await `shutdown()`
    /// between lifecycle operations).
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
            SDKLogger.event(
                "manager_deinit_without_shutdown",
                category: .lifecycle,
                severity: .warning,
                fields: ["fallback_teardown_scheduled": .boolean(true)]
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
        // Drain loop: close admission for new async creates, then wait for
        // every already-admitted create to finish its FULL transaction
        // (native create + MainActor epilogue). Draining before take-once
        // means a create whose FFI already persisted wallet data can never
        // be failed retroactively by this teardown — the caller would roll
        // back its mnemonic and orphan the persisted rows. Each await can
        // interleave with other MainActor work, so every idempotency /
        // no-op condition is re-checked after resuming.
        while true {
            if let task = shutdownTask {
                return await task.value
            }
            guard handle != NULL_HANDLE else {
                // Never configured (or a test double without a handle):
                // nothing to tear down. Do not cache this no-op: a manager
                // may still be configured later, and that live handle must
                // then be torn down.
                return PlatformWalletShutdownMetrics(
                    steps: [],
                    totalMilliseconds: 0,
                    ranOffMainThread: false)
            }
            shutdownRequested = true
            if activeNativeOpCount == 0, activeCoreDiagnosticsNativeOpCount == 0 { break }
            await withCheckedContinuation { continuation in
                nativeOpDrainContinuations.append(continuation)
            }
        }

        // Take-once: from this point every FFI entry gated on
        // `ensureConfigured()` / `handle != NULL_HANDLE` rejects cleanly,
        // and the generation bumps drop any trailing sync event the main
        // actor delivers after this turn.
        let h = handle
        SDKLogger.event(
            "manager_shutdown_started",
            category: .lifecycle,
            fields: ["wallet_count": .integer(Int64(wallets.count))]
        )
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
        let metrics = await task.value
        SDKLogger.event(
            "manager_shutdown_completed",
            category: .lifecycle,
            fields: [
                "duration_ms": .integer(Int64(metrics.totalMilliseconds)),
                "off_main_thread": .boolean(metrics.ranOffMainThread),
                "step_count": .integer(Int64(metrics.steps.count)),
            ]
        )
        return metrics
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
                SDKLogger.event(
                    "manager_shutdown_step_failed",
                    category: .lifecycle,
                    severity: .error,
                    fields: [
                        "duration_ms": .integer(Int64(ms)),
                        "ffi_code": .integer(Int64(result.code.rawValue)),
                        "step": .publicText(name),
                    ],
                    error: PlatformWalletError(code: result.code, message: result.message)
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
        SDKLogger.event(
            "manager_native_teardown_completed",
            category: .lifecycle,
            fields: [
                "duration_ms": .integer(Int64(metrics.totalMilliseconds)),
                "failed_steps": .integer(Int64(steps.filter { $0.ffiCode != 0 }.count)),
                "off_main_thread": .boolean(offMain),
                "step_count": .integer(Int64(steps.count)),
            ]
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
        SDKLogger.event(
            "manager_configuration_started",
            category: .lifecycle,
            fields: [
                "network": .publicText(network.map { String(describing: $0) } ?? "unknown"),
                "persistence_enabled": .boolean(modelContainer != nil),
            ]
        )
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
            SDKLogger.event(
                "manager_configuration_failed",
                category: .lifecycle,
                severity: .error,
                fields: ["phase": .publicText("native_create")],
                error: error
            )
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
            SDKLogger.event(
                "manager_configuration_failed",
                category: .lifecycle,
                severity: .error,
                fields: ["phase": .publicText("persistence_capabilities")],
                error: error
            )
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
        SDKLogger.event(
            "manager_configuration_completed",
            category: .lifecycle,
            fields: [
                "capabilities_version": .unsignedInteger(UInt64(effectiveCapabilities.version)),
                "persistence_enabled": .boolean(handler != nil),
            ]
        )
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

    /// Admission gate for the SYNCHRONOUS native entrypoints (`createWallet`,
    /// `createWalletFromSeed`, `loadFromPersistor`, `deleteWallet`). Rejects:
    ///
    /// - once shutdown has closed admission — including the drain window,
    ///   where the manager's handle is intentionally still live for an
    ///   already-admitted async op while the MainActor is reentrant at the
    ///   drain's `await`;
    /// - while an async native op is in flight: the synchronous overloads
    ///   run their FFI on the calling thread, so admitting one would run a
    ///   second native op CONCURRENTLY with the admitted op on the destroy
    ///   queue. For load that is a real race — Rust's loader inserts into
    ///   `wallet_manager` and `self.wallets` in two steps, and a parallel
    ///   loader that sees the first insert skips the wallet before the
    ///   second lands (load.rs treats "already present" as "fully
    ///   hydrated", which only holds for sequential loaders).
    ///
    /// Async entrypoints use [`admitNativeOp`] instead (they serialize on
    /// the destroy queue, so async-with-async is safe).
    private func ensureSyncNativeOpAllowed(_ name: String) throws {
        try ensureConfigured()
        guard !shutdownRequested else {
            throw PlatformWalletError.invalidHandle(
                "manager shutdown is in progress; \(name) rejected")
        }
        guard activeNativeOpCount == 0 else {
            throw PlatformWalletError.invalidHandle(
                "an async native operation is in flight; synchronous \(name) rejected")
        }
    }

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
        try ensureSyncNativeOpAllowed("createWallet")
        SDKLogger.event(
            "wallet_create_started",
            category: .lifecycle,
            fields: [
                "birth_height_provided": .boolean(birthHeight != nil),
                "network": .publicText(String(describing: network)),
                "source": .publicText("mnemonic"),
            ]
        )
        var walletHandle: Handle = NULL_HANDLE
        var walletId: FFIByteTuple32 =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)

        let accountOptions: UInt32 = createDefaultAccounts ? 1 : 0

        do {
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
        } catch {
            SDKLogger.event(
                "wallet_create_failed",
                category: .lifecycle,
                severity: .error,
                fields: [
                    "network": .publicText(String(describing: network)),
                    "source": .publicText("mnemonic"),
                ],
                error: error,
                redacting: [mnemonic]
            )
            throw error
        }

        let idData = withUnsafeBytes(of: &walletId) { Data($0) }
        if let name = name, !name.isEmpty {
            persistenceHandler?.setWalletName(walletId: idData, name: name)
        }
        let w = ManagedPlatformWallet(handle: walletHandle, walletId: idData)
        self.wallets[idData] = w
        SDKLogger.event(
            "wallet_create_completed",
            category: .lifecycle,
            fields: [
                "source": .publicText("mnemonic"),
                "wallet_reference": .reference(idData),
            ]
        )
        return w
    }

    /// Off-main variant of [`createWallet(mnemonic:...)`]: identical
    /// semantics (same FFI call, `birthHeight` contract, name persistence,
    /// and published-state update), but the blocking native create — key
    /// derivation for every account plus a synchronous persistence flush,
    /// seconds of work — runs on [`destroyQueue`] instead of the main
    /// thread. In an `async` context overload resolution prefers this
    /// variant; sync contexts keep the sync one.
    ///
    /// Throws `PlatformWalletError.invalidHandle` when the manager is shut
    /// down or a shutdown is already in progress — always BEFORE any native
    /// work: an admitted create is guaranteed to run its full transaction
    /// (native create + publish); [`shutdown()`] drains admitted creates
    /// before taking the handle, so a create whose FFI persisted wallet
    /// data can never be failed retroactively by a concurrent teardown.
    @discardableResult
    public func createWallet(
        mnemonic: String,
        network: Network,
        name: String? = nil,
        createDefaultAccounts: Bool = true,
        birthHeight: UInt32? = nil
    ) async throws -> ManagedPlatformWallet {
        // Admission is checked on the MainActor with no suspension before
        // the count increment, so `shutdown()`'s drain can never miss an
        // admitted create.
        try ensureConfigured()
        try admitNativeOp("createWallet")
        defer { finishNativeOp() }

        let h = handle
        let params = PlatformWalletCreateParams(
            mnemonic: mnemonic,
            network: network,
            accountOptions: createDefaultAccounts ? 1 : 0,
            birthHeight: birthHeight)
        let calls = nativeCreateCalls

        // Deliberately a direct continuation, NOT the `Task {}` wrapper
        // `shutdown()` uses (that wrapper exists only to share
        // `shutdownTask`): the dispatch below happens synchronously at this
        // suspension point, so an admitted create is enqueued on the shared
        // FIFO queue before any later `shutdown()`'s teardown block —
        // teardown can never overtake an already-admitted create. A
        // `shutdown()` that completed BEFORE this point already failed the
        // `ensureConfigured()` above, so nothing was enqueued.
        let created: Result<ManagedPlatformWallet, PlatformWalletError> =
            await withCheckedContinuation { continuation in
                Self.destroyQueue.async {
                    continuation.resume(
                        returning: Self.performCreateWallet(h, params: params, calls: calls))
                }
            }
        let w = try created.get()

        // Defense in depth only: `shutdown()` drains admitted creates
        // before taking the handle, so this cannot fire from the production
        // shutdown path. It guards the invariant that the manager never
        // publishes a wallet after its handle was torn down (dropping `w`
        // lets its deinit release the wrapper handle — a registry no-op
        // after manager teardown).
        guard handle != NULL_HANDLE else {
            assertionFailure("shutdown took the handle under an admitted create despite the drain")
            throw PlatformWalletError.invalidHandle(
                "manager was shut down while createWallet ran off-main")
        }
        if let name, !name.isEmpty {
            persistenceHandler?.setWalletName(walletId: w.walletId, name: name)
        }
        self.wallets[w.walletId] = w
        return w
    }

    /// The blocking native create body of the async
    /// [`createWallet(mnemonic:...)`] overload: runs the injected create
    /// call, maps the FFI result to Swift types on the queue (the raw
    /// result's Rust-owned message string never crosses the continuation),
    /// and logs duration + which thread it ran on — the whole point is
    /// `offMain=true`.
    nonisolated static func performCreateWallet(
        _ handle: Handle,
        params: PlatformWalletCreateParams,
        calls: PlatformWalletNativeCreateCalls = .live
    ) -> Result<ManagedPlatformWallet, PlatformWalletError> {
        let offMain = !Thread.isMainThread
        let start = CFAbsoluteTimeGetCurrent()
        SDKLogger.event(
            "wallet_create_started",
            category: .lifecycle,
            fields: [
                "birth_height_provided": .boolean(params.birthHeight != nil),
                "network": .publicText(String(describing: params.network)),
                "off_main_thread": .boolean(offMain),
                "source": .publicText("mnemonic"),
            ]
        )
        let outcome = calls.createFromMnemonic(handle, params)
        let result = PlatformWalletResult(outcome.result)
        let ms = Int((CFAbsoluteTimeGetCurrent() - start) * 1000)
        guard result.isSuccess else {
            SDKLogger.event(
                "wallet_create_failed",
                category: .lifecycle,
                severity: .error,
                fields: [
                    "duration_ms": .integer(Int64(ms)),
                    "network": .publicText(String(describing: params.network)),
                    "off_main_thread": .boolean(offMain),
                    "source": .publicText("mnemonic"),
                ],
                error: PlatformWalletError(code: result.code, message: result.message),
                redacting: [params.mnemonic]
            )
            return .failure(PlatformWalletError(code: result.code, message: result.message))
        }
        SDKLogger.event(
            "wallet_create_completed",
            category: .lifecycle,
            fields: [
                "duration_ms": .integer(Int64(ms)),
                "off_main_thread": .boolean(offMain),
                "source": .publicText("mnemonic"),
                "wallet_reference": .reference(outcome.walletId),
            ]
        )
        return .success(
            ManagedPlatformWallet(handle: outcome.walletHandle, walletId: outcome.walletId))
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
        try ensureSyncNativeOpAllowed("createWalletFromSeed")
        guard seed.count == 64 else {
            throw PlatformWalletError.invalidParameter(
                "seed must be 64 bytes, got \(seed.count)"
            )
        }
        SDKLogger.event(
            "wallet_create_started",
            category: .lifecycle,
            fields: [
                "birth_height_provided": .boolean(birthHeight != nil),
                "network": .publicText(String(describing: network)),
                "source": .publicText("seed"),
            ]
        )

        var walletHandle: Handle = NULL_HANDLE
        var walletId: FFIByteTuple32 =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)

        let accountOptions: UInt32 = createDefaultAccounts ? 1 : 0

        do {
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
        } catch {
            SDKLogger.event(
                "wallet_create_failed",
                category: .lifecycle,
                severity: .error,
                fields: [
                    "network": .publicText(String(describing: network)),
                    "source": .publicText("seed"),
                ],
                error: error
            )
            throw error
        }

        let idData = withUnsafeBytes(of: &walletId) { Data($0) }
        if let name = name, !name.isEmpty {
            persistenceHandler?.setWalletName(walletId: idData, name: name)
        }
        let w = ManagedPlatformWallet(handle: walletHandle, walletId: idData)
        self.wallets[idData] = w
        SDKLogger.event(
            "wallet_create_completed",
            category: .lifecycle,
            fields: [
                "source": .publicText("seed"),
                "wallet_reference": .reference(idData),
            ]
        )
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
        // Same synchronous-admission gate as the sync creates: rejected
        // during the shutdown drain AND while an async native op is in
        // flight — a second Rust loader running concurrently with the one
        // on the destroy queue races load.rs's two-step hydration.
        try ensureSyncNativeOpAllowed("loadFromPersistor")
        SDKLogger.event(
            "wallet_restore_started",
            category: .lifecycle,
            fields: ["off_main_thread": .boolean(!Thread.isMainThread)]
        )

        do {
            try platform_wallet_manager_load_from_persistor(handle).check()
        } catch {
            SDKLogger.event(
                "wallet_restore_failed",
                category: .lifecycle,
                severity: .error,
                fields: ["phase": .publicText("bulk_load")],
                error: error
            )
            throw error
        }

        // Ask SwiftData for the list of wallet ids we just told Rust
        // to load. We reuse the same container rather than shipping a
        // separate FFI "list ids" entry, because SwiftData already is
        // the source of truth.
        guard let persistenceHandler = persistenceHandler else {
            SDKLogger.event(
                "wallet_restore_completed",
                category: .lifecycle,
                fields: ["wallet_count": .integer(0)]
            )
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
                // (watch-only) wallet — shared with the async overload's
                // epilogue; see `unlockRestoredWalletLoggingOutcome`.
                unlockRestoredWalletLoggingOutcome(managedWallet)
            } catch {
                // Log and skip — one wallet failing doesn't fail the
                // whole restore. Usually means wallet_id / xpub
                // disagreement (SwiftData drift vs. Rust recompute).
                self.lastError = error
                SDKLogger.event(
                    "wallet_restore_item_failed",
                    category: .lifecycle,
                    severity: .error,
                    fields: ["wallet_reference": .reference(walletId)],
                    error: error
                )
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

        SDKLogger.event(
            "wallet_restore_completed",
            category: .lifecycle,
            fields: ["wallet_count": .integer(Int64(restored.count))]
        )

        return restored
    }

    /// What the off-main half of the async [`loadFromPersistor()`] hands
    /// back to the MainActor epilogue: one entry per restorable wallet id,
    /// in the persistence handler's id order — either the raw wallet handle
    /// (the owning `ManagedPlatformWallet` wrapper is built on the
    /// MainActor) or the copied lookup error. Keeping the ORDERED sequence
    /// lets the epilogue replay publication, unlock and `lastError`
    /// assignment in exactly the sync overload's per-wallet interleaving.
    private struct OffMainLoadOutcome: @unchecked Sendable {
        enum Lookup {
            case restored(walletId: Data, walletHandle: Handle)
            case skipped(PlatformWalletError)
        }

        let bulkResult: Result<Void, PlatformWalletError>
        let lookups: [Lookup]
    }

    /// The blocking body of the async [`loadFromPersistor()`] overload:
    /// bulk restore FFI (fires the persistence callbacks synchronously on
    /// this queue — the handler is thread-safe by design, the same
    /// callbacks fire from Rust sync threads in steady state), the
    /// SwiftData id fetch (the handler serializes on its own queue and
    /// background context), and the per-wallet handle lookups. Maps FFI
    /// results on the queue so no Rust-owned message string crosses the
    /// continuation. Mirrors `performCreateWallet`'s timing log.
    private nonisolated static func performLoadFromPersistor(
        _ handle: Handle,
        handler: PlatformWalletPersistenceHandler?,
        calls: PlatformWalletNativeLoadCalls
    ) -> OffMainLoadOutcome {
        let started = CFAbsoluteTimeGetCurrent()
        let offMain = !Thread.isMainThread
        SDKLogger.event(
            "wallet_restore_started",
            category: .lifecycle,
            fields: ["off_main_thread": .boolean(offMain)]
        )

        let bulk = PlatformWalletResult(calls.loadFromPersistor(handle))
        guard bulk.isSuccess else {
            let ms = Int((CFAbsoluteTimeGetCurrent() - started) * 1000)
            SDKLogger.event(
                "wallet_restore_failed",
                category: .lifecycle,
                severity: .error,
                fields: [
                    "duration_ms": .integer(Int64(ms)),
                    "off_main_thread": .boolean(offMain),
                    "phase": .publicText("bulk_load"),
                ],
                error: PlatformWalletError(code: bulk.code, message: bulk.message)
            )
            return OffMainLoadOutcome(
                bulkResult: .failure(PlatformWalletError(code: bulk.code, message: bulk.message)),
                lookups: [])
        }

        var lookups: [OffMainLoadOutcome.Lookup] = []
        let walletIds = calls.restorableWalletIds(handler)
        lookups.reserveCapacity(walletIds.count)
        for walletId in walletIds where walletId.count == 32 {
            let lookup = calls.getWallet(handle, walletId)
            let lookupResult = PlatformWalletResult(lookup.result)
            guard lookupResult.isSuccess else {
                // Log-and-skip parity with the sync overload: one wallet
                // failing (usually SwiftData drift vs. Rust recompute)
                // doesn't fail the whole restore. Recorded IN SEQUENCE so
                // the epilogue's `lastError` replay matches the sync
                // overload's per-wallet ordering.
                let error = PlatformWalletError(
                    code: lookupResult.code,
                    message: lookupResult.message)
                SDKLogger.event(
                    "wallet_restore_item_failed",
                    category: .lifecycle,
                    severity: .error,
                    fields: ["wallet_reference": .reference(walletId)],
                    error: error
                )
                lookups.append(.skipped(error))
                continue
            }
            lookups.append(.restored(walletId: walletId, walletHandle: lookup.walletHandle))
        }

        let restoredCount = lookups.reduce(into: 0) { count, entry in
            if case .restored = entry { count += 1 }
        }
        let ms = Int((CFAbsoluteTimeGetCurrent() - started) * 1000)
        SDKLogger.event(
            "wallet_restore_native_completed",
            category: .lifecycle,
            fields: [
                "duration_ms": .integer(Int64(ms)),
                "off_main_thread": .boolean(offMain),
                "wallet_count": .integer(Int64(restoredCount)),
            ]
        )
        return OffMainLoadOutcome(bulkResult: .success(()), lookups: lookups)
    }

    /// Off-main variant of [`loadFromPersistor()`]: identical semantics
    /// (bulk restore, per-wallet handle lookup, publish, best-effort
    /// keychain unlock, asset-lock catch-up), but the blocking native work
    /// — the bulk restore's persister reads and Rust wallet
    /// reconstruction plus the per-wallet lookups, measured at roughly
    /// 400ms per persisted wallet — runs on [`destroyQueue`] instead of
    /// the main thread. In an `async` context overload resolution prefers
    /// this variant; sync contexts keep the sync one.
    ///
    /// The per-wallet keychain unlock stays in the MainActor epilogue
    /// (it is actor-isolated and marker-cached) and is timed separately —
    /// its measured share decides whether it ever moves too.
    ///
    /// Participates in [`shutdown()`]'s admission/drain exactly like the
    /// async `createWallet`: rejected up front while a shutdown drains,
    /// and once admitted the teardown waits for the full transaction.
    @discardableResult
    public func loadFromPersistor() async throws -> [ManagedPlatformWallet] {
        try ensureConfigured()
        try admitNativeOp("loadFromPersistor")
        defer { finishNativeOp() }

        let h = handle
        let handler = persistenceHandler
        let calls = nativeLoadCalls

        // Direct continuation for the same FIFO reason as the async
        // create: an admitted load is enqueued on the shared queue before
        // any later shutdown's teardown block.
        let outcome: OffMainLoadOutcome = await withCheckedContinuation { continuation in
            Self.destroyQueue.async {
                continuation.resume(
                    returning: Self.performLoadFromPersistor(h, handler: handler, calls: calls))
            }
        }
        try outcome.bulkResult.get()

        // Own every returned wallet handle IMMEDIATELY, before anything can
        // throw: if the defensive guard below fires, dropping the wrappers
        // releases the Rust-side aliases through their deinit instead of
        // leaking raw handles in the global registry. Nothing is published
        // until the guard has passed. The sequence order is preserved.
        enum OwnedLookup {
            case wallet(ManagedPlatformWallet)
            case skipped(PlatformWalletError)
        }
        let owned: [OwnedLookup] = outcome.lookups.map { entry in
            switch entry {
            case .restored(let walletId, let walletHandle):
                return .wallet(ManagedPlatformWallet(handle: walletHandle, walletId: walletId))
            case .skipped(let error):
                return .skipped(error)
            }
        }

        // Defense in depth only — the shutdown drain waits for this op, so
        // the handle cannot have been torn down (see the async create's
        // matching guard).
        guard handle != NULL_HANDLE else {
            assertionFailure("shutdown took the handle under an admitted load despite the drain")
            throw PlatformWalletError.invalidHandle(
                "manager was shut down while loadFromPersistor ran off-main")
        }

        // Replay in sequence, exactly like the sync overload's per-wallet
        // loop: a skipped lookup assigns its error to `lastError` in place,
        // a restored wallet is published and then unlocked before the next
        // entry — so `lastError` ends up reflecting the SAME (latest-in-
        // order) failure either overload would leave behind. The unlock
        // share is accumulated separately: it is the remaining MainActor
        // cost of the load and the data for deciding whether it ever moves
        // off-main.
        var restored: [ManagedPlatformWallet] = []
        var unlockSeconds: TimeInterval = 0
        for entry in owned {
            switch entry {
            case .skipped(let error):
                self.lastError = error
            case .wallet(let managedWallet):
                restored.append(managedWallet)
                self.wallets[managedWallet.walletId] = managedWallet
                let unlockStarted = CFAbsoluteTimeGetCurrent()
                unlockRestoredWalletLoggingOutcome(managedWallet)
                unlockSeconds += CFAbsoluteTimeGetCurrent() - unlockStarted
            }
        }
        let unlockMs = Int(unlockSeconds * 1000)
        SDKLogger.event(
            "wallet_restore_completed",
            category: .lifecycle,
            fields: [
                "unlock_duration_ms": .integer(Int64(unlockMs)),
                "wallet_count": .integer(Int64(restored.count)),
            ]
        )

        catchUpStuckAssetLocks(wallets: restored)
        return restored
    }

    /// The per-wallet unlock body shared by both `loadFromPersistor`
    /// overloads: verify the Keychain-resolved seed binds to the restored
    /// wallet and drain any deferred contact-crypto. Best-effort, per
    /// wallet — a wallet with no stored mnemonic (genuine watch-only)
    /// stays watch-only, and any unlock error is logged-and-continued
    /// (feeding `lastError`) so one wallet can't fail the whole restore.
    private func unlockRestoredWalletLoggingOutcome(_ managedWallet: ManagedPlatformWallet) {
        let walletId = managedWallet.walletId
        do {
            let unlocked = try unlockWalletFromKeychain(managedWallet)
            SDKLogger.event(
                "wallet_unlock_completed",
                category: .lifecycle,
                fields: [
                    "result": .publicText(unlocked ? "seed_verified" : "watch_only"),
                    "wallet_reference": .reference(walletId),
                ]
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
                SDKLogger.event(
                    "wallet_unlock_rejected",
                    category: .lifecycle,
                    severity: .error,
                    fields: [
                        "reason": .publicText("seed_binding_mismatch"),
                        "wallet_reference": .reference(walletId),
                    ],
                    error: error
                )
            } else {
                // Transient (resolver/Keychain unavailable, …) — not
                // retried this pass; a later signer-present action re-tries.
                SDKLogger.event(
                    "wallet_unlock_failed",
                    category: .lifecycle,
                    severity: .warning,
                    fields: [
                        "transient": .boolean(true),
                        "wallet_reference": .reference(walletId),
                    ],
                    error: error
                )
            }
            self.lastError = error
        } catch {
            SDKLogger.event(
                "wallet_unlock_failed",
                category: .lifecycle,
                severity: .error,
                fields: ["wallet_reference": .reference(walletId)],
                error: error
            )
            self.lastError = error
        }
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
                SDKLogger.event(
                    "seed_binding_verified",
                    category: .lifecycle,
                    fields: [
                        "marker_persisted": .boolean(true),
                        "wallet_reference": .reference(walletId),
                    ]
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
            SDKLogger.event(
                "contact_crypto_drain_skipped",
                category: .lifecycle,
                severity: .debug,
                fields: [
                    "reason": .publicText("empty_queue"),
                    "wallet_reference": .reference(walletId),
                ]
            )
            return true
        }
        SDKLogger.event(
            "contact_crypto_drain_scheduled",
            category: .lifecycle,
            fields: [
                "pending_count": .unsignedInteger(UInt64(pendingOps)),
                "wallet_reference": .reference(walletId),
            ]
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
            SDKLogger.event(
                "contact_crypto_drain_failed",
                category: .lifecycle,
                severity: .warning,
                fields: ["wallet_reference": .reference(walletId)],
                error: drainError
            )
        } else if drained > 0 {
            // `drained` counts cleared queue entries — both completed
            // and permanently-failed (channel-broken) ops — so report
            // it neutrally rather than implying all succeeded.
            SDKLogger.event(
                "contact_crypto_drain_completed",
                category: .lifecycle,
                fields: [
                    "processed_count": .unsignedInteger(UInt64(drained)),
                    "wallet_reference": .reference(walletId),
                ]
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
            // A `@MainActor` closure is the only piece of `self` the
            // detached task needs: it hops back to the main actor to
            // publish, and capturing it (rather than `self`) keeps the
            // task's captures Sendable under strict concurrency.
            let publishConflict: @MainActor @Sendable (PlatformWalletError) -> Void = {
                [weak self] verdict in
                self?.lastError = verdict
            }
            Task.detached(priority: .background) {
                await withTaskGroup(of: PlatformWalletError?.self) { group in
                    let maxConcurrent = 4
                    var nextIndex = 0
                    var published = false
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
                    // As each finishes, queue the next pending entry —
                    // and publish the FIRST double-spend verdict the
                    // moment its own task returns. A sibling catch-up
                    // can legitimately sit in its proof wait, and the
                    // host must not wait on that drain to learn a lock
                    // is stuck. `lastError` is the manager's one public
                    // error surface; a UI that explains the stalled lock
                    // and its pending retry (48 — the only verdict
                    // emitted; 47 stays reserved) reads it from here.
                    //
                    // The verdict arrives at the END of its own lock's
                    // bounded wait, not ahead of it: Rust deliberately
                    // does not refuse a resume on a conflict sighting,
                    // because the sighting can be a restored block record
                    // that no live event will ever retract, and refusing
                    // would strand a lock that is free to confirm. The
                    // wait it runs under is capped below this call's 300s
                    // ceiling for exactly that case.
                    while let outcome = await group.next() {
                        if !published, let verdict = outcome {
                            published = true
                            await publishConflict(verdict)
                        }
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
    /// Returns the typed double-spend verdict when the catch-up hits one —
    /// the one outcome a host must see so its UI can explain why the lock
    /// is stuck instead of spinning — and `nil` for every expected failure.
    /// In practice that verdict is always the provisional
    /// `assetLockInputContested`; the terminal `assetLockInputConflict` is
    /// reserved with no emitter and is matched so it would surface intact
    /// if that ever changes.
    nonisolated private static func runCatchUp(assetLockManager: ManagedAssetLockManager, txid: Data, vout: UInt32) -> PlatformWalletError? {
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
        // the production resume path uses. Wrapping the raw struct in
        // `PlatformWalletResult` frees the Rust-owned message when the
        // wrapper deinits — the raw struct must never be dropped bare.
        let result = PlatformWalletResult(
            asset_lock_manager_catch_up_blocking(
                assetLockManager.handle, &txidTuple, vout, 300
            )
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
        switch result.code {
        case .errorInvalidHandle:
            NSLog(
                "[catch-up] asset_lock_manager_catch_up_blocking returned errorInvalidHandle for outpoint %@:%u — handle invalid despite task-owned wrapper retain",
                txid.map { String(format: "%02x", $0) }.joined(),
                vout
            )
            return nil
        case .errorAssetLockInputConflict, .errorAssetLockInputContested:
            return PlatformWalletError(result: result)
        default:
            return nil
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
        // Same synchronous-admission gate as the sync creates and load: a
        // deletion interleaved with an in-flight async restore could remove
        // the native registration between the restore's snapshot read and
        // its publication (or wipe SwiftData/Keychain that the loader then
        // re-inserts), violating this method's full-wipe semantics.
        try ensureSyncNativeOpAllowed("deleteWallet")
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
