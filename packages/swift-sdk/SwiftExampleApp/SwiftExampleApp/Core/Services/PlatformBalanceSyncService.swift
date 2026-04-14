// PlatformBalanceSyncService.swift
// SwiftExampleApp
//
// App-level service that performs periodic BLAST address sync.
// Uses platform-wallet-ffi when a PlatformAddressWallet is configured,
// otherwise falls back to the direct rs-sdk-ffi path.

import Foundation
import SwiftUI
import SwiftDashSDK

/// Observable service managing periodic BLAST address balance sync.
@MainActor
class PlatformBalanceSyncService: ObservableObject {
    // MARK: - Published State

    @Published var isSyncing = false
    @Published var lastSyncTime: Date?
    @Published var addressBalances: [UInt32: UInt64] = [:]
    @Published var addressNonces: [UInt32: UInt32] = [:]
    @Published var totalPlatformBalance: UInt64 = 0
    @Published var activeAddressCount: Int = 0
    @Published var checkpointHeight: UInt64 = 0
    @Published var chainTipHeight: UInt64 = 0
    @Published var lastSyncHeight: UInt64 = 0
    @Published var lastKnownRecentBlock: UInt64 = 0
    @Published var lastSyncBlockTime: Date?
    @Published var lastMetrics: AddressSyncMetrics?
    @Published var syncCountSinceLaunch: Int = 0
    @Published var totalTrunkQueries: UInt32 = 0
    @Published var totalBranchQueries: UInt32 = 0
    @Published var totalCompactedQueries: UInt32 = 0
    @Published var totalRecentQueries: UInt32 = 0
    @Published var totalRecentEntries: UInt32 = 0
    @Published var totalCompactedEntries: UInt32 = 0
    @Published var lastRecentProof: Data = Data()
    @Published var lastError: String?

    /// Found addresses from the last sync — passed back as known balances for incremental mode.
    private(set) var lastFoundAddresses: [FoundAddress] = []

    // MARK: - Internal State

    /// Platform address wallet handle (when using platform-wallet path).
    private var platformAddressWallet: ManagedPlatformAddressWallet?

    /// UserDefaults state for the legacy SDK path.
    private var lastSyncTimestamp: UInt64 {
        get { UInt64(UserDefaults.standard.integer(forKey: "\(keyPrefix)_timestamp")) }
        set { UserDefaults.standard.set(Int(newValue), forKey: "\(keyPrefix)_timestamp") }
    }
    private var persistedSyncHeight: UInt64 {
        get { UInt64(UserDefaults.standard.integer(forKey: "\(keyPrefix)_height")) }
        set { UserDefaults.standard.set(Int(newValue), forKey: "\(keyPrefix)_height") }
    }
    private var persistedBlockTime: UInt64 {
        get { UInt64(UserDefaults.standard.integer(forKey: "\(keyPrefix)_blockTime")) }
        set { UserDefaults.standard.set(Int(newValue), forKey: "\(keyPrefix)_blockTime") }
    }
    private var persistedLastKnownRecentBlock: UInt64 {
        get { UInt64(UserDefaults.standard.integer(forKey: "\(keyPrefix)_lastKnownRecent")) }
        set { UserDefaults.standard.set(Int(newValue), forKey: "\(keyPrefix)_lastKnownRecent") }
    }
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
    private var keyPrefix: String { "platformAddressSync_\(networkName)" }
    private var networkName: String = "testnet"

    // MARK: - Configuration

    /// Configure for platform-wallet path (preferred).
    func configure(platformAddressWallet: ManagedPlatformAddressWallet) {
        self.platformAddressWallet = platformAddressWallet
    }

    /// Initialize periodic sync. Restores persisted state for legacy path.
    func startPeriodicSync(network: AppNetwork) {
        networkName = network.rawValue

        // Restore persisted state for legacy SDK path
        let height = persistedSyncHeight
        if height > 0 { lastSyncHeight = height }
        let blockTs = persistedBlockTime
        if blockTs > 0 { lastSyncBlockTime = Date(timeIntervalSince1970: TimeInterval(blockTs)) }
        let recentBlock = persistedLastKnownRecentBlock
        if recentBlock > 0 { lastKnownRecentBlock = recentBlock }

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
        lastSyncTimestamp = 0
        persistedSyncHeight = 0
        persistedBlockTime = 0
        persistedLastKnownRecentBlock = 0
        persistedFoundAddresses = []
        platformAddressWallet = nil
    }

    func manualSync(sdk: SDK, addresses: [(index: UInt32, key: Data)]) async {
        await performSync(sdk: sdk, addresses: addresses)
    }

    // MARK: - Sync (dispatches to platform-wallet or legacy path)

    func performSync(sdk: SDK, addresses: [(index: UInt32, key: Data)]) async {
        guard !isSyncing else { return }

        // TODO: When PlatformWalletManager is wired up in the app,
        // the platform-wallet path will be used automatically.
        // For now, always use the legacy SDK path since we don't
        // have PlatformWalletManager creating the wallet yet.
        await performSyncLegacy(sdk: sdk, addresses: addresses)
    }

    // MARK: - Legacy SDK Path

    private func performSyncLegacy(sdk: SDK, addresses: [(index: UInt32, key: Data)]) async {
        guard !addresses.isEmpty else { return }

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
            if result.checkpointHeight > 0 { checkpointHeight = result.checkpointHeight }
            if result.newSyncHeight > chainTipHeight { chainTipHeight = result.newSyncHeight }
            if result.newSyncTimestamp > 0 {
                lastSyncBlockTime = Date(timeIntervalSince1970: TimeInterval(result.newSyncTimestamp))
                persistedBlockTime = result.newSyncTimestamp
                lastSyncTimestamp = result.newSyncTimestamp
            }
            lastSyncTime = Date()
            syncCountSinceLaunch += 1
            totalTrunkQueries += result.metrics.trunkQueries
            totalBranchQueries += result.metrics.branchQueries
            totalCompactedQueries += result.metrics.compactedQueries
            totalRecentQueries += result.metrics.recentQueries
            totalRecentEntries += result.metrics.recentEntriesReturned
            totalCompactedEntries += result.metrics.compactedEntriesReturned

            persistedFoundAddresses = result.found

        } catch {
            lastError = error.localizedDescription
        }

        isSyncing = false
    }
}
