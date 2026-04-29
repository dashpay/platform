import SwiftUI
import SwiftDashSDK
import SwiftData

// MARK: - Account List View
struct AccountListView: View {
    let wallet: PersistentWallet

    @Query private var accounts: [PersistentAccount]

    init(wallet: PersistentWallet) {
        self.wallet = wallet
        let walletId = wallet.walletId
        _accounts = Query(
            filter: #Predicate<PersistentAccount> { acc in
                acc.wallet.walletId == walletId
            }
        )
    }

    /// Stable display order — grouped by logical priority rather
    /// than by raw `accountType` tag so BIP44 leads, PlatformPayment
    /// sits next, BIP32 follows, CoinJoin after, and every special-
    /// purpose account tails off in tag order.
    private var orderedAccounts: [PersistentAccount] {
        accounts.sorted { lhs, rhs in
            let lhsKey = AccountListView.sortKey(for: lhs)
            let rhsKey = AccountListView.sortKey(for: rhs)
            return lhsKey < rhsKey
        }
    }

    private static func sortKey(
        for account: PersistentAccount
    ) -> (UInt8, UInt32, UInt8, UInt32) {
        // (group, accountType, standardTag, accountIndex). Lower
        // `group` sorts earlier. The three extra fields keep the
        // ordering deterministic when multiple accounts share a
        // group (e.g. BIP44 #0 before BIP44 #1).
        let group: UInt8
        switch account.accountType {
        case 0:
            // Standard: BIP44 (standardTag=0) ahead of BIP32 (=1).
            group = account.standardTag == 0 ? 0 : 2
        case 14:
            group = 1 // PlatformPayment — user-facing "second" slot.
        case 1:
            group = 3 // CoinJoin.
        default:
            group = 4 // Identity / provider / asset-lock / dashpay.
        }
        return (group, account.accountType, account.standardTag, account.accountIndex)
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
                List(orderedAccounts) { account in
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

    /// Friendly label for the account. Indexed account types get a
    /// trailing "#<index>"; other types keep the bare name emitted by
    /// the FFI (identity / provider / etc.).
    private var label: String {
        switch account.accountType {
        case 0, 1, 14: // Standard (BIP44/BIP32), CoinJoin, PlatformPayment
            return "\(account.accountTypeName) #\(account.accountIndex)"
        default:
            return account.accountTypeName
        }
    }

    /// Whether this account should surface a numeric balance on the
    /// summary row. Keyed on the FFI `accountType` tag — name
    /// matching was fragile because the Rust side emits different
    /// labels (e.g. "BIP44 Account" vs "Standard BIP44") across
    /// releases.
    private var shouldShowBalance: Bool {
        switch account.accountType {
        case 0, 1, 14: return true
        default: return false
        }
    }

    /// PlatformPayment balance/unit differs from Core (credits, not
    /// duffs) so the row needs a dedicated render path. `true` when
    /// this row should use the `platformBalanceRow` helper.
    private var isPlatformPayment: Bool {
        account.accountType == 14
    }

    private var iconName: String {
        switch account.accountType {
        case 0:
            return account.standardTag == 0 ? "star.circle.fill" : "tray.full"
        case 1: return "shuffle.circle"
        case 14: return "creditcard"
        case 2, 3, 4, 5: return "person.crop.circle"
        case 6, 7: return "arrow.up.circle"
        case 8: return "key.viewfinder"
        case 9: return "key.horizontal"
        case 10: return "wrench.and.screwdriver"
        case 11: return "network"
        default: return "folder"
        }
    }

    private var iconColor: Color {
        switch account.accountType {
        case 0: return account.standardTag == 0
            ? (account.accountIndex == 0 ? .green : .blue)
            : .teal
        case 1: return .orange
        case 14: return .indigo
        case 2, 3, 4, 5, 6, 7: return .purple
        case 8: return .red
        case 9: return .pink
        case 10: return .indigo
        case 11: return .cyan
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

            if isPlatformPayment {
                platformBalanceRow
            } else if shouldShowBalance {
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

            if isPlatformPayment {
                platformPoolSummary
            } else {
                // Core-chain address pool summary.
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
        }
        .padding(.vertical, 8)
    }

    // MARK: - Platform Payment row helpers

    /// Sum + "Total" balance display for a PlatformPayment account,
    /// rendered in DASH via the credit divisor (1e11).
    private var platformBalanceRow: some View {
        let total = account.platformAddresses.reduce(0) { $0 + $1.balance }
        return HStack(spacing: 16) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Balance")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Text(formatCredits(total))
                    .font(.subheadline)
                    .fontWeight(.medium)
                    .foregroundColor(iconColor)
            }
            Spacer()
        }
    }

    /// "X used of Y addresses" footer for a PlatformPayment account.
    /// No external/internal split — DIP-17 pools are flat.
    private var platformPoolSummary: some View {
        let total = account.platformAddresses.count
        let used = account.platformAddresses.filter { $0.isUsed }.count
        return HStack(spacing: 16) {
            Label("\(used) used", systemImage: "checkmark.circle")
                .font(.caption)
                .foregroundColor(.secondary)
            Label("\(total) total", systemImage: "tray.full")
                .font(.caption)
                .foregroundColor(.secondary)
            Spacer()
        }
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

    /// Credits → DASH (1 DASH = 100_000_000_000 credits).
    private func formatCredits(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000_000.0
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
