import SwiftUI
import SwiftData

// MARK: - Account Model (UI)

public enum AccountCategory: Equatable, Hashable {
    case bip44
    case bip32
    case coinjoin
    case identityRegistration
    case identityInvitation
    case identityTopupNotBound
    case identityTopup
    case providerVotingKeys
    case providerOwnerKeys
    case providerOperatorKeys
    case providerPlatformKeys
}

public struct AccountInfo: Identifiable, Hashable {
    public let id: String
    public let category: AccountCategory
    public let index: UInt32? // present only for indexed account types
    public let label: String
    public let balance: (confirmed: UInt64, unconfirmed: UInt64)
    public let addressCount: (external: Int, internal: Int)
    public let nextReceiveAddress: String?

    public init(category: AccountCategory,
                index: UInt32? = nil,
                label: String,
                balance: (confirmed: UInt64, unconfirmed: UInt64),
                addressCount: (external: Int, internal: Int),
                nextReceiveAddress: String?) {
        self.category = category
        self.index = index
        self.label = label
        self.balance = balance
        self.addressCount = addressCount
        self.nextReceiveAddress = nextReceiveAddress
        // Build a stable id
        if let idx = index {
            self.id = "\(category)-\(idx)"
        } else {
            self.id = "\(category)"
        }
    }
}

extension AccountInfo: Equatable {
    public static func == (lhs: AccountInfo, rhs: AccountInfo) -> Bool {
        return lhs.id == rhs.id
    }
}

extension AccountInfo {
    public func hash(into hasher: inout Hasher) {
        hasher.combine(id)
    }
}

// MARK: - Account List View
struct AccountListView: View {
    @EnvironmentObject var walletService: WalletService
    let wallet: HDWallet
    @State private var accounts: [AccountInfo] = []
    @State private var isLoading = true
    @State private var errorMessage: String?
    
    var body: some View {
        ZStack {
            if isLoading {
                ProgressView("Loading accounts...")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let error = errorMessage {
                ContentUnavailableView(
                    "Failed to Load Accounts",
                    systemImage: "exclamationmark.triangle",
                    description: Text(error)
                )
            } else if accounts.isEmpty {
                ContentUnavailableView(
                    "No Accounts",
                    systemImage: "folder",
                    description: Text("Create an account to get started")
                )
            } else {
                List(accounts) { account in
                    NavigationLink(destination: AccountDetailView(wallet: wallet, account: account)) {
                        AccountRowView(account: account)
                    }
                }
                .listStyle(.plain)
                .refreshable {
                    await loadAccounts()
                }
            }
        }
        .task {
            await loadAccounts()
        }
    }
    
    private func loadAccounts() async {
        isLoading = true
        errorMessage = nil
        
        do {
            // Get accounts from wallet manager
            let fetchedAccounts = try await walletService.walletManager?.getAccounts(for: wallet) ?? []
            await MainActor.run {
                self.accounts = fetchedAccounts
                self.isLoading = false
            }
        } catch {
            await MainActor.run {
                self.errorMessage = error.localizedDescription
                self.isLoading = false
            }
        }
    }
}

// MARK: - Account Row View
struct AccountRowView: View {
    let account: AccountInfo
    
    /// Determines if this account type should show balance in UI
    var shouldShowBalance: Bool {
        switch account.category {
        case .bip44, .bip32, .coinjoin:
            return true
        default:
            return false
        }
    }
    
    var accountTypeBadge: String {
        switch account.category {
        case .bip44: return (account.index == 0) ? "Main" : (account.index.map { "#\($0)" } ?? "BIP44")
        case .bip32: return account.index.map { "BIP32 #\($0)" } ?? "BIP32"
        case .coinjoin: return account.index.map { "CoinJoin #\($0)" } ?? "CoinJoin"
        case .identityRegistration: return "Identity"
        case .identityInvitation: return "Invitation"
        case .identityTopupNotBound: return "Top-up"
        case .identityTopup: return account.index.map { "Top-up #\($0)" } ?? "Top-up"
        case .providerVotingKeys: return "Voting"
        case .providerOwnerKeys: return "Owner"
        case .providerOperatorKeys: return "Operator"
        case .providerPlatformKeys: return "Platform"
        }
    }
    
    var accountTypeIcon: String {
        switch account.category {
        case .bip44: return account.index == 0 ? "star.circle.fill" : "folder"
        case .bip32: return "tray.full"
        case .coinjoin: return "shuffle.circle"
        case .identityRegistration: return "person.crop.circle"
        case .identityInvitation: return "envelope.circle"
        case .identityTopupNotBound, .identityTopup: return "arrow.up.circle"
        case .providerVotingKeys: return "key.viewfinder"
        case .providerOwnerKeys: return "key.horizontal"
        case .providerOperatorKeys: return "wrench.and.screwdriver"
        case .providerPlatformKeys: return "network"
        }
    }
    
    var accountTypeColor: Color {
        switch account.category {
        case .bip44: return (account.index == 0) ? .green : .blue
        case .bip32: return .teal
        case .coinjoin: return .orange
        case .identityRegistration, .identityInvitation, .identityTopupNotBound, .identityTopup: return .purple
        case .providerVotingKeys: return .red
        case .providerOwnerKeys: return .pink
        case .providerOperatorKeys: return .indigo
        case .providerPlatformKeys: return .teal
        }
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Account header
            HStack {
                Label(account.label, systemImage: accountTypeIcon)
                    .font(.headline)
                    .foregroundColor(accountTypeColor)
                
                Spacer()
                
                // Account type badge
                Text(accountTypeBadge)
                    .font(.caption)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 2)
                    .background(accountTypeColor.opacity(0.2))
                    .cornerRadius(4)
            }
            
            // Balance information - only show for appropriate account types
            if shouldShowBalance {
                HStack(spacing: 16) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Confirmed")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text(formatBalance(account.balance.confirmed))
                            .font(.subheadline)
                            .fontWeight(.medium)
                    }
                    
                    if account.balance.unconfirmed > 0 {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Pending")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(formatBalance(account.balance.unconfirmed))
                                .font(.subheadline)
                                .fontWeight(.medium)
                                .foregroundColor(.orange)
                        }
                    }
                    
                    Spacer()
                    
                    // Total balance
                    VStack(alignment: .trailing, spacing: 2) {
                        Text("Total")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text(formatBalance(account.balance.confirmed + account.balance.unconfirmed))
                            .font(.subheadline)
                            .fontWeight(.semibold)
                            .foregroundColor(accountTypeColor)
                    }
                }
            } else {
                // For special-purpose accounts, show their purpose instead of balance
                HStack {
                    Text("Special Purpose Account")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .italic()
                    Spacer()
                }
            }
            
            // Address count information (only for accounts with addresses)
            if account.addressCount.external > 0 || account.addressCount.internal > 0 {
                HStack(spacing: 16) {
                    if account.addressCount.external > 0 {
                        Label("\(account.addressCount.external) receive", systemImage: "arrow.down.circle")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    
                    if account.addressCount.internal > 0 {
                        Label("\(account.addressCount.internal) change", systemImage: "arrow.up.arrow.down.circle")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    
                    Spacer()
                }
            }
            
            // Next receive address (if available and appropriate for account type)
            if shouldShowBalance, let address = account.nextReceiveAddress {
                HStack {
                    Text("Receive:")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Text(address)
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .foregroundColor(.secondary)
                    
                    Button(action: {
                        // Copy address to clipboard
                        #if os(iOS)
                        UIPasteboard.general.string = address
                        #endif
                    }) {
                        Image(systemName: "doc.on.doc")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    .buttonStyle(.plain)
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
