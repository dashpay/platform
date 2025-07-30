import SwiftUI
import SwiftData

struct CoreContentView: View {
    @EnvironmentObject var walletService: WalletService
    @Environment(\.modelContext) private var modelContext
    @Query private var wallets: [HDWallet]
    @State private var showCreateWallet = false
    
    var body: some View {
        NavigationStack {
            if wallets.isEmpty {
                ContentUnavailableView {
                    Label("No Wallets", systemImage: "wallet.pass")
                } description: {
                    Text("Create a wallet to get started")
                } actions: {
                    Button("Create Wallet") {
                        showCreateWallet = true
                    }
                    .buttonStyle(.borderedProminent)
                }
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
                            showCreateWallet = true
                        } label: {
                            Image(systemName: "plus")
                        }
                    }
                }
            }
        }
        .sheet(isPresented: $showCreateWallet) {
            CreateWalletView()
        }
    }
}

struct WalletRowView: View {
    let wallet: HDWallet
    
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
                
                Text(formatBalance(wallet.totalBalance))
                    .font(.subheadline)
                    .fontWeight(.medium)
            }
        }
        .padding(.vertical, 4)
    }
    
    private func formatBalance(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
}