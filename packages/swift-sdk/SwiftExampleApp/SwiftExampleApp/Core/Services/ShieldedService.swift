// ShieldedService.swift
// SwiftExampleApp
//
// Display-state surface for the Rust-owned shielded (Orchard) sync
// coordinator. The service binds to a single wallet, subscribes to
// the platform-wallet manager's shielded sync events, and exposes
// `@Published` properties for the UI. It does not own any of the
// shielded crypto: bind, sync, and persistence all live on the Rust
// `platform-wallet` side.

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

        let dbPath = Self.dbPath(for: network)
        let sortedAccounts = Array(Set(accounts)).sorted()
        do {
            try walletManager.bindShielded(
                walletId: walletId,
                resolver: resolver,
                accounts: sortedAccounts,
                dbPath: dbPath
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
            .sink { [weak self] isSyncing in
                self?.isSyncing = isSyncing
            }

        syncEventCancellable = walletManager.$lastShieldedSyncEvent
            .sink { [weak self] event in
                guard let self, let event else { return }
                self.handleShieldedSyncEvent(event)
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
        guard let walletManager else {
            lastError = "Shielded service not configured"
            return
        }

        isSyncing = true
        lastError = nil
        defer { isSyncing = false }
        do {
            try await walletManager.syncShieldedNow()
        } catch {
            lastError = "Shielded sync error: \(error.localizedDescription)"
            SDKLogger.log(lastError ?? "", minimumLevel: .medium)
        }
    }

    /// Reset display state. Cancels the manager subscriptions but
    /// does not stop the manager-wide background loop — that's the
    /// caller's responsibility (see
    /// [`PlatformWalletManager.stopShieldedSync`]).
    func reset() {
        syncStateCancellable?.cancel()
        syncEventCancellable?.cancel()
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
    }

    /// Wipe this wallet's persisted shielded state and stop. The
    /// service is left unbound — no auto-rebind, no auto-rescan.
    ///
    /// Bare [`reset`] just tears down subscriptions and nils the
    /// in-memory mirror — the SwiftData rows
    /// (`PersistentShieldedNote`, `PersistentShieldedSyncState`)
    /// survive, so the next bind hydrates the wallet right back to
    /// the state the user just tried to clear.
    ///
    /// The user reaches this through the Clear button on the Sync
    /// Status screen and wants "delete and walk away" semantics:
    /// the persisted rows go, the published mirror zeroes out, and
    /// the service stays inert until the next explicit bind.
    /// Re-syncing means navigating back through the wallet detail
    /// screen, which retriggers `rebindWalletScopedServices` and
    /// rebinds from scratch (SwiftData is empty → Rust's
    /// `restore_from_snapshot` starts at zero).
    ///
    /// What it does NOT touch:
    ///   * The on-disk commitment-tree SQLite file at
    ///     `dbPath(for:)`. That tree is **per-network** — every
    ///     wallet on the same network shares the same `cmx` stream
    ///     and the same frontier, so deleting it would corrupt
    ///     other wallets' state. Rust's `sync_notes` already skips
    ///     positions already in the tree, so re-using the existing
    ///     leaves on the next bind is the right behaviour.
    ///   * The manager-wide shielded sync loop. Other wallets
    ///     bound on the same `PlatformWalletManager` keep syncing.
    ///   * The Rust-side shielded sub-wallet binding (there's no
    ///     unbind FFI today; the next `bindShielded` call replaces
    ///     the binding wholesale).
    ///
    /// No-op if the service hasn't been bound yet.
    func clearLocalState(modelContext: ModelContext) async {
        guard let walletId = boundWalletId else {
            SDKLogger.log(
                "ShieldedService.clearLocalState called before initial bind — ignoring",
                minimumLevel: .medium
            )
            return
        }

        // 1) Delete this wallet's persisted shielded rows from
        //    SwiftData. Scoped to `walletId` so other wallets'
        //    state on the same SwiftData store stays intact.
        do {
            try modelContext.delete(
                model: PersistentShieldedNote.self,
                where: #Predicate { $0.walletId == walletId }
            )
            try modelContext.delete(
                model: PersistentShieldedSyncState.self,
                where: #Predicate { $0.walletId == walletId }
            )
            try modelContext.save()
        } catch {
            lastError = "Failed to wipe persisted shielded state: \(error.localizedDescription)"
            SDKLogger.error(lastError ?? "")
            return
        }

        // 2) Tear down the in-memory mirror + subscriptions. The
        //    service is now unbound; no further sync events flow
        //    in and Sync Now will surface "Shielded service not
        //    configured" until something re-binds (typically the
        //    next navigation into a wallet detail screen).
        reset()
    }

    // MARK: - Sync event handling

    private func handleShieldedSyncEvent(_ event: ShieldedSyncEvent) {
        guard let walletId = boundWalletId,
              let result = event.result(for: walletId) else {
            return
        }

        if result.success {
            lastError = nil
            isBound = true
            shieldedBalance = result.balance
            lastNewNotes = result.newNotes
            lastNewlySpent = result.newlySpent
            lastSyncTime = Date(timeIntervalSince1970: TimeInterval(event.syncUnixSeconds))
            syncCountSinceLaunch += 1
            totalScanned += result.totalScanned
            totalNewNotes += UInt64(result.newNotes)
            totalNewlySpent += UInt64(result.newlySpent)
        } else if result.skipped {
            // Skipped means the wallet hasn't been bound yet on the
            // Rust side. The UI can prompt the user to retry the
            // bind step.
            isBound = false
        } else {
            lastError = result.errorMessage ?? "Shielded sync failed"
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
