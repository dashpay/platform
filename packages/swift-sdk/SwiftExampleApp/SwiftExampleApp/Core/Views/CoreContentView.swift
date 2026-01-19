import SwiftUI
import SwiftDashSDK
import SwiftData

struct CoreContentView: View {
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    @Environment(\.modelContext) private var modelContext
    @Query private var wallets: [HDWallet]
    @Query private var platformSyncStates: [PersistentPlatformSyncState]
    @State private var showingCreateWallet = false

    // Platform sync state
    @State private var isPlatformSyncing = false
    @State private var platformSyncError: String?

    // Check if we have a stored platform wallet
    private var hasStoredPlatformWallet: Bool {
        guard let wallet = walletsForCurrentNetwork.first else { return false }
        let key = "platform_wallet_\(wallet.id.uuidString)"
        return KeychainManager.shared.retrieveKeyData(identifier: key) != nil
    }
    
    // Filter wallets by current network - show wallets that support the current network
    private var walletsForCurrentNetwork: [HDWallet] {
        return wallets
    }
    // Progress values come from WalletService (kept in sync with SPV callbacks)
    
    // Computed properties to ensure progress values are always valid
    private var safeHeaderProgress: Double { min(max(walletService.headerProgress, 0.0), 1.0) }
    private var safeFilterHeaderProgress: Double { min(max(walletService.filterHeaderProgress, 0.0), 1.0) }
    private var safeTransactionProgress: Double {
        // Use only the event-driven value to avoid misleading jumps
        min(max(walletService.transactionProgress, 0.0), 1.0)
    }

    // Display helpers
    private var headerHeightsDisplay: String? {
        let cur = max(walletService.headerCurrentHeight, 0)
        let tot = walletService.headerTargetHeight

        // Format current height allowing zero to render as "0" rather than "—"
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."
        let curStr = formatter.string(from: NSNumber(value: cur)) ?? String(cur)

        // When the chain tip is unknown (tot <= 0) fall back to the current baseline only.
        guard tot > 0 else { return curStr }

        let totStr = formattedHeight(tot)
        return "\(curStr)/\(totStr)"
    }

    private var filterHeaderHeightsDisplay: String? {
        let cur = max(walletService.latestFilterHeaderHeight, 0)
        let tot = walletService.headerTargetHeight

        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."

        let numerator = formatter.string(from: NSNumber(value: cur)) ?? String(cur)

        guard tot > 0 else { return numerator }

        let denominator = formattedHeight(tot)
        return "\(numerator)/\(denominator)"
    }

    private var filterHeightsDisplay: String? {
        let cur = max(walletService.latestFilterHeight, 0)
        let tot = walletService.headerTargetHeight

        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."

        let numerator = formatter.string(from: NSNumber(value: cur)) ?? String(cur)

        // When the chain tip is unknown (tot <= 0) show only the baseline.
        guard tot > 0 else { return numerator }

        let denominator = formattedHeight(tot)
        return "\(numerator)/\(denominator)"
    }
    
var body: some View {
    List {
            // Section 1: Core Sync Status (compact)
            Section {
                VStack(spacing: 8) {
                    // Compact progress rows
                    CompactSyncRow(
                        title: "Headers",
                        progress: safeHeaderProgress,
                        value: headerHeightsDisplay
                    )

                    CompactSyncRow(
                        title: "Filter Headers",
                        progress: safeFilterHeaderProgress,
                        value: filterHeaderHeightsDisplay
                    )

                    if walletService.shouldSyncMasternodes {
                        CompactSyncRow(
                            title: "Masternodes",
                            progress: walletService.masternodeProgress,
                            value: formattedHeight(walletService.latestMasternodeListHeight)
                        )
                    }

                    CompactSyncRow(
                        title: "Filters",
                        progress: safeTransactionProgress,
                        value: filterHeightsDisplay
                    )

                    // Controls row
                    HStack(spacing: 8) {
                        Text("Blocks hit: \(walletService.blocksHit)")
                            .font(.caption2)
                            .foregroundColor(.secondary)

                        Spacer()

                        Button(action: toggleSync) {
                            Text(walletService.isSyncing ? "Pause" : "Start")
                                .font(.caption)
                                .fontWeight(.medium)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(walletService.isSyncing ? .orange : .blue)
                        .controlSize(.mini)

                        Button(action: clearSyncData) {
                            Text("Clear")
                                .font(.caption)
                                .fontWeight(.medium)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.red)
                        .controlSize(.mini)
                        .disabled(walletService.isSyncing || walletService.isInitializing)
                        .opacity((walletService.isSyncing || walletService.isInitializing) ? 0.5 : 1.0)
                    }
                }
                .padding(.vertical, 4)
            } header: {
                Text("Core Sync Status")
            }

            // Section 2: Platform Sync Status
            Section {
                VStack(spacing: 8) {
                    // Show aggregated platform sync info
                    if let latestSync = latestPlatformSyncState {
                        HStack {
                            Text("Last Sync")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(latestSync.formattedLastSync)
                                .font(.subheadline)
                                .fontWeight(.medium)
                                .foregroundColor(latestSync.needsFullSync ? .orange : .green)
                        }

                        HStack {
                            Text("Platform Balance")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(latestSync.formattedBalance)
                                .font(.subheadline)
                                .fontWeight(.medium)
                        }

                        HStack {
                            Text("Checkpoint Height")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(latestSync.checkpointHeight > 0 ? "\(latestSync.checkpointHeight)" : "—")
                                .font(.subheadline)
                                .fontWeight(.medium)
                        }
                    } else {
                        HStack {
                            Text("Status")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text("Not synced")
                                .font(.subheadline)
                                .foregroundColor(.orange)
                        }
                    }

                    // Error display
                    if let error = platformSyncError {
                        Text(error)
                            .font(.caption)
                            .foregroundColor(.red)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }

                    HStack {
                        Spacer()

                        Button(action: startPlatformSync) {
                            if isPlatformSyncing {
                                ProgressView()
                                    .progressViewStyle(CircularProgressViewStyle())
                                    .scaleEffect(0.7)
                            } else {
                                Text("Start")
                                    .font(.caption)
                                    .fontWeight(.medium)
                            }
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.blue)
                        .controlSize(.mini)
                        .disabled(isPlatformSyncing || walletsForCurrentNetwork.isEmpty)

                        Button(action: clearPlatformSyncData) {
                            Text("Clear")
                                .font(.caption)
                                .fontWeight(.medium)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.red)
                        .controlSize(.mini)
                        .disabled(isPlatformSyncing)
                    }
                }
                .padding(.vertical, 4)
            } header: {
                HStack {
                    Text("Platform Sync Status")
                    Spacer()
                    NavigationLink(destination: PlatformAddressSyncView()) {
                        Image(systemName: "info.circle")
                            .foregroundColor(.blue)
                    }
                }
            }

            // Section 2: Wallets
            Section("Wallets (\(unifiedAppState.platformState.currentNetwork.displayName))") {
                if walletsForCurrentNetwork.isEmpty {
                    VStack(spacing: 12) {
                        Image(systemName: "wallet.pass")
                            .font(.system(size: 40))
                            .foregroundColor(.gray)
                        
                        Text("No \(unifiedAppState.platformState.currentNetwork.displayName) Wallets")
                            .font(.headline)
                        
                        Text("Create a wallet for \(unifiedAppState.platformState.currentNetwork.displayName)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        
                        Button {
                            showingCreateWallet = true
                        } label: {
                            Text("Create Wallet")
                                .foregroundColor(.white)
                                .padding(.horizontal, 16)
                                .padding(.vertical, 8)
                                .background(Color.blue)
                                .cornerRadius(8)
                        }
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 20)
                } else {
                    ForEach(walletsForCurrentNetwork) { wallet in
                        NavigationLink {
                            WalletDetailView(wallet: wallet)
                                .environmentObject(unifiedAppState)
                        } label: {
                            WalletRowView(wallet: wallet)
                        }
                    }
                }
            }
        }
        .navigationTitle("Wallets")
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    showingCreateWallet = true
                } label: {
                    Image(systemName: "plus")
                }
            }
        }
        .sheet(isPresented: $showingCreateWallet) {
            NavigationStack {
                CreateWalletView()
                    .environmentObject(walletService)
                    .environmentObject(unifiedAppState)
                    .environment(\.modelContext, modelContext)
            }
        }
        .onAppear {
            // Show detailed sync banner only on the Wallets root
            unifiedAppState.showWalletsSyncDetails = true
        }
        .onDisappear {
            unifiedAppState.showWalletsSyncDetails = false
        }
        // No local polling; rows bind to WalletService progress directly
    }
    
    // MARK: - Sync Methods
    
    private func toggleSync() {
        if walletService.isSyncing {
            pauseSync()
        } else {
            startSync()
        }
    }
    
    private func startSync() {
        Task {
            await walletService.startSync()
        }
    }
    
    private func pauseSync() {
        walletService.stopSync()
    }
    
    private func restartHeaderSync() {
        if walletService.isSyncing {
            // TODO: Call walletService.restartHeaderSync() when implemented
            print("Restarting header sync...")
        }
    }

    private func restartFilterHeaderSync() {
        if walletService.isSyncing {
            // TODO: Call walletService.restartFilterHeaderSync() when implemented
            print("Restarting filter header sync...")
        }
    }

    private func restartMasternodeSync() {
        if walletService.isSyncing {
            // TODO: Call walletService.restartMasternodeSync() when implemented
            print("Restarting masternode sync...")
        }
    }

    private func restartTransactionSync() {
        if walletService.isSyncing {
            // TODO: Call walletService.restartTransactionSync() when implemented
            print("Restarting transaction sync...")
        }
    }

    private func clearSyncData() {
        // Button is disabled during sync and initialization
        guard !walletService.isSyncing && !walletService.isInitializing else {
            print("⚠️ Clear button should be disabled during sync/initialization")
            return
        }

        walletService.clearSpvStorage(fullReset: true)
    }

    // MARK: - Platform Sync

    /// Get the latest platform sync state for display
    private var latestPlatformSyncState: PersistentPlatformSyncState? {
        // Return the most recently updated sync state
        platformSyncStates.max(by: { $0.lastUpdated < $1.lastUpdated })
    }

    private func startPlatformSync() {
        platformSyncError = nil
        if hasStoredPlatformWallet {
            Swift.print("DEBUG: Starting platform sync using stored wallet")
            Task {
                await performPlatformSync()
            }
        } else {
            Swift.print("DEBUG: No platform wallet available")
            platformSyncError = "No platform wallet available. Please set up a platform payment account first."
        }
    }

    private func clearPlatformSyncData() {
        // Clear persistent sync states
        for state in platformSyncStates {
            modelContext.delete(state)
        }
        try? modelContext.save()

        // Also clear cached platform wallet data
        for wallet in walletsForCurrentNetwork {
            let key = "platform_wallet_\(wallet.id.uuidString)"
            KeychainManager.shared.deleteKeyData(identifier: key)
        }
        Swift.print("DEBUG: Cleared platform sync data and cached wallets")
    }

    /// Perform platform sync using stored wallet data
    private func performPlatformSync() async {
        Swift.print("DEBUG: performPlatformSync started")
        await MainActor.run { isPlatformSyncing = true; platformSyncError = nil }

        defer {
            Task { @MainActor in
                isPlatformSyncing = false
            }
        }

        do {
            // 1. Get first available wallet
            guard let wallet = walletsForCurrentNetwork.first else {
                throw PlatformSyncError.noWalletAvailable
            }
            Swift.print("DEBUG: Using wallet: \(wallet.label)")

            // 2. Get SDK
            guard let sdk = unifiedAppState.sdk else {
                throw PlatformSyncError.sdkNotInitialized
            }

            // 3. Restore PlatformWallet from cached data
            Swift.print("DEBUG: Restoring platform wallet from cache...")
            let walletKey = "platform_wallet_\(wallet.id.uuidString)"
            guard let cachedData = KeychainManager.shared.retrieveKeyData(identifier: walletKey) else {
                throw PlatformSyncError.noPlatformWallet
            }
            Swift.print("DEBUG: Found cached data (\(cachedData.count) bytes)")

            let platformWallet = try PlatformWallet.restore(from: cachedData)
            Swift.print("DEBUG: Platform wallet restored from cache")

            // 4. Create or restore sync state
            let existingState = platformSyncStates.first { $0.walletId == wallet.id.uuidString && $0.accountCategory == "bip44" }
            let syncManager: PlatformSyncStateManager
            if let existing = existingState {
                syncManager = try PlatformSyncStateManager.create(from: existing.toPlatformSyncState())
            } else {
                syncManager = try PlatformSyncStateManager.create()
            }

            // 5. Perform sync
            Swift.print("DEBUG: Starting sync...")
            let result = try syncManager.syncAddresses(
                wallet: platformWallet,
                sdk: sdk
            )
            Swift.print("DEBUG: Sync completed! Balance: \(result.totalBalance)")

            // 6. Save state
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
                try? modelContext.save()
            }

        } catch {
            Swift.print("DEBUG: Platform sync error: \(error)")
            await MainActor.run {
                platformSyncError = error.localizedDescription
            }
        }
    }
}

// MARK: - Platform Sync Errors

enum PlatformSyncError: LocalizedError {
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
            return "Platform SDK not initialized."
        }
    }
}

// MARK: - Compact Sync Row

struct CompactSyncRow: View {
    let title: String
    let progress: Double
    let value: String?

    private var safeProgress: Double {
        min(max(progress, 0.0), 1.0)
    }

    var body: some View {
        HStack(spacing: 8) {
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
                .frame(width: 80, alignment: .leading)

            ProgressView(value: safeProgress)
                .progressViewStyle(LinearProgressViewStyle())
                .tint(progressColor)

            if let value = value {
                Text(value)
                    .font(.caption2)
                    .foregroundColor(.secondary)
                    .frame(minWidth: 60, alignment: .trailing)
            }
        }
    }

    private var progressColor: Color {
        if safeProgress >= 1.0 {
            return .green
        } else if safeProgress >= 0.5 {
            return .blue
        } else {
            return .orange
        }
    }
}

// MARK: - Sync Progress Row (Legacy)

struct SyncProgressRow: View {
    let title: String
    let progress: Double
    let detail: String
    let icon: String
    let trailingValue: String?
    let onRestart: () -> Void
    var navigationDestination: AnyView? = nil

    // Ensure progress is always between 0 and 1
    private var safeProgress: Double {
        min(max(progress, 0.0), 1.0)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                // Make only the label tappable if there's a navigation destination
                if let destination = navigationDestination {
                    NavigationLink(destination: destination) {
                        HStack(spacing: 6) {
                            Image(systemName: icon)
                                .font(.subheadline)
                            Text(title)
                                .font(.subheadline)
                                .fontWeight(.semibold)
                        }
                        .foregroundColor(.blue)
                    }
                    .buttonStyle(PlainButtonStyle())
                } else {
                    Label(title, systemImage: icon)
                        .font(.subheadline)
                        .foregroundColor(.primary)
                }

                Spacer()

                if let trailingValue = trailingValue {
                    Text(trailingValue)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Button(action: onRestart) {
                    Image(systemName: "arrow.clockwise")
                        .font(.caption)
                        .foregroundColor(.blue)
                }
                .buttonStyle(BorderlessButtonStyle())
            }

            VStack(alignment: .leading, spacing: 4) {
                ProgressView(value: safeProgress)
                    .progressViewStyle(LinearProgressViewStyle())
                    .tint(progressColor(for: safeProgress))

                Text(detail)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 4)
    }
    
    private func progressColor(for value: Double) -> Color {
        if value >= 1.0 {
            return .green
        } else if value >= 0.5 {
            return .blue
        } else {
            return .orange
        }
    }
}

// MARK: - Wallet Row View

struct WalletRowView: View {
    let wallet: HDWallet
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    
    private func getNetworksList() -> String {
        // Wallets are now single-network, just return the wallet's network
        return wallet.dashNetwork.rawValue.capitalized
    }
    
    var platformBalance: UInt64 {
        // Only sum balances of identities that belong to this specific wallet
        // and are on the same network
        
        // For now, if wallet doesn't have a walletId (not yet initialized with FFI),
        // don't show any platform balance
        guard let walletId = wallet.walletId else {
            return 0
        }
        
        return unifiedAppState.platformState.identities
            .filter { identity in
                // Check if identity belongs to this wallet and is on the same network
                // Only count identities that have been explicitly associated with this wallet
                identity.walletId == walletId &&
                identity.network == wallet.dashNetwork.rawValue
            }
            .reduce(0) { sum, identity in
                sum + identity.balance
            }
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(wallet.label)
                    .font(.headline)
                
                Spacer()
                
                if wallet.syncProgress < 1.0 {
                    ProgressView(value: min(max(wallet.syncProgress, 0.0), 1.0))
                        .frame(width: 50)
                }
            }
            
            HStack {
                // Show all networks this wallet supports
                HStack(spacing: 4) {
                    Image(systemName: "network")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    // Build the network list
                    Text(getNetworksList())
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                VStack(alignment: .trailing, spacing: 2) {
                    // Show wallet balance or "Empty"
                    if wallet.totalBalance == 0 {
                        Text("Empty")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else {
                        Text(formatBalance(wallet.totalBalance))
                            .font(.subheadline)
                            .fontWeight(.medium)
                    }
                    
                    // Show platform balance if any
                    if platformBalance > 0 {
                        HStack(spacing: 3) {
                            Image(systemName: "p.circle.fill")
                                .font(.system(size: 9))
                            Text(formatBalance(platformBalance))
                        }
                        .font(.caption2)
                        .foregroundColor(.blue)
                    }
                }
            }
        }
        .padding(.vertical, 4)
    }
    
    private func formatBalance(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        
        // Special case for zero
        if dash == 0 {
            return "0 DASH"
        }
        
        // Format with up to 8 decimal places, removing trailing zeros
        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."
        
        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return "\(formatted) DASH"
        }
        
        // Fallback formatting
        let formatted = String(format: "%.8f", dash)
        let trimmed = formatted.replacingOccurrences(of: "0+$", with: "", options: .regularExpression)
            .replacingOccurrences(of: "\\.$", with: "", options: .regularExpression)
        return "\(trimmed) DASH"
    }
}

// MARK: - Formatting Helpers
extension CoreContentView {
    func formattedHeight(_ height: Int) -> String {
        guard height > 0 else { return "—" }
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."
        return formatter.string(from: NSNumber(value: height)) ?? String(height)
    }
}
