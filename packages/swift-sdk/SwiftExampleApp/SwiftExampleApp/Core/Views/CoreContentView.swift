import SwiftUI
import SwiftData

struct CoreContentView: View {
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    @Environment(\.modelContext) private var modelContext
    @Query private var wallets: [HDWallet]
    @State private var showingCreateWallet = false
    
    // Filter wallets by current network - show wallets that support the current network
    private var walletsForCurrentNetwork: [HDWallet] {
        let currentNetwork = unifiedAppState.platformState.currentNetwork
        // No conversion needed, just use currentNetwork directly
        
        // Check if wallet supports the current network using the networks bitfield
        let networkBit: UInt32
        switch currentNetwork {
        case .mainnet:
            networkBit = 1  // DASH
        case .testnet:
            networkBit = 2  // TESTNET
        case .devnet:
            networkBit = 8  // DEVNET
        }
        
        return wallets.filter { wallet in
            // Check if the wallet has this network enabled in its bitfield
            (wallet.networks & networkBit) != 0
        }
    }
    // Progress values come from WalletService (kept in sync with SPV callbacks)
    
    // Computed properties to ensure progress values are always valid
    private var safeHeaderProgress: Double { min(max(walletService.headerProgress, 0.0), 1.0) }
    private var safeMasternodeProgress: Double { min(max(walletService.masternodeProgress, 0.0), 1.0) }
    private var safeTransactionProgress: Double { min(max(walletService.transactionProgress, 0.0), 1.0) }
    
var body: some View {
    List {
            // Section 1: Sync Status
            Section("Sync Status") {
                VStack(spacing: 16) {
                    // Main sync control
                    HStack {
                        if walletService.isSyncing {
                            Label("Syncing", systemImage: "arrow.triangle.2.circlepath")
                                .font(.headline)
                                .foregroundColor(.blue)
                        } else {
                            Label("Sync Paused", systemImage: "pause.circle")
                                .font(.headline)
                                .foregroundColor(.secondary)
                        }
                        
                        Spacer()
                        
                        Button(action: toggleSync) {
                            HStack(spacing: 4) {
                                Image(systemName: walletService.isSyncing ? "pause.fill" : "play.fill")
                                Text(walletService.isSyncing ? "Pause" : "Start")
                            }
                            .padding(.horizontal, 16)
                            .padding(.vertical, 8)
                            .background(walletService.isSyncing ? Color.orange : Color.blue)
                            .foregroundColor(.white)
                            .cornerRadius(8)
                        }
                    }
                    
                    // Headers sync progress
                    SyncProgressRow(
                        title: "Headers",
                        progress: safeHeaderProgress,
                        detail: "\(Int(safeHeaderProgress * 100))% complete",
                        icon: "doc.text",
                        trailingValue: formattedHeight(walletService.latestHeaderHeight),
                        onRestart: restartHeaderSync
                    )
                    
                    // Masternode list sync progress
                    SyncProgressRow(
                        title: "Masternode List",
                        progress: safeMasternodeProgress,
                        detail: "\(Int(safeMasternodeProgress * 100))% complete",
                        icon: "server.rack",
                        trailingValue: formattedHeight(walletService.latestMasternodeListHeight),
                        onRestart: restartMasternodeSync
                    )
                    
                    // Transactions sync progress (filters/blocks)
                    SyncProgressRow(
                        title: "Transactions",
                        progress: safeTransactionProgress,
                        detail: "Filters & Blocks: \(Int(safeTransactionProgress * 100))%",
                        icon: "arrow.left.arrow.right",
                        trailingValue: formattedHeight(walletService.latestFilterHeight),
                        onRestart: restartTransactionSync
                    )
                }
                .padding(.vertical, 8)
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
}

// MARK: - Sync Progress Row

struct SyncProgressRow: View {
    let title: String
    let progress: Double
    let detail: String
    let icon: String
    let trailingValue: String?
    let onRestart: () -> Void
    
    // Ensure progress is always between 0 and 1
    private var safeProgress: Double {
        min(max(progress, 0.0), 1.0)
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Label(title, systemImage: icon)
                    .font(.subheadline)
                    .foregroundColor(.primary)
                
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
        var networks: [String] = []
        
        // Check each network bit
        if (wallet.networks & 1) != 0 {
            networks.append("Mainnet")
        }
        if (wallet.networks & 2) != 0 {
            networks.append("Testnet")
        }
        if (wallet.networks & 8) != 0 {
            networks.append("Devnet")
        }
        
        // If no networks set (shouldn't happen after migration), show the original network
        if networks.isEmpty {
            return wallet.dashNetwork.rawValue.capitalized
        }
        
        return networks.joined(separator: ", ")
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
