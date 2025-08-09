import SwiftUI
import SwiftDashSDK

struct IdentitiesView: View {
    @EnvironmentObject var appState: AppState
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
                    Button(action: { showingLoadIdentity = true }) {
                        Image(systemName: "square.and.arrow.down")
                    }
                }
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
                    
                    VStack(alignment: .trailing, spacing: 2) {
                        Text(identity.formattedBalance)
                            .font(.headline)
                            .foregroundColor(.primary)
                        Text("Dash Credits")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
                
                Text(identity.idString)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                
                if identity.isLocal {
                    HStack {
                        Image(systemName: "location")
                            .font(.caption2)
                        Text("Local Only")
                            .font(.caption2)
                    }
                    .foregroundColor(.orange)
                } else {
                    HStack {
                        Image(systemName: "checkmark.circle.fill")
                            .font(.caption2)
                        Text("On Network")
                            .font(.caption2)
                        
                        Spacer()
                        
                        Button(action: {
                            isRefreshing = true
                            Task {
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