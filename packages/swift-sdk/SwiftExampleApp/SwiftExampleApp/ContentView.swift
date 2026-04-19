import SwiftUI
import SwiftDashSDK
import SwiftData
import LocalAuthentication

enum RootTab: Hashable {
    case sync, wallets, friends, platform, settings
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
    /// `HDWallet` rows means we landed in a reinstalled / container-
    /// wiped state and should offer to re-derive or delete.
    @Query private var hdWallets: [HDWallet]

    @State private var selectedTab: RootTab = .sync

    // Orphan-mnemonic recovery flow.
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

                // Tab 2: Wallets (includes identities)
                WalletsTabView()
                    .tabItem {
                        Label("Wallets", systemImage: "wallet.pass")
                    }
                    .tag(RootTab.wallets)

                // Tab 3: Friends
                FriendsView()
                    .tabItem {
                        Label("Friends", systemImage: "person.2")
                    }
                    .tag(RootTab.friends)

                // Tab 4: Platform
                PlatformView()
                    .tabItem {
                        Label("Platform", systemImage: "network")
                    }
                    .tag(RootTab.platform)

                // Tab 5: Settings
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
            .onChange(of: hdWallets.count) { _, _ in
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

    /// Detect the "keychain has a mnemonic but SwiftData has no wallet
    /// records" case and kick off the recovery alert. Runs once per
    /// launch after the first tab becomes visible; subsequent
    /// `hdWallets` changes re-evaluate so creating a wallet clears the
    /// state without re-prompting.
    @MainActor
    private func checkForOrphanMnemonic() {
        guard isInitialized, !orphanCheckDone else { return }
        guard hdWallets.isEmpty else {
            // Wallet exists locally — nothing to recover.
            orphanCheckDone = true
            return
        }
        guard (try? WalletStorage().retrieveMnemonic()) != nil else {
            // No keychain mnemonic — nothing to recover.
            orphanCheckDone = true
            return
        }
        orphanCheckDone = true
        showRecoverAlert = true
    }

    /// Authorize via device passcode / biometrics, then fetch the
    /// keychain-stored mnemonic and re-create the wallet.
    @MainActor
    private func authorizeAndRecover() async {
        guard !recoveryInProgress else { return }
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
            mnemonic = try WalletStorage().retrieveMnemonic()
        } catch {
            recoveryError = "Failed to read stored mnemonic: \(error.localizedDescription)"
            return
        }

        do {
            // Default the restored wallet to testnet with a
            // recognizable label. The user can rename via the wallet
            // list afterwards.
            let platformNetwork: PlatformNetwork = .testnet
            let appNetwork: AppNetwork = .testnet
            let label = "Recovered Wallet"
            let seed = try SwiftDashSDK.Mnemonic.toSeed(mnemonic: mnemonic)
            let managed = try walletManager.createWallet(
                mnemonic: mnemonic,
                network: platformNetwork,
                name: label
            )
            let hdWallet = HDWallet(
                walletId: managed.walletId,
                serializedWalletBytes: seed,
                label: label,
                network: appNetwork,
                isWatchOnly: false,
                isImported: true
            )
            modelContext.insert(hdWallet)
            try modelContext.save()
        } catch {
            recoveryError = "Failed to recreate wallet: \(error.localizedDescription)"
        }
    }

    /// Remove the orphaned mnemonic from the keychain. Leaves an empty
    /// app state — the user can create a fresh wallet from scratch.
    @MainActor
    private func deleteStoredMnemonic() {
        do {
            try WalletStorage().deleteMnemonic()
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

struct SettingsView: View {
    var body: some View {
        OptionsView()
    }
}
