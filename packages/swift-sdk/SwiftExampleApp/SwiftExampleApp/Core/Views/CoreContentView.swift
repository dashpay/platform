import SwiftUI
import SwiftDashSDK
import SwiftData

struct CoreContentView: View {
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService
    // Progress values come from WalletService (kept in sync with SPV callbacks)

    // Display helpers
    private var headerHeightsDisplay: String? {
        let headers = walletService.syncProgress.headers
        let cur = (headers?.currentHeight ?? 0) + (headers?.buffered ?? 0)
        let tot = headers?.targetHeight ?? 0

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
                        Spacer()

                        Button(action: toggleSync) {
                            Text(walletService.syncProgress.state.isRunning() ? "Pause" : "Start")
                                .font(.caption)
                                .fontWeight(.medium)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(walletService.syncProgress.state.isRunning() ? .orange : .blue)
                        .controlSize(.mini)

                        Button(action: clearSyncData) {
                            Text("Clear")
                                .font(.caption)
                                .fontWeight(.medium)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.red)
                        .controlSize(.mini)
                        .disabled(walletService.syncProgress.state.isRunning())
                        .opacity((walletService.syncProgress.state.isRunning()) ? 0.5 : 1.0)
                    }
                }
                .padding(.vertical, 4)
            } header: {
                Text("Core Sync Status")
            }

            // Section 2: Platform Sync Status (BLAST address sync)
            Section {
                VStack(spacing: 8) {
                    // Sync state row
                    HStack {
                        if platformBalanceSyncService.isSyncing {
                            ProgressView()
                                .scaleEffect(0.7)
                            Text("Syncing...")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        } else if let lastSync = platformBalanceSyncService.lastSyncTime {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                                .font(.caption)
                            Text("Last sync: \(lastSync, style: .relative)")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        } else {
                            Image(systemName: "circle.dashed")
                                .foregroundColor(.secondary)
                                .font(.caption)
                            Text("Not synced yet")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }
                        Spacer()
                    }

                    // Balance summary
                    HStack {
                        Text("Platform Balance")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                        Spacer()
                        if platformBalanceSyncService.totalPlatformBalance > 0 {
                            Text(formatCredits(platformBalanceSyncService.totalPlatformBalance))
                                .font(.subheadline)
                                .fontWeight(.medium)
                        } else {
                            Text("0")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }
                    }

                    // Active addresses
                    HStack {
                        Text("Active Addresses")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                        Spacer()
                        Text("\(platformBalanceSyncService.activeAddressCount)")
                            .font(.subheadline)
                            .fontWeight(.medium)
                    }

                    // Chain tip height
                    if platformBalanceSyncService.chainTipHeight > 0 {
                        HStack {
                            Text("Chain Tip Height")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(formattedHeight(UInt32(platformBalanceSyncService.chainTipHeight)))
                                .font(.subheadline)
                                .fontWeight(.medium)
                        }
                    }

                    // Sync checkpoint (from tree scan)
                    if platformBalanceSyncService.checkpointHeight > 0 {
                        HStack {
                            Text("Sync Checkpoint")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(formattedHeight(UInt32(platformBalanceSyncService.checkpointHeight)))
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }
                    }

                    // Last known recent block (for compaction detection)
                    if platformBalanceSyncService.lastKnownRecentBlock > 0 {
                        HStack {
                            Text("Last Recent Block")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(formattedHeight(UInt32(platformBalanceSyncService.lastKnownRecentBlock)))
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }
                    }

                    // Block time
                    if let blockTime = platformBalanceSyncService.lastSyncBlockTime {
                        HStack {
                            Text("Block Time")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(blockTime, style: .date)
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(blockTime, style: .time)
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }

                    // Query counts since launch
                    if platformBalanceSyncService.syncCountSinceLaunch > 0 {
                        let svc = platformBalanceSyncService
                        VStack(spacing: 4) {
                            HStack {
                                Text("Queries Since Launch")
                                    .font(.subheadline)
                                    .foregroundColor(.secondary)
                                Spacer()
                                Text("\(svc.syncCountSinceLaunch) syncs")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            HStack(spacing: 12) {
                                QueryCountBadge(label: "Trunk", count: svc.totalTrunkQueries, color: .blue)
                                QueryCountBadge(label: "Branch", count: svc.totalBranchQueries, color: .indigo)
                                QueryCountBadge(label: "Compacted", count: svc.totalCompactedQueries, detail: svc.totalCompactedEntries, color: .orange)
                                QueryCountBadge(label: "Recent", count: svc.totalRecentQueries, detail: svc.totalRecentEntries, color: .green)
                            }
                        }
                    }

                    // Error display
                    if let error = platformBalanceSyncService.lastError {
                        Text(error)
                            .font(.caption)
                            .foregroundColor(.red)
                            .lineLimit(2)
                    }

                    // Action buttons
                    HStack {
                        Spacer()

                        Button {
                            Task {
                                await unifiedAppState.performPlatformBalanceSync()
                            }
                        } label: {
                            HStack(spacing: 4) {
                                Image(systemName: "arrow.clockwise")
                                Text("Sync Now")
                            }
                            .font(.caption)
                            .fontWeight(.medium)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.blue)
                        .controlSize(.mini)
                        .disabled(platformBalanceSyncService.isSyncing)

                        Button {
                            platformBalanceSyncService.reset()
                        } label: {
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

        }
        .navigationTitle("Sync Status")
        .onAppear {
            unifiedAppState.showWalletsSyncDetails = true
        }
        .onDisappear {
            unifiedAppState.showWalletsSyncDetails = false
        }
    }

    // MARK: - Sync Methods

    private func toggleSync() {
        if walletService.syncProgress.state.isRunning() {
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
        if walletService.syncProgress.state.isRunning() {
            // TODO: Call walletService.restartHeaderSync() when implemented
            print("Restarting header sync...")
        }
    }

    private func restartFilterHeaderSync() {
        if walletService.syncProgress.state.isRunning() {
            // TODO: Call walletService.restartFilterHeaderSync() when implemented
            print("Restarting filter header sync...")
        }
    }

    private func restartMasternodeSync() {
        if walletService.syncProgress.state.isRunning() {
            // TODO: Call walletService.restartMasternodeSync() when implemented
            print("Restarting masternode sync...")
        }
    }

    private func restartTransactionSync() {
        if walletService.syncProgress.state.isRunning() {
            // TODO: Call walletService.restartTransactionSync() when implemented
            print("Restarting transaction sync...")
        }
    }

    private func clearSyncData() {
        // Button is disabled during sync
        guard !walletService.syncProgress.state.isRunning() else {
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
    @EnvironmentObject var walletService: WalletService

    private func getNetworksList() -> String {
        // Wallets are now single-network, just return the wallet's network
        return wallet.network.rawValue.capitalized
    }

    var platformBalance: UInt64 {
        // Only sum balances of identities that belong to this specific wallet
        // and are on the same network

        return unifiedAppState.platformState.identities
            .filter { identity in
                // Check if identity belongs to this wallet and is on the same network
                // Only count identities that have been explicitly associated with this wallet
                identity.walletId == wallet.walletId &&
                identity.network == wallet.network.rawValue
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
                    let balance = walletService.walletManager.getBalance(for: wallet)
                    if balance.total == 0 {
                        Text("Empty")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else {
                        Text(balance.formattedTotal)
                            .font(.subheadline)
                            .fontWeight(.medium)
                    }

                    // Show platform balance if any
                    if platformBalance > 0 {
                        HStack(spacing: 3) {
                            Image(systemName: "p.circle.fill")
                                .font(.system(size: 9))
                            Text(platformBalance.formatted())
                        }
                        .font(.caption2)
                        .foregroundColor(.blue)
                    }
                }
            }
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Query Count Badge

private struct QueryCountBadge: View {
    let label: String
    let count: UInt32
    var detail: UInt32 = 0
    let color: Color

    var body: some View {
        VStack(spacing: 2) {
            if detail > 0 {
                Text("\(count)/\(detail)")
                    .font(.caption)
                    .fontWeight(.semibold)
                    .foregroundColor(count > 0 ? color : .secondary)
            } else {
                Text("\(count)")
                    .font(.caption)
                    .fontWeight(.semibold)
                    .foregroundColor(count > 0 ? color : .secondary)
            }
            Text(label)
                .font(.caption2)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
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

    /// Format platform credits as DASH string (1 DASH = 100,000,000,000 credits)
    func formatCredits(_ credits: UInt64) -> String {
        let dash = Double(credits) / 100_000_000_000.0
        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."
        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return "\(formatted) DASH"
        }
        return String(format: "%.8f DASH", dash)
    }
}
