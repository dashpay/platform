import SwiftUI
import SwiftData

struct ContentView: View {
    @EnvironmentObject var unifiedState: UnifiedAppState
    @EnvironmentObject var walletService: WalletService
    
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
            TabView {
                // Core features
                CoreWalletView()
                    .tabItem {
                        Label("Wallets", systemImage: "wallet.pass")
                    }
                
                CoreTransactionsView()
                    .tabItem {
                        Label("Transactions", systemImage: "list.bullet")
                    }
                
                // Platform features
                IdentitiesView()
                    .tabItem {
                        Label("Identities", systemImage: "person.3")
                    }
                
                PlatformView()
                    .tabItem {
                        Label("Platform", systemImage: "network")
                    }
                
                // Settings
                SettingsView()
                    .tabItem {
                        Label("Settings", systemImage: "gearshape")
                    }
            }
            .overlay(alignment: .top) {
                if walletService.isSyncing {
                    GlobalSyncIndicator()
                        .environmentObject(walletService)
                }
            }
        }
    }
}

struct GlobalSyncIndicator: View {
    @EnvironmentObject var walletService: WalletService
    
    var body: some View {
        VStack(spacing: 0) {
            if let progress = walletService.detailedSyncProgress as? SyncProgress {
                HStack {
                    Image(systemName: "arrow.triangle.2.circlepath")
                        .font(.caption)
                        .symbolEffect(.pulse)
                    
                    Text("Syncing: \(Int(progress.progress * 100))%")
                        .font(.caption)
                    
                    Spacer()
                    
                    Text("\(progress.current)/\(progress.total)")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                    
                    Button(action: {
                        walletService.stopSync()
                    }) {
                        Image(systemName: "xmark.circle.fill")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                .padding(.horizontal)
                .padding(.vertical, 8)
                .background(Material.thin)
                
                // Progress bar
                GeometryReader { geometry in
                    Rectangle()
                        .fill(Color.blue)
                        .frame(width: geometry.size.width * progress.progress)
                }
                .frame(height: 2)
            }
        }
    }
}

// Wrapper views
struct CoreWalletView: View {
    @EnvironmentObject var unifiedState: UnifiedAppState
    
    var body: some View {
        NavigationStack {
            CoreContentView()
                .environmentObject(unifiedState.walletService)
                .environment(\.modelContext, unifiedState.modelContainer.mainContext)
        }
    }
}

struct CoreTransactionsView: View {
    @EnvironmentObject var unifiedState: UnifiedAppState
    @Query private var wallets: [HDWallet]
    
    var body: some View {
        NavigationStack {
            if let firstWallet = wallets.first {
                WalletDetailView(wallet: firstWallet)
            } else {
                ContentUnavailableView(
                    "No Wallets",
                    systemImage: "wallet.pass",
                    description: Text("Create a wallet to view transactions")
                )
            }
        }
        .environmentObject(unifiedState.walletService)
    }
}

struct SettingsView: View {
    @EnvironmentObject var unifiedState: UnifiedAppState
    
    var body: some View {
        NavigationStack {
            List {
                Section("Network") {
                    HStack {
                        Text("Network")
                        Spacer()
                        Text("Testnet")
                            .foregroundColor(.secondary)
                    }
                    
                    HStack {
                        Text("Core Sync")
                        Spacer()
                        if let progress = unifiedState.walletService.detailedSyncProgress as? SyncProgress {
                            Text("\(Int(progress.progress * 100))%")
                                .foregroundColor(.secondary)
                        } else {
                            Text("Not syncing")
                                .foregroundColor(.secondary)
                        }
                    }
                    
                    HStack {
                        Text("Platform Sync")
                        Spacer()
                        Text(unifiedState.unifiedState.isPlatformSynced ? "Synced" : "Offline")
                            .foregroundColor(.secondary)
                    }
                }
                
                Section("Data") {
                    NavigationLink(destination: LocalDataContractsView()) {
                        Text("Local Data Contracts")
                    }
                }
                
                Section("About") {
                    HStack {
                        Text("Version")
                        Spacer()
                        Text(Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "1.0")
                            .foregroundColor(.secondary)
                    }
                    
                    HStack {
                        Text("Build")
                        Spacer()
                        Text(AppVersion.gitCommit)
                            .foregroundColor(.secondary)
                            .font(.system(.caption, design: .monospaced))
                    }
                }
            }
            .navigationTitle("Settings")
        }
    }
}