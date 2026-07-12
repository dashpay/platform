import SwiftUI
import SwiftDashSDK
import SwiftData

struct CoreContentView: View {
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var appUIState: AppUIState
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService
    @EnvironmentObject var shieldedService: ShieldedService
    /// Threaded into `ShieldedService.clearLocalState(modelContext:)`
    /// so the Clear button can scope its delete-by-predicate to
    /// the bound wallet's persisted rows.
    @Environment(\.modelContext) private var modelContext
    @State private var showProofDetail = false
    @State private var masternodesEnabled: Bool = true
    @State private var platformSyncExpanded: Bool = false
    /// Last completed DashPay sync pass, polled from the FFI on appear
    /// and refreshed whenever an in-flight pass finishes (the
    /// `dashPaySyncIsSyncing` falling edge) or Sync Now completes.
    @State private var dashPayLastSync: Date?
    // Progress values come from PlatformWalletManager (polled from FFI each second)

    /// Rescan controls: the height-choice dialog, the follow-up
    /// custom-height alert + its numeric field, and a transient
    /// caption summarizing the last arm attempt (the Core section
    /// has no other feedback surface — Start/Clear only `print`).
    @State private var showRescanDialog = false
    @State private var showCustomHeightAlert = false
    @State private var customHeightText = ""
    @State private var rescanStatus: String?

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
        walletManager.spvIsRunning
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

                    // Block time of the SPV chain tip — a stale
                    // value across polls means core stopped
                    // producing blocks even though our SPV client
                    // is healthy. Hidden until the first tip
                    // header is stored.
                    if let tipTime = walletManager.spvTipBlockTime {
                        HStack {
                            Text("Block Time")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text("\(tipTime, style: .relative) ago")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(AppDate.formatted(tipTime, dateStyle: .omitted, timeStyle: .shortened))
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }

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

                        Button(action: { showRescanDialog = true }) {
                            Text("Rescan")
                                .font(.caption)
                                .fontWeight(.medium)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.indigo)
                        .controlSize(.mini)
                        .disabled(walletIdsOnNetwork.isEmpty)
                        .accessibilityIdentifier("coreSync.rescanFiltersButton")

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

                    // Last rescan-arm outcome (success summary or the
                    // per-wallet failures). Cleared on the next arm.
                    if let rescanStatus {
                        Text(rescanStatus)
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(3)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
                .padding(.vertical, 4)
                .confirmationDialog(
                    "Rescan Compact Filters",
                    isPresented: $showRescanDialog,
                    titleVisibility: .visible
                ) {
                    Button("Last 1,000 blocks") { armRescan(lastBlocks: 1_000) }
                    Button("Last 10,000 blocks") { armRescan(lastBlocks: 10_000) }
                    Button("Everything (from height 0)") { armRescan(fromHeight: 0) }
                    Button("Custom height…") { showCustomHeightAlert = true }
                    Button("Cancel", role: .cancel) {}
                } message: {
                    Text("Rewind the filter scan to re-download and re-match "
                        + "compact filters for every wallet on this network.")
                }
                .alert("Rescan From Height", isPresented: $showCustomHeightAlert) {
                    TextField("Block height", text: $customHeightText)
                        .keyboardType(.numberPad)
                    Button("Rescan") {
                        if let height = UInt32(
                            customHeightText.trimmingCharacters(in: .whitespaces)
                        ) {
                            armRescan(fromHeight: height)
                        }
                        customHeightText = ""
                    }
                    Button("Cancel", role: .cancel) { customHeightText = "" }
                } message: {
                    Text("Enter the core block height to rescan compact filters from.")
                }
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

                    // Chain tip height
                    HStack {
                        Text("Chain Tip Height")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                        Spacer()
                        if platformBalanceSyncService.chainTipHeight > 0 {
                            Text(formattedHeight(UInt32(platformBalanceSyncService.chainTipHeight)))
                                .font(.subheadline)
                                .fontWeight(.medium)
                        } else {
                            Text("—")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }
                    }

                    // Expandable details
                    DisclosureGroup(isExpanded: $platformSyncExpanded) {
                        VStack(spacing: 8) {
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
                        }
                        .padding(.top, 4)
                    } label: {
                        Text(platformSyncExpanded ? "Hide details" : "Show details")
                            .font(.caption)
                            .foregroundColor(.blue)
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
                        // Also block Sync Now while a Clear is running so
                        // it can't interleave with the Rust reset +
                        // SwiftData wipe.
                        .disabled(
                            platformBalanceSyncService.isSyncing
                                || platformBalanceSyncService.isClearing
                        )

                        Button {
                            Task {
                                await platformBalanceSyncService.clearLocalState(
                                    modelContext: modelContext,
                                    network: platformState.currentNetwork,
                                    walletIdsOnNetwork: walletIdsOnNetwork
                                )
                            }
                        } label: {
                            Text("Clear")
                                .font(.caption)
                                .fontWeight(.medium)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.red)
                        .controlSize(.mini)
                        // Clear is fire-and-forget; gate it against a
                        // concurrent sync and against a second Clear.
                        .disabled(
                            platformBalanceSyncService.isSyncing
                                || platformBalanceSyncService.isClearing
                        )
                    }
                }
                .padding(.vertical, 4)
            } header: {
                Text("Platform Sync Status")
            }

            // Section 3: DashPay Sync Status — the recurring
            // contact-request/profile/payment-reconcile loop
            // (`DashPaySyncManager`). State mirrors the sibling
            // sections: spinner while a pass is in flight, relative
            // last-sync stamp after, manual Sync Now.
            Section {
                VStack(spacing: 8) {
                    HStack {
                        if walletManager.dashPaySyncIsSyncing {
                            ProgressView()
                                .scaleEffect(0.7)
                            Text("Syncing...")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        } else if let lastSync = dashPayLastSync {
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
                        if (try? walletManager.isDashPaySyncRunning()) == true {
                            Text("Recurring")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        } else {
                            Text("Stopped")
                                .font(.caption)
                                .foregroundColor(.orange)
                        }
                    }

                    HStack {
                        Spacer()
                        Button {
                            Task {
                                _ = try? await walletManager.dashPaySyncNow()
                                refreshDashPayLastSync()
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
                        .disabled(walletManager.dashPaySyncIsSyncing)
                        .accessibilityIdentifier("sync.dashpay.syncNow")
                    }
                }
                .padding(.vertical, 4)
                .onAppear { refreshDashPayLastSync() }
                .onChange(of: walletManager.dashPaySyncIsSyncing) { _, syncing in
                    if !syncing { refreshDashPayLastSync() }
                }
            } header: {
                Text("DashPay Sync Status")
            }

            // Section 4: ZK Shielded Sync Status
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
                        } else if let lastSync = shieldedService.lastSyncTime {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                                .font(.caption)
                            Text("Last sync: \(lastSync, style: .relative)")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        } else if !shieldedService.isBound {
                            Image(systemName: "shield.slash")
                                .foregroundColor(.secondary)
                                .font(.caption)
                            Text("Not bound")
                                .font(.subheadline)
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

                    // Aggregate shielded balance + notes-synced
                    // watermark across every wallet/account on disk.
                    // Read straight from SwiftData (network-wide, no
                    // wallet scoping) so the figures survive restart
                    // and reflect the whole pool rather than a single
                    // bound wallet.
                    ShieldedNetworkSummaryRows(walletIds: walletIdsOnNetwork)

                    // Per-pass wall-clock timing. While a sync is
                    // in-flight, shows the live ticker (driven by a
                    // 1Hz timer on ShieldedService). After
                    // completion, shows the most recent non-cooldown
                    // pass duration. Mono digits keep the number
                    // readable as it ticks during long initial
                    // syncs (e.g. 10 min at N=1M). See
                    // `docs/shielded-sync-timing-spec.md`.
                    if shieldedService.isSyncing,
                       let elapsed = shieldedService.currentSyncElapsed {
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text("Syncing… elapsed")
                                    .font(.subheadline)
                                    .foregroundColor(.secondary)
                                Spacer()
                                Text(String(format: "%.1f s", elapsed))
                                    .font(.caption)
                                    .fontWeight(.medium)
                                    .monospacedDigit()
                            }
                            // Dual per-pass progress (P1.2). Two bars
                            // share the same denominator — the on-chain
                            // MMR total leaf count (total notes == total
                            // leaves) — carried straight from Rust in the
                            // tree-progress callback's second arg. The
                            // "Downloaded" bar tracks notes pulled off the
                            // wire; the "Checked" bar tracks commitments
                            // appended to the local Orchard tree. When the
                            // total is unknown (RPC unavailable, or before
                            // the first tree batch lands) `currentTreeTotal`
                            // is nil/0 and each bar falls back to an
                            // indeterminate spinner with the raw count.
                            if shieldedService.currentSyncScanned != nil
                                || shieldedService.currentTreeCommitted != nil {
                                ShieldedDualProgressRows(
                                    downloaded: shieldedService.currentSyncScanned,
                                    checked: shieldedService.currentTreeCommitted,
                                    total: shieldedService.currentTreeTotal
                                )
                            }
                        }
                    } else if let duration = shieldedService.lastSyncDuration {
                        HStack {
                            Text("Last sync duration")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(String(format: "%.2f s", max(0, duration)))
                                .font(.caption)
                                .fontWeight(.medium)
                                .monospacedDigit()
                        }
                    }

                    // "Longest pass" row — survives short steady-state
                    // re-passes so the cold-sync wall clock (the
                    // headline number for 1M-note devnet stress) stays
                    // visible after subsequent fast deltas overwrite
                    // `lastSyncDuration`. Only rendered when it
                    // actually exceeds the most recent pass to avoid
                    // redundant display.
                    if let longest = shieldedService.longestSyncDuration,
                       let last = shieldedService.lastSyncDuration,
                       longest > last + 0.05 {
                        HStack {
                            Text("Longest pass")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(String(format: "%.2f s", longest))
                                .font(.caption)
                                .fontWeight(.medium)
                                .monospacedDigit()
                                .foregroundColor(.secondary)
                        }
                    }

                    // Sync counters since launch — `total_scanned`
                    // is the wire-level encrypted-note count (every
                    // pass), while new + spent are the wallet-side
                    // outcomes (only ours).
                    if shieldedService.syncCountSinceLaunch > 0 {
                        let svc = shieldedService
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
                                QueryCountBadge(
                                    label: "Scanned",
                                    count: UInt32(min(svc.totalScanned, UInt64(UInt32.max))),
                                    color: .blue
                                )
                                QueryCountBadge(
                                    label: "New",
                                    count: UInt32(min(svc.totalNewNotes, UInt64(UInt32.max))),
                                    color: .purple
                                )
                                QueryCountBadge(
                                    label: "Spent",
                                    count: UInt32(min(svc.totalNewlySpent, UInt64(UInt32.max))),
                                    color: .orange
                                )
                            }
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
                        // Disabled while a sync is in flight, and
                        // pre-first-bind when there are no stashed
                        // credentials to resume from. Post-Clear
                        // is *not* disabled — `manualSync` rebinds
                        // on demand from the credentials kept by
                        // `clearLocalState`, so the user has a
                        // path back to a synced state without
                        // having to navigate away.
                        .disabled(shieldedService.isSyncing || !shieldedService.canResume)

                        Button {
                            // Stop the manager-wide shielded sync
                            // loop, then wipe every wallet's
                            // persisted shielded rows (notes +
                            // sync state). The Swift mirror zeros
                            // out and the service goes unbound,
                            // but the bind credentials are kept
                            // so the user can tap Sync Now to
                            // self-rebind from this screen — no
                            // navigation detour required. The
                            // on-disk SQLite tree is intentionally
                            // NOT deleted (Rust still holds its
                            // handle open via FileBackedShieldedStore;
                            // see clearLocalState's doc).
                            Task {
                                await shieldedService.clearLocalState(
                                    modelContext: modelContext
                                )
                            }
                        } label: {
                            Text("Clear")
                                .font(.caption)
                                .fontWeight(.medium)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.red)
                        .controlSize(.mini)
                        // Gated on `isSyncing` to close the
                        // double-tap window where the user could
                        // hit Clear *while* a sync is in flight.
                        // `clearLocalState` calls
                        // `stopShieldedSync()` first, but stop is
                        // best-effort and the persister callback
                        // can still drain rows into SwiftData
                        // between our delete and the loop
                        // actually quiescing.
                        .disabled(shieldedService.isSyncing)
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

    /// Pull the last completed DashPay pass timestamp from the FFI.
    /// `0` means "no pass has ever completed" — render as nil.
    private func refreshDashPayLastSync() {
        let unix = (try? walletManager.dashPayLastSyncUnixSeconds()) ?? 0
        dashPayLastSync = unix > 0 ? Date(timeIntervalSince1970: TimeInterval(unix)) : nil
    }

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

            // Devnet requires a name so `DevnetConfig` can embed
            // `devnet.devnet-<name>` in the SPV user agent (Dash
            // Core devnet peers drop inbound handshakes without it).
            // Read from the same UserDefaults key OptionsView writes.
            let devnetName: String? = platformState.currentNetwork == .devnet
                ? UserDefaults.standard.string(forKey: "platformDevnetName").flatMap {
                    let trimmed = $0.trimmingCharacters(in: .whitespaces)
                    return trimmed.isEmpty ? nil : trimmed
                }
                : nil

            let config = PlatformSpvStartConfig(
                dataDir: dataDirURL.path,
                network: platformState.currentNetwork,
                peers: peers,
                restrictToConfiguredPeers: restrictToConfiguredPeers,
                devnetName: devnetName
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
        // Devnet: auto-discover SPV peers from the quorum-list
        // service's `/masternodes` endpoint. Each masternode reports
        // its own `address` field (`ip:CoreP2PPort`) — use the
        // verbatim values rather than guessing the canonical 29999
        // port (paloma reports 20001 per masternode, for example).
        // No manual SPV input on devnet — the quorum URL is the
        // single source of truth (see `OptionsView`'s devnet branch).
        if platformState.currentNetwork == .devnet {
            guard
                let quorum = UserDefaults.standard.string(forKey: "platformQuorumURL"),
                !quorum.isEmpty,
                let active = SDK.discoverActiveMasternodes(quorumBase: quorum)
            else { return [] }
            return active.map(\.spvPeer)
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

    // MARK: - Rescan

    /// Height the "last N blocks" rescan choices count back from:
    /// filter-scan tip, falling back to filter-headers then headers
    /// when the earlier stages haven't produced a height yet.
    private var rescanReferenceTip: UInt32 {
        let filters = walletManager.spvProgress.filters?.currentHeight ?? 0
        if filters > 0 { return filters }
        let filterHeaders = walletManager.spvProgress.filterHeaders?.currentHeight ?? 0
        if filterHeaders > 0 { return filterHeaders }
        return walletManager.spvProgress.headers?.currentHeight ?? 0
    }

    private func armRescan(lastBlocks: UInt32) {
        let tip = rescanReferenceTip
        armRescan(fromHeight: tip > lastBlocks ? tip - lastBlocks : 0)
    }

    /// Rewind the filter-scan checkpoint to `fromHeight` for every
    /// wallet on the active network. Collects per-wallet failures
    /// instead of aborting on the first, then reports the outcome in
    /// `rescanStatus`.
    private func armRescan(fromHeight: UInt32) {
        let ids = walletIdsOnNetwork
        guard !ids.isEmpty else { return }

        var failures: [String] = []
        for walletId in ids {
            do {
                try walletManager.spvRescanFilters(walletId: walletId, fromHeight: fromHeight)
            } catch {
                failures.append("\(rescanShortId(walletId)): \(error.localizedDescription)")
            }
        }

        let armed = ids.count - failures.count
        var msg = "Rescan armed from height \(fromHeight.formatted()) "
            + "for \(armed) wallet\(armed == 1 ? "" : "s")"
        if !isSpvRunning {
            msg += " — applies on next SPV start"
        }
        if !failures.isEmpty {
            msg += ". Failed: " + failures.joined(separator: ", ")
        }
        rescanStatus = msg
    }

    /// First 4 bytes of a wallet id as hex, for compact failure lines.
    private func rescanShortId(_ walletId: Data) -> String {
        walletId.prefix(4).map { String(format: "%02x", $0) }.joined()
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

    /// Per-wallet unspent shielded notes. Same persisted-truth
    /// source as the Sync Status diagnostic — sum of `value` over
    /// unspent rows for this wallet, in credits. Without this the
    /// wallet row's combined DASH total under-reported the wallet's
    /// real value by every shielded note: a wallet with funds in
    /// the shielded pool would show Core + Platform on the list but
    /// silently exclude Shielded.
    @Query private var shieldedNotes: [PersistentShieldedNote]

    init(wallet: PersistentWallet) {
        self.wallet = wallet
        let walletId = wallet.walletId
        _addressBalances = Query(
            filter: #Predicate<PersistentPlatformAddress> { $0.walletId == walletId }
        )
        _shieldedNotes = Query(
            filter: #Predicate<PersistentShieldedNote> {
                $0.walletId == walletId && !$0.isSpent
            }
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
    ///
    /// Skips identities whose `modelContext` is nil — that's
    /// SwiftData's marker for an invalidated row (e.g. mid-wallet-
    /// delete, where the relationship array is briefly visible but
    /// the underlying rows have already been removed from the
    /// store). Reading any persisted property on an invalidated
    /// model crashes with `BackingData.swift:866: This model
    /// instance was invalidated…`.
    private var platformBalance: UInt64 {
        let blastBalance = addressBalances.reduce(UInt64(0)) { $0 + $1.balance }
        if blastBalance > 0 { return blastBalance }
        return identitiesForWallet
            .filter { $0.modelContext != nil }
            .reduce(UInt64(0)) {
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

    /// Sum of unspent shielded note values in credits. Same scale
    /// as `platformBalance` (1e11 credits/DASH), so it folds into
    /// the same divisor in [`combinedDashAmount(coreTotal:)`].
    private var shieldedBalance: UInt64 {
        shieldedNotes.reduce(UInt64(0)) { $0 + $1.value }
    }

    /// Combined wallet balance expressed in DASH for a precomputed
    /// totals tuple. Core uses 1e8 duffs/DASH; Platform and Shielded
    /// both use 1e11 credits/DASH.
    private func combinedDashAmount(coreTotal: UInt64) -> Double {
        Double(coreTotal) / 100_000_000.0
            + Double(platformBalance) / 100_000_000_000.0
            + Double(shieldedBalance) / 100_000_000_000.0
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
        let hasAny = coreTotal > 0 || platformBalance > 0 || shieldedBalance > 0
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

// MARK: - ShieldedNetworkSummaryRows

/// Network-wide shielded summary: aggregate unspent balance and the
/// notes-synced watermark across every wallet/account **on the active
/// network**.
///
/// Both figures are read **directly from SwiftData** (scoped to the
/// active network via `walletIds`) rather than from the
/// `ShieldedService.shieldedBalance` mirror that's updated
/// per-bound-wallet from sync events, so they survive restart and
/// reflect the whole on-network pool:
///
///   * **Total Shielded Balance** — sum of `value` over every unspent
///     `PersistentShieldedNote` whose wallet is on this network.
///
///   * **Notes Synced** — the highest `lastSyncedIndex` across this
///     network's `PersistentShieldedSyncState` rows. The Orchard
///     commitment tree is chain-wide and shared by every wallet/account
///     **on a given network**, so each subwallet advances toward the
///     same tip; the max is the furthest-scanned position and climbs as
///     sync progresses. Scoping matters: trees are per-chain, so a
///     `max()` across networks would blend unrelated tip positions.
private struct ShieldedNetworkSummaryRows: View {
    /// Wallet ids on the active network. Both queries are filtered
    /// against this so a multi-network install (e.g. regtest + testnet)
    /// doesn't blend balances or take a watermark `max()` across
    /// unrelated per-chain commitment trees — matching the Platform
    /// Sync Status section's `walletIdsOnNetwork` scoping.
    let walletIds: Set<Data>

    @Query private var allNotes: [PersistentShieldedNote]
    @Query private var syncStates: [PersistentShieldedSyncState]

    /// Sum of `value` over this network's unspent notes, in credits.
    private var totalUnspentCredits: UInt64 {
        allNotes.lazy
            .filter { !$0.isSpent && walletIds.contains($0.walletId) }
            .reduce(UInt64(0)) { $0 &+ $1.value }
    }

    /// Furthest-scanned commitment-tree index across this network's subwallets.
    private var notesSynced: UInt64 {
        syncStates.lazy
            .filter { walletIds.contains($0.walletId) }
            .map(\.lastSyncedIndex)
            .max() ?? 0
    }

    /// 1 DASH = 100,000,000,000 credits — matches `formatCredits`.
    private func formatCredits(_ credits: UInt64) -> String {
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
        return String(format: "%.4f DASH", dash)
    }

    var body: some View {
        VStack(spacing: 8) {
            // Aggregate unspent balance across all wallets.
            HStack {
                Text("Total Shielded Balance")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                Spacer()
                if totalUnspentCredits > 0 {
                    Text(formatCredits(totalUnspentCredits))
                        .font(.subheadline)
                        .fontWeight(.medium)
                } else {
                    Text("0")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }
            }

            // Notes-synced watermark — climbs as sync progresses.
            HStack {
                Text("Notes Synced")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                Spacer()
                Text(notesSynced, format: .number)
                    .font(.system(.caption, design: .monospaced))
                    .fontWeight(.medium)
            }
        }
    }
}

/// Dual live progress for an in-flight shielded sync pass: a
/// "Downloaded" bar (notes pulled off the wire) stacked over a
/// "Checked" bar (commitments appended to the local Orchard tree).
///
/// Both bars share the same denominator — `total`, the on-chain MMR
/// total leaf count carried straight from Rust in the tree-progress
/// callback (total notes == total leaves). No Swift-side math derives
/// it. When `total` is nil or 0 the total is indeterminate (the count
/// RPC was unavailable, or no tree batch has landed yet) and each bar
/// degrades to an indeterminate spinner alongside its raw count.
private struct ShieldedDualProgressRows: View {
    /// Cumulative notes downloaded this pass; nil before the first
    /// download chunk.
    let downloaded: UInt64?
    /// Cumulative commitments appended to the tree this pass; nil
    /// before the first committed batch.
    let checked: UInt64?
    /// Shared denominator (on-chain MMR total). nil/0 ⇒ indeterminate.
    let total: UInt64?

    /// Determinate only when Rust handed us a positive total.
    private var hasTotal: Bool {
        if let total, total > 0 { return true }
        return false
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            progressRow(label: "Downloaded", value: downloaded)
            progressRow(label: "Checked", value: checked)
        }
    }

    @ViewBuilder
    private func progressRow(label: String, value: UInt64?) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack {
                Text(label)
                    .font(.caption2)
                    .foregroundColor(.secondary)
                Spacer()
                Text(countText(value: value))
                    .font(.caption2)
                    .monospacedDigit()
                    .foregroundColor(.secondary)
            }
            if hasTotal, let value, let total {
                // Clamp so a value that briefly overshoots the cached
                // total (batch lands before the denominator refreshes)
                // can't push the bar past 1.0.
                ProgressView(
                    value: Double(min(value, total)),
                    total: Double(total)
                )
                .progressViewStyle(.linear)
                .tint(.purple)
            } else {
                // Indeterminate: total unknown ⇒ spinner, not a fake bar.
                ProgressView()
                    .progressViewStyle(.linear)
                    .tint(.purple)
            }
        }
    }

    /// "12,288 / 1,000,000 notes" when the total is known, else
    /// "12,288 notes" — matches the existing scanned-count presentation.
    private func countText(value: UInt64?) -> String {
        let count = value ?? 0
        let countStr = count.formatted(.number)
        if hasTotal, let total {
            return "\(countStr) / \(total.formatted(.number)) notes"
        }
        return "\(countStr) notes"
    }
}
