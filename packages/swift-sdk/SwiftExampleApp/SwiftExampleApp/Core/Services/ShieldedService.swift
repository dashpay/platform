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
            // `bind` is best-effort; if it failed (e.g. the
            // mnemonic resolver was declined), `isBound` stays
            // false and `lastError` is populated. Bail rather
            // than chain a sync that will fail the same way.
            guard isBound else { return }
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
    /// "Clear" therefore wipes every wallet's shielded rows + the
    /// per-network commitment-tree SQLite file, so the rebind on
    /// the next Sync Now walks the cmx stream from genesis.
    ///
    /// What it does NOT touch:
    ///   * The manager-wide shielded sync loop is `stopShieldedSync`'d
    ///     first so the persister callback can't re-derive the
    ///     rows we're deleting; it restarts when something binds
    ///     (either `manualSync` self-binding or
    ///     `rebindWalletScopedServices` firing on a navigation).
    ///   * The Rust-side shielded sub-wallet binding (there's no
    ///     unbind FFI today; the next `bindShielded` call replaces
    ///     the binding wholesale, and the freshly-bound store
    ///     starts empty because both the SwiftData snapshot and
    ///     the SQLite tree are gone).
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
        // Capture the network + manager BEFORE `reset()` nils
        // them out. The network drives the per-network SQLite
        // path; the manager is needed to stop the background
        // shielded sync loop so Rust doesn't rewrite the rows we
        // just wiped.
        let networkForDB = network
        let managerForStop = walletManager

        // 1) Stop the manager-wide shielded sync loop BEFORE
        //    touching state on disk. The Swift `ShieldedService`
        //    is per-wallet-at-a-time, but the Rust
        //    `PlatformWalletManager` keeps **every** wallet that
        //    ever ran `bind_shielded` bound at the Rust level,
        //    and the background sync iterates all of them on its
        //    own cadence. If we don't stop the loop, the next
        //    pass fires per wallet and the persister callback
        //    immediately re-creates the `PersistentShieldedNote`
        //    / `PersistentShieldedSyncState` rows we're about to
        //    delete (this is exactly the "Clear left a row
        //    behind / re-derived a fresh row" symptom we saw on
        //    the prior attempt). Stopping is best-effort —
        //    failure logs but doesn't abort the wipe.
        if let managerForStop {
            do {
                try managerForStop.stopShieldedSync()
            } catch {
                SDKLogger.error(
                    "ShieldedService.clearLocalState: stopShieldedSync failed: \(error.localizedDescription)"
                )
            }
        }

        // 2) Delete every shielded SwiftData row across all
        //    wallets on this device. The Clear button is on the
        //    global Sync Status surface, so its semantics are
        //    "blow away shielded persistence", not "scope to one
        //    wallet".
        do {
            try modelContext.delete(model: PersistentShieldedNote.self)
            try modelContext.delete(model: PersistentShieldedSyncState.self)
            try modelContext.save()
        } catch {
            lastError = "Failed to wipe persisted shielded state: \(error.localizedDescription)"
            SDKLogger.error(lastError ?? "")
            return
        }

        // 3) Delete the per-network commitment-tree SQLite file
        //    (plus its WAL/SHM/journal sidecars) so the next
        //    `bind_shielded` on this network starts the tree at
        //    leaf 0. Without this the SwiftData snapshot says
        //    "fresh wallet" but the on-disk tree still carries
        //    every commitment from prior syncs — and the
        //    watermark/checkpoint asymmetry that produces is the
        //    most common source of the "Merkle witness
        //    unavailable" failures we're chasing.
        if let networkForDB {
            let dbPath = Self.dbPath(for: networkForDB)
            let fm = FileManager.default
            for suffix in ["", "-wal", "-shm", "-journal"] {
                let path = dbPath + suffix
                if fm.fileExists(atPath: path) {
                    do {
                        try fm.removeItem(atPath: path)
                    } catch {
                        SDKLogger.error(
                            "ShieldedService.clearLocalState: failed to delete \(path): \(error.localizedDescription)"
                        )
                    }
                }
            }
        }

        // 4) Soft cleanup: zero the published mirror + cancel
        //    subscriptions, but KEEP the bind credentials
        //    (walletManager / boundWalletId / network / resolver
        //    / boundAccounts) so [`manualSync`] can re-bind on
        //    the next Sync Now tap. Bare [`reset`] would nil
        //    them and leave the user stranded on this screen.
        syncStateCancellable?.cancel()
        syncEventCancellable?.cancel()
        isBound = false
        isSyncing = false
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
