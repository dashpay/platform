import SwiftUI
import SwiftDashSDK

struct IdentitiesView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingLoadIdentity = false
    
    var body: some View {
        NavigationView {
            List {
                ForEach(appState.identities) { identity in
                    IdentityRow(identity: identity)
                }
                .onDelete { indexSet in
                    deleteIdentities(at: indexSet)
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
    
    private func deleteIdentities(at offsets: IndexSet) {
        for index in offsets {
            if index < appState.identities.count {
                appState.removeIdentity(appState.identities[index])
            }
        }
    }
}

struct IdentityRow: View {
    let identity: IdentityModel
    @EnvironmentObject var appState: AppState
    @State private var isRefreshing = false
    @State private var currentIdentity: IdentityModel?
    
    private func formatBalanceShort(_ balance: UInt64) -> String {
        let dashAmount = Double(balance) / 100_000_000_000 // 1 DASH = 100B credits
        return String(format: "%.2f DASH", dashAmount)
    }
    
    var body: some View {
        // Use currentIdentity if available, otherwise use the passed identity
        let displayIdentity = currentIdentity ?? identity
        
        return NavigationLink(destination: IdentityDetailView(identityId: identity.id)) {
            VStack(alignment: .leading, spacing: 4) {
                HStack(alignment: .top) {
                    VStack(alignment: .leading, spacing: 4) {
                        // Show DPNS name if available, otherwise alias or "Identity"
                        if let dpnsName = displayIdentity.dpnsName {
                            Text(dpnsName)
                                .font(.headline)
                                .foregroundColor(.blue)
                            
                            if let alias = displayIdentity.alias {
                                Text(alias)
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        } else {
                            Text(displayIdentity.alias ?? "Identity")
                                .font(.headline)
                        }
                    }
                    
                    Spacer()
                    
                    Text(formatBalanceShort(displayIdentity.balance))
                        .font(.headline)
                        .foregroundColor(.primary)
                }
                
                Text(displayIdentity.idString)
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
        .onAppear {
            // Update currentIdentity from appState when the view appears
            if let updatedIdentity = appState.identities.first(where: { $0.id == identity.id }) {
                currentIdentity = updatedIdentity
            }
        }
        .onReceive(appState.$identities) { updatedIdentities in
            // Update currentIdentity when identities array changes
            if let updatedIdentity = updatedIdentities.first(where: { $0.id == identity.id }) {
                currentIdentity = updatedIdentity
            }
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