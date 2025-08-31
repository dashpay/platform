import SwiftUI
import SwiftData

// MARK: - Account Info from FFI
public struct AccountInfo {
    public let index: UInt32
    public let label: String
    public let balance: (confirmed: UInt64, unconfirmed: UInt64)
    public let addressCount: (external: Int, internal: Int)
    public let nextReceiveAddress: String?
    
    public init(index: UInt32, label: String, balance: (confirmed: UInt64, unconfirmed: UInt64), addressCount: (external: Int, internal: Int), nextReceiveAddress: String?) {
        self.index = index
        self.label = label
        self.balance = balance
        self.addressCount = addressCount
        self.nextReceiveAddress = nextReceiveAddress
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
                List(accounts, id: \.index) { account in
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
        WalletManager.shouldShowBalance(for: account.index)
    }
    
    var accountTypeBadge: String {
        switch account.index {
        case 0: return "Main"
        case 1...999: return "#\(account.index)"
        case 1000...1999: return "CoinJoin"
        case 9000: return "Identity"
        case 9001: return "Invitation"
        case 9002: return "Top-up"
        case 10000...10999: return "Voting"
        case 11000...11999: return "Owner"
        case 12000...12999: return "Operator"
        case 13000...13999: return "Platform"
        default: return "Special"
        }
    }
    
    var accountTypeIcon: String {
        // Special account types have different icons
        switch account.index {
        case 0: return "star.circle.fill" // Main account
        case 1...999: return "folder" // Regular BIP44 accounts
        case 1000...1999: return "shuffle.circle" // CoinJoin accounts
        case 9000: return "person.crop.circle" // Identity Registration
        case 9001: return "envelope.circle" // Identity Invitation
        case 9002: return "arrow.up.circle" // Identity Top-up
        case 10000...10999: return "key.viewfinder" // Provider Voting Keys
        case 11000...11999: return "key.horizontal" // Provider Owner Keys
        case 12000...12999: return "wrench.and.screwdriver" // Provider Operator Keys
        case 13000...13999: return "network" // Provider Platform Keys
        default: return "questionmark.circle" // Unknown special accounts
        }
    }
    
    var accountTypeColor: Color {
        switch account.index {
        case 0: return .green // Main account
        case 1...999: return .blue // Regular accounts
        case 1000...1999: return .orange // CoinJoin accounts
        case 9000...9002: return .purple // Identity accounts
        case 10000...10999: return .red // Provider Voting Keys
        case 11000...11999: return .pink // Provider Owner Keys
        case 12000...12999: return .indigo // Provider Operator Keys
        case 13000...13999: return .teal // Provider Platform Keys
        default: return .gray // Unknown accounts
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