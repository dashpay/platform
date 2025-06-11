import SwiftUI

struct OptionsView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingDataManagement = false
    @State private var showingAbout = false
    @State private var showingContracts = false
    
    var body: some View {
        NavigationView {
            Form {
                Section("Network") {
                    Picker("Current Network", selection: $appState.currentNetwork) {
                        ForEach(Network.allCases, id: \.self) { network in
                            Text(network.displayName).tag(network)
                        }
                    }
                    .pickerStyle(SegmentedPickerStyle())
                    
                    HStack {
                        Text("Network Status")
                        Spacer()
                        if appState.sdk != nil {
                            Label("Connected", systemImage: "checkmark.circle.fill")
                                .font(.caption)
                                .foregroundColor(.green)
                        } else {
                            Label("Disconnected", systemImage: "xmark.circle.fill")
                                .font(.caption)
                                .foregroundColor(.red)
                        }
                    }
                }
                
                Section("Data") {
                    NavigationLink(destination: ContractsView()) {
                        Label("Browse Contracts", systemImage: "doc.plaintext")
                    }
                    
                    Button(action: { showingDataManagement = true }) {
                        Label("Manage Local Data", systemImage: "internaldrive")
                    }
                    
                    if let stats = appState.dataStatistics {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Storage Statistics")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            HStack {
                                Text("Identities:")
                                Spacer()
                                Text("\(stats.identities)")
                            }
                            .font(.caption)
                            HStack {
                                Text("Documents:")
                                Spacer()
                                Text("\(stats.documents)")
                            }
                            .font(.caption)
                            HStack {
                                Text("Contracts:")
                                Spacer()
                                Text("\(stats.contracts)")
                            }
                            .font(.caption)
                            HStack {
                                Text("Token Balances:")
                                Spacer()
                                Text("\(stats.tokenBalances)")
                            }
                            .font(.caption)
                        }
                        .padding(.vertical, 4)
                    }
                }
                
                Section("Developer") {
                    Toggle("Show Test Data", isOn: .constant(false))
                        .disabled(true)
                    
                    Toggle("Enable Debug Logging", isOn: .constant(false))
                        .disabled(true)
                    
                    Button(action: {
                        Task {
                            await appState.loadSampleIdentities()
                        }
                    }) {
                        Label("Load Sample Identities", systemImage: "person.badge.plus")
                    }
                }
                
                Section("About") {
                    Button(action: { showingAbout = true }) {
                        HStack {
                            Text("About Dash SDK Example")
                            Spacer()
                            Image(systemName: "chevron.right")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                    
                    HStack {
                        Text("SDK Version")
                        Spacer()
                        Text("1.0.0")
                            .foregroundColor(.secondary)
                    }
                    
                    HStack {
                        Text("App Version")
                        Spacer()
                        Text("1.0.0")
                            .foregroundColor(.secondary)
                    }
                }
            }
            .navigationTitle("Options")
            .task {
                await loadDataStatistics()
            }
            .sheet(isPresented: $showingDataManagement) {
                DataManagementView()
                    .environmentObject(appState)
            }
            .sheet(isPresented: $showingAbout) {
                AboutView()
            }
        }
    }
    
    private func loadDataStatistics() async {
        if let stats = await appState.getDataStatistics() {
            await MainActor.run {
                appState.dataStatistics = stats
            }
        }
    }
}

struct DataManagementView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    @State private var showingClearConfirmation = false
    
    var body: some View {
        NavigationView {
            Form {
                Section("Clear Data by Type") {
                    Button(role: .destructive, action: {
                        // Clear identities
                    }) {
                        Label("Clear All Identities", systemImage: "person.crop.circle.badge.xmark")
                    }
                    
                    Button(role: .destructive, action: {
                        // Clear documents
                    }) {
                        Label("Clear All Documents", systemImage: "doc.badge.xmark")
                    }
                    
                    Button(role: .destructive, action: {
                        // Clear contracts
                    }) {
                        Label("Clear All Contracts", systemImage: "doc.plaintext.badge.xmark")
                    }
                }
                
                Section("Clear All Data") {
                    Button(role: .destructive, action: {
                        showingClearConfirmation = true
                    }) {
                        Label("Clear All Data", systemImage: "trash")
                            .foregroundColor(.red)
                    }
                }
                
                Section {
                    Text("Warning: Clearing data will remove all locally stored information for the current network. This action cannot be undone.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .navigationTitle("Manage Data")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
            .alert("Clear All Data?", isPresented: $showingClearConfirmation) {
                Button("Cancel", role: .cancel) { }
                Button("Clear", role: .destructive) {
                    // Implement clear all data
                }
            } message: {
                Text("This will permanently delete all data for the \(appState.currentNetwork.displayName) network. This action cannot be undone.")
            }
        }
    }
}

struct AboutView: View {
    @Environment(\.dismiss) var dismiss
    
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 20) {
                    Image(systemName: "app.fill")
                        .font(.system(size: 80))
                        .foregroundColor(.blue)
                    
                    Text("Dash SDK Example")
                        .font(.title)
                        .fontWeight(.bold)
                    
                    Text("A demonstration app showcasing the capabilities of the Dash Platform SDK for iOS.")
                        .multilineTextAlignment(.center)
                        .padding(.horizontal)
                    
                    VStack(alignment: .leading, spacing: 16) {
                        FeatureRow(
                            icon: "person.3.fill",
                            title: "Identity Management",
                            description: "Create and manage Dash Platform identities"
                        )
                        
                        FeatureRow(
                            icon: "doc.text.fill",
                            title: "Document Storage",
                            description: "Store and retrieve documents on the platform"
                        )
                        
                        FeatureRow(
                            icon: "dollarsign.circle.fill",
                            title: "Token Support",
                            description: "Manage tokens and token balances"
                        )
                        
                        FeatureRow(
                            icon: "network",
                            title: "Multi-Network",
                            description: "Switch between mainnet, testnet, and devnet"
                        )
                    }
                    .padding()
                    
                    Link("Learn More", destination: URL(string: "https://www.dash.org/platform/")!)
                        .buttonStyle(.borderedProminent)
                }
                .padding()
            }
            .navigationTitle("About")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
        }
    }
}

struct FeatureRow: View {
    let icon: String
    let title: String
    let description: String
    
    var body: some View {
        HStack(alignment: .top, spacing: 16) {
            Image(systemName: icon)
                .font(.title2)
                .foregroundColor(.blue)
                .frame(width: 40)
            
            VStack(alignment: .leading, spacing: 4) {
                Text(title)
                    .font(.headline)
                Text(description)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
        }
    }
}

