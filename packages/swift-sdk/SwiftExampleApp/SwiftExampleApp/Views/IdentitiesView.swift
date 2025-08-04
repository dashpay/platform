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
        
        // Get all non-local identities
        let nonLocalIdentities = appState.identities.filter { !$0.isLocal }
        
        guard !nonLocalIdentities.isEmpty else { return }
        
        // Fetch each identity's balance and DPNS name
        await withTaskGroup(of: Void.self) { group in
            for identity in nonLocalIdentities {
                group.addTask {
                    do {
                        // Fetch identity data
                        let fetchedIdentity = try await sdk.identityGet(identityId: identity.idString)
                        
                        // Update balance
                        if let balanceValue = fetchedIdentity["balance"] {
                            var newBalance: UInt64 = 0
                            if let balanceNum = balanceValue as? NSNumber {
                                newBalance = balanceNum.uint64Value
                            } else if let balanceString = balanceValue as? String,
                                      let balanceUInt = UInt64(balanceString) {
                                newBalance = balanceUInt
                            }
                            
                            await MainActor.run {
                                appState.updateIdentityBalance(id: identity.id, newBalance: newBalance)
                            }
                        }
                        
                        // Also try to fetch DPNS name if we don't have one
                        if identity.dpnsName == nil {
                            do {
                                let usernames = try await sdk.dpnsGetUsername(
                                    identityId: identity.idString,
                                    limit: 1
                                )
                                
                                if let firstUsername = usernames.first,
                                   let label = firstUsername["label"] as? String {
                                    await MainActor.run {
                                        appState.updateIdentityDPNSName(id: identity.id, dpnsName: label)
                                    }
                                }
                            } catch {
                                // Silently fail - not all identities have DPNS names
                            }
                        }
                    } catch {
                        // Log error but continue with other identities
                        print("Failed to refresh identity \(identity.idString): \(error)")
                    }
                }
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
        NavigationLink(destination: IdentityDetailView(identityId: identity.id)) {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(identity.alias ?? identity.dpnsName ?? "Identity")
                            .font(.headline)
                        
                        if let dpnsName = identity.dpnsName, identity.alias != nil {
                            Text(dpnsName)
                                .font(.caption)
                                .foregroundColor(.blue)
                        }
                    }
                    
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
                
                Text(identity.idHexString)
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
    }
    
    private func refreshBalance() async {
        guard let sdk = appState.sdk else { return }
        
        do {
            // Fetch identity data
            let fetchedIdentity = try await sdk.identityGet(identityId: identity.idString)
            
            // Update balance
            if let balanceValue = fetchedIdentity["balance"] {
                if let balanceNum = balanceValue as? NSNumber {
                    appState.updateIdentityBalance(id: identity.id, newBalance: balanceNum.uint64Value)
                } else if let balanceString = balanceValue as? String,
                          let balanceUInt = UInt64(balanceString) {
                    appState.updateIdentityBalance(id: identity.id, newBalance: balanceUInt)
                }
            }
            
            // Also try to fetch DPNS name if we don't have one
            if identity.dpnsName == nil {
                do {
                    let usernames = try await sdk.dpnsGetUsername(
                        identityId: identity.idString,
                        limit: 1
                    )
                    
                    if let firstUsername = usernames.first,
                       let label = firstUsername["label"] as? String {
                        appState.updateIdentityDPNSName(id: identity.id, dpnsName: label)
                    }
                } catch {
                    // Silently fail - not all identities have DPNS names
                }
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
                            Text("ID: \(fetchedIdentity.idHexString)")
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
            
            // Validate identity ID
            let trimmedId = identityId.trimmingCharacters(in: .whitespacesAndNewlines)
            var idData: Data?
            
            // Try hex first, then Base58
            if let hexData = Data(hexString: trimmedId), hexData.count == 32 {
                idData = hexData
            } else if let base58Data = Data.identifier(fromBase58: trimmedId), base58Data.count == 32 {
                idData = base58Data
            }
            
            guard let validIdData = idData else {
                appState.showError(message: "Invalid identity ID format")
                isLoading = false
                return
            }
            
            // Fetch identity from network
            let identityData = try await sdk.identityGet(identityId: validIdData.toHexString())
            
            // Extract balance
            var balance: UInt64 = 0
            if let balanceValue = identityData["balance"] {
                if let balanceNum = balanceValue as? NSNumber {
                    balance = balanceNum.uint64Value
                } else if let balanceString = balanceValue as? String,
                          let balanceUInt = UInt64(balanceString) {
                    balance = balanceUInt
                }
            }
            
            // Create identity model
            let model = IdentityModel(
                id: validIdData,
                balance: balance,
                isLocal: false
            )
            
            fetchedIdentity = model
            appState.addIdentity(model)
            
            // Also try to fetch DPNS name
            Task {
                do {
                    let usernames = try await sdk.dpnsGetUsername(
                        identityId: validIdData.toHexString(),
                        limit: 1
                    )
                    
                    if let firstUsername = usernames.first,
                       let label = firstUsername["label"] as? String {
                        appState.updateIdentityDPNSName(id: validIdData, dpnsName: label)
                    }
                } catch {
                    // Silently fail - not all identities have DPNS names
                }
            }
            
            isLoading = false
        } catch {
            appState.showError(message: "Failed to fetch identity: \(error.localizedDescription)")
            isLoading = false
        }
    }
}
