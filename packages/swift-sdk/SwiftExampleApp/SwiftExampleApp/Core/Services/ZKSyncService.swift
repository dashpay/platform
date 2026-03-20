// ZKSyncService.swift
// SwiftExampleApp
//
// App-level service that performs periodic ZK shielded sync (notes + nullifiers)
// with UI status display. Follows the same pattern as PlatformBalanceSyncService.

import Foundation
import SwiftUI
import SwiftDashSDK

/// Observable service managing periodic ZK shielded pool sync.
///
/// Syncs every 30 seconds while the app is active, or on manual pull-to-refresh.
/// Persists `shieldedBalance` and `orchardAddress` in UserDefaults for display across launches.
@MainActor
class ZKSyncService: ObservableObject {
    // MARK: - Published State

    /// Whether a sync is currently in progress.
    @Published var isSyncing: Bool = false

    /// Last successful sync time (local clock).
    @Published var lastSyncTime: Date?

    /// Current shielded balance (in credits).
    @Published var shieldedBalance: UInt64 = 0

    /// Orchard display address (Bech32m-encoded).
    @Published var orchardAddress: String?

    /// Number of new notes found in the most recent sync.
    @Published var notesSynced: Int = 0

    /// Number of nullifiers spent in the most recent sync.
    @Published var nullifiersSpent: Int = 0

    /// Cumulative notes synced since launch.
    @Published var totalNotesSynced: Int = 0

    /// Cumulative nullifiers spent since launch.
    @Published var totalNullifiersSpent: Int = 0

    /// Total number of successful syncs since launch.
    @Published var syncCountSinceLaunch: Int = 0

    /// Last error message, cleared on successful sync.
    @Published var lastError: String?

    // MARK: - Persisted State

    /// Persisted shielded balance (credits).
    private var persistedBalance: UInt64 {
        get { UInt64(UserDefaults.standard.integer(forKey: "\(keyPrefix)_balance")) }
        set { UserDefaults.standard.set(Int(newValue), forKey: "\(keyPrefix)_balance") }
    }

    /// Persisted orchard address string.
    private var persistedOrchardAddress: String? {
        get { UserDefaults.standard.string(forKey: "\(keyPrefix)_orchardAddress") }
        set { UserDefaults.standard.set(newValue, forKey: "\(keyPrefix)_orchardAddress") }
    }

    /// UserDefaults key prefix scoped to network.
    private var keyPrefix: String {
        "zkSync_\(networkName)"
    }

    private var networkName: String = "testnet"

    // MARK: - Lifecycle

    /// Initialize for a network. Restores persisted balance and address.
    /// The actual periodic loop is managed by UnifiedAppState.
    func startPeriodicSync(network: AppNetwork) {
        networkName = network.rawValue

        // Restore persisted state from previous session
        let savedBalance = persistedBalance
        if savedBalance > 0 {
            shieldedBalance = savedBalance
        }

        let savedAddress = persistedOrchardAddress
        if let addr = savedAddress, !addr.isEmpty {
            orchardAddress = addr
        }
    }

    /// Perform a single ZK shielded sync (notes then nullifiers).
    ///
    /// - Parameters:
    ///   - sdk: The initialized SDK instance.
    ///   - shieldedService: The shielded service with an initialized pool client.
    func performSync(sdk: SDK, shieldedService: ShieldedService) async {
        guard !isSyncing else { return }
        guard let poolClient = shieldedService.poolClient else { return }

        isSyncing = true
        lastError = nil

        do {
            // Step 1: Sync notes
            let notesResult = try await poolClient.syncNotes(sdk: sdk)
            let newNotes = notesResult.newNotes

            // Step 2: Sync nullifiers
            let nullifiersResult = try await poolClient.syncNullifiers(sdk: sdk)
            let spentCount = nullifiersResult.spentCount
            let finalBalance = nullifiersResult.balance

            // Update per-sync stats
            notesSynced = newNotes
            nullifiersSpent = spentCount

            // Update cumulative stats
            totalNotesSynced += newNotes
            totalNullifiersSpent += spentCount

            // Update balance and address
            shieldedBalance = finalBalance
            orchardAddress = shieldedService.orchardDisplayAddress

            // Persist balance and address
            persistedBalance = finalBalance
            persistedOrchardAddress = shieldedService.orchardDisplayAddress

            // Update sync metadata
            lastSyncTime = Date()
            syncCountSinceLaunch += 1

            SDKLogger.log(
                "ZK sync complete: \(newNotes) notes, \(spentCount) spent, balance: \(finalBalance)",
                minimumLevel: .medium
            )

        } catch {
            lastError = error.localizedDescription
            SDKLogger.log(
                "ZK sync error: \(error.localizedDescription)",
                minimumLevel: .medium
            )
        }

        isSyncing = false
    }

    /// Reset all state (e.g. on wallet deletion or network switch).
    func reset() {
        isSyncing = false
        lastSyncTime = nil
        shieldedBalance = 0
        orchardAddress = nil
        notesSynced = 0
        nullifiersSpent = 0
        totalNotesSynced = 0
        totalNullifiersSpent = 0
        syncCountSinceLaunch = 0
        lastError = nil

        // Clear persisted state
        persistedBalance = 0
        persistedOrchardAddress = nil
    }
}
