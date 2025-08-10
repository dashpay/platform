import SwiftUI
import SwiftDashSDK
import SwiftDashSDK

struct IdentityDetailView: View {
    let identityId: Data
    @EnvironmentObject var appState: AppState
    
    private var identity: IdentityModel? {
        appState.identities.first { $0.id == identityId }
    }
    @State private var isRefreshing = false
    @State private var showingEditAlias = false
    @State private var newAlias = ""
    @State private var isLoadingDPNS = false
    @State private var showingRegisterName = false
    
    // Computed properties that get DPNS names from the identity model
    private var dpnsNames: [String] {
        identity?.dpnsNames ?? []
    }
    
    private var contestedDpnsNames: [String] {
        identity?.contestedDpnsNames ?? []
    }
    
    private var contestedDpnsInfo: [String: Any] {
        identity?.contestedDpnsInfo ?? [:]
    }
    
    var body: some View {
        if let identity = identity {
            List {
                // Basic Info Section
                Section("Identity Information") {
                    VStack(alignment: .leading, spacing: 8) {
                        if let alias = identity.alias {
                            Label(alias, systemImage: "person.text.rectangle")
                                .font(.headline)
                        }
                    
                    if let dpnsName = identity.dpnsName {
                        Label(dpnsName, systemImage: "at")
                            .font(.subheadline)
                            .foregroundColor(.blue)
                    }
                    
                    Label(identity.idHexString, systemImage: "number")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(.vertical, 4)
                
                HStack {
                    Label("Balance", systemImage: "dollarsign.circle")
                    Spacer()
                    Text(identity.formattedBalance)
                        .foregroundColor(.blue)
                        .fontWeight(.medium)
                }
                
                HStack {
                    Label("Type", systemImage: "person.badge.shield.checkmark")
                    Spacer()
                    Text(identity.type.rawValue)
                        .foregroundColor(identity.type == .user ? .primary : 
                                      identity.type == .masternode ? .purple : .orange)
                }
                
                if identity.isLocal {
                    HStack {
                        Label("Status", systemImage: "location")
                        Spacer()
                        Text("Local Only")
                            .foregroundColor(.secondary)
                    }
                }
            }
            
            // DPNS Names Section
            if !dpnsNames.isEmpty || !contestedDpnsNames.isEmpty || !identity.isLocal {
                Section("DPNS Names") {
                    if isLoadingDPNS {
                        HStack {
                            ProgressView()
                            Text("Loading DPNS names...")
                                .foregroundColor(.secondary)
                        }
                    } else if dpnsNames.isEmpty && contestedDpnsNames.isEmpty {
                        Text("No DPNS names found")
                            .foregroundColor(.secondary)
                    } else {
                        // Show registered names
                        ForEach(dpnsNames, id: \.self) { name in
                            HStack {
                                Text(name)
                                Spacer()
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundColor(.green)
                            }
                        }
                        
                        // Show contested names
                        ForEach(contestedDpnsNames, id: \.self) { name in
                            NavigationLink(destination: ContestDetailView(
                                contestName: name,
                                contestInfo: contestedDpnsInfo[name] as? [String: Any] ?? [:],
                                currentIdentityId: identity.idString
                            ).environmentObject(appState)) {
                                HStack {
                                    Text(name)
                                    Spacer()
                                    Label("Contested", systemImage: "flag.fill")
                                        .font(.caption)
                                        .foregroundColor(.orange)
                                }
                            }
                        }
                    }
                    
                    // Register name button
                    if !identity.isLocal {
                        Button(action: { showingRegisterName = true }) {
                            HStack {
                                Image(systemName: "plus.circle")
                                Text(dpnsNames.isEmpty ? "Register a name" : "Register another name")
                            }
                            .foregroundColor(.blue)
                        }
                    }
                }
            }
            
            // Keys Section
            Section("Keys") {
                NavigationLink(destination: KeysListView(identity: identity)) {
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Image(systemName: "key.fill")
                            Text("Identity Keys")
                                .fontWeight(.medium)
                        }
                        
                        HStack(spacing: 16) {
                            Label("\(identity.publicKeys.count) public", systemImage: "key")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            
                            if !identity.privateKeys.isEmpty {
                                Label("\(identity.privateKeys.count) private", systemImage: "key.fill")
                                    .font(.caption)
                                    .foregroundColor(.green)
                            }
                        }
                    }
                    .padding(.vertical, 4)
                }
            }
            
            // Actions Section
            if !identity.isLocal {
                Section {
                    Button(action: refreshIdentityData) {
                        HStack {
                            Image(systemName: "arrow.clockwise")
                            Text("Refresh Identity Data")
                            Spacer()
                            if isRefreshing {
                                ProgressView()
                            }
                        }
                    }
                    .disabled(isRefreshing)
                }
            }
        }
        .navigationTitle("Identity Details")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            if identity.alias == nil {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Add Alias") {
                        newAlias = ""
                        showingEditAlias = true
                    }
                }
            }
        }
        .sheet(isPresented: $showingEditAlias) {
            EditAliasView(identity: identity, newAlias: $newAlias)
        }
        .sheet(isPresented: $showingRegisterName) {
            RegisterNameView(identity: identity)
                .environmentObject(appState)
        }
        .onAppear {
            print("🔵 IdentityDetailView onAppear - dpnsName: \(identity.dpnsName ?? "nil"), isLocal: \(identity.isLocal)")
            
            // Load DPNS names from network if we don't have any cached or if they're empty
            if (dpnsNames.isEmpty && contestedDpnsNames.isEmpty) && !identity.isLocal {
                print("🔵 No cached DPNS names, loading from network...")
                loadDPNSNames()
            } else if !dpnsNames.isEmpty || !contestedDpnsNames.isEmpty {
                print("🔵 Using cached DPNS names: \(dpnsNames.count) regular, \(contestedDpnsNames.count) contested")
            }
        }
        } else {
            Text("Identity not found")
                .foregroundColor(.secondary)
                .navigationTitle("Identity Details")
                .navigationBarTitleDisplayMode(.inline)
        }
    }
    
    private func refreshIdentityData() {
        Task {
            isRefreshing = true
            defer { isRefreshing = false }
            
            guard let sdk = appState.sdk,
                  let identity = identity else { return }
            
            do {
                // Refresh identity data
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
                
                // Parse and update public keys
                var parsedPublicKeys: [IdentityPublicKey] = []
                print("🔵 Checking for public keys in fetched identity...")
                if let publicKeysArray = fetchedIdentity["publicKeys"] as? [[String: Any]] {
                    print("🔵 Found \(publicKeysArray.count) public keys")
                    parsedPublicKeys = publicKeysArray.compactMap { keyData -> IdentityPublicKey? in
                        print("🔵 Parsing key data: \(keyData)")
                        guard let id = keyData["id"] as? Int,
                              let purpose = keyData["purpose"] as? Int,
                              let securityLevel = keyData["securityLevel"] as? Int,
                              let keyType = keyData["type"] as? Int,
                              let dataStr = keyData["data"] as? String,
                              let data = Data(base64Encoded: dataStr) else {
                            return nil
                        }
                        
                        let readOnly = keyData["readOnly"] as? Bool ?? false
                        let disabledAt = keyData["disabledAt"] as? UInt64
                        
                        return IdentityPublicKey(
                            id: UInt32(id),
                            purpose: KeyPurpose(rawValue: UInt8(purpose)) ?? .authentication,
                            securityLevel: SecurityLevel(rawValue: UInt8(securityLevel)) ?? .high,
                            contractBounds: nil,
                            keyType: KeyType(rawValue: UInt8(keyType)) ?? .ecdsaSecp256k1,
                            readOnly: readOnly,
                            data: data,
                            disabledAt: disabledAt
                        )
                    }
                } else {
                    print("❌ No public keys found in fetched identity")
                }
                
                print("🔵 Parsed \(parsedPublicKeys.count) public keys total")
                
                // Update the identity with public keys
                appState.updateIdentityPublicKeys(id: identity.id, publicKeys: parsedPublicKeys)
                print("🔵 Called updateIdentityPublicKeys")
                
                // Refresh DPNS names from network
                await loadDPNSNamesFromNetwork()
            } catch {
                await MainActor.run {
                    appState.showError(message: "Failed to refresh identity: \(error.localizedDescription)")
                }
            }
        }
    }
    
    private func loadDPNSNames() {
        guard let identity = identity,
              !identity.isLocal else { return }
        
        Task {
            await loadDPNSNamesFromNetwork()
        }
    }
    
    private func loadDPNSNamesFromNetwork() async {
        guard let identity = identity,
              !identity.isLocal else { return }
        
        print("🔵 loadDPNSNamesFromNetwork called for identity \(identity.idString)")
        
        isLoadingDPNS = true
        defer { isLoadingDPNS = false }
        
        guard let sdk = appState.sdk else { return }
        
        // Fetch both regular and contested names in parallel
        async let regularNamesTask = fetchRegularDPNSNames(identity: identity)
        async let contestedNamesTask = fetchContestedDPNSNames(identity: identity)
        
        let (regular, contested) = await (regularNamesTask, contestedNamesTask)
        
        await MainActor.run {
            let regularNames = regular.0
            let contestedNames = contested.0
            let contestedInfo = contested.1
            
            // Update all DPNS names in the identity model
            appState.updateIdentityDPNSNames(
                id: identity.id, 
                dpnsNames: regularNames, 
                contestedNames: contestedNames, 
                contestedInfo: contestedInfo
            )
            
            print("🔵 Updated identity with \(regularNames.count) regular names and \(contestedNames.count) contested names")
        }
    }
    
    private func fetchRegularDPNSNames(identity: IdentityModel) async -> ([String], [String: Any]) {
        guard let sdk = appState.sdk else { return ([], [:]) }
        
        do {
            print("🔵 Fetching regular DPNS names from network...")
            let usernames = try await sdk.dpnsGetUsername(
                identityId: identity.idString,
                limit: 10
            )
            
            print("🔵 Got \(usernames.count) regular DPNS names from network")
            return (usernames.compactMap { $0["label"] as? String }, [:])
        } catch {
            print("❌ No regular DPNS names found for identity: \(error)")
            return ([], [:])
        }
    }
    
    private func fetchContestedDPNSNames(identity: IdentityModel) async -> ([String], [String: Any]) {
        guard let sdk = appState.sdk else { return ([], [:]) }
        
        do {
            print("🔵 Fetching contested DPNS names from network...")
            
            // Use the new dedicated FFI function for getting non-resolved contests for this identity
            let contestsResult = try await sdk.dpnsGetNonResolvedContestsForIdentity(
                identityId: identity.idString,
                limit: 20
            )
            
            var contestedNames: [String] = []
            var contestInfo: [String: Any] = [:]
            
            // Parse the result - it's a dictionary where keys are the contested names
            for (name, info) in contestsResult {
                contestedNames.append(name)
                contestInfo[name] = info
            }
            
            print("🔵 Found \(contestedNames.count) contested DPNS names")
            return (contestedNames, contestInfo)
        } catch {
            print("❌ Failed to fetch contested DPNS names: \(error)")
            return ([], [:])
        }
    }
}

struct EditAliasView: View {
    let identity: IdentityModel
    @Binding var newAlias: String
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    
    var body: some View {
        NavigationView {
            Form {
                Section("Set Alias") {
                    TextField("Enter alias", text: $newAlias)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                }
                
                Section {
                    Text("An alias helps you identify this identity in the app. It's stored locally and not saved to the network.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .navigationTitle("Add Alias")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Save") {
                        saveAlias()
                    }
                    .disabled(newAlias.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            }
        }
    }
    
    private func saveAlias() {
        let trimmedAlias = newAlias.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedAlias.isEmpty else { return }
        
        // Create updated identity with alias
        var updatedIdentity = identity
        updatedIdentity = IdentityModel(
            id: identity.id,
            balance: identity.balance,
            isLocal: identity.isLocal,
            alias: trimmedAlias,
            type: identity.type,
            privateKeys: identity.privateKeys,
            votingPrivateKey: identity.votingPrivateKey,
            ownerPrivateKey: identity.ownerPrivateKey,
            payoutPrivateKey: identity.payoutPrivateKey,
            dpnsName: identity.dpnsName,
            publicKeys: identity.publicKeys
        )
        
        // Update in app state
        appState.updateIdentity(updatedIdentity)
        
        dismiss()
    }
}