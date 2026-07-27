import SwiftUI
import SwiftDashSDK
import SwiftData
import LocalAuthentication

enum RootTab: Hashable {
    case sync, wallets, identities, dashpay, settings
}

struct ContentView: View {
    let isInitialized: Bool
    let bootstrapError: Error?
    let onRetry: () -> Void

    // NOTE: `walletManager` is intentionally NOT declared here. An
    // `@EnvironmentObject` subscribes the entire view to that object's
    // `objectWillChange`, and `walletManager` publishes `spvProgress` on
    // a fast cadence during sync — holding it on this root view
    // re-rendered the whole `TabView` several times a second, tearing
    // down each tab's content and any sheet/pushed view inside it. That
    // observation now lives in the leaf `GlobalSyncIndicatorOverlay`, so
    // sync ticks no longer invalidate `ContentView.body`.
    //
    // `appUIState` IS declared: it owns the root tab selection (so deep
    // views like DashPayTabView / IdentityDetailView can switch tabs)
    // and only publishes on low-frequency UI events (tab switch, banner
    // toggle), never on sync progress.
    @EnvironmentObject var walletManagerStore: WalletManagerStore
    @EnvironmentObject var appUIState: AppUIState
    @EnvironmentObject var platformState: AppState
    @Environment(\.modelContext) private var modelContext

    /// All locally persisted wallet records. Drives the
    /// orphan-mnemonic detection: a keychain mnemonic with zero
    /// matching `PersistentWallet` rows means we landed in a
    /// reinstalled / container-wiped state and should offer to
    /// re-derive or delete.
    @Query private var persistentWallets: [PersistentWallet]

    /// Root tab selection — owned by AppUIState so deep views (e.g.
    /// IdentityDetailView's Contacts row) can switch tabs.
    private var selectedTab: Binding<RootTab> {
        Binding(
            get: { appUIState.selectedTab },
            set: { appUIState.selectedTab = $0 }
        )
    }

    // Orphan-mnemonic recovery flow. The whole batch surfaces in one
    // alert + one sheet now: the primary "Recover Wallets?" alert
    // gates the flow on user intent, the sheet collects per-wallet
    // "Same pin code" choices, and a single auth covers every shared
    // wallet (with separate auth prompts only for the rows the user
    // unchecks). `showDeletePrompt` drives the "Keep these Wallets?"
    // fallback when the user picks No.
    @State private var orphanEntries: [OrphanWalletEntry] = []
    @State private var showRecoverAlert = false
    @State private var showRecoverSheet = false
    @State private var showDeletePrompt = false
    @State private var recoveryInProgress = false
    @State private var recoveryError: String?
    @State private var orphanCheckDone = false

    var body: some View {
        if !isInitialized {
            VStack(spacing: 20) {
                ProgressView("Initializing...")
                    .scaleEffect(1.5)

                if let error = bootstrapError {
                    VStack(spacing: 10) {
                        Text("Initialization Error")
                            .font(.headline)
                            .foregroundColor(.red)

                        Text(error.localizedDescription)
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal)

                        Button("Retry") {
                            onRetry()
                        }
                        .buttonStyle(.borderedProminent)
                    }
                    .padding()
                    .background(Color.red.opacity(0.1))
                    .cornerRadius(10)
                    .padding()
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            TabView(selection: selectedTab) {
                // Tab 1: Sync Status
                SyncStatusView()
                    .tabItem {
                        Label("Sync", systemImage: "arrow.triangle.2.circlepath")
                    }
                    .tag(RootTab.sync)

                // Tab 2: Wallets
                WalletsTabView()
                    .accessibilityIdentifier("rootTab.wallets")
                    .tabItem {
                        Label("Wallets", systemImage: "wallet.pass")
                    }
                    .tag(RootTab.wallets)

                // Tab 3: Identities
                IdentitiesTabView(network: platformState.currentNetwork)
                    .tabItem {
                        Label("Identities", systemImage: "person.crop.circle")
                    }
                    .tag(RootTab.identities)

                // Tab 4: DashPay — first-class contacts / requests /
                // payments surface. The root-tab
                // selection binding lets its empty states
                // deep-link to the Wallets / Identities tabs.
                DashPayTabView(
                    network: platformState.currentNetwork,
                    selectedTab: selectedTab
                )
                .accessibilityIdentifier("dashpay.tab")
                .tabItem {
                    Label("DashPay", systemImage: "person.2.fill")
                }
                .tag(RootTab.dashpay)

                // Tab 5: Settings (Platform section + Contracts, which
                // moved here from its own tab so the bar shows 5 tabs
                // directly instead of collapsing into iOS's "More").
                SettingsView()
                    .tabItem {
                        Label("Settings", systemImage: "gearshape")
                    }
                    .tag(RootTab.settings)
            }
            .overlay(alignment: .top) {
                // The sync indicator depends on `walletManager.spvProgress`,
                // which publishes on a fast cadence while syncing. Reading
                // it directly in `ContentView.body` would subscribe the
                // whole `TabView` to every progress tick, re-creating each
                // tab's content (and any sheet it presents) several times a
                // second — which tears down a pushed drill-down and
                // dismisses sheets presented from it (e.g. the
                // document-create flow). Isolating the volatile observation
                // in this leaf keeps the tab content stable; only the
                // overlay re-renders on progress.
                GlobalSyncIndicatorOverlay(isSyncTab: selectedTab.wrappedValue == .sync)
            }
            .onAppear { checkForOrphanMnemonic() }
            .onChange(of: persistentWallets.count) { _, _ in
                checkForOrphanMnemonic()
            }
            .alert(recoverAlertTitle, isPresented: $showRecoverAlert) {
                Button("Authorize") {
                    showRecoverSheet = true
                }
                Button("No", role: .cancel) {
                    showDeletePrompt = true
                }
            } message: {
                Text(recoverAlertMessage)
            }
            .sheet(isPresented: $showRecoverSheet) {
                RecoverWalletsSheet(
                    entries: orphanEntries,
                    onAuthorize: { choices in
                        Task { await authorizeAndRecover(choices: choices) }
                    },
                    onCancel: {
                        // Sheet cancel = "give me the keep/delete
                        // path" so the user isn't trapped in a loop
                        // between the alert and the sheet.
                        showDeletePrompt = true
                    }
                )
            }
            .alert(deletePromptTitle, isPresented: $showDeletePrompt) {
                Button("Recreate") {
                    showRecoverAlert = true
                }
                Button(deletePromptDestructiveLabel, role: .destructive) {
                    deleteStoredMnemonics()
                }
            } message: {
                Text(deletePromptMessage)
            }
            .alert(
                "Recovery Failed",
                isPresented: Binding(
                    get: { recoveryError != nil },
                    set: { if !$0 { recoveryError = nil } }
                ),
                presenting: recoveryError
            ) { _ in
                Button("OK", role: .cancel) { recoveryError = nil }
            } message: { message in
                Text(message)
            }
        }
    }

    // MARK: - Orphan mnemonic recovery

    private var recoverAlertTitle: String {
        orphanEntries.count <= 1 ? "Recover Wallet?" : "Recover Wallets?"
    }

    private var recoverAlertMessage: String {
        let count = orphanEntries.count
        if count <= 1 {
            return "A wallet mnemonic is stored on this device, but no "
                + "wallet data was found. Authorize to re-derive the "
                + "wallet's public keys from the stored mnemonic."
        }
        return "\(count) wallet mnemonics are stored on this device, but "
            + "no matching wallet data was found. Authorize to re-derive "
            + "their public keys from the stored mnemonics."
    }

    private var deletePromptTitle: String {
        orphanEntries.count <= 1 ? "Keep this Wallet?" : "Keep these Wallets?"
    }

    private var deletePromptMessage: String {
        if orphanEntries.count <= 1 {
            return "Recreate will re-derive the wallet from the stored "
                + "mnemonic. Delete will permanently remove the "
                + "mnemonic from this device."
        }
        return "Recreate will re-derive every wallet from its stored "
            + "mnemonic. Delete will permanently remove "
            + "\(orphanEntries.count) mnemonics from this device."
    }

    private var deletePromptDestructiveLabel: String {
        orphanEntries.count <= 1 ? "Delete" : "Delete All"
    }

    /// Detect keychain mnemonics with no matching `PersistentWallet`
    /// row and prime the recovery flow. Runs once per launch after
    /// the first tab becomes visible. Subsequent `persistentWallets`
    /// changes re-evaluate so already-recovered wallets drop out of
    /// the orphan set and the alert reflects the new count.
    @MainActor
    private func checkForOrphanMnemonic() {
        guard isInitialized, !orphanCheckDone else { return }

        let storage = WalletStorage()
        let keychainIds = (try? storage.listWalletIdsWithMnemonic()) ?? []
        let localIds = Set(persistentWallets.map(\.walletId))
        let orphans = keychainIds.filter { !localIds.contains($0) }

        guard !orphans.isEmpty else {
            // Nothing to do; mark the check done so we don't re-scan
            // every time a wallet is added/removed mid-session.
            orphanCheckDone = true
            return
        }

        // Resolve display name + network for each orphan up-front so
        // the alert + sheet render with stable, recognizable rows
        // instead of bare 32-byte ids. Metadata reads are best-effort
        // — older installs predate the metadata blob and just fall
        // back to the generic placeholder name.
        orphanEntries = orphans.map { walletId in
            let metadata = (try? storage.metadata(for: walletId)) ?? nil
            let displayName: String = {
                guard let raw = metadata?.name else { return "Recovered Wallet" }
                let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
                return trimmed.isEmpty ? "Recovered Wallet" : trimmed
            }()
            return OrphanWalletEntry(
                walletId: walletId,
                displayName: displayName,
                network: metadata?.resolvedNetworks.first,
                samePinCode: true
            )
        }
        orphanCheckDone = true
        showRecoverAlert = true
    }

    /// Run the auth + re-derivation pipeline for the user's choices.
    /// All wallets with `samePinCode == true` collapse into a single
    /// `LAContext.evaluatePolicy` call; everything else gets its own
    /// prompt. A cancel / policy failure on the shared prompt aborts
    /// the shared group and bounces to the keep/delete decision; a
    /// failure on a per-wallet prompt skips just that wallet.
    @MainActor
    private func authorizeAndRecover(choices: [OrphanWalletEntry]) async {
        guard !recoveryInProgress else { return }
        recoveryInProgress = true
        defer { recoveryInProgress = false }

        let shared = choices.filter(\.samePinCode)
        let separate = choices.filter { !$0.samePinCode }
        var recovered: Set<Data> = []

        SDKLogger.log(
            "Recovery: authorize+recover — \(choices.count) wallet(s) "
                + "(\(shared.count) shared-prompt, \(separate.count) per-wallet)",
            minimumLevel: .low
        )

        // Both auth failures (per-wallet biometric prompt errors) and
        // recovery failures (SDK init / manager prep / createWallet
        // errors returned from `recoverWallet`) accumulate here so the
        // user sees every problem at the end rather than the last one
        // overwriting earlier messages.
        var perWalletFailures: [String] = []

        if !shared.isEmpty {
            let reason = shared.count == 1
                ? "Re-derive your wallet from the stored recovery phrase."
                : "Re-derive \(shared.count) wallets from the stored recovery phrases."
            switch await runAuthPrompt(reason: reason) {
            case .authorized:
                for entry in shared {
                    if let failure = await recoverWallet(entry: entry) {
                        perWalletFailures.append(failure)
                    } else {
                        recovered.insert(entry.walletId)
                    }
                }
            case .denied:
                // User actively bailed on the shared prompt —
                // surface the keep/delete decision and let them
                // pick again from there.
                showDeletePrompt = true
                return
            case .unavailable(let detail):
                let message = "Authentication is unavailable on this device: \(detail)"
                SDKLogger.error("Recovery: \(message)")
                recoveryError = message
                showDeletePrompt = true
                return
            case .failed(let detail):
                let message = "Authorization failed: \(detail)"
                SDKLogger.error("Recovery: \(message)")
                recoveryError = message
                showDeletePrompt = true
                return
            }
        }

        for entry in separate {
            let reason = "Re-derive \"\(entry.displayName)\" from its stored recovery phrase."
            switch await runAuthPrompt(reason: reason) {
            case .authorized:
                if let failure = await recoverWallet(entry: entry) {
                    perWalletFailures.append(failure)
                } else {
                    recovered.insert(entry.walletId)
                }
            case .denied:
                // Skip this one — user said no to this specific
                // wallet — but keep going through the rest.
                SDKLogger.log(
                    "Recovery: user denied auth for \"\(entry.displayName)\"; skipping",
                    minimumLevel: .medium
                )
                continue
            case .unavailable(let detail):
                // Same shape as the shared-branch handler: surface
                // the error AND route into the keep/delete prompt
                // so the user has a path forward instead of being
                // left with stale orphans queued internally with
                // no UI to act on them.
                let message = "Authentication is unavailable on this device: \(detail)"
                SDKLogger.error("Recovery: \(message)")
                recoveryError = message
                showDeletePrompt = true
                return
            case .failed(let detail):
                let entryFailure = "\(entry.displayName): \(detail)"
                SDKLogger.error("Recovery: auth failed — \(entryFailure)")
                perWalletFailures.append(entryFailure)
                continue
            }
        }
        if !perWalletFailures.isEmpty {
            // One combined prompt at the end, joining every wallet's
            // failure into one message so none get lost.
            let prefix = perWalletFailures.count == 1
                ? "Recovery failed: "
                : "Recovery failed for \(perWalletFailures.count) wallets:\n"
            let combined = prefix + perWalletFailures.joined(separator: "\n")
            SDKLogger.error("Recovery: aggregated failures — \(combined)")
            recoveryError = combined
        }

        // Drop the entries we just recreated. If the user skipped
        // some (cancelled per-wallet auths) the remaining orphans
        // stay queued — the next `checkForOrphanMnemonic()` (after
        // `persistentWallets` reactivity fires) will re-arm the
        // alert with the trimmed set.
        orphanEntries.removeAll { recovered.contains($0.walletId) }
        orphanCheckDone = false
        if !orphanEntries.isEmpty {
            // Small defer so the sheet dismiss has time to settle
            // before the alert pops back up.
            Task { @MainActor in
                try? await Task.sleep(nanoseconds: 250_000_000)
                showRecoverAlert = true
            }
        }
    }

    private enum AuthOutcome {
        case authorized
        case denied
        case unavailable(String)
        case failed(String)
    }

    /// Single LAContext-backed system prompt. Maps every cancel /
    /// policy / error path to a typed outcome so callers can decide
    /// whether to abort or skip.
    private func runAuthPrompt(reason: String) async -> AuthOutcome {
        let context = LAContext()
        context.localizedCancelTitle = "Cancel"
        var policyError: NSError?
        guard context.canEvaluatePolicy(
            .deviceOwnerAuthentication,
            error: &policyError
        ) else {
            return .unavailable(policyError?.localizedDescription ?? "unknown")
        }
        do {
            let authorized = try await context.evaluatePolicy(
                .deviceOwnerAuthentication,
                localizedReason: reason
            )
            return authorized ? .authorized : .denied
        } catch {
            return .failed(error.localizedDescription)
        }
    }

    /// Read the keychain mnemonic + metadata for `entry`, then
    /// re-create the wallet. Returns `nil` on success or the
    /// failure message on failure. The caller aggregates failures
    /// across multiple recoveries — relying on `recoveryError`
    /// (a single `String?`) loses every error but the last when a
    /// multi-wallet recovery has more than one failure.
    @MainActor
    private func recoverWallet(entry: OrphanWalletEntry) async -> String? {
        let storage = WalletStorage()
        let mnemonic: String
        do {
            mnemonic = try storage.retrieveMnemonic(for: entry.walletId)
        } catch {
            let message = "\"\(entry.displayName)\": failed to read stored mnemonic — "
                + error.localizedDescription
            SDKLogger.error("Recovery: \(message)")
            return message
        }

        // Re-fetch metadata at recovery time rather than relying on
        // what the alert captured — ensures the description /
        // birth-height carried over reflect the current state of the
        // keychain blob rather than a possibly-stale snapshot.
        let metadata = (try? storage.metadata(for: entry.walletId)) ?? nil
        let restoredName: String = {
            guard let raw = metadata?.name else { return entry.displayName }
            let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
            return trimmed.isEmpty ? entry.displayName : trimmed
        }()
        let restoredDescription = metadata?.walletDescription
        // First valid stored network wins; the
        // `walletManager.createWallet` API still takes a single
        // network. Multi-network support is a TODO on the Rust side
        // — when it lands, `metadata?.resolvedNetworks` is already
        // there to feed in directly. Final fallback is the user's
        // active network rather than a hard-coded testnet — the
        // only rows that hit the fallback are legacy / corrupted
        // metadata blobs, and re-deriving them on whatever network
        // the user is currently looking at matches their context
        // far better than silently restoring on testnet.
        let restoredNetwork: Network =
            entry.network ?? metadata?.resolvedNetworks.first ?? platformState.currentNetwork
        let restoredBirthHeight = metadata?.birthHeight

        // Route recovery to the manager for the wallet's original
        // network — not the active manager. The Rust side stamps
        // `wallet.network = self.sdk.network` at registration time,
        // so creating a regtest wallet through the testnet manager
        // would persist the row as testnet. `backgroundManager`
        // lazy-builds the per-network manager (and its SDK) without
        // changing the user's currently visible network.
        let recoveryManager: PlatformWalletManager
        do {
            recoveryManager = try walletManagerStore.backgroundManager(for: restoredNetwork)
        } catch {
            let message = "\"\(entry.displayName)\" (\(restoredNetwork.displayName)): "
                + "failed to prepare wallet manager — \(error.localizedDescription)"
            SDKLogger.error("Recovery: \(message) (raw: \(error))")
            return message
        }

        do {
            // Recovery restores an existing wallet that may have prior on-chain
            // history (incl. DashPay payments). Scan from the wallet's persisted
            // birth height when known (avoids re-scanning years of irrelevant
            // history), falling back to genesis only for wallets that predate
            // that metadata — never the current tip, which would skip the
            // history recovery exists to recover.
            let managed = try recoveryManager.createWallet(
                mnemonic: mnemonic,
                network: restoredNetwork,
                name: restoredName,
                birthHeight: restoredBirthHeight ?? 0
            )
            let walletIdMatch = managed.walletId
            let descriptor = FetchDescriptor<PersistentWallet>(
                predicate: #Predicate { $0.walletId == walletIdMatch }
            )

            // Cross-context propagation: the row we just created
            // landed in the target manager's background context,
            // not in the main context that drives every `@Query`
            // consumer in the UI. SwiftData merges sibling-context
            // saves through `NSPersistentStoreRemoteChange`
            // notifications, but that pipeline is asynchronous —
            // an immediate `fetch` on the main context may still
            // miss the row, in which case the post-recovery
            // `isImported` / metadata flush is skipped *and* no
            // main-context save fires, so `@Query` observers
            // never re-evaluate. The wallet then "doesn't appear"
            // until the next app launch when the main context is
            // built fresh against disk. Retry a few times with a
            // short yield between attempts so the merge has a
            // chance to settle before we move on.
            var row: PersistentWallet? = nil
            for attempt in 0..<10 {
                row = try? modelContext.fetch(descriptor).first
                if row != nil { break }
                if attempt < 9 {
                    try? await Task.sleep(nanoseconds: 50_000_000)
                }
            }

            if let row {
                row.isImported = true
                if row.walletDescription == nil {
                    row.walletDescription = restoredDescription
                }
                // Persisted birth height wins over the synthetic
                // value the persister stamped from the live SPV
                // tip — otherwise a recovered wallet would scan
                // forward from "now" and lose every transaction
                // older than the recovery moment.
                if let stored = restoredBirthHeight {
                    row.birthHeight = stored
                }
                try? modelContext.save()
            }
            return nil
        } catch {
            let message = "\"\(entry.displayName)\" (\(restoredNetwork.displayName)): "
                + error.localizedDescription
            SDKLogger.error("Recovery: createWallet failed — \(message) (raw: \(error))")
            return message
        }
    }

    /// Drop every queued orphan's mnemonic + metadata from the
    /// keychain. Best-effort: failures on individual rows surface
    /// in `recoveryError` but the loop continues so a single bad
    /// row can't block the rest of the cleanup.
    @MainActor
    private func deleteStoredMnemonics() {
        let storage = WalletStorage()
        var failures: [String] = []
        for entry in orphanEntries {
            do {
                try storage.deleteMnemonic(for: entry.walletId)
            } catch {
                failures.append("\(entry.displayName): \(error.localizedDescription)")
                continue
            }
            // Metadata follows the mnemonic. If this fails the row
            // is harmless (no secret material) and gets overwritten
            // on the next `setMetadata`.
            try? storage.deleteMetadata(for: entry.walletId)
        }
        orphanEntries.removeAll()
        if !failures.isEmpty {
            recoveryError = "Failed to delete mnemonic"
                + (failures.count == 1 ? "" : "s")
                + ": " + failures.joined(separator: "; ")
        }
    }
}

/// Leaf wrapper that owns the volatile `walletManager` / `appUIState`
/// observations so the sync-progress publish cadence re-renders only
/// this overlay, not the parent `TabView`. `isSyncTab` is passed as a
/// plain value (it changes only on tab switch), keeping this view's
/// only fast-publishing dependency local.
struct GlobalSyncIndicatorOverlay: View {
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var appUIState: AppUIState
    let isSyncTab: Bool

    var body: some View {
        let state = walletManager.spvProgress.overallState
        if state == .syncing || state == .waitingForConnections {
            GlobalSyncIndicator(showDetails: isSyncTab && appUIState.showWalletsSyncDetails)
        }
    }
}

struct GlobalSyncIndicator: View {
    @EnvironmentObject var walletManager: PlatformWalletManager
    /// Used so the stop button cancels any in-flight SPV start before
    /// stopping — see `stopSpvCancellingPendingStarts`.
    @EnvironmentObject var walletManagerStore: WalletManagerStore
    let showDetails: Bool

    // Helpers
    private var phaseTitle: String {
        switch walletManager.spvProgress.overallState {
        case .waitingForConnections: return "Waiting for Connection"
        case .waitForEvents: return "Waiting for Events"
        case .syncing: return "Syncing"
        case .synced: return "Synced"
        case .error:
            let errMsg = walletManager.lastError?.localizedDescription ?? "Unknown error"
            return "Error occurred during sync \(errMsg)"
        }
    }

    private var fillProgress: Double {
        walletManager.spvProgress.overallPercentage
    }

    var body: some View {
        VStack(spacing: 0) {
            if showDetails {
                HStack {
                    Image(systemName: "arrow.triangle.2.circlepath")
                        .font(.caption)
                        .symbolEffect(.pulse)
                    Text(phaseTitle)
                        .font(.caption)
                    Spacer()
                    Button(action: {
                        walletManagerStore.stopSpvCancellingPendingStarts(walletManager)
                    }) {
                        Image(systemName: "xmark.circle.fill")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                .padding(.horizontal)
                .padding(.vertical, 8)
                .background(Material.thin)
            }
            // Thin progress bar always shown
            GeometryReader { geometry in
                // Use current phase progress for the thin bar (filters → filter headers → headers)
                Rectangle()
                    .fill(Color.blue)
                    .frame(width: geometry.size.width * fillProgress)
            }
            .frame(height: 2)
        }
        // When not showing details, don't intercept touches (so back buttons work)
        .allowsHitTesting(showDetails)
    }
}

// Wrapper views
struct SyncStatusView: View {
    var body: some View {
        NavigationStack {
            CoreContentView()
        }
    }
}

struct WalletsTabView: View {
    var body: some View {
        NavigationStack {
            WalletsContentView()
        }
    }
}

struct IdentitiesTabView: View {
    let network: Network

    var body: some View {
        NavigationStack {
            IdentitiesContentView(network: network)
        }
    }
}

struct SettingsView: View {
    var body: some View {
        OptionsView()
    }
}
