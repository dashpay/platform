import SwiftUI
import SwiftDashSDK
import SwiftData

// MARK: - Account List View
struct AccountListView: View {
    let wallet: HDWallet

    @Query private var accounts: [PersistentAccount]

    init(wallet: HDWallet) {
        self.wallet = wallet
        let walletId = wallet.walletId
        _accounts = Query(
            filter: #Predicate<PersistentAccount> { acc in
                acc.wallet?.walletId == walletId
            },
            sort: [SortDescriptor(\PersistentAccount.accountIndex)]
        )
    }

    var body: some View {
        ZStack {
            if accounts.isEmpty {
                ContentUnavailableView(
                    "No Accounts",
                    systemImage: "folder",
                    description: Text("Accounts are created automatically when the wallet syncs.")
                )
            } else {
                List(accounts) { account in
                    NavigationLink(destination: AccountDetailView(wallet: wallet, account: account)) {
                        AccountRowView(account: account)
                    }
                }
                .listStyle(.plain)
            }
        }
    }
}

// MARK: - Account Row View
struct AccountRowView: View {
    let account: PersistentAccount

    /// Friendly label for the account. The FFI currently emits a type name
    /// string which is displayed directly; indexed account types get a
    /// trailing "#<index>".
    private var label: String {
        let name = account.accountTypeName
        if name == "Standard BIP44" || name == "BIP32" || name == "CoinJoin" {
            return "\(name) #\(account.accountIndex)"
        }
        return name
    }

    /// Whether this account type should show balance information.
    private var shouldShowBalance: Bool {
        ["Standard BIP44", "BIP32", "CoinJoin"].contains(account.accountTypeName)
    }

    private var iconName: String {
        switch account.accountTypeName {
        case "Standard BIP44": return "star.circle.fill"
        case "BIP32": return "tray.full"
        case "CoinJoin": return "shuffle.circle"
        case "Identity Registration": return "person.crop.circle"
        case "Identity Invitation": return "envelope.circle"
        case "Identity Topup (Not Bound)", "Identity Topup": return "arrow.up.circle"
        case "Provider Voting Keys": return "key.viewfinder"
        case "Provider Owner Keys": return "key.horizontal"
        case "Provider Operator Keys (BLS)": return "wrench.and.screwdriver"
        case "Provider Platform Keys (EdDSA)": return "network"
        default: return "folder"
        }
    }

    private var iconColor: Color {
        switch account.accountTypeName {
        case "Standard BIP44": return account.accountIndex == 0 ? .green : .blue
        case "BIP32": return .teal
        case "CoinJoin": return .orange
        case let n where n.hasPrefix("Identity"): return .purple
        case "Provider Voting Keys": return .red
        case "Provider Owner Keys": return .pink
        case "Provider Operator Keys (BLS)": return .indigo
        case "Provider Platform Keys (EdDSA)": return .cyan
        default: return .gray
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Label(label, systemImage: iconName)
                    .font(.headline)
                    .foregroundColor(iconColor)

                Spacer()

                Text(account.accountTypeName)
                    .font(.caption)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 2)
                    .background(iconColor.opacity(0.2))
                    .cornerRadius(4)
            }

            if shouldShowBalance {
                HStack(spacing: 16) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Confirmed")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text(formatBalance(account.balanceConfirmed))
                            .font(.subheadline)
                            .fontWeight(.medium)
                    }

                    if account.balanceUnconfirmed > 0 {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Pending")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(formatBalance(account.balanceUnconfirmed))
                                .font(.subheadline)
                                .fontWeight(.medium)
                                .foregroundColor(.orange)
                        }
                    }

                    Spacer()

                    VStack(alignment: .trailing, spacing: 2) {
                        Text("Total")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text(formatBalance(account.balanceConfirmed + account.balanceUnconfirmed))
                            .font(.subheadline)
                            .fontWeight(.semibold)
                            .foregroundColor(iconColor)
                    }
                }
            } else {
                HStack {
                    Text("Special Purpose Account")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .italic()
                    Spacer()
                }
            }

            // Address pool summary.
            let ext = max(Int(account.externalHighestUsed) + 1, 0)
            let intc = max(Int(account.internalHighestUsed) + 1, 0)
            if ext > 0 || intc > 0 {
                HStack(spacing: 16) {
                    if ext > 0 {
                        Label("\(ext) receive", systemImage: "arrow.down.circle")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    if intc > 0 {
                        Label("\(intc) change", systemImage: "arrow.up.arrow.down.circle")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    Spacer()
                }
            }
        }
        .padding(.vertical, 8)
    }

    private func formatBalance(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0

        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."

        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return "\(formatted) DASH"
        }

        return String(format: "%.8f DASH", dash)
    }
}
