import SwiftUI

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
            
            Text(identity.id)
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
            if let fetchedIdentity = try sdk.identities.get(id: identity.id),
               let balance = fetchedIdentity.balance() {
                appState.updateIdentityBalance(id: identity.id, newBalance: balance)
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
        let identity = IdentityModel(
            id: identityId,
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
                            Text("ID: \(fetchedIdentity.id)")
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