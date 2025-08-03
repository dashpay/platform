import SwiftUI
import SwiftDashSDK

struct IdentityDetailView: View {
    let identity: IdentityModel
    @EnvironmentObject var appState: AppState
    @State private var isRefreshing = false
    @State private var showingEditAlias = false
    @State private var newAlias = ""
    @State private var dpnsNames: [String] = []
    @State private var isLoadingDPNS = false
    
    var body: some View {
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
                    
                    Label(identity.idString, systemImage: "number")
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
            if !dpnsNames.isEmpty || !identity.isLocal {
                Section("DPNS Names") {
                    if isLoadingDPNS {
                        HStack {
                            ProgressView()
                            Text("Loading DPNS names...")
                                .foregroundColor(.secondary)
                        }
                    } else if dpnsNames.isEmpty {
                        Text("No DPNS names found")
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(dpnsNames, id: \.self) { name in
                            HStack {
                                Text(name)
                                Spacer()
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundColor(.green)
                            }
                        }
                    }
                }
            }
            
            // Public Keys Section
            if !identity.publicKeys.isEmpty {
                Section("Public Keys") {
                    ForEach(Array(identity.publicKeys.enumerated()), id: \.offset) { index, key in
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text("Key #\(key.id)")
                                    .font(.caption)
                                    .fontWeight(.medium)
                                Spacer()
                                Text("Purpose: \(key.purpose)")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                            }
                            
                            Text(key.data.toHexString())
                                .font(.caption2)
                                .lineLimit(1)
                                .truncationMode(.middle)
                                .foregroundColor(.secondary)
                        }
                        .padding(.vertical, 2)
                    }
                }
            }
            
            // Private Keys Section (if any)
            if !identity.privateKeys.isEmpty {
                Section("Private Keys") {
                    Text("\(identity.privateKeys.count) key(s) loaded")
                        .foregroundColor(.secondary)
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
        .onAppear {
            loadDPNSNames()
        }
    }
    
    private func refreshIdentityData() {
        Task {
            isRefreshing = true
            defer { isRefreshing = false }
            
            guard let sdk = appState.sdk else { return }
            
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
                
                // Refresh DPNS names
                loadDPNSNames()
            } catch {
                await MainActor.run {
                    appState.showError(message: "Failed to refresh identity: \(error.localizedDescription)")
                }
            }
        }
    }
    
    private func loadDPNSNames() {
        guard !identity.isLocal else { return }
        
        Task {
            isLoadingDPNS = true
            defer { isLoadingDPNS = false }
            
            guard let sdk = appState.sdk else { return }
            
            do {
                let usernames = try await sdk.dpnsGetUsername(
                    identityId: identity.idString,
                    limit: 10
                )
                
                await MainActor.run {
                    dpnsNames = usernames.compactMap { $0["label"] as? String }
                    
                    // Update the primary DPNS name if we found one
                    if let firstUsername = dpnsNames.first, identity.dpnsName == nil {
                        appState.updateIdentityDPNSName(id: identity.id, dpnsName: firstUsername)
                    }
                }
            } catch {
                // Silently fail - not all identities have DPNS names
                print("No DPNS names found for identity: \(error)")
            }
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
            dppIdentity: identity.dppIdentity,
            publicKeys: identity.publicKeys
        )
        
        // Update in app state
        appState.updateIdentity(updatedIdentity)
        
        dismiss()
    }
}