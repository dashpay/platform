import SwiftUI

struct PlatformView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var selectedOperation: PlatformOperation = .queries
    
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
                
                Section(header: Text("SDK Status")) {
                    HStack {
                        Text("SDK Initialized")
                        Spacer()
                        Image(systemName: appState.platformState.sdk != nil ? "checkmark.circle.fill" : "xmark.circle.fill")
                            .foregroundColor(appState.platformState.sdk != nil ? .green : .red)
                    }
                    
                    HStack {
                        Text("Network")
                        Spacer()
                        Text("Testnet")
                            .foregroundColor(.secondary)
                    }
                }
            }
            .navigationTitle("Platform")
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