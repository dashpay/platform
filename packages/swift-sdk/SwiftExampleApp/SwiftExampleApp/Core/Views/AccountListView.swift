import SwiftUI
import SwiftDashSDK
import SwiftData

// MARK: - Account List View
struct AccountListView: View {
    let wallet: PersistentWallet
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState

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
    ///
    /// DashPay friendship accounts (tags 12 receiving / 13 external)
    /// are hidden here: they're per-contact protocol plumbing, one
    /// pair per friendship, and would crowd the list as contacts
    /// grow. Their funds already roll into the wallet's Core
    /// Balance, and the DashPay tab surfaces the received-from-
    /// contacts number; the Storage Explorer still lists the raw
    /// rows for debugging.
    private var orderedAccounts: [PersistentAccount] {
        accounts
            .filter { $0.accountType != 12 && $0.accountType != 13 }
            .sorted { lhs, rhs in
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

    /// Bound shielded accounts to render in their own section
    /// below the Core / Platform accounts. Empty until this wallet's
    /// engine binding lands (`rebindWalletScopedServices`).
    private var shieldedAccountsForThisWallet: [UInt32] {
        // Engine-bound wallets expose account 0 by default. Resolve
        // per-wallet from the engine (via `shieldedAddress(for:)`) rather
        // than the single UI mirror so the section shows for ANY loaded
        // wallet, not just `firstWallet`.
        shieldedAddress(for: 0) != nil ? [0] : []
    }

    /// Bech32m Orchard receive address for `account` on the viewed
    /// wallet, resolved per-wallet from the engine (rather than the
    /// single UI mirror's `addressesByAccount`). `nil` until this
    /// wallet's bind lands or if encoding fails.
    private func shieldedAddress(for account: UInt32) -> String? {
        walletManager.shieldedDisplayAddress(
            walletId: wallet.walletId,
            account: account,
            network: platformState.currentNetwork
        )
    }

    var body: some View {
        ZStack {
            // Gate on `orderedAccounts` (the FILTERED list actually rendered),
            // not the raw `accounts` query: a wallet whose only rows are
            // DashPay friendship accounts (tags 12/13, hidden here) has a
            // non-empty `accounts` but an empty `orderedAccounts`, which would
            // otherwise show an empty Section instead of the empty state.
            if orderedAccounts.isEmpty && shieldedAccountsForThisWallet.isEmpty {
                ContentUnavailableView(
                    "No Accounts",
                    systemImage: "folder",
                    description: Text("Accounts are created automatically when the wallet syncs.")
                )
            } else {
                let balances = walletManager.accountBalances(for: wallet.walletId)
                List {
                    if !orderedAccounts.isEmpty {
                        Section {
                            ForEach(orderedAccounts) { account in
                                NavigationLink(
                                    destination: AccountDetailView(wallet: wallet, account: account)
                                ) {
                                    let match = balances.first { b in
                                        UInt32(b.typeTag) == account.accountType &&
                                        b.standardTag == account.standardTag &&
                                        b.index == account.accountIndex
                                    }
                                    AccountRowView(
                                        account: account,
                                        coreConfirmedBalance: match?.confirmed ?? 0,
                                        coreUnconfirmedBalance: match?.unconfirmed ?? 0
                                    )
                                }
                            }
                        }
                    }
                    if !shieldedAccountsForThisWallet.isEmpty {
                        Section("Shielded") {
                            ForEach(shieldedAccountsForThisWallet, id: \.self) { account in
                                ShieldedAccountRowView(
                                    accountIndex: account,
                                    address: shieldedAddress(for: account)
                                )
                            }
                        }
                    }
                }
                .listStyle(.plain)
            }
        }
    }
}

// MARK: - Shielded Account Row

/// Compact row that mirrors `AccountRowView` for shielded ZIP-32
/// accounts. There's no `PersistentShieldedAccount` SwiftData
/// model — bound accounts live on `ShieldedService.boundAccounts`
/// — so the row is purely a display projection of `(index,
/// address)`.
private struct ShieldedAccountRowView: View {
    let accountIndex: UInt32
    let address: String?

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: "lock.shield.fill")
                .foregroundColor(.purple)
                .font(.title3)
            VStack(alignment: .leading, spacing: 2) {
                Text("Shielded #\(accountIndex)")
                    .font(.subheadline)
                    .fontWeight(.medium)
                if let address {
                    Text(address)
                        .font(.system(.caption2, design: .monospaced))
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                } else {
                    Text("address not available")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                        .italic()
                }
            }
        }
    }
}

// MARK: - Account Row View
struct AccountRowView: View {
    let account: PersistentAccount
    /// Per-account confirmed balance queried from Rust's in-memory state.
    let coreConfirmedBalance: UInt64
    /// Per-account unconfirmed balance queried from Rust's in-memory state.
    let coreUnconfirmedBalance: UInt64

    private var label: String {
        switch account.accountType {
        case 0, 1, 14:
            return "\(account.accountTypeName) #\(account.accountIndex)"
        default:
            return account.accountTypeName
        }
    }

    private var shouldShowBalance: Bool {
        switch account.accountType {
        case 0, 1, 14: return true
        default: return false
        }
    }

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
                        Text(formatBalance(coreConfirmedBalance))
                            .font(.subheadline)
                            .fontWeight(.medium)
                    }

                    if coreUnconfirmedBalance > 0 {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Pending")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(formatBalance(coreUnconfirmedBalance))
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
                        Text(formatBalance(coreConfirmedBalance + coreUnconfirmedBalance))
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
