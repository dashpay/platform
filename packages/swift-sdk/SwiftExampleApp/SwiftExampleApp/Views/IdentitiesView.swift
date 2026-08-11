import SwiftUI
import SwiftData
import SwiftDashSDK

// The `IdentitiesView` that used to live here was a legacy
// duplicate of `IdentitiesContentView`. Nothing mounts it; the
// Identities tab renders `IdentitiesContentView` directly.
// Only `IdentityRow` stays — it's the row cell used by
// `IdentitiesContentView`.

/// One row in an identities list. Navigates to `IdentityDetailView`
/// on tap. Takes a live `PersistentIdentity` so balance / DPNS name
/// edits propagate reactively via `@Query` upstream without any
/// per-row observation ceremony.
struct IdentityRow: View {
    let identity: PersistentIdentity
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext
    @State private var isRefreshing = false

    private func formatBalanceShort(_ balance: UInt64) -> String {
        let dashAmount = Double(balance) / 100_000_000_000 // 1 DASH = 100B credits
        return String(format: "%.2f DASH", dashAmount)
    }

    private var hasAnyPrivateKey: Bool {
        // A stored *PrivateKeyIdentifier only proves an identifier string
        // was persisted on the row — the backing Keychain item can still
        // be missing (wiped, never written, restored on another device).
        // Rely solely on concrete key-presence checks against the
        // Keychain so the "No Keys" badge reflects what's actually there.
        let km = KeychainManager.shared
        for publicKey in identity.identityPublicKeys {
            if km.hasPrivateKey(identityId: identity.identityId, keyIndex: Int32(publicKey.id)) {
                return true
            }
            if km.hasIdentityPrivateKey(publicKeyHex: publicKey.data.toHexString()) {
                return true
            }
        }
        return false
    }

    var body: some View {
        NavigationLink(destination: IdentityDetailView(identityId: identity.identityId)) {
            VStack(alignment: .leading, spacing: 4) {
                HStack(alignment: .top) {
                    VStack(alignment: .leading, spacing: 4) {
                        // Show display name with star if main name is selected
                        HStack(spacing: 4) {
                            Text(identity.displayName)
                                .font(.headline)
                                .foregroundColor(identity.mainDpnsName != nil || identity.dpnsName != nil ? .blue : .primary)

                            // Show star icon if this is the selected main name
                            if identity.mainDpnsName != nil {
                                Image(systemName: "star.fill")
                                    .font(.caption)
                                    .foregroundColor(.yellow)
                            }
                        }

                        // Show alias as subtitle if we're displaying a DPNS name
                        if (identity.mainDpnsName != nil || identity.dpnsName != nil),
                           let alias = identity.alias {
                            Text(alias)
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }

                    Spacer()

                    Text(formatBalanceShort(UInt64(bitPattern: identity.balance)))
                        .font(.headline)
                        .foregroundColor(.primary)
                }

                Text(identity.identityIdBase58)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)

                let identityType = identity.identityTypeEnum
                if identityType != .user || !hasAnyPrivateKey || identity.wallet == nil {
                    HStack(spacing: 6) {
                        if identityType == .masternode {
                            IdentityBadge(text: "Masternode", icon: "server.rack", color: .purple)
                        } else if identityType == .evonode {
                            IdentityBadge(text: "Evonode", icon: "server.rack", color: .indigo)
                        }
                        if !hasAnyPrivateKey {
                            IdentityBadge(text: "No Keys", icon: "key.slash", color: .red)
                        }
                        if identity.wallet == nil {
                            IdentityBadge(text: "No Wallet", icon: "wallet.pass", color: .orange)
                        }
                    }
                }

                // Wallet-owned (can sign) vs observed read-only. The
                // balance refresh is a plain Platform fetch, valid
                // for both.
                HStack {
                    if identity.isWalletOwned {
                        Image(systemName: "wallet.pass")
                            .font(.caption2)
                        Text("In Wallet")
                            .font(.caption2)
                    } else {
                        Image(systemName: "eye")
                            .font(.caption2)
                        Text("Observed")
                            .font(.caption2)
                    }

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
                .foregroundColor(identity.isWalletOwned ? .green : .secondary)
            }
            .padding(.vertical, 4)
        }
    }

    private func refreshBalance() async {
        guard let sdk = appState.sdk else { return }

        do {
            // Fetch identity data from Platform — one-shot refresh
            // on user pull. Writes land in SwiftData via the mutator
            // helpers; `@Query` upstream re-renders this row
            // automatically.
            let fetchedIdentity = try await sdk.identityGet(identityId: identity.identityIdBase58)

            // Update balance
            if let balanceValue = fetchedIdentity["balance"] {
                let newBalance: UInt64? = {
                    if let num = balanceValue as? NSNumber {
                        return num.uint64Value
                    }
                    if let str = balanceValue as? String, let v = UInt64(str) {
                        return v
                    }
                    return nil
                }()
                if let newBalance {
                    PersistentIdentity.updateBalance(
                        in: modelContext,
                        identityId: identity.identityId,
                        balance: newBalance
                    )
                }
            }

            // Fetch a DPNS name if we don't already have one — but
            // only once (silent failure), since not every identity
            // has a DPNS name and the request can 404.
            if identity.dpnsName == nil && identity.mainDpnsName == nil {
                if let usernames = try? await sdk.dpnsGetUsername(
                    identityId: identity.identityIdBase58,
                    limit: 1
                ),
                   let firstUsername = usernames.first,
                   let label = firstUsername["label"] as? String {
                    PersistentIdentity.updateDpnsName(
                        in: modelContext,
                        identityId: identity.identityId,
                        dpnsName: label
                    )
                }
            }

            try? modelContext.save()
        } catch {
            // Every persisted row exists on Platform (the persister
            // fires post-confirmation), so a failed refresh is worth
            // surfacing for wallet-owned and observed rows alike.
            appState.showError(
                message: "Failed to refresh balance: \(error.localizedDescription)"
            )
        }
    }
}

private struct IdentityBadge: View {
    let text: String
    let icon: String
    let color: Color

    var body: some View {
        HStack(spacing: 3) {
            Image(systemName: icon)
            Text(text)
        }
        .font(.caption2)
        .padding(.horizontal, 6)
        .padding(.vertical, 2)
        .background(color.opacity(0.15))
        .foregroundColor(color)
        .cornerRadius(4)
    }
}
