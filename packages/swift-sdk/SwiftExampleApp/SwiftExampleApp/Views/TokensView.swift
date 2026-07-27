import SwiftUI
import SwiftData
import SwiftDashSDK

struct TokensView: View {
    @EnvironmentObject var appState: AppState
    @Query private var identities: [PersistentIdentity]

    /// Picker selection — `PersistentIdentity` is a `@Model` class,
    /// so `Identifiable` + `Hashable` come for free.
    @State private var selectedIdentity: PersistentIdentity?

    var body: some View {
        NavigationStack {
            VStack {
                if identities.isEmpty {
                    EmptyStateView(
                        systemImage: "person.3",
                        title: "No Identities",
                        message: "Add identities in the Identities tab to use tokens"
                    )
                } else {
                    List {
                        Section("Select Identity") {
                            Picker("Identity", selection: $selectedIdentity) {
                                Text("Select an identity").tag(nil as PersistentIdentity?)
                                ForEach(identities) { identity in
                                    Text(identity.alias ?? identity.identityIdBase58)
                                        .tag(identity as PersistentIdentity?)
                                }
                            }
                            .pickerStyle(MenuPickerStyle())
                        }

                        if let identity = selectedIdentity {
                            tokenList(for: identity)
                        }
                    }
                }
            }
            .navigationTitle("Tokens")
        }
    }

    @ViewBuilder
    private func tokenList(for identity: PersistentIdentity) -> some View {
        Section("Token Balances") {
            if identity.tokenBalances.isEmpty {
                Text("No token balances yet")
                    .foregroundColor(.secondary)
                    .font(.subheadline)
            } else {
                ForEach(identity.tokenBalances) { balance in
                    if let token = balance.token {
                        NavigationLink {
                            TokenActionPermissionsView(token: token, identity: identity)
                        } label: {
                            TokenRow(balance: balance)
                        }
                    } else {
                        TokenRow(balance: balance)
                    }
                }
            }
        }
    }
}

struct TokenRow: View {
    let balance: PersistentTokenBalance

    private var headlineName: String {
        balance.token?.name ?? balance.tokenName ?? "Token"
    }

    private var trailingSymbol: String? {
        balance.tokenSymbol ?? balance.token?.name
    }

    private var totalSupplyLine: String? {
        // Prefer the cap; fall back to the base supply. Both are
        // stored as strings on PersistentToken (they're bignum-safe).
        guard let token = balance.token else { return nil }
        if let cap = token.maxSupply, !cap.isEmpty {
            return "Max Supply: \(cap)"
        }
        if !token.baseSupply.isEmpty {
            return "Base Supply: \(token.baseSupply)"
        }
        return nil
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(headlineName)
                    .font(.headline)
                    .foregroundColor(.primary)
                Spacer()
                if let symbol = trailingSymbol {
                    Text(symbol)
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }
            }

            HStack {
                Text("Balance: \(balance.displayBalance)")
                    .font(.subheadline)
                    .foregroundColor(.blue)

                if balance.frozen {
                    Text("(frozen)")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }

            if let supply = totalSupplyLine {
                Text(supply)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 4)
    }
}

struct EmptyStateView: View {
    let systemImage: String
    let title: String
    let message: String

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: systemImage)
                .font(.system(size: 60))
                .foregroundColor(.gray)

            Text(title)
                .font(.title2)
                .fontWeight(.semibold)

            Text(message)
                .font(.body)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
        }
        .padding()
    }
}
