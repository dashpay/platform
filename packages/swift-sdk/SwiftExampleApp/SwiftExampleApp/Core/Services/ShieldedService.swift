// ShieldedService.swift
// SwiftExampleApp
//
// Single UI mirror + multi-engine-bind for the Rust-owned shielded
// (Orchard) sync coordinator.
//
// The service mirrors exactly ONE wallet — the app-level
// `firstWallet` — for the GLOBAL Sync-status surface: `bind(...)`
// attaches the published mirror (`boundWalletId`, `shieldedBalance`,
// subscriptions, timing) to that wallet and drives the Sync tab. It
// does not own any of the shielded crypto: bind, sync, and
// persistence all live on the Rust `platform-wallet` side.
//
// `bindEngine(...)` is the additive companion used by
// `rebindWalletScopedServices()` to engine-register EVERY OTHER
// loaded wallet into the same network-scoped coordinator (no mirror
// repoint). A single shielded sync pass then trial-decrypts against
// the union of all wallets' viewing keys and routes note hits to each
// wallet's own persister (SH-14/15/16 cross-wallet flows). Per-wallet
// receive addresses and balances are read on demand
// (`walletManager.shieldedDefaultAddress(walletId:)`,
// `PersistentShieldedNote` rows) rather than from this singleton
// mirror.

import Foundation
import SwiftUI
import Combine
import SwiftData
import SwiftDashSDK

/// Observable service mirroring Rust-owned shielded sync state.
@MainActor
class ShieldedService: ObservableObject {
    // MARK: - Published state

    /// Whether a shielded sync pass is currently in flight.
    @Published var isSyncing: Bool = false

    /// Current shielded balance reported by the most recent sync.
    @Published var shieldedBalance: UInt64 = 0

    /// New decrypted notes detected on the most recent sync pass.
    @Published var lastNewNotes: UInt32 = 0

    /// Notes newly detected as spent on the most recent sync pass.
    @Published var lastNewlySpent: UInt32 = 0

    /// Whether the bound wallet has a shielded sub-wallet on the Rust
    /// side. Until [`bind`] runs successfully every pass marks the
    /// wallet as `skipped` and we surface that here so the UI can
    /// show a clear "not yet bound" state instead of stale zeros.
    @Published var isBound: Bool = false

    /// Local clock timestamp of the last completed sync pass.
    @Published var lastSyncTime: Date?

    /// Number of successful shielded sync passes observed since
    /// launch (skipped passes don't count).
    @Published var syncCountSinceLaunch: Int = 0

    /// Cumulative encrypted notes scanned since launch — sum of
    /// every pass's `total_scanned`.
    @Published var totalScanned: UInt64 = 0

    /// Cumulative decrypted notes accepted since launch.
    @Published var totalNewNotes: UInt64 = 0

    /// Cumulative notes newly detected as spent since launch.
    @Published var totalNewlySpent: UInt64 = 0

    /// Last error from a shielded operation. Cleared on a successful
    /// pass.
    @Published var lastError: String?

    /// Bech32m-encoded Orchard payment address for account 0.
    /// Kept for the existing Receive sheet which is still
    /// single-account; multi-account-aware UI uses
    /// `addressesByAccount` instead.
    @Published var orchardDisplayAddress: String?

    /// Bound shielded ZIP-32 accounts, in ascending order. Driven
    /// by `bind` — every entry of `accounts:` becomes a row here.
    @Published var boundAccounts: [UInt32] = []

    /// Bech32m-encoded Orchard payment address per bound account.
    /// Populated alongside `boundAccounts` from per-account
    /// `shieldedDefaultAddress` calls. Empty for accounts that
    /// failed to bind.
    @Published var addressesByAccount: [UInt32: String] = [:]

    // MARK: - Internals

    /// Wallet manager whose shielded sync events we mirror.
    private weak var walletManager: PlatformWalletManager?

    /// Wallet id we filter sync results by. Exposed read-only so
    /// diagnostics views (e.g. Sync Status) can resolve persisted
    /// per-account watermarks without each one knowing the active
    /// wallet through a separate path.
    @Published private(set) var boundWalletId: Data?

    /// Network of the currently-bound wallet. Stashed so
    /// `switchTo(walletId:)` can reach the right per-network
    /// dbPath without re-plumbing it from the call site.
    private var network: Network?

    /// Mnemonic resolver stashed from the first `bind`. Reused by
    /// `switchTo(walletId:)` so detail views can rebind without
    /// pulling a fresh resolver out of the SwiftUI environment.
    private var resolver: MnemonicResolver?

    /// Subscription to `walletManager.$shieldedSyncIsSyncing`.
    private var syncStateCancellable: AnyCancellable?

    /// Subscription to `walletManager.$lastShieldedSyncEvent`.
    private var syncEventCancellable: AnyCancellable?

    // MARK: - Timing instrumentation
    //
    // Surfaces wall-clock of each sync pass so devnet stress tests
    // (1M shielded notes via `dashpay/drive:3.1-shielded.*`) can be
    // measured from the iOS client. See
    // `docs/shielded-sync-timing-spec.md` for the design.

    /// Wall-clock of the most recent NON-cooldown completed sync
    /// pass. Nil until the first such pass after `bind()`.
    @Published var lastSyncDuration: TimeInterval?

    /// Maximum wall-clock observed across every completed sync pass
    /// since the last `bind()` / `reset()` / `clearLocalState`. Used
    /// to preserve the cold-sync headline number when subsequent
    /// steady-state passes (3 s deltas) would otherwise clobber
    /// `lastSyncDuration` in the UI. Nil until the first completed
    /// pass.
    @Published var longestSyncDuration: TimeInterval?

    /// Running wall-clock of the in-flight sync pass. Updated by a
    /// 1Hz timer while `isSyncing == true`; nil otherwise.
    @Published var currentSyncElapsed: TimeInterval?

    /// Cumulative encrypted notes scanned in the in-flight pass.
    /// Republished from `PlatformWalletManager.currentShieldedSyncScanned`
    /// — fired once per chunk (~2048 notes) by the Rust progress
    /// callback. Nil between passes. Lets the UI render a live
    /// counter / ProgressView during a cold sync.
    @Published var currentSyncScanned: UInt64?

    /// Latest Platform block height observed during the in-flight
    /// pass. Pairs with `currentSyncScanned` (same callback).
    @Published var currentSyncBlockHeight: UInt64?

    /// Cumulative note commitments appended to the local Orchard tree
    /// in the in-flight pass — the "checked / committed-to-tree"
    /// signal, distinct from `currentSyncScanned` (which counts
    /// *downloaded* notes). Republished from
    /// `PlatformWalletManager.currentShieldedTreeCommitted`, fired once
    /// per committed batch by the Rust tree-progress callback. Nil
    /// between passes.
    @Published var currentTreeCommitted: UInt64?

    /// On-chain MMR total leaf count, the denominator for both the
    /// "downloaded" and "checked" bars (total notes == total leaves).
    /// Pairs with `currentTreeCommitted` (same callback). A value of 0
    /// (or nil) means the total is indeterminate — render a spinner
    /// rather than a determinate bar.
    @Published var currentTreeTotal: UInt64?

    /// Subscription to `walletManager.$currentShieldedSyncScanned`
    /// and `…BlockHeight` for live progress. Created in `bind` /
    /// `bind`, dropped in `reset` / `clearLocalState`.
    private var progressCancellable: AnyCancellable?

    /// Subscription to `walletManager.$currentShieldedTreeCommitted`
    /// and `…Total` for the "checked / committed-to-tree" bar. Created
    /// in `bind`, dropped in `reset` / `clearLocalState`.
    private var treeProgressCancellable: AnyCancellable?

    /// `Date()` at the moment `isSyncing` flipped false → true.
    /// Drives both `lastSyncDuration` (at completion) and
    /// `currentSyncElapsed` (live).
    private var currentSyncStartedAt: Date?

    /// 1Hz timer that ticks `currentSyncElapsed` while syncing.
    /// Started on false→true edge, invalidated on true→false edge.
    private var syncTickTimer: Timer?

    // MARK: - Lifecycle

    /// Bind the service to a wallet. Drives `bindShielded` on the
    /// Rust side first (resolver-driven mnemonic lookup, ZIP-32
    /// derivation per `accounts`, per-network commitment tree
    /// open) and then subscribes to shielded sync events for
    /// `walletId`.
    ///
    /// `accounts` is the list of ZIP-32 account indices to bind.
    /// Defaults to `[0]` for the single-account default; pass
    /// `[0, 1, …]` to bind multiple accounts up front. Each
    /// gets its own subwallet bookkeeping inside the store; the
    /// commitment tree is shared per network.
    ///
    /// Failure during the Rust-side bind sets `lastError`; the
    /// service continues to subscribe to events so a successful
    /// `bind` retried later picks up automatically.
    func bind(
        walletManager: PlatformWalletManager,
        walletId: Data,
        network: Network,
        resolver: MnemonicResolver,
        accounts: [UInt32] = [0]
    ) {
        self.walletManager = walletManager
        self.boundWalletId = walletId
        self.network = network
        self.resolver = resolver
        self.syncStateCancellable?.cancel()
        self.syncEventCancellable?.cancel()
        self.progressCancellable?.cancel()
        self.treeProgressCancellable?.cancel()

        // Clear the previous wallet's snapshot up front. Without
        // this, switching wallets (or a failed rebind) leaves the
        // prior wallet's balance / counters / orchard address on
        // the UI until the new wallet's first sync event lands —
        // which can be tens of seconds, or never if the new bind
        // fails. Per-published-field reset rather than `reset()`
        // because the manager subscriptions get re-attached just
        // below; we don't want to nil out walletManager/walletId.
        isBound = false
        isSyncing = false
        shieldedBalance = 0
        lastNewNotes = 0
        lastNewlySpent = 0
        lastSyncTime = nil
        lastError = nil
        orchardDisplayAddress = nil
        boundAccounts = []
        addressesByAccount = [:]
        syncCountSinceLaunch = 0
        totalScanned = 0
        totalNewNotes = 0
        totalNewlySpent = 0
        lastSyncDuration = nil
        longestSyncDuration = nil
        currentSyncElapsed = nil
        currentSyncStartedAt = nil
        currentSyncScanned = nil
        currentSyncBlockHeight = nil
        currentTreeCommitted = nil
        currentTreeTotal = nil
        syncTickTimer?.invalidate()
        syncTickTimer = nil

        let dbPath = Self.dbPath(for: network)
        let sortedAccounts = Array(Set(accounts)).sorted()
        do {
            // The per-network SQLite tree handle now lives on the
            // manager (one shared `NetworkShieldedCoordinator`),
            // not per-wallet. `configureShielded` is idempotent at
            // the path level — first call opens the file, every
            // subsequent same-path call no-ops. Has to run before
            // `bindShielded`; doing it inline keeps the call shape
            // simple and avoids a separate bootstrap step.
            try walletManager.configureShielded(dbPath: dbPath)
            try walletManager.bindShielded(
                walletId: walletId,
                resolver: resolver,
                accounts: sortedAccounts
            )
            isBound = true
            lastError = nil
            boundAccounts = sortedAccounts

            // Populate per-account default addresses. Best-effort —
            // a failure on any one account leaves that entry
            // missing from `addressesByAccount` (the row in the UI
            // shows blank) but doesn't unbind the wallet.
            for account in sortedAccounts {
                if let raw = try? walletManager.shieldedDefaultAddress(
                    walletId: walletId,
                    account: account
                ) {
                    addressesByAccount[account] = DashAddress.encodeOrchard(
                        rawBytes: raw,
                        network: network
                    )
                }
            }
            // Backwards-compat: `orchardDisplayAddress` still drives
            // the existing Receive sheet which only renders one
            // address. Use account 0 if bound, else the lowest
            // bound account.
            let primary = sortedAccounts.contains(0) ? 0 : (sortedAccounts.first ?? 0)
            orchardDisplayAddress = addressesByAccount[primary]

            SDKLogger.log(
                "Shielded bound: walletId=\(walletId.prefix(4).map { String(format: "%02x", $0) }.joined())… network=\(network.networkName) accounts=\(sortedAccounts) tree=\(dbPath)",
                minimumLevel: .medium
            )
        } catch {
            lastError = "Shielded bind failed: \(error.localizedDescription)"
            SDKLogger.log(lastError ?? "", minimumLevel: .medium)
        }

        syncStateCancellable = walletManager.$shieldedSyncIsSyncing
            .sink { [weak self] newValue in
                guard let self else { return }
                let wasSyncing = self.isSyncing
                self.isSyncing = newValue
                // Detect false → true edge. `.sink` fires on every
                // republished value, including duplicates, so we
                // gate on the previous mirror to avoid log spam. The
                // timing bracket itself is opened by
                // `beginSyncTimingIfNeeded()`, which is idempotent —
                // `manualSync()`'s fast path may have already opened it
                // for a pass that completes before this publisher flips.
                if newValue && !wasSyncing {
                    self.beginSyncTimingIfNeeded()
                    SDKLogger.log(
                        "Shielded sync started",
                        minimumLevel: .medium
                    )
                }
                // Detect true → false edge. Tear down the ticker and
                // zero the live elapsed, but do NOT clear
                // `currentSyncStartedAt` here —
                // `handleShieldedSyncEvent` still needs it to compute
                // `lastSyncDuration` and calls `endSyncTiming()` itself
                // on every terminal path. This branch only covers the
                // edge where the publisher flips false with no paired
                // event (defensive); the event handler is the
                // authoritative close.
                if !newValue && wasSyncing {
                    self.syncTickTimer?.invalidate()
                    self.syncTickTimer = nil
                    self.currentSyncElapsed = nil
                }
            }

        syncEventCancellable = walletManager.$lastShieldedSyncEvent
            .sink { [weak self] event in
                guard let self, let event else { return }
                self.handleShieldedSyncEvent(event)
            }

        // Bridge per-chunk progress from the manager. Pair
        // `currentShieldedSyncScanned` and `…BlockHeight`; they're
        // emitted by the same Rust callback so a `combineLatest`
        // round-trips them coherently into our two @Published mirrors.
        progressCancellable = walletManager.$currentShieldedSyncScanned
            .combineLatest(walletManager.$currentShieldedSyncBlockHeight)
            .sink { [weak self] scanned, height in
                guard let self else { return }
                self.currentSyncScanned = scanned
                self.currentSyncBlockHeight = height
            }

        // Bridge the second "checked / committed-to-tree" signal from
        // the manager. Pair `currentShieldedTreeCommitted` and `…Total`;
        // they're emitted by the same Rust callback so a `combineLatest`
        // round-trips them coherently into our two @Published mirrors.
        treeProgressCancellable = walletManager.$currentShieldedTreeCommitted
            .combineLatest(walletManager.$currentShieldedTreeTotal)
            .sink { [weak self] committed, total in
                guard let self else { return }
                self.currentTreeCommitted = committed
                self.currentTreeTotal = total
            }
    }

    /// Register `walletId`'s shielded sub-wallet with the Rust
    /// coordinator WITHOUT repointing this service's display mirror.
    ///
    /// `bind(...)` attaches the single UI mirror (boundWalletId,
    /// shieldedBalance, subscriptions, …) to exactly one wallet — the
    /// app-level `firstWallet`. `bindEngine(...)` is the additive
    /// companion: it engine-binds EVERY OTHER loaded wallet into the
    /// same network-scoped coordinator so a single shielded sync pass
    /// trial-decrypts against the union of all wallets' viewing keys and
    /// routes note hits to each wallet's own persister. Per-wallet
    /// receive addresses and balances are then read on demand
    /// (`walletManager.shieldedDefaultAddress(walletId:)`,
    /// `PersistentShieldedNote` rows) rather than from this singleton
    /// mirror.
    ///
    /// Best-effort and independent per wallet: a missing mnemonic /
    /// declined resolver for one wallet logs and returns without
    /// affecting the others or the mirror. Idempotent — safe to call
    /// every rebind pass (`configureShielded` no-ops on the same path;
    /// `bindShielded` replaces that wallet's registration).
    ///
    /// Returns whether the engine registration succeeded; existing
    /// callers may ignore it.
    @discardableResult
    func bindEngine(
        walletManager: PlatformWalletManager,
        walletId: Data,
        network: Network,
        resolver: MnemonicResolver,
        accounts: [UInt32] = [0]
    ) -> Bool {
        let dbPath = Self.dbPath(for: network)
        let sortedAccounts = Array(Set(accounts)).sorted()

        // No "already bound" fast path on purpose: the only cheap probe,
        // `shieldedDefaultAddress`, reflects the wallet-level sub-wallet
        // binding — which SURVIVES `clearShielded` (Clear drops only the
        // coordinator registrations; there is no sub-wallet unbind FFI).
        // Skipping on that signal would silently leave post-Clear wallets
        // unregistered (sync passes would never scan them again). Coordinator
        // registration has no cheap query, so we always re-bind; the
        // mnemonic read + ZIP-32 re-derivation is low-millisecond per wallet
        // and rebind fires are rare (wallet-set change, network switch,
        // Sync Now).
        do {
            try walletManager.configureShielded(dbPath: dbPath)
            try walletManager.bindShielded(
                walletId: walletId,
                resolver: resolver,
                accounts: sortedAccounts
            )
            SDKLogger.log(
                "Shielded engine-bound: walletId=\(walletId.prefix(4).map { String(format: "%02x", $0) }.joined())… network=\(network.networkName) accounts=\(sortedAccounts)",
                minimumLevel: .medium
            )
            return true
        } catch {
            SDKLogger.log(
                "Shielded engine-bind failed for walletId=\(walletId.prefix(4).map { String(format: "%02x", $0) }.joined())…: \(error.localizedDescription)",
                minimumLevel: .medium
            )
            return false
        }
    }

    /// Re-bind the singleton service to a different wallet using the
    /// `walletManager` / `resolver` / `network` stashed by the first
    /// `bind(...)`. Per-detail-view code paths call this when the
    /// user navigates into a wallet other than the one
    /// `rebindWalletScopedServices()` initially selected — without
    /// it, the published `shieldedBalance` stays pinned to the
    /// first-bound wallet and every detail screen shows that
    /// wallet's balance.
    ///
    /// No-op if the requested wallet is already bound. Logs and
    /// returns early if `bind(...)` was never called yet.
    func switchTo(walletId: Data) {
        if self.boundWalletId == walletId, isBound {
            return
        }
        guard
            let walletManager,
            let resolver,
            let network
        else {
            SDKLogger.log(
                "ShieldedService.switchTo called before initial bind — ignoring",
                minimumLevel: .medium
            )
            return
        }
        bind(
            walletManager: walletManager,
            walletId: walletId,
            network: network,
            resolver: resolver
        )
    }

    /// Trigger a manual shielded sync pass. No-op if a pass is
    /// already in flight.
    ///
    /// Drives `isSyncing` directly around the await so the spinner
    /// flashes even when the underlying Rust pass completes faster
    /// than the manager's 1 Hz `isShieldedSyncing` poll cadence —
    /// the published `$shieldedSyncIsSyncing` stays `false` the
    /// whole time on a fast (e.g. empty-tree) sync, so we can't
    /// rely on the subscription alone to flip it back.
    func manualSync() async {
        guard !isSyncing else { return }
        guard let walletManager else { return }

        // If we're unbound (typically because the user pressed
        // Clear earlier) but still have the bind credentials,
        // re-bind first so the next `syncShieldedNow()` call
        // has a Rust-side shielded sub-wallet to walk. Without
        // this, post-Clear Sync Now would no-op forever and the
        // user has no path back to a synced state from this
        // screen.
        if !isBound, canResume {
            guard
                let walletId = boundWalletId,
                let resolver,
                let network
            else { return }
            let accounts = boundAccounts.isEmpty ? [0] : boundAccounts
            bind(
                walletManager: walletManager,
                walletId: walletId,
                network: network,
                resolver: resolver,
                accounts: accounts
            )
            // Mirror bind is best-effort — on failure `lastError` is
            // already populated by `bind(...)`, and we still run the
            // engine pass below so other wallets with intact mnemonics
            // re-register (a mirror-only failure must not dark the whole
            // fleet).
        }

        // Re-register any loaded wallet that lost its engine binding — see
        // prior comment (post-Clear recovery): `clearShielded` drops EVERY
        // wallet, and a detail-view `switchTo` in between re-binds only the
        // wallet being viewed, so the recovery branch above may not even
        // run. This runs on every Sync Now: with no cheap
        // coordinator-registration probe (see `bindEngine`), each pass
        // re-derives every other wallet's keys (low-millisecond per wallet)
        // — the price of correct post-Clear re-registration. Track whether
        // ANYTHING is registered: if the mirror bind failed AND no other
        // wallet bound, a sync pass would skip every wallet and produce a
        // meaningless result over the bind error the user needs to see.
        var anyWalletRegistered = isBound
        if let mirrorWalletId = boundWalletId, let resolver, let network {
            engineBindOtherWallets(
                allWalletIds: walletManager.wallets.keys,
                mirrorWalletId: mirrorWalletId
            ) { otherWalletId in
                if bindEngine(
                    walletManager: walletManager,
                    walletId: otherWalletId,
                    network: network,
                    resolver: resolver
                ) {
                    anyWalletRegistered = true
                }
            }
        }
        // Nothing registered (mirror failed + every other bind failed, or no
        // bind credentials at all — the Sync Now button is disabled when
        // `!canResume`, so this mainly covers the all-binds-failed case):
        // bail rather than chain a sync that would skip every wallet —
        // preserves the pre-existing "don't chain a sync that will fail the
        // same way" intent, per-wallet-ized.
        guard anyWalletRegistered else { return }

        isSyncing = true
        lastError = nil
        // Open the timing bracket here too, not just on the
        // `$shieldedSyncIsSyncing` false→true edge. A fast pass
        // (e.g. empty-tree sync) can complete before that publisher
        // ever flips, so without this `currentSyncStartedAt` would be
        // nil at completion and `lastSyncDuration` would be dropped.
        // `beginSyncTimingIfNeeded()` is idempotent, so if the
        // publisher does flip first it simply no-ops there.
        beginSyncTimingIfNeeded()
        defer { isSyncing = false }
        do {
            try await walletManager.syncShieldedNow()
        } catch {
            lastError = "Shielded sync error: \(error.localizedDescription)"
            SDKLogger.log(lastError ?? "", minimumLevel: .medium)
        }

        // Restart the manager-wide shielded sync loop AFTER the
        // manual `syncShieldedNow()` call completes. `start()`
        // spawns a background thread whose first iteration calls
        // `sync_now(false)` immediately, and the manager's
        // `is_syncing` CAS in `sync_now` means whichever caller
        // gets there first wins — the other silently no-ops with
        // an empty summary. Starting *after* the manual pass
        // returns lets the user-initiated tap run uncontested,
        // and the loop's first tick happens on its own cadence.
        // The guard against double-start mirrors the equivalent
        // call in `SwiftExampleAppApp.rebindWalletScopedServices`.
        do {
            if try !walletManager.isShieldedSyncRunning() {
                try walletManager.startShieldedSync()
            }
        } catch {
            SDKLogger.error(
                "ShieldedService.manualSync: failed to (re)start shielded sync loop: \(error.localizedDescription)"
            )
        }
    }

    /// Whether the service has enough stashed state to perform a
    /// `bind` on demand from a Clear → Sync Now flow. Distinct
    /// from [`isBound`]: after Clear we are not currently bound,
    /// but the credentials live on so [`manualSync`] can rebind
    /// without the user navigating away from the Sync Status
    /// screen. False on a fresh session (no bind has ever run)
    /// or after [`reset`].
    var canResume: Bool {
        walletManager != nil
            && boundWalletId != nil
            && resolver != nil
            && network != nil
    }

    /// Reset display state. Cancels the manager subscriptions but
    /// does not stop the manager-wide background loop — that's the
    /// caller's responsibility (see
    /// [`PlatformWalletManager.stopShieldedSync`]).
    func reset() {
        syncStateCancellable?.cancel()
        syncEventCancellable?.cancel()
        progressCancellable?.cancel()
        treeProgressCancellable?.cancel()
        walletManager = nil
        boundWalletId = nil
        isSyncing = false
        shieldedBalance = 0
        lastNewNotes = 0
        lastNewlySpent = 0
        isBound = false
        lastSyncTime = nil
        lastError = nil
        orchardDisplayAddress = nil
        boundAccounts = []
        addressesByAccount = [:]
        syncCountSinceLaunch = 0
        totalScanned = 0
        totalNewNotes = 0
        totalNewlySpent = 0
        lastSyncDuration = nil
        longestSyncDuration = nil
        currentSyncElapsed = nil
        currentSyncStartedAt = nil
        currentSyncScanned = nil
        currentSyncBlockHeight = nil
        currentTreeCommitted = nil
        currentTreeTotal = nil
        syncTickTimer?.invalidate()
        syncTickTimer = nil
    }

    /// Wipe every wallet's persisted shielded state and stop. The
    /// service is left unbound, but the stashed bind credentials
    /// (`walletManager` / `boundWalletId` / `network` / `resolver`
    /// / `boundAccounts`) survive so [`manualSync`] can rebind on
    /// demand without the user navigating away. The "Sync Now"
    /// button on the Sync Status screen is the path back —
    /// pressing it self-binds + syncs from a clean SQLite tree
    /// and an empty SwiftData snapshot.
    ///
    /// The user reaches this through the Clear button on the
    /// **global** Sync Status surface, not a per-wallet screen.
    /// "Clear" therefore wipes every wallet's shielded rows and
    /// empties the per-network commitment tree, so the rebind on
    /// the next Sync Now walks the cmx stream from genesis.
    ///
    /// What survives the reset:
    ///   * The per-network commitment-tree SQLite file at
    ///     `dbPath(for:)`. `clearShielded` empties the tree through
    ///     the live Rust store but leaves the open database file in
    ///     place. Unlinking that file or its sidecars under the live
    ///     connection risks SQLite corruption.
    ///   * The stashed credentials on the service itself — bare
    ///     [`reset`] would nil them, leaving the user with no
    ///     path back to a synced state from this screen. The
    ///     inline soft-cleanup below only zeroes the published
    ///     mirror; [`canResume`] therefore stays `true` and the
    ///     Sync Now button stays usable.
    ///
    /// Per-wallet scoping was tried first and rejected because the
    /// Clear button doesn't carry wallet context — other wallets'
    /// `PersistentShieldedSyncState` rows would silently survive
    /// (the symptom the user reported when "Clear" left a row
    /// behind for a non-active wallet).
    func clearLocalState(modelContext: ModelContext) async {
        // Capture the manager before the soft-cleanup below
        // touches anything, so we can stop the background loop
        // first. (We used to capture `network` here too for the
        // per-network SQLite delete; that step is gone — see
        // doc above for why.)
        let managerForStop = walletManager

        // 1) Reset the Rust-side shielded state BEFORE touching
        //    state on disk. The Swift `ShieldedService` is
        //    per-wallet-at-a-time, but the Rust
        //    `PlatformWalletManager` keeps **every** wallet that
        //    ever ran `bind_shielded` registered on the
        //    network-scoped coordinator. Without this call the
        //    coordinator's next sync iterates the still-registered
        //    wallets and the persister callback immediately
        //    re-creates the `PersistentShieldedNote` /
        //    `PersistentShieldedSyncState` rows we're about to
        //    delete (the "Clear left a row behind / re-derived a
        //    fresh row" symptom from before). `clearShielded`
        //    does three things on the Rust side in one call:
        //      - stops the background sync loop
        //      - drops every wallet registration from the
        //        coordinator (`accounts` + `persisters` maps)
        //      - resets the network-wide caught-up cooldown
        //    The single SQLite commitment-tree file stays open;
        //    the next `bindShielded` call repopulates the
        //    registries and the next sync re-saves notes via
        //    the changeset path. This reset is load-bearing: if it
        //    cannot run, abort the host-row wipe so the tree file and
        //    SwiftData rows cannot diverge.
        //
        //    Re-binding scope after Clear: `clearShielded` drops
        //    EVERY wallet (not just the mirror's `firstWallet`)
        //    from the coordinator. "Sync Now" (`manualSync()`)
        //    UNCONDITIONALLY re-registers EVERY loaded wallet on each
        //    tap: the mirror wallet via `bind(...)` (in the recovery
        //    branch, only when unbound), and every OTHER loaded wallet
        //    via a `engineBindOtherWallets` / `bindEngine` pass that
        //    runs on every Sync Now regardless of the recovery branch.
        //    That unconditional pass matters because a detail-view
        //    `switchTo` between Clear and Sync Now re-binds only the
        //    viewed wallet (flipping `isBound` true and skipping the
        //    recovery branch), which would otherwise leave the other
        //    wallets engine-unregistered. The pass is also best-effort
        //    across the mirror: a mirror-bind FAILURE (missing mnemonic
        //    / declined resolver) no longer bails Sync Now — the engine
        //    pass still runs so every OTHER wallet with an intact
        //    mnemonic re-registers, and Sync Now only bails when NOTHING
        //    registered (mirror + every other bind failed). So
        //    cross-wallet shielded flows (SH-14/15/16) come back
        //    immediately on the first post-Clear Sync Now, not only on
        //    the next `rebindWalletScopedServices()` fire.
        //    `rebindWalletScopedServices` remains the recovery path for
        //    wallets loaded LATER (a wallet added after the Clear isn't
        //    in the manager's set at Sync-Now time); it re-`bindEngine`s
        //    every wallet on any wallet-set change or network switch. We
        //    keep the WIPE scope global on purpose (see the class-level
        //    doc below) — this note is about the re-BIND scope.
        prepareForShieldedRebind()
        guard let managerForStop else {
            lastError = "Failed to reset shielded state: no wallet manager is bound."
            SDKLogger.error(lastError ?? "")
            return
        }

        do {
            try Self.executeClearPersistenceSequence(
                resetRustState: {
                    try managerForStop.clearShielded()
                },
                clearHostState: {
                    // Delete every shielded SwiftData row across all
                    // wallets on this device. The Clear button is on the
                    // global Sync Status surface, so this is intentionally
                    // global rather than wallet-scoped.
                    try modelContext.delete(model: PersistentShieldedNote.self)
                    try modelContext.delete(model: PersistentShieldedOutgoingNote.self)
                    try modelContext.delete(model: PersistentShieldedSyncState.self)
                    try modelContext.delete(model: PersistentShieldedActivity.self)
                    // Viewing keys are included so a corrupted row cannot
                    // outlive Clear; the next bind re-persists them.
                    try modelContext.delete(model: PersistentShieldedViewingKey.self)
                    try modelContext.save()
                }
            )
        } catch {
            switch error {
            case .rustReset(let underlyingError):
                lastError =
                    "Failed to reset shielded state: \(underlyingError.localizedDescription)"
            case .hostPersistence(let underlyingError):
                lastError =
                    "Failed to wipe persisted shielded state: "
                    + underlyingError.localizedDescription
            }
            SDKLogger.error(lastError ?? "")
            return
        }

        // 3) Finish zeroing the published mirror. The subscriptions
        //    were cancelled before the reset attempt. Keep the bind credentials
        //    (walletManager / boundWalletId / network / resolver
        //    / boundAccounts) so [`manualSync`] can re-bind on
        //    the next Sync Now tap. Bare [`reset`] would nil
        //    them and leave the user stranded on this screen.
        shieldedBalance = 0
        lastNewNotes = 0
        lastNewlySpent = 0
        lastSyncTime = nil
        lastError = nil
        orchardDisplayAddress = nil
        addressesByAccount = [:]
        syncCountSinceLaunch = 0
        totalScanned = 0
        totalNewNotes = 0
        totalNewlySpent = 0
        lastSyncDuration = nil
        longestSyncDuration = nil
    }

    enum ClearLocalStateFailure: Error {
        case rustReset(Error)
        case hostPersistence(Error)
    }

    /// Enforces reset-before-delete ordering for the two persistence halves.
    /// Kept separate from the user-facing operation so tests can exercise
    /// failure ordering without exposing a reset bypass on `clearLocalState`.
    static func executeClearPersistenceSequence(
        resetRustState: () throws -> Void,
        clearHostState: () throws -> Void
    ) throws(ClearLocalStateFailure) {
        do {
            try resetRustState()
        } catch {
            throw ClearLocalStateFailure.rustReset(error)
        }
        do {
            try clearHostState()
        } catch {
            throw ClearLocalStateFailure.hostPersistence(error)
        }
    }

    /// A clear attempt quiesces Rust before it can report success or failure,
    /// and a store failure may occur after one reset half has already landed.
    /// Treat every attempt as requiring a fresh bind while retaining the
    /// persisted host rows until the whole sequence succeeds.
    private func prepareForShieldedRebind() {
        syncStateCancellable?.cancel()
        syncEventCancellable?.cancel()
        progressCancellable?.cancel()
        treeProgressCancellable?.cancel()
        isBound = false
        isSyncing = false
        endSyncTiming()
        currentSyncScanned = nil
        currentSyncBlockHeight = nil
        currentTreeCommitted = nil
        currentTreeTotal = nil
    }

    // MARK: - Sync timing brackets

    /// Open the timing bracket for a sync pass: stamp the start, zero
    /// the live elapsed, and start the 1 Hz ticker. Idempotent via the
    /// `currentSyncStartedAt == nil` guard so it can be called from
    /// either the `$shieldedSyncIsSyncing` false→true edge OR the
    /// `manualSync()` fast path — whichever observes the pass starting
    /// first wins, and the other no-ops. This closes the gap where a
    /// fast pass completes before the publisher flips, dropping
    /// `lastSyncDuration`.
    private func beginSyncTimingIfNeeded() {
        guard currentSyncStartedAt == nil else { return }
        currentSyncStartedAt = Date()
        currentSyncElapsed = 0
        syncTickTimer?.invalidate()
        syncTickTimer = Timer.scheduledTimer(
            withTimeInterval: 1.0,
            repeats: true
        ) { [weak self] _ in
            Task { @MainActor [weak self] in
                guard let self,
                      let started = self.currentSyncStartedAt
                else { return }
                self.currentSyncElapsed = max(
                    0,
                    Date().timeIntervalSince(started)
                )
            }
        }
    }

    /// Close the timing bracket: tear down the ticker and clear both
    /// the live elapsed and the start stamp. Every terminal flow must
    /// route through this (normal completion, cooldown-skip, skipped,
    /// failure) so no stale `currentSyncStartedAt` survives to be
    /// reused — and re-misreport — on the next pass's completion.
    /// Callers that report `lastSyncDuration` must read
    /// `currentSyncStartedAt` BEFORE calling this.
    private func endSyncTiming() {
        syncTickTimer?.invalidate()
        syncTickTimer = nil
        currentSyncElapsed = nil
        currentSyncStartedAt = nil
    }

    // MARK: - Sync event handling

    private func handleShieldedSyncEvent(_ event: ShieldedSyncEvent) {
        // Drop completion events that arrive while unbound. Clear
        // (`clearLocalState`) sets `isBound = false` and cancels this
        // subscription, but the Rust completion event is hopped onto the
        // main actor and a final, already-dispatched one can land just
        // after Clear returns — applying it would briefly repopulate the
        // mirror Clear just zeroed. The Rust quiesce barrier already
        // guarantees no *persistence* happens after Clear; this guards
        // the in-memory display mirror. `bind()` sets `isBound = true`
        // before its sync events flow, so a legitimate post-bind event
        // is never dropped.
        guard isBound else { return }
        guard let walletId = boundWalletId,
              let result = event.result(for: walletId) else {
            return
        }

        if result.success {
            lastError = nil
            isBound = true

            // Suppress counter / timestamp / balance updates on
            // cooldown skips. The Rust side returns
            // `result.cooldownSkip = true` with zeroed counters
            // *and* an empty balances payload (`result.balance ==
            // 0`) because no work was attempted; updating the
            // host's cached balance would clobber it to zero
            // every cooldown tick on a wallet that actually has
            // a balance. Genuine steady-state caught-up passes
            // (background loop ran, found nothing, returned the
            // real balance) still advance `lastSyncTime` so users
            // have a live signal that the loop is running. Per
            // swift-sdk/CLAUDE.md the policy decision (real sync
            // vs. cooldown skip) lives on the Rust side; Swift
            // just marshals the flag.
            if !result.cooldownSkip {
                shieldedBalance = result.balance
                lastNewNotes = result.newNotes
                lastNewlySpent = result.newlySpent
                lastSyncTime = Date(
                    timeIntervalSince1970: TimeInterval(event.syncUnixSeconds)
                )
                syncCountSinceLaunch += 1
                totalScanned += result.totalScanned
                totalNewNotes += UInt64(result.newNotes)
                totalNewlySpent += UInt64(result.newlySpent)

                // Record per-pass wall-clock and log it. `Date()`
                // here is the Swift-side timestamp of when the
                // event handler runs (≈ when isSyncing flipped
                // true → false), pairing with `currentSyncStartedAt`
                // captured on the false → true edge. Clamp to >= 0
                // defensively — should never be negative with
                // Swift-edge endpoints, but if the start timestamp
                // is missing (e.g. event arrived without a paired
                // start, post-Clear race) we surface nil rather
                // than a misleading number.
                if let started = currentSyncStartedAt {
                    let elapsed = max(0, Date().timeIntervalSince(started))
                    lastSyncDuration = elapsed
                    // Preserve the longest pass observed since the
                    // last reset. Cold sync (~20 min for 1M notes on
                    // paloma) would otherwise get clobbered by the
                    // next steady-state pass (~3 s). The cold number
                    // is the headline measurement; keep it visible.
                    if let prev = longestSyncDuration {
                        if elapsed > prev { longestSyncDuration = elapsed }
                    } else {
                        longestSyncDuration = elapsed
                    }
                    let rateString: String
                    if elapsed > 0.05 && result.totalScanned > 0 {
                        let rate = Double(result.totalScanned) / elapsed
                        rateString = String(format: " rate=%.0f/s", rate)
                    } else {
                        rateString = ""
                    }
                    SDKLogger.log(
                        String(
                            format: "Shielded sync done  pass=%d  elapsed=%.2fs%@  scanned=%llu  new=%u  spent=%u  balance=%llu",
                            syncCountSinceLaunch,
                            elapsed,
                            rateString,
                            result.totalScanned,
                            result.newNotes,
                            result.newlySpent,
                            result.balance
                        ),
                        minimumLevel: .medium
                    )
                } else {
                    lastSyncDuration = nil
                    SDKLogger.log(
                        "Shielded sync done (no paired start) pass=\(syncCountSinceLaunch) scanned=\(result.totalScanned) balance=\(result.balance)",
                        minimumLevel: .medium
                    )
                }
                // Close the timing bracket AFTER reading
                // `currentSyncStartedAt` above. Tears down the ticker,
                // clears `currentSyncElapsed`, and nils the start stamp.
                endSyncTiming()
            } else {
                // Cooldown-skip terminal: no work ran, so we leave the
                // cached balance / counters alone — but the timing
                // bracket still has to close, otherwise a stale start
                // stamp would be reused on the next pass's completion.
                endSyncTiming()
            }
        } else if result.skipped {
            // Skipped means the wallet hasn't been bound yet on the
            // Rust side. The UI can prompt the user to retry the
            // bind step. Close the timing bracket so no stale start
            // stamp survives to the next pass.
            isBound = false
            endSyncTiming()
        } else {
            // Failure terminal: surface the error and close the timing
            // bracket so the stale start stamp isn't reused next pass.
            lastError = result.errorMessage ?? "Shielded sync failed"
            endSyncTiming()
        }
    }

    // MARK: - Private

    /// Per-network commitment-tree DB.
    ///
    /// The Orchard tree is a chain-wide structure: every wallet
    /// and every account on the same network sees the same `cmx`
    /// stream in the same order, so they all back the same
    /// frontier and share anchors. `FileBackedShieldedStore` now
    /// scopes per-`(walletId, accountIndex)` notes inside the
    /// store via `SubwalletId`, so multiple wallets cohabiting
    /// the same SQLite file no longer leak notes across each
    /// other. (See `wallet/shielded/store.rs` for the trait.)
    private static func dbPath(for network: Network) -> String {
        let docs = FileManager.default
            .urls(for: .documentDirectory, in: .userDomainMask)
            .first!
        return docs
            .appendingPathComponent("shielded_tree_\(network.networkName).sqlite")
            .path
    }
}
