// PlatformBalanceSyncService.swift
// SwiftExampleApp
//
// App-level service that performs periodic BLAST address sync via
// PlatformAddressWallet (platform-wallet-ffi). All address derivation,
// incremental state, and balance tracking is handled on the Rust side.

import Foundation
import SwiftUI
import SwiftDashSDK

/// Observable service managing periodic BLAST address balance sync.
///
/// Syncs every 15 seconds while the app is active, or on manual pull-to-refresh.
/// Incremental sync state (timestamps, heights, known balances) is retained
/// by the Rust-side provider between calls — no UserDefaults needed.
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

    /// Persistence handler for loading cached balances.
    private var persistenceHandler: PlatformWalletPersistenceHandler?

    /// Wallet ID for querying cached balances.
    private var walletId: Data?

    // MARK: - Lifecycle

    /// Configure for a wallet. Call after wallet creation/switch.
    func configure(
        platformAddressWallet: ManagedPlatformAddressWallet,
        persistenceHandler: PlatformWalletPersistenceHandler? = nil,
        walletId: Data? = nil
    ) {
        self.platformAddressWallet = platformAddressWallet
        self.persistenceHandler = persistenceHandler
        self.walletId = walletId

        // Load cached state from SwiftData for immediate display and
        // incremental sync resume.
        if let handler = persistenceHandler, let wid = walletId {
            // Restore address balances for UI.
            let cached = handler.loadCachedBalances(walletId: wid)
            if !cached.isEmpty {
                var newBalances: [String: UInt64] = [:]
                var total: UInt64 = 0
                var nonZero = 0

                for (_, hash, balance) in cached {
                    let key = hash.map { String(format: "%02x", $0) }.joined()
                    newBalances[key] = balance
                    total += balance
                    if balance > 0 { nonZero += 1 }
                }

                addressBalances = newBalances
                totalPlatformBalance = total
                activeAddressCount = nonZero
            }

            // Restore sync state so next sync is incremental.
            if let state = handler.loadCachedSyncState(walletId: wid) {
                try? platformAddressWallet.restoreSyncState(
                    syncHeight: state.syncHeight,
                    syncTimestamp: state.syncTimestamp,
                    lastKnownRecentBlock: state.lastKnownRecentBlock
                )
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
    }

    /// Initialize periodic sync. The actual loop is managed by UnifiedAppState.
    func startPeriodicSync(network: AppNetwork) {
        // No state to restore — cached balances are loaded in configure().
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
    }

    /// Trigger a manual sync. No-op if already syncing.
    func manualSync() async {
        await performSync()
    }

    // MARK: - Sync

    /// Perform the actual BLAST address sync via platform-wallet.
    func performSync() async {
        guard !isSyncing else { return }
        guard let wallet = platformAddressWallet else {
            lastError = "Platform address wallet not configured"
            return
        }

        isSyncing = true
        lastError = nil

        do {
            // Unified provider performs one combined trunk/branch scan
            // across every account and returns a single result.
            let result = try wallet.syncBalances()

            if result.checkpointHeight > 0 {
                checkpointHeight = result.checkpointHeight
            }
            if result.newSyncHeight > chainTipHeight {
                chainTipHeight = result.newSyncHeight
            }
            lastSyncHeight = result.newSyncHeight
            if result.newSyncTimestamp > 0 {
                lastSyncBlockTime = Date(timeIntervalSince1970: TimeInterval(result.newSyncTimestamp))
            }

            totalTrunkQueries += result.metrics.trunkQueries
            totalBranchQueries += result.metrics.branchQueries
            totalCompactedQueries += result.metrics.compactedQueries
            totalRecentQueries += result.metrics.recentQueries
            totalRecentEntries += result.metrics.recentEntriesReturned
            totalCompactedEntries += result.metrics.compactedEntriesReturned

            // Read balances from the wallet (canonical source of truth)
            let balances = try wallet.addressesWithBalances()
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
            totalPlatformBalance = total
            activeAddressCount = nonZero

            // Update total credits as a cross-check
            let credits = try wallet.totalCredits()
            if credits != total {
                totalPlatformBalance = credits
            }

            lastSyncTime = Date()
            syncCountSinceLaunch += 1

            SDKLogger.log(
                "BLAST sync complete: \(balances.count) addresses, total balance: \(total)",
                minimumLevel: .medium
            )

        } catch {
            lastError = error.localizedDescription
            SDKLogger.log(
                "BLAST sync error: \(error.localizedDescription)",
                minimumLevel: .medium
            )
        }

        isSyncing = false
    }
}
