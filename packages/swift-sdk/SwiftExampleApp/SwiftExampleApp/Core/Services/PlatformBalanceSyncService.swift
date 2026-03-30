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

    /// Last block height with recent address changes (for compaction detection).
    @Published var lastKnownRecentBlock: UInt64 = 0

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
    @Published var totalRecentEntries: UInt32 = 0
    @Published var totalCompactedEntries: UInt32 = 0

    /// Raw GroveDB proof from the most recent sync (for debugging).
    @Published var lastRecentProof: Data = Data()

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

    /// Persisted last known recent block height (for compaction detection).
    private var persistedLastKnownRecentBlock: UInt64 {
        get { UInt64(UserDefaults.standard.integer(forKey: "\(keyPrefix)_lastKnownRecent")) }
        set { UserDefaults.standard.set(Int(newValue), forKey: "\(keyPrefix)_lastKnownRecent") }
    }

    /// Persisted found addresses (JSON-encoded) for balance restoration across launches.
    private var persistedFoundAddresses: [FoundAddress] {
        get {
            guard let data = UserDefaults.standard.data(forKey: "\(keyPrefix)_foundAddresses"),
                  let addresses = try? JSONDecoder().decode([FoundAddress].self, from: data) else {
                return []
            }
            return addresses
        }
        set {
            if let data = try? JSONEncoder().encode(newValue) {
                UserDefaults.standard.set(data, forKey: "\(keyPrefix)_foundAddresses")
            }
        }
    }

    /// UserDefaults key prefix scoped to network.
    private var keyPrefix: String {
        "platformAddressSync_\(networkName)"
    }

    private var networkName: String = "testnet"

    // MARK: - Lifecycle

    /// Initialize for a network. Restores persisted state.
    /// The actual periodic loop is managed by UnifiedAppState.
    func startPeriodicSync(network: AppNetwork) {
        networkName = network.rawValue

        // Restore persisted state from previous session
        let height = persistedSyncHeight
        if height > 0 {
            lastSyncHeight = height
        }
        let blockTs = persistedBlockTime
        if blockTs > 0 {
            lastSyncBlockTime = Date(timeIntervalSince1970: TimeInterval(blockTs))
        }
        let recentBlock = persistedLastKnownRecentBlock
        if recentBlock > 0 {
            lastKnownRecentBlock = recentBlock
        }

        // Restore found addresses so balances show immediately on launch
        let saved = persistedFoundAddresses
        if !saved.isEmpty {
            lastFoundAddresses = saved
            var restoredBalances: [UInt32: UInt64] = [:]
            var restoredNonces: [UInt32: UInt32] = [:]
            var total: UInt64 = 0
            var nonZero = 0
            for addr in saved {
                restoredBalances[addr.index] = addr.balance
                restoredNonces[addr.index] = addr.nonce
                total += addr.balance
                if addr.balance > 0 { nonZero += 1 }
            }
            addressBalances = restoredBalances
            addressNonces = restoredNonces
            totalPlatformBalance = total
            activeAddressCount = nonZero
        }
    }

    /// Reset all state (e.g. on wallet deletion or network switch).
    func reset() {
        addressBalances.removeAll()
        addressNonces.removeAll()
        totalPlatformBalance = 0
        activeAddressCount = 0
        checkpointHeight = 0
        chainTipHeight = 0
        lastSyncHeight = 0
        lastKnownRecentBlock = 0
        lastSyncBlockTime = nil
        lastFoundAddresses = []
        lastRecentProof = Data()
        lastMetrics = nil
        lastError = nil
        lastSyncTime = nil
        syncCountSinceLaunch = 0
        totalTrunkQueries = 0
        totalBranchQueries = 0
        totalCompactedQueries = 0
        totalRecentQueries = 0
        totalRecentEntries = 0
        totalCompactedEntries = 0
        // Clear persisted state so next sync does a full tree scan
        lastSyncTimestamp = 0
        persistedSyncHeight = 0
        persistedBlockTime = 0
        persistedLastKnownRecentBlock = 0
        persistedFoundAddresses = []
    }

    /// Trigger a manual sync (e.g. pull-to-refresh). No-op if already syncing.
    func manualSync(sdk: SDK, addresses: [(index: UInt32, key: Data)]) async {
        await performSync(sdk: sdk, addresses: addresses)
    }

    // MARK: - Internal

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
                lastSyncTimestamp: lastSyncTimestamp,
                lastKnownRecentBlock: persistedLastKnownRecentBlock
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
            lastRecentProof = result.recentProof
            activeAddressCount = result.nonZeroBalanceCount
            lastMetrics = result.metrics
            lastFoundAddresses = result.found
            lastSyncHeight = result.newSyncHeight
            persistedSyncHeight = result.newSyncHeight
            if result.lastKnownRecentBlock > 0 {
                lastKnownRecentBlock = result.lastKnownRecentBlock
                persistedLastKnownRecentBlock = result.lastKnownRecentBlock
            }
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
            totalRecentEntries += result.metrics.recentEntriesReturned
            totalCompactedEntries += result.metrics.compactedEntriesReturned

            // Persist sync checkpoint for incremental mode
            if result.newSyncTimestamp > 0 {
                lastSyncTimestamp = result.newSyncTimestamp
            }

            // Persist found addresses for balance restoration across launches
            persistedFoundAddresses = result.found

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

