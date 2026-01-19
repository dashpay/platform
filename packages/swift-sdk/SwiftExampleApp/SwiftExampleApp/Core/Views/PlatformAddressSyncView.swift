import SwiftUI
import SwiftDashSDK
import SwiftData

/// View for synchronizing platform payment address balances
struct PlatformAddressSyncView: View {
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    @Environment(\.modelContext) private var modelContext

    // Query all wallets and sync states
    @Query private var wallets: [HDWallet]
    @Query private var platformSyncStates: [PersistentPlatformSyncState]

    // Sync state
    @State private var isSyncing = false
    @State private var syncError: String?
    @State private var lastSyncResult: PlatformSyncResult?

    // Check if we have a stored platform wallet
    private var hasStoredPlatformWallet: Bool {
        guard let wallet = currentWallet else { return false }
        let key = "platform_wallet_\(wallet.id.uuidString)"
        return KeychainManager.shared.retrieveKeyData(identifier: key) != nil
    }

    // First wallet for current network
    private var currentWallet: HDWallet? {
        wallets.first { $0.dashNetwork == unifiedAppState.platformState.currentNetwork }
    }

    // Get most recent sync state
    private var latestSyncState: PersistentPlatformSyncState? {
        platformSyncStates.max(by: { $0.lastUpdated < $1.lastUpdated })
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                // Header Card
                headerCard

                // Current State Card
                currentStateCard

                // Last Sync Result Card (if available)
                if let result = lastSyncResult {
                    syncResultCard(result: result)
                }

                // Sync Button
                syncButton

                // Error display
                if let error = syncError {
                    errorCard(error: error)
                }
            }
            .padding()
        }
        .navigationTitle("Platform Address Sync")
        .navigationBarTitleDisplayMode(.large)
    }

    // MARK: - View Components

    private var headerCard: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("Privacy-Preserving Sync", systemImage: "shield.checkered")
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            Text("Synchronize your platform payment address balances using a privacy-preserving protocol. This helps detect any credits received to your payment addresses without revealing your addresses to the network.")
                .font(.subheadline)
                .foregroundColor(.secondary)

            VStack(alignment: .leading, spacing: 4) {
                if let wallet = currentWallet {
                    HStack {
                        Text("Wallet:")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(wallet.label)
                            .fontWeight(.medium)
                    }

                    HStack {
                        Text("Network:")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(wallet.dashNetwork.rawValue.capitalized)
                            .fontWeight(.medium)
                    }

                    HStack {
                        Text("Account:")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text("BIP44 #0 (Main)")
                            .fontWeight(.medium)
                    }

                    HStack {
                        Text("Platform Wallet:")
                            .foregroundColor(.secondary)
                        Spacer()
                        if hasStoredPlatformWallet {
                            HStack(spacing: 4) {
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundColor(.green)
                                Text("Available")
                                    .fontWeight(.medium)
                                    .foregroundColor(.green)
                            }
                        } else {
                            HStack(spacing: 4) {
                                Image(systemName: "xmark.circle.fill")
                                    .foregroundColor(.orange)
                                Text("Not configured")
                                    .fontWeight(.medium)
                                    .foregroundColor(.orange)
                            }
                        }
                    }
                } else {
                    HStack {
                        Image(systemName: "exclamationmark.triangle")
                            .foregroundColor(.orange)
                        Text("No wallet available")
                            .foregroundColor(.orange)
                    }
                }
            }
            .padding(.top, 8)
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    private var currentStateCard: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("Sync Status", systemImage: "arrow.triangle.2.circlepath")
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            if let state = latestSyncState {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("Last Full Sync:")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(state.formattedLastSync)
                            .fontWeight(.medium)
                            .foregroundColor(state.needsFullSync ? .orange : .green)
                    }

                    HStack {
                        Text("Total Balance:")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(state.formattedBalance)
                            .fontWeight(.semibold)
                            .foregroundColor(.primary)
                    }

                    HStack {
                        Text("Funded Addresses:")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text("\(state.fundedAddressCount)")
                            .fontWeight(.medium)
                    }

                    if state.highestFoundIndex >= 0 {
                        HStack {
                            Text("Highest Index Found:")
                                .foregroundColor(.secondary)
                            Spacer()
                            Text("#\(state.highestFoundIndex)")
                                .font(.system(.body, design: .monospaced))
                        }
                    }

                    HStack {
                        Text("Checkpoint Height:")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(state.checkpointHeight > 0 ? "\(state.checkpointHeight)" : "N/A")
                            .font(.system(.body, design: .monospaced))
                    }

                    HStack {
                        Text("Terminal Block:")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(state.lastTerminalBlock > 0 ? "\(state.lastTerminalBlock)" : "N/A")
                            .font(.system(.body, design: .monospaced))
                    }

                    if state.needsFullSync {
                        HStack {
                            Image(systemName: "exclamationmark.triangle.fill")
                                .foregroundColor(.orange)
                            Text("Full sync recommended")
                                .font(.caption)
                                .foregroundColor(.orange)
                        }
                        .padding(.top, 4)
                    }
                }
            } else {
                VStack(alignment: .center, spacing: 8) {
                    Image(systemName: "arrow.down.circle")
                        .font(.largeTitle)
                        .foregroundColor(.secondary)
                    Text("No sync data available")
                        .foregroundColor(.secondary)
                    Text("Tap 'Sync Now' to check for balances")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 8)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    private func syncResultCard(result: PlatformSyncResult) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Label("Latest Sync Result", systemImage: "checkmark.circle.fill")
                    .font(.headline)
                    .foregroundColor(.green)

                Spacer()

                if result.fullSyncPerformed {
                    Text("Full Sync")
                        .font(.caption)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(Color.blue.opacity(0.2))
                        .foregroundColor(.blue)
                        .cornerRadius(8)
                }
            }

            Divider()

            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Total Balance Found:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text(formatCredits(result.totalBalance))
                        .fontWeight(.semibold)
                        .foregroundColor(result.totalBalance > 0 ? .green : .primary)
                }

                HStack {
                    Text("Funded Addresses:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("\(result.fundedAddressCount)")
                        .fontWeight(.medium)
                }

                if let highestIndex = result.highestFoundIndex {
                    HStack {
                        Text("Highest Index:")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text("#\(highestIndex)")
                            .font(.system(.body, design: .monospaced))
                    }
                }

                HStack {
                    Text("Checkpoint Height:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("\(result.checkpointHeight)")
                        .font(.system(.body, design: .monospaced))
                }
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    private var syncButton: some View {
        VStack(spacing: 12) {
            Button(action: {
                syncError = nil
                if hasStoredPlatformWallet {
                    Swift.print("DEBUG: Sync button tapped, using stored wallet")
                    Task {
                        await performSync()
                    }
                } else {
                    Swift.print("DEBUG: No platform wallet available")
                    syncError = "No platform wallet available. Please set up a platform payment account first."
                }
            }) {
                HStack {
                    if isSyncing {
                        ProgressView()
                            .progressViewStyle(CircularProgressViewStyle(tint: .white))
                            .scaleEffect(0.8)
                    } else {
                        Image(systemName: "arrow.triangle.2.circlepath")
                    }
                    Text(isSyncing ? "Syncing..." : "Sync Now")
                        .fontWeight(.semibold)
                }
                .frame(maxWidth: .infinity)
                .padding()
                .background(isSyncing || currentWallet == nil || !hasStoredPlatformWallet ? Color.gray : Color.blue)
                .foregroundColor(.white)
                .cornerRadius(12)
            }
            .disabled(isSyncing || currentWallet == nil || !hasStoredPlatformWallet)

            // Show status message
            if hasStoredPlatformWallet {
                HStack {
                    Image(systemName: "checkmark.shield.fill")
                        .foregroundColor(.green)
                    Text("Platform wallet ready for sync")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            } else {
                HStack {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(.orange)
                    Text("Set up a platform payment account to enable sync")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }
        }
    }

    private func errorCard(error: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundColor(.red)
                Text("Sync Error")
                    .font(.headline)
                    .foregroundColor(.red)
            }

            Text(error)
                .font(.subheadline)
                .foregroundColor(.secondary)
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.red.opacity(0.1))
        .cornerRadius(12)
    }

    // MARK: - Helper Methods

    private func formatCredits(_ credits: UInt64) -> String {
        // Credits to DASH: 1 DASH = 100,000,000,000 credits (10^11)
        let dash = Double(credits) / 100_000_000_000.0
        return String(format: "%.8f DASH", dash)
    }

    /// Perform sync using stored platform wallet data
    private func performSync() async {
        Swift.print("DEBUG: performSync started")
        await MainActor.run { isSyncing = true; syncError = nil }

        defer {
            Task { @MainActor in
                isSyncing = false
                Swift.print("DEBUG: performSync completed")
            }
        }

        do {
            // 1. Get wallet
            guard let wallet = currentWallet else {
                throw SyncError.noWalletAvailable
            }
            Swift.print("DEBUG: Using wallet: \(wallet.label)")

            // 2. Get the Platform SDK
            guard let sdk = unifiedAppState.sdk else {
                Swift.print("DEBUG: ERROR - SDK not initialized")
                throw SyncError.sdkNotInitialized
            }

            // 3. Restore PlatformWallet from cached data
            Swift.print("DEBUG: Restoring platform wallet from cache")
            let walletKey = "platform_wallet_\(wallet.id.uuidString)"
            guard let cachedData = KeychainManager.shared.retrieveKeyData(identifier: walletKey) else {
                throw SyncError.noPlatformWallet
            }
            Swift.print("DEBUG: Found cached data (\(cachedData.count) bytes)")

            let platformWallet = try PlatformWallet.restore(from: cachedData)
            Swift.print("DEBUG: Platform wallet restored from cache")

            // 4. Create or restore sync state manager
            let existingState = platformSyncStates.first {
                $0.walletId == wallet.id.uuidString && $0.accountCategory == "bip44"
            }
            let syncManager: PlatformSyncStateManager
            if let existing = existingState {
                Swift.print("DEBUG: Restoring from existing sync state")
                let platformSyncState = existing.toPlatformSyncState()
                syncManager = try PlatformSyncStateManager.create(from: platformSyncState)
            } else {
                Swift.print("DEBUG: Creating new sync state")
                syncManager = try PlatformSyncStateManager.create()
            }

            // 5. Perform sync
            Swift.print("DEBUG: Performing sync")
            let result = try syncManager.syncAddresses(
                wallet: platformWallet,
                sdk: sdk
            )
            Swift.print("DEBUG: Sync completed, balance: \(result.totalBalance)")

            // 6. Update or create persistent state
            await MainActor.run {
                if let existing = existingState {
                    existing.update(from: result)
                } else {
                    let newState = PersistentPlatformSyncState(
                        walletId: wallet.id.uuidString,
                        accountCategory: "bip44",
                        accountIndex: 0,
                        network: wallet.dashNetwork.rawValue
                    )
                    newState.update(from: result)
                    modelContext.insert(newState)
                }

                do {
                    try modelContext.save()
                } catch {
                    print("Failed to save sync state: \(error)")
                }

                lastSyncResult = result
            }

        } catch {
            Swift.print("DEBUG: Sync error: \(error)")
            await MainActor.run {
                syncError = error.localizedDescription
            }
        }
    }
}

// MARK: - Sync Errors

enum SyncError: LocalizedError {
    case noWalletAvailable
    case noPlatformWallet
    case sdkNotInitialized

    var errorDescription: String? {
        switch self {
        case .noWalletAvailable:
            return "No wallet available. Create a wallet first."
        case .noPlatformWallet:
            return "No platform wallet available. Please set up a platform payment account first."
        case .sdkNotInitialized:
            return "Platform SDK is not initialized. Please wait for initialization to complete."
        }
    }
}

// MARK: - Preview

struct PlatformAddressSyncView_Previews: PreviewProvider {
    static var previews: some View {
        Text("Preview requires app context")
            .padding()
    }
}
