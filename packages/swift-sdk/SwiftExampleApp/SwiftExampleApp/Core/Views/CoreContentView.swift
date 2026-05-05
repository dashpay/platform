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

    /// All persisted platform addresses across every wallet. Summed
    /// directly here so the global Sync Status view survives app
    /// restarts / wallet reconfigures without depending on the
    /// singleton `PlatformBalanceSyncService.totalPlatformBalance`
    /// (which only reflects the currently-configured wallet).
    @Query private var platformAddresses: [PersistentPlatformAddress]

    /// All persisted wallets — used as the network-scoping pivot for
    /// `platformAddresses`. `PersistentPlatformAddress` doesn't carry
    /// a `networkRaw` column itself; the canonical join is through
    /// `walletId` to the parent `PersistentWallet.networkRaw`. We
    /// build a `Set<Data>` for the active network and filter the
    /// address aggregate against it so switching to local doesn't
    /// keep showing testnet sums.
    @Query private var allWallets: [PersistentWallet]

    private var walletIdsOnNetwork: Set<Data> {
        let raw = platformState.currentNetwork.rawValue
        return Set(allWallets.lazy
            .filter { $0.networkRaw == raw }
            .map(\.walletId))
    }

    /// Platform addresses scoped to the active network.
    private var scopedPlatformAddresses: [PersistentPlatformAddress] {
        let ids = walletIdsOnNetwork
        return platformAddresses.filter { ids.contains($0.walletId) }
    }

    /// Aggregate platform credit balance across every wallet on the
    /// active network.
    private var aggregatePlatformBalance: UInt64 {
        scopedPlatformAddresses.reduce(0) { $0 + $1.balance }
    }

    /// Addresses with a non-zero balance across every wallet on the
    /// active network.
    private var aggregateActiveAddressCount: Int {
        scopedPlatformAddresses.reduce(0) { $1.balance > 0 ? $0 + 1 : $0 }
    }

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

                    // Balance summary — aggregated across every wallet
                    // on disk (SwiftData-backed, so survives restart).
                    HStack {
                        Text("Platform Balance")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                        Spacer()
                        if aggregatePlatformBalance > 0 {
                            Text(formatCredits(aggregatePlatformBalance))
                                .font(.subheadline)
                                .fontWeight(.medium)
                        } else {
                            Text("0")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }
                    }

                    // Active addresses — count non-zero balance rows
                    // across every wallet.
                    HStack {
                        Text("Active Addresses")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                        Spacer()
                        Text("\(aggregateActiveAddressCount)")
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
                            Text(AppDate.formatted(blockTime, dateStyle: .abbreviated, timeStyle: .omitted))
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(AppDate.formatted(blockTime, dateStyle: .omitted, timeStyle: .shortened))
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
                            Task { await shieldedService.manualSync() }
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
                .appendingPathComponent(platformState.currentNetwork.networkName)
            try? FileManager.default.createDirectory(at: dataDirURL, withIntermediateDirectories: true)

            let peers = spvPeerOverride()
            let restrictToConfiguredPeers = !peers.isEmpty

            let config = PlatformSpvStartConfig(
                dataDir: dataDirURL.path,
                network: platformState.currentNetwork,
                peers: peers,
                restrictToConfiguredPeers: restrictToConfiguredPeers,
                masternodeSyncEnabled: masternodesEnabled
            )
            try walletManager.startSpv(config: config)
        } catch {
            print("❌ Sync failed: \(error)")
        }
    }

    /// Resolve the SPV peer override for the current network /
    /// docker combo.
    ///
    /// Three modes coexist on top of the same `useLocalhostCore` /
    /// `localCorePeers` `UserDefaults` keys, which used to bleed into
    /// each other when the user reconfigured between sessions:
    ///
    ///   1. **regtest + docker** — connect to dashmate's `local_seed`
    ///      Core P2P port. The default 3-node setup maps the seed to
    ///      `127.0.0.1:20301` (`getLocalConfigFactory.js` base 20001
    ///      + `setupLocalPresetTaskFactory.js` `+ i*100` with seed
    ///      at index = `nodeCount`, typically 3). Anything sitting
    ///      in `localCorePeers` from a previous testnet / mainnet
    ///      "custom peers" session is ignored — the UI doesn't show
    ///      that knob on regtest+docker so a stale value is always
    ///      bleed-through, never user intent.
    ///   2. **non-regtest + custom peers** — honor `localCorePeers`
    ///      verbatim. The OptionsView "Use Custom SPV Peers" toggle
    ///      seeds and edits this string.
    ///   3. **everything else** — empty list, FFI uses the network's
    ///      built-in seed nodes.
    private func spvPeerOverride() -> [String] {
        let useDocker = UserDefaults.standard.bool(forKey: "useDockerSetup")
        if platformState.currentNetwork == .regtest && useDocker {
            return ["127.0.0.1:20301"]
        }
        let useLocalCore = UserDefaults.standard.bool(forKey: "useLocalhostCore")
        guard useLocalCore else { return [] }
        let raw = UserDefaults.standard.string(forKey: "localCorePeers") ?? ""
        return raw
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
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
    let wallet: PersistentWallet
    @EnvironmentObject var platformState: AppState
    /// Canonical Core-balance source. The previously-persisted
    /// `PersistentWallet.balanceConfirmed`/etc. fields were removed —
    /// Rust's in-memory account totals (via `accountBalances(for:)`)
    /// are the single source of truth, mirroring `BalanceCardView`.
    @EnvironmentObject var walletManager: PlatformWalletManager

    /// Per-wallet BLAST-synced platform-address balances. Mirrors
    /// `BalanceCardView` so the summary row sees the same balance as
    /// the detail view — previously the row only summed identity
    /// credits, so a wallet funded purely via Platform Payment
    /// addresses (no identities) showed "Empty".
    @Query private var addressBalances: [PersistentPlatformAddress]

    init(wallet: PersistentWallet) {
        self.wallet = wallet
        let walletId = wallet.walletId
        _addressBalances = Query(
            filter: #Predicate<PersistentPlatformAddress> { $0.walletId == walletId }
        )
    }

    /// Identities on this wallet — via the SwiftData relationship.
    /// The wallet↔identity relationship is the canonical source
    /// now; no need to filter `appState.identities` by walletId.
    private var identitiesForWallet: [PersistentIdentity] {
        wallet.identities
    }

    /// Platform balance in credits: prefer BLAST address sync, fall
    /// back to summing identity credits when no addresses have been
    /// synced yet.
    private var platformBalance: UInt64 {
        let blastBalance = addressBalances.reduce(UInt64(0)) { $0 + $1.balance }
        if blastBalance > 0 { return blastBalance }
        return identitiesForWallet.reduce(UInt64(0)) {
            $0 + UInt64(bitPattern: $1.balance)
        }
    }

    /// One-shot snapshot of the wallet's per-account Core balances.
    /// `accountBalances(for:)` is a blocking FFI call; the prior
    /// shape (a `coreBalances` computed property + four `coreX` sums)
    /// hit the FFI four times per render and again from
    /// `balanceBreakdown`. Capturing in `body` and threading the
    /// tuple through reduces every render to a single FFI roundtrip.
    private typealias CoreBalanceTotals = (
        confirmed: UInt64,
        unconfirmed: UInt64,
        immature: UInt64,
        locked: UInt64
    )

    private func coreBalanceTotals() -> CoreBalanceTotals {
        walletManager.accountBalances(for: wallet.walletId)
            .reduce(into: (UInt64(0), UInt64(0), UInt64(0), UInt64(0))) { acc, b in
                acc.0 += b.confirmed
                acc.1 += b.unconfirmed
                acc.2 += b.immature
                acc.3 += b.locked
            }
    }

    private static func sumCoreBalance(_ totals: CoreBalanceTotals) -> UInt64 {
        totals.confirmed + totals.unconfirmed + totals.immature + totals.locked
    }

    /// Combined wallet balance expressed in DASH for a precomputed
    /// totals tuple. Core uses 1e8 duffs/DASH; Platform uses 1e11
    /// credits/DASH.
    private func combinedDashAmount(coreTotal: UInt64) -> Double {
        Double(coreTotal) / 100_000_000.0
            + Double(platformBalance) / 100_000_000_000.0
    }

    private var walletIdShort: String {
        let hex = wallet.walletId.prefix(6)
            .map { String(format: "%02x", $0) }
            .joined()
        // Render as `aabbcc…ddeeff` using first/last 6 hex chars.
        guard hex.count >= 12 else { return hex }
        return "\(String(hex.prefix(6)))…\(String(hex.suffix(6)))"
    }

    private var lastSyncedText: String {
        let lastSynced = wallet.lastSynced
        guard lastSynced > 0 else { return "never synced" }
        let date = Date(timeIntervalSince1970: TimeInterval(lastSynced))
        return Self.relativeFormatter.localizedString(
            for: date,
            relativeTo: Date()
        )
    }

    private func formatBalance(_ amount: UInt64) -> String {
        formatDash(Double(amount) / 100_000_000.0)
    }

    private func formatDash(_ dash: Double) -> String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return "\(formatted) DASH"
        }
        return String(format: "%.8f DASH", dash)
    }

    /// Platform credits use 100B credits = 1 DASH (vs Core's 100M duffs).
    private func formatCredits(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000_000.0
        return String(format: "%.4f DASH", dash)
    }

    private func balanceBreakdown(_ totals: CoreBalanceTotals) -> String? {
        var parts: [String] = []
        if totals.confirmed > 0 {
            parts.append("\(formatBalance(totals.confirmed)) confirmed")
        }
        if totals.unconfirmed > 0 {
            parts.append("\(formatBalance(totals.unconfirmed)) unconfirmed")
        }
        if totals.immature > 0 {
            parts.append("\(formatBalance(totals.immature)) immature")
        }
        if totals.locked > 0 {
            parts.append("\(formatBalance(totals.locked)) locked")
        }
        return parts.isEmpty ? nil : parts.joined(separator: " • ")
    }

    private static let dateFormatter: DateFormatter = {
        let f = DateFormatter.gregorian()
        f.dateStyle = .medium
        f.timeStyle = .none
        return f
    }()

    private static let relativeFormatter: RelativeDateTimeFormatter = {
        let f = RelativeDateTimeFormatter()
        f.unitsStyle = .abbreviated
        return f
    }()

    var body: some View {
        // Single FFI snapshot per render — `coreBalanceTotals()` calls
        // `walletManager.accountBalances(for:)` once; everything below
        // reads from `core` / `coreTotal` / `hasAny` instead of
        // re-invoking the accessor.
        let core = coreBalanceTotals()
        let coreTotal = Self.sumCoreBalance(core)
        let hasAny = coreTotal > 0 || platformBalance > 0
        return VStack(alignment: .leading, spacing: 6) {
            // Header: label (+ status badges) and total Core balance.
            HStack(alignment: .firstTextBaseline) {
                HStack(spacing: 6) {
                    Text(wallet.label)
                        .font(.headline)
                    if wallet.isImported {
                        Image(systemName: "tray.and.arrow.down")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .help("Imported")
                    }
                }
                Spacer()
                Text(hasAny ? formatDash(combinedDashAmount(coreTotal: coreTotal)) : "Empty")
                    .font(.subheadline)
                    .fontWeight(.medium)
                    .foregroundColor(hasAny ? .primary : .secondary)
            }

            // Row 1: network + created date.
            WalletInfoRow(
                icon: "network",
                iconColor: .blue,
                text: "\(wallet.network?.displayName ?? "Unknown") • Created "
                    + Self.dateFormatter.string(from: wallet.createdAt)
            )

            // Row 2: Core balance breakdown. Falls back to a terse
            // message when the wallet has no UTXOs yet so the row
            // count stays consistent across all wallets.
            WalletInfoRow(
                icon: "bitcoinsign.circle",
                iconColor: .green,
                text: balanceBreakdown(core) ?? "No Core balance"
            )

            // Row 3: account + identity counts.
            WalletInfoRow(
                icon: "square.stack.3d.up",
                iconColor: .purple,
                text: {
                    let accounts = wallet.accounts.count
                    let ids = identitiesForWallet.count
                    let acctWord = accounts == 1 ? "account" : "accounts"
                    let idWord = ids == 1 ? "identity" : "identities"
                    return "\(accounts) \(acctWord) • \(ids) \(idWord)"
                }()
            )

            // Row 4: SPV sync progress.
            WalletInfoRow(
                icon: "arrow.triangle.2.circlepath",
                iconColor: .indigo,
                text: {
                    let height = wallet.syncedHeight
                    if height == 0 {
                        return "Not yet synced"
                    }
                    return "Synced to block \(height.formatted()) • \(lastSyncedText)"
                }()
            )

            // Row 5: Platform balance (if any identities hold credits)
            // or wallet id fingerprint as a stable fallback so the
            // row count stays at 5.
            if platformBalance > 0 {
                WalletInfoRow(
                    icon: "p.circle.fill",
                    iconColor: .blue,
                    text: "Platform: \(formatCredits(platformBalance)) • \(walletIdShort)"
                )
            } else {
                WalletInfoRow(
                    icon: "tag",
                    iconColor: .secondary,
                    text: "ID \(walletIdShort)"
                )
            }
        }
        .padding(.vertical, 4)
    }
}

/// Single-line icon + caption row used by `WalletRowView` to keep
/// rows visually aligned.
private struct WalletInfoRow: View {
    let icon: String
    let iconColor: Color
    let text: String

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: icon)
                .font(.caption)
                .foregroundColor(iconColor)
                .frame(width: 16)
            Text(text)
                .font(.caption)
                .foregroundColor(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
            Spacer(minLength: 0)
        }
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
