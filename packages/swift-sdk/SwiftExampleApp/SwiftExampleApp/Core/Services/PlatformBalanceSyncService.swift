// PlatformBalanceSyncService.swift
// SwiftExampleApp
//
// App-level service that reflects Rust-owned BLAST address sync state
// into SwiftUI. Scheduling, incremental state, and balance tracking
// are handled on the Rust side.

import Foundation
import SwiftUI
import Combine
import SwiftData
import SwiftDashSDK

/// Observable service managing BLAST address balance sync UI state.
///
/// The Rust manager owns the background sync loop; this service loads
/// cached balances for immediate display, issues manual sync requests,
/// and applies Rust-emitted completion events to the UI.
@MainActor
class PlatformBalanceSyncService: ObservableObject {
    // MARK: - Published State

    /// Whether a sync is currently in progress.
    @Published var isSyncing = false

    /// Whether a `clearLocalState` is currently in progress. Gates the
    /// Clear / Sync Now controls so a second Clear or a Sync Now can't
    /// interleave with the Rust reset + SwiftData wipe sequence.
    @Published var isClearing = false

    /// Monotonic token bumped at the start of every `clearLocalState`. A
    /// `refreshBalanceSnapshot` captures it before its `await` and
    /// re-checks it before publishing, so a snapshot that was already in
    /// flight when a Clear ran can't republish pre-clear balances over
    /// the cleared UI. `@MainActor` serializes the bump and the check.
    private var balanceSnapshotGeneration: UInt64 = 0

    /// Last successful sync time (local clock).
    @Published var lastSyncTime: Date?

    /// Per-address balances (address hash hex → balance).
    @Published var addressBalances: [String: UInt64] = [:]

    /// Aggregate platform balance across all synced addresses.
    @Published var totalPlatformBalance: UInt64 = 0

    /// Number of addresses with non-zero balance.
    @Published var activeAddressCount: Int = 0

    /// Checkpoint height from the last trunk/branch tree scan.
    @Published var checkpointHeight: UInt64 = 0

    /// Chain tip height (highest block seen from incremental catch-up).
    @Published var chainTipHeight: UInt64 = 0

    /// Last sync height for display.
    @Published var lastSyncHeight: UInt64 = 0

    /// Last block height with recent address changes (for compaction detection).
    @Published var lastKnownRecentBlock: UInt64 = 0

    /// Platform block time from the most recent sync (Unix seconds).
    @Published var lastSyncBlockTime: Date?

    /// Total number of successful syncs since launch.
    @Published var syncCountSinceLaunch: Int = 0

    /// Cumulative query counts since launch.
    @Published var totalTrunkQueries: UInt32 = 0
    @Published var totalBranchQueries: UInt32 = 0
    @Published var totalCompactedQueries: UInt32 = 0
    @Published var totalRecentQueries: UInt32 = 0
    @Published var totalRecentEntries: UInt32 = 0
    @Published var totalCompactedEntries: UInt32 = 0

    /// Raw GroveDB proof from the most recent sync (for debugging).
    @Published var lastRecentProof: Data = Data()

    /// Last error message, cleared on successful sync.
    @Published var lastError: String?

    // MARK: - Internal

    /// The platform address wallet handle used to refresh address
    /// balances after Rust-owned sync passes complete.
    private var platformAddressWallet: ManagedPlatformAddressWallet?

    /// Wallet manager used to control the Rust-owned background sync loop.
    private weak var walletManager: PlatformWalletManager?

    /// Persistence handler for loading cached balances.
    private var persistenceHandler: PlatformWalletPersistenceHandler?

    /// Wallet ID for querying cached balances.
    private var walletId: Data?

    /// Observes Rust-side sync completion events.
    private var syncEventCancellable: AnyCancellable?

    /// Mirrors the Rust manager's current `is_syncing` flag for the UI.
    private var syncStateCancellable: AnyCancellable?

    // MARK: - Lifecycle

    /// Configure for a wallet. Call after wallet creation/switch.
    func configure(
        platformAddressWallet: ManagedPlatformAddressWallet,
        walletManager: PlatformWalletManager,
        persistenceHandler: PlatformWalletPersistenceHandler? = nil,
        walletId: Data? = nil
    ) {
        // Same invalidation as `reset()`: a snapshot still in flight for
        // the previous wallet must not publish over this configuration.
        balanceSnapshotGeneration &+= 1
        self.platformAddressWallet = platformAddressWallet
        self.walletManager = walletManager
        self.persistenceHandler = persistenceHandler
        self.walletId = walletId
        self.syncEventCancellable?.cancel()
        self.syncStateCancellable?.cancel()

        // Load cached state from SwiftData for immediate display.
        if let handler = persistenceHandler, let wid = walletId {
            // Restore address balances for UI.
            let cached = handler.loadCachedBalances(walletId: wid)
            if !cached.isEmpty {
                var newBalances: [String: UInt64] = [:]
                var total: UInt64 = 0
                var nonZero = 0

                for (_, hash, balance, _, _, _, _) in cached {
                    let key = hash.map { String(format: "%02x", $0) }.joined()
                    newBalances[key] = balance
                    total += balance
                    if balance > 0 { nonZero += 1 }
                }

                addressBalances = newBalances
                totalPlatformBalance = total
                activeAddressCount = nonZero
            }

            // Load the last persisted network sync markers for display.
            if let state = handler.loadCachedSyncState(walletId: wid) {
                chainTipHeight = state.syncHeight
                lastSyncHeight = state.syncHeight
                if state.syncTimestamp > 0 {
                    lastSyncBlockTime = Date(timeIntervalSince1970: TimeInterval(state.syncTimestamp))
                }
                lastKnownRecentBlock = state.lastKnownRecentBlock

                SDKLogger.log(
                    "Restored network sync state: height=\(state.syncHeight), timestamp=\(state.syncTimestamp)",
                    minimumLevel: .medium
                )
            }
        }

        syncStateCancellable = walletManager.$platformAddressSyncIsSyncing
            .sink { [weak self] isSyncing in
                self?.isSyncing = isSyncing
            }

        syncEventCancellable = walletManager.$lastPlatformAddressSyncEvent
            .sink { [weak self] event in
                guard let self, let event else { return }
                self.handlePlatformAddressSyncEvent(event)
            }
    }

    /// Clear UI display state — balances, metrics, last sync timestamps.
    ///
    /// Does NOT nil out the wallet handle, so the user can tap "Sync Now"
    /// immediately after clearing. Use [`fullReset`] for wallet deletion
    /// or network switches.
    func clearDisplay() {
        addressBalances.removeAll()
        totalPlatformBalance = 0
        activeAddressCount = 0
        checkpointHeight = 0
        chainTipHeight = 0
        lastSyncHeight = 0
        lastKnownRecentBlock = 0
        lastSyncBlockTime = nil
        lastRecentProof = Data()
        lastError = nil
        lastSyncTime = nil
        syncCountSinceLaunch = 0
        totalTrunkQueries = 0
        totalBranchQueries = 0
        totalCompactedQueries = 0
        totalRecentQueries = 0
        totalRecentEntries = 0
        totalCompactedEntries = 0
    }

    /// Full reset — clears display state AND nils out the wallet handle.
    /// Use for wallet deletion or network switch. Caller must re-configure
    /// before the next sync.
    func reset() {
        // Invalidate any in-flight `refreshBalanceSnapshot` — its
        // detached task captured the OLD wallet handle strongly, so
        // without this bump a snapshot racing a wallet/network switch
        // would publish the old wallet's balances over the reset UI.
        balanceSnapshotGeneration &+= 1
        clearDisplay()
        platformAddressWallet = nil
        walletManager = nil
        syncEventCancellable?.cancel()
        syncStateCancellable?.cancel()
    }

    /// Clear platform-address sync data for real — what the Platform
    /// Sync "Clear" button calls.
    ///
    /// Plain [`clearDisplay`] only zeroes the in-memory `@Published`
    /// mirror, so the next sync resumed from the surviving watermark in
    /// ~2s (the "Clear didn't work" symptom). This clears the stores the
    /// synced data actually lives in, mirroring
    /// `ShieldedService.clearLocalState`: Rust-side reset FIRST (so the
    /// next sync can't re-persist stale rows), then the SwiftData clear,
    /// then the published-mirror reset.
    ///
    /// The `PersistentPlatformAddress` rows are **zeroed in place, not
    /// deleted** — they carry durable derivation metadata that no sync
    /// re-emits in-session and that the balance callback only updates
    /// (never creates), so deleting them would leave the UI empty until
    /// app restart. Only the volatile balance/sync fields are cleared;
    /// the network-keyed `PersistentPlatformAddressesSyncState` watermark
    /// row is deleted to force a full rescan.
    ///
    /// Scoped to the **active network**. The SwiftData store holds rows
    /// for every network at once (the UI filters them by network), so
    /// clearing must not touch other networks' state. `PersistentPlatformAddress`
    /// carries no network column, so it's scoped via `walletIdsOnNetwork`
    /// — the same wallet-id-per-network pivot the view uses;
    /// `PersistentPlatformAddressesSyncState` is network-keyed and is
    /// scoped by `networkRaw`. This matches the manager-level Rust reset,
    /// which only touches the active network's registered wallets.
    ///
    /// Fails closed: if the Rust reset OR the SwiftData delete throws, it
    /// surfaces the error in `lastError` and returns WITHOUT calling
    /// `clearDisplay()` — the UI never shows a false "cleared" state over
    /// data that is still on disk / in Rust memory.
    func clearLocalState(
        modelContext: ModelContext,
        network: Network,
        walletIdsOnNetwork: Set<Data>
    ) async {
        // Invalidate any in-flight `refreshBalanceSnapshot`: a snapshot
        // queued by a sync that completed just before this Clear must not
        // land its (pre-clear) balances over the cleared UI. It captured
        // the old generation and will drop its publish once we bump here.
        balanceSnapshotGeneration &+= 1
        // Gate the Clear / Sync Now controls for the whole sequence.
        isClearing = true
        defer { isClearing = false }

        // 1) Reset the Rust-owned state BEFORE touching disk. Without
        //    this the in-memory watermark survives and the next "Sync
        //    Now" resumes incrementally (fast) instead of doing a full
        //    rescan; a still-registered background pass could also
        //    re-persist the rows we're about to delete. Fail closed —
        //    the reset is load-bearing for the wipe, so abort (surfacing
        //    the error) rather than leave a half-cleared state.
        if let walletManager {
            do {
                try await walletManager.resetPlatformAddressSyncState()
            } catch {
                lastError = "Failed to reset platform-address sync state: \(error.localizedDescription)"
                SDKLogger.error(lastError ?? "")
                return
            }
        }

        // 2) Zero the volatile balance/sync fields of this network's
        //    address rows IN PLACE — do NOT delete them. A
        //    `PersistentPlatformAddress` row is durable derivation state
        //    (bech32m address, 20-byte hash, 33-byte public key,
        //    account/address index, derivation path), not just a balance
        //    cache. The BLAST balance callback `persistAddressBalances`
        //    only *updates* existing rows (it skips by `addressHash` when
        //    the row is missing), and no sync re-emits the rows in-session:
        //    Rust's `reset_sync_state` preserves the address bijection, so
        //    there is no pool extension to re-fire the address-emit path.
        //    Deleting would therefore leave the next "Sync Now" with
        //    nothing to upsert against — balances (and the spend/signing
        //    derivation lookups) would stay gone until an app restart
        //    re-seeds the rows. Zeroing clears what the user sees while
        //    keeping the metadata the rescan needs.
        do {
            let addresses = try modelContext.fetch(FetchDescriptor<PersistentPlatformAddress>())
            for row in addresses where walletIdsOnNetwork.contains(row.walletId) {
                row.balance = 0
                row.nonce = 0
                row.isUsed = false
                row.firstSeenHeight = 0
                row.lastSeenHeight = 0
                row.lastUpdated = Date()
            }

            // The sync-state watermark is pure volatile sync state (no
            // durable metadata) and is what actually forces a full rescan,
            // so delete it outright — the next sync re-creates it.
            let networkRaw = network.rawValue
            let syncStates = try modelContext.fetch(FetchDescriptor<PersistentPlatformAddressesSyncState>())
            for row in syncStates where row.networkRaw == networkRaw {
                modelContext.delete(row)
            }

            try modelContext.save()
        } catch {
            lastError = "Failed to clear persisted platform-address state: \(error.localizedDescription)"
            SDKLogger.error(lastError ?? "")
            return
        }

        // 3) Zero the published display mirror — only on full success.
        clearDisplay()
    }

    /// Trigger a manual sync. No-op if already syncing.
    func manualSync() async {
        await performSync()
    }

    // MARK: - Sync

    /// Perform the actual BLAST address sync via platform-wallet.
    ///
    /// Drives `isSyncing` directly around the await so the spinner
    /// flashes even when the underlying Rust pass completes faster
    /// than the manager's 1 Hz `isPlatformAddressSyncing` poll
    /// cadence. Empty-pool regtest syncs return in tens of ms — the
    /// polled `$platformAddressSyncIsSyncing` stays `false` for the
    /// whole call and the subscription never fires, so we have to
    /// reset locally regardless of outcome.
    func performSync() async {
        guard !isSyncing else { return }
        // The Clear/Sync mutual exclusion must live here, not only on
        // CoreContentView's buttons: pull-to-refresh (Wallets/Identities
        // tabs) and the post-submit resyncs (Transfer/Withdraw views)
        // call performSync directly and would otherwise start a sync
        // between clearLocalState's Rust reset and its SwiftData wipe.
        guard !isClearing else { return }
        guard let walletManager = walletManager else {
            lastError = "Platform address wallet not configured"
            return
        }

        isSyncing = true
        lastError = nil
        defer { isSyncing = false }

        do {
            try await walletManager.syncPlatformAddressNow()
        } catch {
            lastError = error.localizedDescription
            SDKLogger.log(
                "BLAST sync error: \(error.localizedDescription)",
                minimumLevel: .medium
            )
        }
    }

    private func handlePlatformAddressSyncEvent(_ event: PlatformAddressSyncEvent) {
        guard let walletId, let result = event.result(for: walletId) else {
            return
        }

        if result.success {
            lastError = nil
            if result.checkpointHeight > 0 {
                checkpointHeight = result.checkpointHeight
            }
            if result.newSyncHeight > chainTipHeight {
                chainTipHeight = result.newSyncHeight
            }
            lastSyncHeight = result.newSyncHeight
            lastKnownRecentBlock = result.lastKnownRecentBlock
            if result.newSyncTimestamp > 0 {
                lastSyncBlockTime = Date(timeIntervalSince1970: TimeInterval(result.newSyncTimestamp))
            }

            totalTrunkQueries += result.metrics.trunkQueries
            totalBranchQueries += result.metrics.branchQueries
            totalCompactedQueries += result.metrics.compactedQueries
            totalRecentQueries += result.metrics.recentQueries
            totalRecentEntries += result.metrics.recentEntriesReturned
            totalCompactedEntries += result.metrics.compactedEntriesReturned

            lastSyncTime = Date(timeIntervalSince1970: TimeInterval(event.syncUnixSeconds))
            syncCountSinceLaunch += 1

            Task {
                await refreshBalanceSnapshot()
            }
        } else {
            lastError = result.errorMessage ?? "Platform address sync failed"
        }
    }

    private func refreshBalanceSnapshot() async {
        guard let wallet = platformAddressWallet else { return }

        // Capture the generation before the off-actor read. If a
        // `clearLocalState` bumps it while we await, the balances below
        // are pre-clear and must NOT be published over the cleared UI.
        let generation = balanceSnapshotGeneration

        do {
            let (balances, credits) = try await Task.detached { [wallet] in
                let balances = try wallet.addressesWithBalances()
                let credits = try wallet.totalCredits()
                return (balances, credits)
            }.value

            // A Clear ran while this snapshot was in flight — drop it.
            guard generation == balanceSnapshotGeneration else {
                SDKLogger.log(
                    "BLAST snapshot dropped: superseded by a Clear",
                    minimumLevel: .medium
                )
                return
            }

            var newBalances: [String: UInt64] = [:]
            var total: UInt64 = 0
            var nonZero = 0

            for entry in balances {
                let key = entry.hash.map { String(format: "%02x", $0) }.joined()
                newBalances[key] = entry.balance
                total += entry.balance
                if entry.balance > 0 { nonZero += 1 }
            }

            addressBalances = newBalances
            totalPlatformBalance = credits == total ? total : credits
            activeAddressCount = nonZero

            SDKLogger.log(
                "BLAST sync complete: \(balances.count) addresses, total balance: \(totalPlatformBalance)",
                minimumLevel: .medium
            )
        } catch {
            lastError = error.localizedDescription
            SDKLogger.log(
                "BLAST sync snapshot refresh error: \(error.localizedDescription)",
                minimumLevel: .medium
            )
        }
    }
}
