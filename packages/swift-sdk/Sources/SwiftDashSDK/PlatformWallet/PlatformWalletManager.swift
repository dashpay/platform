import Foundation
import SwiftData
import Combine
import DashSDKFFI

/// Lock-guarded monotonic generation counter, safe to read and bump from
/// any thread. Used to drop sync completion events that belong to a
/// generation already superseded by a `stop`/`clear`/`reset`, even when a
/// restart happens in the same `@MainActor` turn (a plain boolean gate
/// can't, because the restart re-opens the gate before the stale,
/// previously-enqueued completion task runs). Shared by the shielded and
/// platform-address sync paths.
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
    // MARK: - Published observables

    /// Whether [`configure`] has been called successfully.
    @Published public private(set) var isConfigured: Bool = false

    /// The current SPV sync progress. Updated by the polling task
    /// started in [`configure`].
    @Published public private(set) var spvProgress: PlatformSpvSyncProgress = .empty

    @Published public private(set) var spvIsRunning: Bool = false

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
    /// `stop` cancels the in-flight pass before its completion fires;
    /// `clear` goes further with a full Rust-side quiesce. Either way, an
    /// already-dispatched completion event can still land on this
    /// `@MainActor` after stop/clear returns. A plain boolean gate is
    /// bypassable: a caller can stop (set the flag) and restart (clear it)
    /// in the same actor turn, re-opening the gate before the stale
    /// completion task runs — so the old event leaks into the new run.
    /// Tying suppression to a generation closes that race: the stale task
    /// carries the pre-stop generation, the restart does not reset the
    /// counter, so the snapshot mismatches and the event is dropped even
    /// on a same-turn restart.
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

    /// Wallets Rust skipped during the most recent [`loadFromPersistor`]
    /// because their persisted row was structurally corrupt. Empty after
    /// a clean load. Rust treats these as non-fatal — the load still
    /// succeeds — so they are surfaced here rather than through
    /// [`lastError`], letting UI offer to inspect / clear the bad rows.
    @Published public private(set) var lastLoadSkippedWallets: [SkippedWalletOnLoad] = []

    // MARK: - Internals

    /// FFI handle; `NULL_HANDLE` until [`configure`] is called.
    internal private(set) var handle: Handle = NULL_HANDLE

    /// Retained for the lifetime of the FFI handle so the callback
    /// context pointer remains valid. `nonisolated(unsafe)` so the
    /// nonisolated `deinit` can `passRetained` the handler on an
    /// incomplete-shutdown without Swift 6 strict-concurrency rejecting
    /// the cross-isolation access — the FFI lifetime contract makes the
    /// access safe (the handler is only ever read after the Rust side
    /// has signalled the worker is wedged + the wrapper class itself is
    /// being destroyed).
    nonisolated(unsafe) private var persistenceHandler: PlatformWalletPersistenceHandler?

    /// SwiftData container + network captured at `configure`, used to build a
    /// `KeychainSigner` (the identity document signer) for the unlock-time
    /// auto-accept drain. Nil when configured without persistence (no Keychain
    /// signing possible → the drain runs provider-only).
    private var modelContainer: ModelContainer?
    private var signerNetwork: Network?

    /// Retained for the lifetime of the FFI handle so the event-handler
    /// context pointer remains valid. See `persistenceHandler` for the
    /// `nonisolated(unsafe)` rationale.
    nonisolated(unsafe) private var eventHandler: PlatformWalletEventHandler?

    /// Background task that polls SPV progress.
    private var progressPollTask: Task<Void, Never>?

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
        guard handle != NULL_HANDLE else { return }

        // Tear down the Rust manager: signal the host-driven sync loops
        // (platform-address, shielded, DashPay) to cancel and `discard()`
        // them — all cancel-only on the Rust side and none report an
        // incomplete drain. Identity-sync, the DashPay drain, and the event
        // adapter are joined inside `destroy` (which also drains the three
        // loops signalled here), the single host-visible join point.
        platform_wallet_manager_platform_address_sync_stop(handle).discard()
        platform_wallet_manager_shielded_sync_stop(handle).discard()
        platform_wallet_manager_dashpay_sync_stop(handle).discard()

        // Capture the CODE (not just free the message) for the one call
        // that CAN report `.errorShutdownIncomplete`: `destroy`. Rust
        // returns that code when a background coordinator did not drain
        // within the join deadline, or when a prior-generation shielded
        // thread is still parked alive as an orphan (a tight
        // `stop()`→`start()` reap that had to detach it past the wedge
        // backstop). In either case a lingering `!Send` coordinator
        // thread may still hold the `passUnretained` context pointers
        // Rust was handed for our `persistenceHandler` / `eventHandler`
        // and fire ONE final callback through them. The contract: on
        // that code the host must NOT free the callback context
        // immediately.
        let destroyCode =
            platform_wallet_manager_destroy(handle).discardReturningCode()

        // Both handlers are passed to Rust via `Unmanaged.passUnretained`
        // (see `PlatformWalletPersistenceHandler`/`PlatformWalletEventHandler`
        // `makeCallbacks()`), so Rust holds non-owning pointers and these
        // objects are kept alive ONLY by the stored properties below. The
        // instant this deinit returns, ARC releases them — which would be a
        // use-after-free if a lingering coordinator then fires its final
        // callback. So, ONLY on an incomplete shutdown, deliberately leak one
        // extra strong reference to each (an unbalanced `passRetained` that is
        // never released) so they outlive any lingering thread. A clean
        // shutdown (the common case) takes neither branch and releases the
        // handlers normally — we never leak unconditionally. The leak is
        // bounded by how often a shutdown wedges (rare) and trades two small
        // objects for guaranteed callback safety, since an incomplete drain
        // gives no later signal that the lingering thread has finally exited.
        if destroyCode == .errorShutdownIncomplete {
            if let persistenceHandler { _ = Unmanaged.passRetained(persistenceHandler) }
            if let eventHandler { _ = Unmanaged.passRetained(eventHandler) }
        }
    }

    // MARK: - Configuration

    /// Configure the manager with an SDK and an optional SwiftData
    /// container. Must be called before any wallet operations.
    ///
    /// Spawns a background task that polls SPV sync progress every
    /// second and publishes it to [`spvProgress`].
    public func configure(sdk: SDK, modelContainer: ModelContainer? = nil) throws {
        precondition(!isConfigured, "PlatformWalletManager already configured")
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
        var handle: Handle = NULL_HANDLE

        let handler: PlatformWalletPersistenceHandler?
        var persistence: PersistenceCallbacks
        if let container = modelContainer {
            let h = PlatformWalletPersistenceHandler(
                modelContainer: container,
                network: network
            )
            persistence = h.makeCallbacks()
            handler = h
        } else {
            persistence = PersistenceCallbacks()
            handler = nil
        }

        let eventHandler = PlatformWalletEventHandler(manager: self)
        var eventHandlerCallbacks = eventHandler.makeCallbacks()

        try platform_wallet_manager_create(
            sdkPointer,
            &persistence,
            &eventHandlerCallbacks,
            &handle
        ).check()

        self.handle = handle
        self.persistenceHandler = handler
        self.eventHandler = eventHandler
        self.modelContainer = modelContainer
        self.signerNetwork = network
        self.isConfigured = true

        startProgressPolling()
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

    /// One wallet Rust skipped during `load_from_persistor` because its
    /// persisted row was skipped. `reasonCode` is one of the Rust-side
    /// `LOAD_SKIP_REASON_*` constants (100 missing manifest, 101 malformed
    /// xpub, 102 decode error, 103 snapshot identity mismatch, 199 other
    /// corrupt row, 200 other skip, 300 already registered);
    /// [`reasonDescription`] renders it for display.
    public struct SkippedWalletOnLoad {
        public let walletId: Data
        public let reasonCode: UInt32

        /// Human-readable rendering of `reasonCode`, mirroring the Rust
        /// `LOAD_SKIP_REASON_*` constants. These numbers are the wire
        /// contract defined in `rs-platform-wallet-ffi/src/manager.rs`;
        /// they are not surfaced as named symbols in the generated C
        /// header, so the cases are matched by value against that source.
        public var reasonDescription: String {
            switch reasonCode {
            case 100: return "missing account manifest"
            case 101: return "malformed account xpub"
            case 102: return "decode error"
            case 103: return "snapshot does not match its persisted row"
            case 199: return "other corrupt row"
            case 200: return "other skip"
            case 300: return "already registered"
            default: return "unknown skip reason (\(reasonCode))"
            }
        }
    }

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

        // Consume the load outcome so Rust's per-wallet skip summary
        // isn't discarded. Rust writes `out_outcome` on every path
        // (including early errors), so freeing it is safe even if the
        // `.check()` below throws — defer the free before that throwing
        // call.
        var outcome = LoadOutcomeFFI(loaded_count: 0, skipped_count: 0, skipped: nil)
        let loadResult = platform_wallet_manager_load_from_persistor(handle, &outcome)
        defer { platform_wallet_load_outcome_free(&outcome) }

        // Collect the ids Rust skipped (a corrupt row, or one already
        // registered). These are non-fatal on the Rust side, so they must
        // not reach `lastError`: they are surfaced through
        // `lastLoadSkippedWallets` and their ids are excluded from the
        // per-id restore loop below (a skipped id is still in SwiftData, so
        // `get_wallet` would return NotFound for it).
        //
        // Rust writes `out_outcome` on every path (zeroed on a hard
        // failure), so assign `lastLoadSkippedWallets` BEFORE the throwing
        // `.check()` — a failed load then clears stale skip state from a
        // prior load instead of leaving it behind.
        var skippedIds = Set<Data>()
        var skippedWallets: [SkippedWalletOnLoad] = []
        if let skipped = outcome.skipped {
            skippedWallets.reserveCapacity(Int(outcome.skipped_count))
            for i in 0..<Int(outcome.skipped_count) {
                var entry = skipped[i]
                let walletId = withUnsafeBytes(of: &entry.wallet_id) { Data($0) }
                skippedIds.insert(walletId)
                let skip = SkippedWalletOnLoad(walletId: walletId, reasonCode: entry.reason_code)
                skippedWallets.append(skip)
                NSLog(
                    "[load-from-persistor] skipped wallet %@ — %@",
                    walletId.prefix(4).map { String(format: "%02x", $0) }.joined(),
                    skip.reasonDescription
                )
            }
        }
        self.lastLoadSkippedWallets = skippedWallets

        try loadResult.check()

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
            // Rust already skipped this id as corrupt; `get_wallet` would
            // return NotFound and pollute `lastError`. Skip it here.
            if skippedIds.contains(walletId) { continue }
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
                    print(
                        "🔓 wallet unlock \(walletId.toHexString().prefix(8)): "
                            + (unlocked ? "seed verified — drain scheduled" : "no mnemonic — stays watch-only")
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
    /// 1. **Verify** the resolved seed binds to this wallet —
    ///    `platform_wallet_verify_seed_binds_to_wallet` derives the BIP44
    ///    account-0 xpub through the resolver and compares it to the
    ///    persisted one. A mis-mapped Keychain slot derives a different xpub
    ///    and the call throws, so a wrong seed never signs for this wallet.
    /// 2. **Drain** (in the background) any contact-crypto deferred while
    ///    the wallet was seedless — `platform_wallet_drain_pending_contact_crypto`.
    ///    The drain re-fetches + decrypts over the network, so it runs in a
    ///    detached task off the caller's thread.
    ///
    /// Per the Swift-SDK FFI boundary rules, the mnemonic → seed conversion
    /// happens entirely inside the resolver vtable in Rust; Swift only
    /// checks the Keychain entry's existence (`hasMnemonic`) and never pulls
    /// the plaintext across.
    ///
    /// - Parameter wallet: the restored `ManagedPlatformWallet`.
    /// - Returns: `true` if the wallet's seed verified (drain scheduled);
    ///   `false` if no mnemonic is stored for this wallet (a genuine
    ///   watch-only wallet), without throwing.
    /// - Throws: `PlatformWalletError` if the verify FFI fails (e.g. the
    ///   resolved seed does not bind — a mis-mapped Keychain slot).
    @discardableResult
    public func unlockWalletFromKeychain(_ wallet: ManagedPlatformWallet) throws -> Bool {
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
        guard WalletStorage().hasMnemonic(for: walletId) else {
            return false
        }

        let walletHandle = wallet.handle
        // Resolver-backed signer: the mnemonic is fetched from the Keychain
        // inside the resolver vtable Rust-side; no resident seed.
        let coreSigner = MnemonicResolver()

        // Wrong-seed / wrong-wallet gate. `withExtendedLifetime` keeps the
        // resolver alive across the synchronous FFI call (its vtable callback
        // fires during it). Throws if the resolved seed derives a different
        // BIP44 account-0 xpub than the wallet's persisted one.
        //
        // Publish the per-wallet `seedMismatch` from the verify result itself,
        // scoped to JUST this call: the verify FFI maps Rust `SeedMismatch` →
        // `.invalidParameter`, and scoping the catch here keeps the earlier
        // 32-byte `walletId` precondition (also `.invalidParameter`) from being
        // mistaken for a seed mismatch. Rethrow so the existing caller handling
        // (loadFromPersistor's log-and-continue) is unchanged.
        do {
            try withExtendedLifetime(coreSigner) {
                try platform_wallet_verify_seed_binds_to_wallet(
                    walletHandle,
                    coreSigner.handle
                ).check()
            }
            setDashPaySeedMismatch(walletId, false)
        } catch let error as PlatformWalletError {
            if case .invalidParameter = error {
                setDashPaySeedMismatch(walletId, true)
            }
            throw error
        }

        // Heal pre-breadcrumb identity keys so they sign via the resolver
        // (derive-sign-destroy) rather than the stored scalar. Idempotent and
        // Keychain-sourced; runs once the seed is confirmed present for this
        // wallet, which is exactly when its identity keys become signable.
        // Fire-and-forget off the main actor — signing falls back to the stored
        // scalar until this heals, so it never needs to block unlock.
        persistenceHandler?.scheduleBackfillIdentityKeyBreadcrumbs(walletId: walletId)

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

    private func ensureConfigured() throws {
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
