// PlatformBalanceSyncService.swift
// SwiftExampleApp
//
// App-level service that performs periodic BLAST address sync to discover
// platform address balances. Wraps the SDK's syncAddressBalances with
// flat address arrays (no callback provider needed from the app side).

import Foundation
import SwiftUI
import SwiftDashSDK

/// Observable service managing periodic BLAST address balance sync.
///
/// Syncs every 15 seconds while the app is active, or on manual pull-to-refresh.
/// Persists `lastSyncTimestamp` in UserDefaults for incremental sync.
@MainActor
class PlatformBalanceSyncService: ObservableObject {
    // MARK: - Published State

    /// Whether a sync is currently in progress.
    @Published var isSyncing = false

    /// Last successful sync time (local clock).
    @Published var lastSyncTime: Date?

    /// Per-address balances keyed by derivation index.
    @Published var addressBalances: [UInt32: UInt64] = [:]

    /// Per-address nonces keyed by derivation index.
    @Published var addressNonces: [UInt32: UInt32] = [:]

    /// Aggregate platform balance across all synced addresses.
    @Published var totalPlatformBalance: UInt64 = 0

    /// Number of addresses with non-zero balance.
    @Published var activeAddressCount: Int = 0

    /// Checkpoint height from the last trunk/branch tree scan.
    @Published var checkpointHeight: UInt64 = 0

    /// Chain tip height (highest block seen from incremental catch-up).
    @Published var chainTipHeight: UInt64 = 0

    /// Sync height used for incremental resume (passed back to FFI).
    @Published var lastSyncHeight: UInt64 = 0

    /// Platform block time from the most recent sync (Unix seconds).
    @Published var lastSyncBlockTime: Date?

    /// Metrics from the most recent sync.
    @Published var lastMetrics: AddressSyncMetrics?

    /// Total number of successful syncs since launch.
    @Published var syncCountSinceLaunch: Int = 0

    /// Cumulative query counts since launch.
    @Published var totalTrunkQueries: UInt32 = 0
    @Published var totalBranchQueries: UInt32 = 0
    @Published var totalCompactedQueries: UInt32 = 0
    @Published var totalRecentQueries: UInt32 = 0

    /// Last error message, cleared on successful sync.
    @Published var lastError: String?

    /// Found addresses from the last sync — passed back as known balances for incremental mode.
    private(set) var lastFoundAddresses: [FoundAddress] = []

    // MARK: - Internal State

    /// Timestamp returned by the last successful sync, passed back for incremental mode.
    private var lastSyncTimestamp: UInt64 {
        get { UInt64(UserDefaults.standard.integer(forKey: "\(keyPrefix)_timestamp")) }
        set { UserDefaults.standard.set(Int(newValue), forKey: "\(keyPrefix)_timestamp") }
    }

    /// Persisted sync height (block height for incremental resume).
    private var persistedSyncHeight: UInt64 {
        get { UInt64(UserDefaults.standard.integer(forKey: "\(keyPrefix)_height")) }
        set { UserDefaults.standard.set(Int(newValue), forKey: "\(keyPrefix)_height") }
    }

    /// Persisted block time (Unix seconds).
    private var persistedBlockTime: UInt64 {
        get { UInt64(UserDefaults.standard.integer(forKey: "\(keyPrefix)_blockTime")) }
        set { UserDefaults.standard.set(Int(newValue), forKey: "\(keyPrefix)_blockTime") }
    }

    /// UserDefaults key prefix scoped to network.
    private var keyPrefix: String {
        "platformAddressSync_\(networkName)"
    }

    private var networkName: String = "testnet"
    private var syncTimer: Timer?
    private var syncTask: Task<Void, Never>?

    /// Sync interval in seconds.
    private let syncInterval: TimeInterval = 15.0

    // MARK: - Lifecycle

    /// Start periodic sync. Call after SDK and wallet are initialized.
    func startPeriodicSync(network: AppNetwork) {
        networkName = network.rawValue
        stopPeriodicSync()

        // Restore persisted state from previous session
        let height = persistedSyncHeight
        if height > 0 {
            lastSyncHeight = height
        }
        let blockTs = persistedBlockTime
        if blockTs > 0 {
            lastSyncBlockTime = Date(timeIntervalSince1970: TimeInterval(blockTs))
        }

        // Delay the first sync to allow SDK quorum prefetch to complete.
        // Subsequent syncs run on the 15-second timer.
        syncTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: 1_500_000_000) // 1.5 seconds
            await self?.performSyncIfNeeded()
        }

        // Schedule repeating timer
        syncTimer = Timer.scheduledTimer(withTimeInterval: syncInterval, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                await self?.performSyncIfNeeded()
            }
        }
    }

    /// Stop periodic sync (e.g. on network switch or app background).
    func stopPeriodicSync() {
        syncTimer?.invalidate()
        syncTimer = nil
        syncTask?.cancel()
        syncTask = nil
    }

    /// Reset all state (e.g. on wallet deletion or network switch).
    func reset() {
        stopPeriodicSync()
        addressBalances.removeAll()
        addressNonces.removeAll()
        totalPlatformBalance = 0
        activeAddressCount = 0
        checkpointHeight = 0
        chainTipHeight = 0
        lastSyncHeight = 0
        lastSyncBlockTime = nil
        lastFoundAddresses = []
        lastMetrics = nil
        lastError = nil
        lastSyncTime = nil
        syncCountSinceLaunch = 0
        totalTrunkQueries = 0
        totalBranchQueries = 0
        totalCompactedQueries = 0
        totalRecentQueries = 0
        // Clear persisted state so next sync does a full tree scan
        lastSyncTimestamp = 0
        persistedSyncHeight = 0
        persistedBlockTime = 0
    }

    /// Trigger a manual sync (e.g. pull-to-refresh). No-op if already syncing.
    func manualSync(sdk: SDK, addresses: [(index: UInt32, key: Data)]) async {
        await performSync(sdk: sdk, addresses: addresses)
    }

    // MARK: - Internal

    /// Called by the timer; needs SDK + addresses injected at call site.
    /// This is a no-op placeholder -- the real sync is triggered from UnifiedAppState
    /// which has access to the SDK and wallet.
    private func performSyncIfNeeded() async {
        // The actual sync is orchestrated by UnifiedAppState which calls manualSync
        // with the right SDK and addresses. This timer just posts a notification.
        NotificationCenter.default.post(
            name: .platformBalanceSyncTick,
            object: nil
        )
    }

    /// Perform the actual BLAST address sync.
    ///
    /// - Parameters:
    ///   - sdk: The initialized SDK instance.
    ///   - addresses: Flat array of (derivation index, address key bytes) to sync.
    func performSync(sdk: SDK, addresses: [(index: UInt32, key: Data)]) async {
        guard !isSyncing, !addresses.isEmpty else { return }

        isSyncing = true
        lastError = nil

        do {
            let result = try await sdk.syncAddressBalances(
                addresses: addresses,
                knownBalances: lastFoundAddresses,
                lastSyncHeight: lastSyncHeight,
                lastSyncTimestamp: lastSyncTimestamp
            )

            // Update published state
            var newBalances: [UInt32: UInt64] = [:]
            var newNonces: [UInt32: UInt32] = [:]
            for found in result.found {
                newBalances[found.index] = found.balance
                newNonces[found.index] = found.nonce
            }

            addressBalances = newBalances
            addressNonces = newNonces
            totalPlatformBalance = result.totalBalance
            activeAddressCount = result.nonZeroBalanceCount
            lastMetrics = result.metrics
            lastFoundAddresses = result.found
            lastSyncHeight = result.newSyncHeight
            persistedSyncHeight = result.newSyncHeight
            if result.checkpointHeight > 0 {
                checkpointHeight = result.checkpointHeight
            }
            if result.newSyncHeight > chainTipHeight {
                chainTipHeight = result.newSyncHeight
            }
            if result.newSyncTimestamp > 0 {
                lastSyncBlockTime = Date(timeIntervalSince1970: TimeInterval(result.newSyncTimestamp))
                persistedBlockTime = result.newSyncTimestamp
            }
            lastSyncTime = Date()
            syncCountSinceLaunch += 1
            totalTrunkQueries += result.metrics.trunkQueries
            totalBranchQueries += result.metrics.branchQueries
            totalCompactedQueries += result.metrics.compactedQueries
            totalRecentQueries += result.metrics.recentQueries

            // Persist sync checkpoint for incremental mode
            if result.newSyncTimestamp > 0 {
                lastSyncTimestamp = result.newSyncTimestamp
            }

            SDKLogger.log(
                "BLAST sync complete: \(result.found.count) found, \(result.absent.count) absent, total balance: \(result.totalBalance)",
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

// MARK: - Notification

extension Notification.Name {
    static let platformBalanceSyncTick = Notification.Name("platformBalanceSyncTick")
}
