// PlatformBalanceSyncService.swift
// SwiftExampleApp
//
// App-level service that reflects Rust-owned BLAST address sync state
// into SwiftUI. Scheduling, incremental state, and balance tracking
// are handled on the Rust side.

import Foundation
import SwiftUI
import Combine
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

    /// The platform address wallet handle (retained for incremental sync state).
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

                for (_, hash, balance, _, _, _) in cached {
                    let key = hash.map { String(format: "%02x", $0) }.joined()
                    newBalances[key] = balance
                    total += balance
                    if balance > 0 { nonZero += 1 }
                }

                addressBalances = newBalances
                totalPlatformBalance = total
                activeAddressCount = nonZero
            }

            // Load the last persisted sync markers for display.
            if let state = handler.loadCachedSyncState(walletId: wid) {
                chainTipHeight = state.syncHeight
                lastSyncHeight = state.syncHeight
                if state.syncTimestamp > 0 {
                    lastSyncBlockTime = Date(timeIntervalSince1970: TimeInterval(state.syncTimestamp))
                }
                lastKnownRecentBlock = state.lastKnownRecentBlock

                SDKLogger.log(
                    "Restored sync state: height=\(state.syncHeight), timestamp=\(state.syncTimestamp)",
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
        clearDisplay()
        platformAddressWallet = nil
        walletManager = nil
        syncEventCancellable?.cancel()
        syncStateCancellable?.cancel()
    }

    /// Trigger a manual sync. No-op if already syncing.
    func manualSync() async {
        await performSync()
    }

    // MARK: - Sync

    /// Perform the actual BLAST address sync via platform-wallet.
    func performSync() async {
        guard !isSyncing else { return }
        guard let walletManager = walletManager else {
            lastError = "Platform address wallet not configured"
            return
        }

        isSyncing = true
        lastError = nil

        do {
            try await walletManager.syncPlatformAddressNow()
        } catch {
            isSyncing = false
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

        do {
            let (balances, credits) = try await Task.detached { [wallet] in
                let balances = try wallet.addressesWithBalances()
                let credits = try wallet.totalCredits()
                return (balances, credits)
            }.value

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
