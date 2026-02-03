import SwiftUI
import SwiftDashSDK
import SwiftData

struct CoreContentView: View {
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    @Environment(\.modelContext) private var modelContext
    @Query private var wallets: [HDWallet]
    @State private var showingCreateWallet = false

    // Filter wallets by current network - show wallets that support the current network
    private var walletsForCurrentNetwork: [HDWallet] {
        return wallets
    }
    // Progress values come from WalletService (kept in sync with SPV callbacks)

    // Display helpers
    private var headerHeightsDisplay: String? {
        let cur = walletService.syncProgress.headers?.currentHeight ?? 0
        let tot = walletService.syncProgress.headers?.targetHeight ?? 0

        return heightDisplay(numerator: cur, denominator: tot)
    }

    private var filterHeaderHeightsDisplay: String? {
        let cur = walletService.syncProgress.filterHeaders?.currentHeight ?? 0
        let tot = walletService.syncProgress.filterHeaders?.targetHeight ?? 0

        return heightDisplay(numerator: cur, denominator: tot)
    }

    private var filterHeightsDisplay: String? {
        let cur = walletService.syncProgress.filters?.currentHeight ?? 0
        let tot = walletService.syncProgress.filters?.targetHeight ?? 0

        return heightDisplay(numerator: cur, denominator: tot)
    }

    private var masternodeHeightsDisplay: String? {
        let cur = walletService.syncProgress.masternodes?.currentHeight ?? 0
        let tot = walletService.syncProgress.masternodes?.targetHeight ?? 0

        return heightDisplay(numerator: cur, denominator: tot)
    }

    private func heightDisplay(numerator: UInt32, denominator: UInt32) -> String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."

        let numeratorStr = formatter.string(from: NSNumber(value: numerator)) ?? String(numerator)
        let denominatorStr = formattedHeight(denominator)
        return "\(numeratorStr)/\(denominatorStr)"
    }

var body: some View {
    List {
            // Section 1: Core Sync Status (compact)
            Section {
                VStack(spacing: 8) {
                    // Compact progress rows
                    CompactSyncRow(
                        title: "Headers",
                        progress: walletService.syncProgress.headers?.percentage ?? 0.0,
                        value: headerHeightsDisplay
                    )

                    CompactSyncRow(
                        title: "Filter Headers",
                        progress: walletService.syncProgress.filterHeaders?.percentage ?? 0.0,
                        value: filterHeaderHeightsDisplay
                    )

                    if walletService.masternodesEnabled {
                        CompactSyncRow(
                            title: "Masternodes",
                            progress: 0.0,
                            value: masternodeHeightsDisplay
                        )
                    }

                    CompactSyncRow(
                        title: "Filters",
                        progress: walletService.syncProgress.filters?.percentage ?? 0.0,
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
                        .disabled(walletService.isSyncing)
                        .opacity((walletService.isSyncing) ? 0.5 : 1.0)
                    }
                }
                .padding(.vertical, 4)
            } header: {
                Text("Core Sync Status")
            }

            // Section 2: Platform Sync Status
            Section {
                VStack(spacing: 8) {
                    HStack {
                        Text("Last Block Height")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                        Spacer()
                        Text("—")
                            .font(.subheadline)
                            .fontWeight(.medium)
                    }

                    HStack {
                        Spacer()

                        Button(action: { /* TODO: Start platform sync */ }) {
                            Text("Start")
                                .font(.caption)
                                .fontWeight(.medium)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.blue)
                        .controlSize(.mini)

                        Button(action: { /* TODO: Clear platform sync */ }) {
                            Text("Clear")
                                .font(.caption)
                                .fontWeight(.medium)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.red)
                        .controlSize(.mini)
                    }
                }
                .padding(.vertical, 4)
            } header: {
                Text("Platform Sync Status")
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
        // Button is disabled during sync
        guard !walletService.isSyncing else {
            print("⚠️ Clear button should be disabled during sync")
            return
        }

        walletService.clearSpvStorage()
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
    func formattedHeight(_ height: UInt32) -> String {
        guard height > 0 else { return "—" }
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."
        return formatter.string(from: NSNumber(value: height)) ?? String(height)
    }
}
