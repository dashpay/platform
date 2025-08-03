import SwiftUI
import SwiftData

struct CoreContentView: View {
    @EnvironmentObject var walletService: WalletService
    @Environment(\.modelContext) private var modelContext
    @Query private var wallets: [HDWallet]
    @State private var showingCreateWallet = false
    @State private var lastTapLocation: CGPoint = .zero
    @State private var showTapCoordinates = false
    
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
                    
                    // Debug button to test tap coordinates
                    Button {
                        showTapCoordinates.toggle()
                    } label: {
                        VStack {
                            Text("Tap Coordinate Test")
                                .font(.headline)
                            Text("Tap anywhere on this button")
                                .font(.caption)
                            if showTapCoordinates {
                                Text("Last tap: (\(Int(lastTapLocation.x)), \(Int(lastTapLocation.y)))")
                                    .font(.system(.caption, design: .monospaced))
                                    .foregroundColor(.green)
                            }
                        }
                        .frame(maxWidth: .infinity, minHeight: 100)
                        .padding()
                        .background(Color.gray.opacity(0.2))
                        .cornerRadius(10)
                    }
                    .padding(.horizontal)
                    .onTapGesture { location in
                        lastTapLocation = location
                        showTapCoordinates = true
                        print("Tapped at: \(location)")
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