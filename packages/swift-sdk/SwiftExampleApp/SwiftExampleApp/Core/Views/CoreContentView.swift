import SwiftUI
import SwiftData

struct CoreContentView: View {
    @EnvironmentObject var walletService: WalletService
    @Environment(\.modelContext) private var modelContext
    @Query private var wallets: [HDWallet]
    @State private var showingCreateWallet = false
    
    var body: some View {
        VStack {
            if wallets.isEmpty {
                VStack(spacing: 20) {
                    Spacer()
                    
                    Image(systemName: "wallet.pass")
                        .font(.system(size: 60))
                        .foregroundColor(.gray)
                    
                    Text("No Wallets")
                        .font(.title)
                        .fontWeight(.semibold)
                    
                    Text("Create a wallet to get started")
                        .foregroundColor(.secondary)
                    
                    Button {
                        showingCreateWallet = true
                    } label: {
                        Text("Create Wallet")
                            .foregroundColor(.white)
                            .padding(.horizontal, 20)
                            .padding(.vertical, 10)
                            .background(Color.blue)
                            .cornerRadius(8)
                    }
                    
                    Spacer()
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .navigationTitle("Wallets")
                .navigationBarTitleDisplayMode(.large)
            } else {
                List(wallets) { wallet in
                    NavigationLink {
                        WalletDetailView(wallet: wallet)
                    } label: {
                        WalletRowView(wallet: wallet)
                    }
                }
                .navigationTitle("Wallets")
                .toolbar {
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button {
                            showingCreateWallet = true
                        } label: {
                            Image(systemName: "plus")
                        }
                    }
                }
            }
        }
        .sheet(isPresented: $showingCreateWallet) {
            NavigationStack {
                CreateWalletView()
                    .environmentObject(walletService)
                    .environment(\.modelContext, modelContext)
            }
        }
    }
}

struct WalletRowView: View {
    let wallet: HDWallet
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    
    var platformBalance: UInt64 {
        // Sum all identity balances linked to this wallet
        unifiedAppState.platformState.identities.reduce(0) { sum, identity in
            sum + identity.balance
        }
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(wallet.label)
                    .font(.headline)
                
                Spacer()
                
                if wallet.syncProgress < 1.0 {
                    ProgressView(value: wallet.syncProgress)
                        .frame(width: 50)
                }
            }
            
            HStack {
                Label(wallet.network.capitalized, systemImage: "network")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Spacer()
                
                VStack(alignment: .trailing, spacing: 2) {
                    // Show wallet balance or "Empty"
                    if wallet.totalBalance == 0 {
                        Text("Empty")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else {
                        Text(formatBalance(wallet.totalBalance))
                            .font(.subheadline)
                            .fontWeight(.medium)
                    }
                    
                    // Show platform balance if any
                    if platformBalance > 0 {
                        HStack(spacing: 3) {
                            Image(systemName: "p.circle.fill")
                                .font(.system(size: 9))
                            Text(formatBalance(platformBalance))
                        }
                        .font(.caption2)
                        .foregroundColor(.blue)
                    }
                }
            }
        }
        .padding(.vertical, 4)
    }
    
    private func formatBalance(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        
        // Special case for zero
        if dash == 0 {
            return "0 DASH"
        }
        
        // Format with up to 8 decimal places, removing trailing zeros
        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."
        
        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return "\(formatted) DASH"
        }
        
        // Fallback formatting
        let formatted = String(format: "%.8f", dash)
        let trimmed = formatted.replacingOccurrences(of: "0+$", with: "", options: .regularExpression)
            .replacingOccurrences(of: "\\.$", with: "", options: .regularExpression)
        return "\(trimmed) DASH"
    }
}