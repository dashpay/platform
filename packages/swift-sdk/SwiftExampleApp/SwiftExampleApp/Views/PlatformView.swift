import SwiftUI
import SwiftDashSDK

struct PlatformView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var selectedOperation: PlatformOperation = .queries
    @State private var sdkStatus: SDKStatus?
    @State private var isLoadingStatus = false
    
    enum PlatformOperation: String, CaseIterable {
        case queries = "Queries"
        case stateTransitions = "State Transitions"
        
        var systemImage: String {
            switch self {
            case .queries: return "magnifyingglass"
            case .stateTransitions: return "arrow.up.arrow.down"
            }
        }
    }
    
    var body: some View {
        NavigationStack {
            List {
                Section(header: Text("Platform Operations")) {
                    ForEach(PlatformOperation.allCases, id: \.self) { operation in
                        NavigationLink(destination: destinationView(for: operation)) {
                            HStack {
                                Image(systemName: operation.systemImage)
                                    .frame(width: 30)
                                    .foregroundColor(.blue)
                                Text(operation.rawValue)
                                    .font(.headline)
                            }
                        }
                    }
                }
                
                Section(header: HStack {
                    Text("SDK Status")
                    Spacer()
                    if isLoadingStatus {
                        ProgressView()
                            .scaleEffect(0.8)
                    } else {
                        Button(action: loadSDKStatus) {
                            Image(systemName: "arrow.clockwise")
                                .font(.caption)
                        }
                    }
                }) {
                    HStack {
                        Text("SDK Initialized")
                        Spacer()
                        Image(systemName: appState.platformState.sdk != nil ? "checkmark.circle.fill" : "xmark.circle.fill")
                            .foregroundColor(appState.platformState.sdk != nil ? .green : .red)
                    }
                    
                    if let status = sdkStatus {
                        HStack {
                            Text("Version")
                            Spacer()
                            Text(status.version)
                                .foregroundColor(.secondary)
                        }
                        
                        HStack {
                            Text("Network")
                            Spacer()
                            Text(status.network.capitalized)
                                .foregroundColor(.secondary)
                        }
                        
                        HStack {
                            Text("Mode")
                            Spacer()
                            Text(status.mode.uppercased())
                                .foregroundColor(status.mode == "trusted" ? .blue : .orange)
                        }
                        
                        HStack {
                            Text("Quorums in Memory")
                            Spacer()
                            Text("\(status.quorumCount)")
                                .foregroundColor(status.quorumCount > 0 ? .green : .red)
                        }
                    } else {
                        HStack {
                            Text("Network")
                            Spacer()
                            Text("Testnet")
                                .foregroundColor(.secondary)
                        }
                    }
                }
            }
            .navigationTitle("Platform")
            .onAppear {
                loadSDKStatus()
            }
        }
    }
    
    private func loadSDKStatus() {
        guard let sdk = appState.platformState.sdk else { return }
        
        isLoadingStatus = true
        
        Task {
            do {
                let status = try sdk.getStatus()
                await MainActor.run {
                    self.sdkStatus = status
                    self.isLoadingStatus = false
                }
            } catch {
                print("Failed to get SDK status: \(error)")
                await MainActor.run {
                    self.isLoadingStatus = false
                }
            }
        }
    }
    
    @ViewBuilder
    private func destinationView(for operation: PlatformOperation) -> some View {
        switch operation {
        case .queries:
            PlatformQueriesView()
        case .stateTransitions:
            PlatformStateTransitionsView()
        }
    }
}

struct PlatformView_Previews: PreviewProvider {
    static var previews: some View {
        PlatformView()
            .environmentObject(UnifiedAppState())
    }
}