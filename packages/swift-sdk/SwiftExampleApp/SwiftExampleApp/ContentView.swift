import SwiftUI
import SwiftData

enum RootTab: Hashable {
    case wallets, identities, friends, platform, settings
}

struct ContentView: View {
    @EnvironmentObject var unifiedState: UnifiedAppState
    @EnvironmentObject var walletService: WalletService
    
    @State private var selectedTab: RootTab = .wallets

    var body: some View {
        if !unifiedState.isInitialized {
            VStack(spacing: 20) {
                ProgressView("Initializing...")
                    .scaleEffect(1.5)
                
                if let error = unifiedState.error {
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
                            Task {
                                unifiedState.error = nil
                                await unifiedState.initialize()
                            }
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
                // Tab 1: Wallets
                CoreWalletView()
                    .tabItem {
                        Label("Wallets", systemImage: "wallet.pass")
                    }
                    .tag(RootTab.wallets)
                
                // Tab 2: Identities
                IdentitiesView()
                    .tabItem {
                        Label("Identities", systemImage: "person.circle")
                    }
                    .tag(RootTab.identities)
                
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
                if walletService.isSyncing {
                    GlobalSyncIndicator(showDetails: selectedTab == .wallets && unifiedState.showWalletsSyncDetails)
                        .environmentObject(walletService)
                }
            }
        }
    }
}

struct GlobalSyncIndicator: View {
    @EnvironmentObject var walletService: WalletService
    let showDetails: Bool
    
    // Helpers
    private var phaseTitle: String {
        let h = min(max(walletService.headerProgress, 0.0), 1.0)
        let fh = min(max(walletService.filterHeaderProgress, 0.0), 1.0)
        let f = min(max(walletService.transactionProgress, 0.0), 1.0)
        if f > 0.0 && f < 1.0 { return "Filters (\(Int(f * 100))%)" }
        if fh > 0.0 && fh < 1.0 { return "Filter Headers (\(Int(fh * 100))%)" }
        if h < 1.0 { return "Headers (\(Int(h * 100))%)" }
        return "Complete"
    }

    private var fillProgress: Double {
        let h = min(max(walletService.headerProgress, 0.0), 1.0)
        let fh = min(max(walletService.filterHeaderProgress, 0.0), 1.0)
        let f = min(max(walletService.transactionProgress, 0.0), 1.0)

        if f > 0.0 && f < 1.0 { return f }
        if fh > 0.0 && fh < 1.0 { return fh }
        if h < 1.0 { return h }
        return 1.0
    }

    var body: some View {
        VStack(spacing: 0) {
            if walletService.detailedSyncProgress != nil {
                if showDetails {
                    HStack {
                        Image(systemName: "arrow.triangle.2.circlepath")
                            .font(.caption)
                            .symbolEffect(.pulse)
                        Text("Syncing: \(phaseTitle)")
                            .font(.caption)
                        Spacer()
                        // No right-side numbers in the top bar per design
                        Button(action: { walletService.stopSync() }) {
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
        }
        // When not showing details, don't intercept touches (so back buttons work)
        .allowsHitTesting(showDetails)
    }
}

// Wrapper views
struct CoreWalletView: View {
    @EnvironmentObject var unifiedState: UnifiedAppState
    
    var body: some View {
        NavigationStack {
            CoreContentView()
                .environmentObject(unifiedState.walletService)
                .environmentObject(unifiedState)
                .environment(\.modelContext, unifiedState.modelContainer.mainContext)
        }
    }
}

struct SettingsView: View {
    @EnvironmentObject var unifiedState: UnifiedAppState
    
    var body: some View {
        OptionsView()
            .environmentObject(unifiedState.platformState)
            .environmentObject(unifiedState)
    }
}
