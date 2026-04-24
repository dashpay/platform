import SwiftUI
import SwiftDashSDK
import SwiftData
import LocalAuthentication

enum RootTab: Hashable {
    case sync, wallets, identities, friends, settings
}

struct ContentView: View {
    let isInitialized: Bool
    let bootstrapError: Error?
    let onRetry: () -> Void

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var appUIState: AppUIState
    @Environment(\.modelContext) private var modelContext

    /// All locally persisted wallet records. Drives the
    /// orphan-mnemonic detection: a keychain mnemonic with zero
    /// matching `PersistentWallet` rows means we landed in a
    /// reinstalled / container-wiped state and should offer to
    /// re-derive or delete.
    @Query private var persistentWallets: [PersistentWallet]

    @State private var selectedTab: RootTab = .sync

    // Orphan-mnemonic recovery flow. Prompts fire sequentially: one
    // wallet at a time, starting with the head of `pendingOrphans`.
    // `showRecoverAlert` drives the primary (Authorize / No) alert and
    // `showDeletePrompt` drives the secondary (Recreate / Delete) one.
    @State private var pendingOrphans: [Data] = []
    @State private var showRecoverAlert = false
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
            TabView(selection: $selectedTab) {
                // Tab 1: Sync Status
                SyncStatusView()
                    .tabItem {
                        Label("Sync", systemImage: "arrow.triangle.2.circlepath")
                    }
                    .tag(RootTab.sync)

                // Tab 2: Wallets
                WalletsTabView()
                    .tabItem {
                        Label("Wallets", systemImage: "wallet.pass")
                    }
                    .tag(RootTab.wallets)

                // Tab 3: Identities
                IdentitiesTabView()
                    .tabItem {
                        Label("Identities", systemImage: "person.crop.circle")
                    }
                    .tag(RootTab.identities)

                // Tab 4: Friends
                FriendsView()
                    .tabItem {
                        Label("Friends", systemImage: "person.2")
                    }
                    .tag(RootTab.friends)

                // Tab 5: Settings (includes Platform section)
                SettingsView()
                    .tabItem {
                        Label("Settings", systemImage: "gearshape")
                    }
                    .tag(RootTab.settings)
            }
            .overlay(alignment: .top) {
                let state = walletManager.spvProgress.overallState
                if state == .syncing || state == .waitingForConnections {
                    GlobalSyncIndicator(showDetails: selectedTab == .sync && appUIState.showWalletsSyncDetails)
                }
            }
            .onAppear { checkForOrphanMnemonic() }
            .onChange(of: persistentWallets.count) { _, _ in
                checkForOrphanMnemonic()
            }
            .alert("Recover Wallet?", isPresented: $showRecoverAlert) {
                Button("Authorize") {
                    Task { await authorizeAndRecover() }
                }
                Button("No", role: .cancel) {
                    showDeletePrompt = true
                }
            } message: {
                Text(
                    "A wallet mnemonic is stored on this device, but no "
                    + "wallet data was found. Authorize to re-derive the "
                    + "wallet's public keys from the stored mnemonic."
                )
            }
            .alert("Keep this Wallet?", isPresented: $showDeletePrompt) {
                Button("Recreate") {
                    showRecoverAlert = true
                }
                Button("Delete", role: .destructive) {
                    deleteStoredMnemonic()
                }
            } message: {
                Text(
                    "Recreate will re-derive the wallet from the stored "
                    + "mnemonic. Delete will permanently remove the "
                    + "mnemonic from this device."
                )
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

    /// Detect keychain mnemonics with no matching `PersistentWallet`
    /// row and kick off the recovery alert for each in turn. Runs
    /// once per launch after the first tab becomes visible.
    /// Subsequent `persistentWallets` changes re-evaluate so
    /// newly-recovered wallets drop out of the queue and we advance
    /// to the next orphan.
    @MainActor
    private func checkForOrphanMnemonic() {
        guard isInitialized, !orphanCheckDone else { return }
        orphanCheckDone = true

        let storage = WalletStorage()
        let keychainIds = (try? storage.listWalletIdsWithMnemonic()) ?? []
        let localIds = Set(persistentWallets.map(\.walletId))
        let orphans = keychainIds.filter { !localIds.contains($0) }

        guard !orphans.isEmpty else { return }
        pendingOrphans = orphans
        showRecoverAlert = true
    }

    /// Drop the just-handled orphan from the queue and re-arm the
    /// primary alert for the next one (if any).
    @MainActor
    private func advanceToNextOrphan() {
        if !pendingOrphans.isEmpty {
            pendingOrphans.removeFirst()
        }
        if !pendingOrphans.isEmpty {
            // Small defer so SwiftUI has a chance to tear the
            // previous alert down before presenting the next.
            Task { @MainActor in
                try? await Task.sleep(nanoseconds: 200_000_000)
                showRecoverAlert = true
            }
        }
    }

    /// Authorize via device passcode / biometrics, then fetch the
    /// keychain-stored mnemonic for the current orphan and re-create
    /// the wallet.
    @MainActor
    private func authorizeAndRecover() async {
        guard !recoveryInProgress else { return }
        guard let walletId = pendingOrphans.first else { return }
        recoveryInProgress = true
        defer { recoveryInProgress = false }

        let context = LAContext()
        context.localizedCancelTitle = "Cancel"
        var policyError: NSError?
        guard context.canEvaluatePolicy(
            .deviceOwnerAuthentication,
            error: &policyError
        ) else {
            recoveryError =
                "Authentication is unavailable on this device: "
                + (policyError?.localizedDescription ?? "unknown")
            showDeletePrompt = true
            return
        }

        do {
            let authorized = try await context.evaluatePolicy(
                .deviceOwnerAuthentication,
                localizedReason: "Re-derive your wallet from the stored recovery phrase."
            )
            guard authorized else {
                showDeletePrompt = true
                return
            }
        } catch {
            // User cancel or policy failure — bounce to the
            // Keep/Delete choice so they don't lose the path forward.
            recoveryError = "Authorization failed: \(error.localizedDescription)"
            showDeletePrompt = true
            return
        }

        let mnemonic: String
        do {
            mnemonic = try WalletStorage().retrieveMnemonic(for: walletId)
        } catch {
            recoveryError = "Failed to read stored mnemonic: \(error.localizedDescription)"
            return
        }

        do {
            // Default the restored wallet to testnet with a
            // recognizable label. The user can rename via the
            // wallet list afterwards. The `PersistentWallet` row
            // is created by the persister callback downstream of
            // `walletManager.createWallet` — we only need to
            // stamp the `isImported` flag here.
            let platformNetwork: PlatformNetwork = .testnet
            let label = "Recovered Wallet"
            let managed = try walletManager.createWallet(
                mnemonic: mnemonic,
                network: platformNetwork,
                name: label
            )
            let walletIdMatch = managed.walletId
            let descriptor = FetchDescriptor<PersistentWallet>(
                predicate: #Predicate { $0.walletId == walletIdMatch }
            )
            if let row = try? modelContext.fetch(descriptor).first {
                row.isImported = true
                try? modelContext.save()
            }
            advanceToNextOrphan()
        } catch {
            recoveryError = "Failed to recreate wallet: \(error.localizedDescription)"
        }
    }

    /// Remove the currently-selected orphan's mnemonic from the
    /// keychain and advance to the next orphan in the queue.
    @MainActor
    private func deleteStoredMnemonic() {
        guard let walletId = pendingOrphans.first else { return }
        do {
            try WalletStorage().deleteMnemonic(for: walletId)
            advanceToNextOrphan()
        } catch {
            recoveryError = "Failed to delete mnemonic: \(error.localizedDescription)"
        }
    }
}

struct GlobalSyncIndicator: View {
    @EnvironmentObject var walletManager: PlatformWalletManager
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
                        try? walletManager.stopSpv()
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
    var body: some View {
        NavigationStack {
            IdentitiesContentView()
        }
    }
}

struct SettingsView: View {
    var body: some View {
        OptionsView()
    }
}
