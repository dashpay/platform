import SwiftUI
import SwiftDashSDK
import SwiftData

enum RootTab: Hashable {
    case sync, wallets, friends, platform, settings
}

struct ContentView: View {
    let isInitialized: Bool
    let bootstrapError: Error?
    let onRetry: () -> Void

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var appUIState: AppUIState

    @State private var selectedTab: RootTab = .sync

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
