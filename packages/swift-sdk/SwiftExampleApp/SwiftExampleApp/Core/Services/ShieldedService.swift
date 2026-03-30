import Foundation
import SwiftDashSDK

/// App-level service for shielded (ZK/Orchard) pool operations.
/// Publishes balance, address, and sync status for the UI layer.
@MainActor
class ShieldedService: ObservableObject {
    @Published var shieldedBalance: UInt64 = 0
    @Published var orchardDisplayAddress: String?
    @Published var isSyncing: Bool = false
    @Published var lastError: String?

    private(set) var poolClient: ShieldedPoolClient?
    private var spendingKey: Data?
    private var currentNetwork: AppNetwork?

    /// Initialize (or reinitialize) the shielded pool client.
    ///
    /// Call after wallet seed is available or on network switch.
    /// Uses the first 32 bytes of the wallet seed as the spending key.
    ///
    /// - Parameters:
    ///   - seed: Wallet seed (>= 32 bytes). First 32 bytes used as spending key.
    ///   - network: Current network.
    func initialize(seed: Data, network: AppNetwork) {
        guard seed.count >= 32 else {
            lastError = "Seed must be at least 32 bytes"
            return
        }

        let sk = seed.prefix(32)
        self.spendingKey = Data(sk)
        self.currentNetwork = network

        let dbPath = Self.dbPath(for: network)

        do {
            poolClient = try ShieldedPoolClient(dbPath: dbPath, spendingKey: Data(sk))
            let rawAddress = try poolClient!.address
            orchardDisplayAddress = DashAddress.encodeOrchard(rawBytes: rawAddress, network: network)
            shieldedBalance = try poolClient!.balance
            lastError = nil
        } catch {
            lastError = "Failed to init shielded pool: \(error.localizedDescription)"
            poolClient = nil
            orchardDisplayAddress = nil
            shieldedBalance = 0
        }
    }

    /// Sync notes from the network.
    func syncNotes(sdk: SDK) async throws {
        guard let client = poolClient else {
            throw SDKError.invalidState("Shielded pool not initialized")
        }

        isSyncing = true
        defer { isSyncing = false }

        let result = try await client.syncNotes(sdk: sdk)
        shieldedBalance = result.balance
    }

    /// Sync nullifiers (mark spent notes).
    func syncNullifiers(sdk: SDK) async throws {
        guard let client = poolClient else {
            throw SDKError.invalidState("Shielded pool not initialized")
        }

        isSyncing = true
        defer { isSyncing = false }

        let result = try await client.syncNullifiers(sdk: sdk)
        shieldedBalance = result.balance
    }

    /// Full sync: notes then nullifiers.
    func fullSync(sdk: SDK) async {
        do {
            try await syncNotes(sdk: sdk)
            try await syncNullifiers(sdk: sdk)
            lastError = nil
        } catch {
            lastError = "Sync error: \(error.localizedDescription)"
        }
    }

    /// Refresh balance from the local database (no network call).
    func refreshBalance() {
        guard let client = poolClient else { return }
        do {
            shieldedBalance = try client.balance
        } catch {
            lastError = "Balance refresh error: \(error.localizedDescription)"
        }
    }

    /// Reset state (e.g., on wallet deletion or logout).
    func reset() {
        poolClient = nil
        spendingKey = nil
        shieldedBalance = 0
        orchardDisplayAddress = nil
        isSyncing = false
        lastError = nil
    }

    // MARK: - Private

    private static func dbPath(for network: AppNetwork) -> String {
        let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
        return docs.appendingPathComponent("shielded_\(network.rawValue).sqlite").path
    }
}
