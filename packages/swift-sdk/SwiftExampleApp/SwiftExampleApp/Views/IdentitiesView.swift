import SwiftUI
import SwiftDashSDK

struct IdentitiesView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingAddIdentity = false
    @State private var showingFetchIdentity = false
    @State private var showingLoadIdentity = false
    
    var body: some View {
        NavigationView {
            List {
                Section("Local Identities") {
                    ForEach(appState.identities.filter { $0.isLocal }) { identity in
                        IdentityRow(identity: identity)
                    }
                    .onDelete { indexSet in
                        deleteLocalIdentities(at: indexSet)
                    }
                }
                
                Section("Fetched Identities") {
                    ForEach(appState.identities.filter { !$0.isLocal }) { identity in
                        IdentityRow(identity: identity)
                    }
                }
            }
            .navigationTitle("Identities")
            .refreshable {
                await refreshAllBalances()
            }
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Menu {
                        Button(action: { showingLoadIdentity = true }) {
                            Label("Load Identity", systemImage: "square.and.arrow.down")
                        }
                        Divider()
                        Button(action: { showingAddIdentity = true }) {
                            Label("Add Local Identity", systemImage: "plus")
                        }
                        Button(action: { showingFetchIdentity = true }) {
                            Label("Fetch Identity", systemImage: "arrow.down.circle")
                        }
                    } label: {
                        Image(systemName: "plus")
                    }
                }
            }
            .sheet(isPresented: $showingAddIdentity) {
                AddIdentityView()
                    .environmentObject(appState)
            }
            .sheet(isPresented: $showingFetchIdentity) {
                FetchIdentityView()
                    .environmentObject(appState)
            }
            .sheet(isPresented: $showingLoadIdentity) {
                LoadIdentityView()
                    .environmentObject(appState)
            }
        }
    }
    
    private func refreshAllBalances() async {
        guard let sdk = appState.sdk else { return }
        
        // Get all non-local identity IDs as Data
        let identityIds = appState.identities
//            .filter { !$0.isLocal }
            .map { $0.id }
        
        guard !identityIds.isEmpty else { return }
        
        do {
            // Fetch all balances in a single request
            let balances = try sdk.identities.fetchBalances(ids: identityIds)
            
            // Update each identity's balance
            await MainActor.run {
                for (id, balance) in balances {
                    if let balance = balance {
                        appState.updateIdentityBalance(id: id, newBalance: balance)
                    }
                }
            }
        } catch {
            await MainActor.run {
                var errorMessage = "Failed to refresh balances: "
                
                // Check if it's an SDKError
                if let sdkError = error as? SDKError {
                    switch sdkError {
                    case .invalidParameter(let detail):
                        errorMessage += "Invalid parameter - \(detail)"
                    case .invalidState(let detail):
                        errorMessage += "Invalid state - \(detail)"
                    case .networkError(let detail):
                        errorMessage += "Network error - \(detail)"
                    case .serializationError(let detail):
                        errorMessage += "Data serialization error - \(detail)"
                    case .protocolError(let detail):
                        errorMessage += "Protocol error - \(detail)"
                    case .cryptoError(let detail):
                        errorMessage += "Cryptographic error - \(detail)"
                    case .notFound(let detail):
                        errorMessage += "Not found - \(detail)"
                    case .timeout(let detail):
                        errorMessage += "Request timed out - \(detail)"
                    case .notImplemented(let detail):
                        errorMessage += "Feature not implemented - \(detail)"
                    case .internalError(let detail):
                        errorMessage += "Internal error - \(detail)"
                    case .unknown(let detail):
                        errorMessage += detail
                    }
                } else {
                    // For other errors, try to get more details
                    let nsError = error as NSError
                    if nsError.domain.isEmpty {
                        errorMessage += error.localizedDescription
                    } else {
                        errorMessage += "\(nsError.domain) - Code: \(nsError.code)"
                        if let reason = nsError.localizedFailureReason {
                            errorMessage += " - \(reason)"
                        }
                        if let suggestion = nsError.localizedRecoverySuggestion {
                            errorMessage += "\n\(suggestion)"
                        }
                    }
                }
                
                appState.showError(message: errorMessage)
            }
        }
    }
    
    private func deleteLocalIdentities(at offsets: IndexSet) {
        let localIdentities = appState.identities.filter { $0.isLocal }
        for index in offsets {
            if index < localIdentities.count {
                appState.removeIdentity(localIdentities[index])
            }
        }
    }
}

struct IdentityRow: View {
    let identity: IdentityModel
    @EnvironmentObject var appState: AppState
    @State private var isRefreshing = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(identity.alias ?? "Identity")
                    .font(.headline)
                Spacer()
                
                if identity.type != .user {
                    Text(identity.type.rawValue)
                        .font(.caption)
                        .foregroundColor(.white)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 2)
                        .background(identity.type == .masternode ? Color.purple : Color.orange)
                        .cornerRadius(4)
                }
                
                if identity.isLocal {
                    Text("Local")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 2)
                        .background(Color.gray.opacity(0.2))
                        .cornerRadius(4)
                }
            }
            
            Text(identity.idString)
                .font(.caption)
                .foregroundColor(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
            
            HStack {
                Text(identity.formattedBalance)
                    .font(.subheadline)
                    .foregroundColor(.blue)
                
                Spacer()
                
                if !identity.isLocal {
                    Button(action: {
                        Task {
                            isRefreshing = true
                            await refreshBalance()
                            isRefreshing = false
                        }
                    }) {
                        Image(systemName: "arrow.clockwise")
                            .font(.caption)
                            .foregroundColor(.blue)
                            .rotationEffect(.degrees(isRefreshing ? 360 : 0))
                            .animation(isRefreshing ? .linear(duration: 1).repeatForever(autoreverses: false) : .default, value: isRefreshing)
                    }
                    .buttonStyle(BorderlessButtonStyle())
                }
            }
        }
        .padding(.vertical, 4)
    }
    
    private func refreshBalance() async {
        guard let sdk = appState.sdk else { return }
        
        do {
            if let fetchedIdentity = try sdk.identities.get(id: identity.idString) {
                appState.updateIdentityBalance(id: identity.id, newBalance: fetchedIdentity.balance)
            }
        } catch {
            // Silently fail for local identities
            if !identity.isLocal {
                appState.showError(message: "Failed to refresh balance: \(error.localizedDescription)")
            }
        }
    }
}

struct AddIdentityView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    @State private var identityId = ""
    @State private var alias = ""
    
    var body: some View {
        NavigationView {
            Form {
                Section("Identity Details") {
                    TextField("Identity ID", text: $identityId)
                        .textContentType(.none)
                        .autocapitalization(.none)
                    
                    TextField("Alias (Optional)", text: $alias)
                        .textContentType(.name)
                }
                
                Section {
                    Text("Local identities are stored only in this app and can be used for testing token transfers.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .navigationTitle("Add Local Identity")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Add") {
                        addLocalIdentity()
                        dismiss()
                    }
                    .disabled(identityId.isEmpty)
                }
            }
        }
    }
    
    private func addLocalIdentity() {
        guard let idData = Data(hexString: identityId), idData.count == 32 else {
            appState.showError(message: "Invalid identity ID. Must be a 64-character hex string.")
            return
        }
        
        let identity = IdentityModel(
            id: idData,
            balance: 0,
            isLocal: true,
            alias: alias.isEmpty ? nil : alias
        )
        
        appState.addIdentity(identity)
    }
}

struct FetchIdentityView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    @State private var identityId = ""
    @State private var isLoading = false
    @State private var fetchedIdentity: IdentityModel?
    
    var body: some View {
        NavigationView {
            Form {
                Section("Fetch Identity from Network") {
                    TextField("Identity ID", text: $identityId)
                        .textContentType(.none)
                        .autocapitalization(.none)
                }
                
                if isLoading {
                    Section {
                        HStack {
                            ProgressView()
                            Text("Fetching identity...")
                                .foregroundColor(.secondary)
                        }
                    }
                }
                
                if let fetchedIdentity = fetchedIdentity {
                    Section("Fetched Identity") {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("ID: \(fetchedIdentity.idString)")
                                .font(.caption)
                            Text("Balance: \(fetchedIdentity.formattedBalance)")
                                .font(.subheadline)
                        }
                    }
                }
            }
            .navigationTitle("Fetch Identity")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Fetch") {
                        Task {
                            await fetchIdentity()
                        }
                    }
                    .disabled(identityId.isEmpty || isLoading)
                }
            }
        }
    }
    
    private func fetchIdentity() async {
        guard let sdk = appState.sdk else {
            appState.showError(message: "SDK not initialized")
            return
        }
        
        do {
            isLoading = true
            if let identity = try sdk.identities.get(id: identityId) {
                if let model = IdentityModel(from: identity) {
                    fetchedIdentity = model
                    appState.addIdentity(model)
                }
            } else {
                appState.showError(message: "Identity not found")
            }
            isLoading = false
        } catch {
            appState.showError(message: "Failed to fetch identity: \(error.localizedDescription)")
            isLoading = false
        }
    }
}
