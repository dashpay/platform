import SwiftUI
import SwiftDashSDK
import SwiftData

struct CoreContentView: View {
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var appUIState: AppUIState
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService
    @EnvironmentObject var shieldedService: ShieldedService
    @State private var showProofDetail = false
    @State private var masternodesEnabled: Bool = true
    // Progress values come from PlatformWalletManager (polled from FFI each second)

    // Display helpers
    private var headerHeightsDisplay: String? {
        let headers = walletManager.spvProgress.headers
        let cur = headers?.currentHeight ?? 0
        let tot = headers?.targetHeight ?? 0

        return heightDisplay(numerator: cur, denominator: tot)
    }

    private var filterHeaderHeightsDisplay: String? {
        let cur = walletManager.spvProgress.filterHeaders?.currentHeight ?? 0
        let tot = walletManager.spvProgress.filterHeaders?.targetHeight ?? 0

        return heightDisplay(numerator: cur, denominator: tot)
    }

    private var filterHeightsDisplay: String? {
        let cur = walletManager.spvProgress.filters?.currentHeight ?? 0
        let tot = walletManager.spvProgress.filters?.targetHeight ?? 0

        return heightDisplay(numerator: cur, denominator: tot)
    }

    private var masternodeHeightsDisplay: String? {
        let cur = walletManager.spvProgress.masternodes?.currentHeight ?? 0
        let tot = walletManager.spvProgress.masternodes?.targetHeight ?? 0

        return heightDisplay(numerator: cur, denominator: tot)
    }

    private var isSpvRunning: Bool {
        walletManager.spvProgress.overallState.isRunning
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
                        progress: walletManager.spvProgress.headers?.percentage ?? 0.0,
                        value: headerHeightsDisplay
                    )

                    CompactSyncRow(
                        title: "Filter Headers",
                        progress: walletManager.spvProgress.filterHeaders?.percentage ?? 0.0,
                        value: filterHeaderHeightsDisplay
                    )

                    if masternodesEnabled {
                        CompactSyncRow(
                            title: "Masternodes",
                            progress: walletManager.spvProgress.masternodes?.percentage ?? 0.0,
                            value: masternodeHeightsDisplay
                        )
                    }

                    CompactSyncRow(
                        title: "Filters",
                        progress: walletManager.spvProgress.filters?.percentage ?? 0.0,
                        value: filterHeightsDisplay
                    )

                    // Controls row
                    HStack(spacing: 8) {
                        Spacer()

                        Button(action: toggleSync) {
                            Text(isSpvRunning ? "Pause" : "Start")
                                .font(.caption)
                                .fontWeight(.medium)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(isSpvRunning ? .orange : .blue)
                        .controlSize(.mini)

                        Button(action: clearSyncData) {
                            Text("Clear")
                                .font(.caption)
                                .fontWeight(.medium)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.red)
                        .controlSize(.mini)
                        .disabled(isSpvRunning)
                        .opacity(isSpvRunning ? 0.5 : 1.0)
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
                    HStack {
                        Text("Last Recent Block")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                        Spacer()
                        if platformBalanceSyncService.lastKnownRecentBlock > 0 {
                            Text(formattedHeight(UInt32(platformBalanceSyncService.lastKnownRecentBlock)))
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        } else {
                            Text("None found")
                                .font(.subheadline)
                                .foregroundColor(.blue)
                                .onTapGesture {
                                    showProofDetail = true
                                }
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
                                await platformBalanceSyncService.performSync()
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
                            platformBalanceSyncService.clearDisplay()
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

            // Section 3: ZK Shielded Sync Status
            Section {
                VStack(spacing: 8) {
                    // Sync state
                    HStack {
                        if shieldedService.isSyncing {
                            ProgressView()
                                .scaleEffect(0.7)
                            Text("Syncing...")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        } else {
                            Image(systemName: "shield.checkered")
                                .foregroundColor(.purple)
                                .font(.caption)
                            Text("Ready")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }
                        Spacer()
                    }

                    // Shielded balance
                    HStack {
                        Text("Shielded Balance")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                        Spacer()
                        if shieldedService.shieldedBalance > 0 {
                            Text(formatCredits(shieldedService.shieldedBalance))
                                .font(.subheadline)
                                .fontWeight(.medium)
                        } else {
                            Text("0")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }
                    }

                    // Orchard address
                    if let address = shieldedService.orchardDisplayAddress {
                        HStack {
                            Text("Orchard Address")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(address.prefix(12) + "..." + address.suffix(8))
                                .font(.caption)
                                .fontWeight(.medium)
                                .foregroundColor(.purple)
                        }
                    }

                    // Error display
                    if let error = shieldedService.lastError {
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
                                if let sdk = platformState.sdk {
                                    await shieldedService.fullSync(sdk: sdk)
                                }
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
                        .tint(.purple)
                        .controlSize(.mini)
                        .disabled(shieldedService.isSyncing)

                        Button {
                            shieldedService.reset()
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
                Text("Shielded Sync Status")
            }

        }
        .navigationTitle("Sync Status")
        .onAppear {
            appUIState.showWalletsSyncDetails = true
            Task { await refreshMasternodeFlag() }
        }
        .onDisappear {
            appUIState.showWalletsSyncDetails = false
        }
        .sheet(isPresented: $showProofDetail) {
            NavigationStack {
                ProofDetailView(proofData: platformBalanceSyncService.lastRecentProof)
            }
        }
    }

    // MARK: - Sync Methods

    private func toggleSync() {
        if isSpvRunning {
            pauseSync()
        } else {
            startSync()
        }
    }

    private func startSync() {
        do {
            let dataDirURL = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
                .first!
                .appendingPathComponent("SPV")
                .appendingPathComponent(platformState.currentNetwork.rawValue)
            try? FileManager.default.createDirectory(at: dataDirURL, withIntermediateDirectories: true)

            let useLocalCore = UserDefaults.standard.bool(forKey: "useLocalhostCore")
            let peers: [String] = useLocalCore
                ? ((UserDefaults.standard.string(forKey: "localCorePeers") ?? "127.0.0.1")
                    .split(separator: ",")
                    .map { $0.trimmingCharacters(in: .whitespaces) })
                : []

            let config = PlatformSpvStartConfig(
                dataDir: dataDirURL.path,
                network: platformNetwork(from: platformState.currentNetwork),
                peers: peers,
                restrictToConfiguredPeers: useLocalCore,
                masternodeSyncEnabled: masternodesEnabled
            )
            try walletManager.startSpv(config: config)
        } catch {
            print("❌ Sync failed: \(error)")
        }
    }

    private func pauseSync() {
        try? walletManager.stopSpv()
    }

    private func clearSyncData() {
        guard !isSpvRunning else {
            print("⚠️ Clear button should be disabled during sync")
            return
        }

        do {
            try walletManager.clearSpvStorage()
        } catch {
            print("❌ Failed to clear SPV storage: \(error)")
        }
    }

    @MainActor
    private func refreshMasternodeFlag() async {
        // Best-effort: honor trusted mode reported by the Platform SDK.
        if let sdk = platformState.sdk {
            let status: SDKStatus? = try? sdk.getStatus()
            if let status = status {
                masternodesEnabled = status.mode.lowercased() != "trusted"
            }
        }
    }

    private func platformNetwork(from network: AppNetwork) -> PlatformNetwork {
        switch network {
        case .mainnet: return .mainnet
        case .testnet: return .testnet
        case .devnet: return .devnet
        case .regtest: return .testnet
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
    @EnvironmentObject var platformState: AppState

    // Observe the persisted wallet record matching this HDWallet's wallet id.
    @Query private var persistentWallets: [PersistentWallet]

    init(wallet: HDWallet) {
        self.wallet = wallet
        let walletId = wallet.walletId
        _persistentWallets = Query(filter: #Predicate<PersistentWallet> { $0.walletId == walletId })
    }

    private func getNetworksList() -> String {
        // Wallets are now single-network, just return the wallet's network
        return wallet.network.rawValue.capitalized
    }

    var platformBalance: UInt64 {
        // Only sum balances of identities that belong to this specific wallet
        // and are on the same network

        return platformState.identities
            .filter { identity in
                identity.walletId == wallet.walletId &&
                identity.network == wallet.network.rawValue
            }
            .reduce(0) { sum, identity in
                sum + identity.balance
            }
    }

    private var totalCoreBalance: UInt64 {
        guard let record = persistentWallets.first else { return 0 }
        return record.balanceConfirmed
            + record.balanceUnconfirmed
            + record.balanceImmature
            + record.balanceLocked
    }

    private func formatBalance(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        formatter.numberStyle = .decimal
        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return "\(formatted) DASH"
        }
        return String(format: "%.8f DASH", dash)
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
                    if totalCoreBalance == 0 {
                        Text("Empty")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else {
                        Text(formatBalance(totalCoreBalance))
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

// MARK: - Proof Detail View

struct ProofDetailView: View {
    let proofData: Data
    @State private var formattedProof: String = "Decoding..."
    @State private var copiedText: String?

    private var proofHex: String {
        proofData.map { String(format: "%02x", $0) }.joined()
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    Text("Proof Size")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("\(proofData.count) bytes")
                        .font(.subheadline)
                }

                if !proofData.isEmpty {
                    Text("Decoded Proof")
                        .font(.subheadline)
                        .foregroundColor(.secondary)

                    Text(formattedProof)
                        .font(.system(.caption2, design: .monospaced))
                        .textSelection(.enabled)
                } else {
                    Text("No proof data available")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }
            }
            .padding()
        }
        .navigationTitle("Recent Query Proof")
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Menu {
                    Button("Copy Formatted") {
                        UIPasteboard.general.string = formattedProof
                    }
                    Button("Copy Hex") {
                        UIPasteboard.general.string = proofHex
                    }
                } label: {
                    Image(systemName: "doc.on.doc")
                }
            }
        }
        .onAppear {
            formatProof()
        }
    }

    private func formatProof() {
        guard !proofData.isEmpty else {
            formattedProof = "No proof data"
            return
        }

        // Call Rust FFI to format the GroveDB proof
        let result = proofData.withUnsafeBytes { buffer -> DashSDKResult in
            guard let base = buffer.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                return DashSDKResult()
            }
            return dash_sdk_format_grovedb_proof(base, UInt32(proofData.count))
        }

        if let error = result.error {
            let msg = error.pointee.message != nil
                ? String(cString: error.pointee.message!)
                : "Unknown error"
            dash_sdk_error_free(error)
            formattedProof = "Failed to decode: \(msg)\n\nRaw hex:\n\(proofHex)"
            return
        }

        guard let dataPtr = result.data else {
            formattedProof = "No formatted output\n\nRaw hex:\n\(proofHex)"
            return
        }

        let cStr = dataPtr.assumingMemoryBound(to: CChar.self)
        formattedProof = String(cString: cStr)
        dash_sdk_string_free(UnsafeMutablePointer(mutating: cStr))
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
