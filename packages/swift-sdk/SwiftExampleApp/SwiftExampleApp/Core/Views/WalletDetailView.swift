import SwiftUI
import SwiftData

struct WalletDetailView: View {
    @EnvironmentObject var walletService: WalletService
    let wallet: HDWallet
    @State private var selectedTab = 0
    @State private var showReceiveAddress = false
    @State private var showSendTransaction = false
    
    var body: some View {
        VStack(spacing: 0) {
            // Balance Card
            BalanceCardView(wallet: wallet)
                .padding()
            
            // Action Buttons
            HStack(spacing: 16) {
                Button {
                    showSendTransaction = true
                } label: {
                    Label("Send", systemImage: "arrow.up.circle.fill")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                
                Button {
                    showReceiveAddress = true
                } label: {
                    Label("Receive", systemImage: "arrow.down.circle.fill")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.bordered)
            }
            .padding(.horizontal)
            
            // Tab Selection
            Picker("View", selection: $selectedTab) {
                Text("Transactions").tag(0)
                Text("Addresses").tag(1)
                Text("UTXOs").tag(2)
            }
            .pickerStyle(.segmented)
            .padding()
            
            // Tab Content
            TabView(selection: $selectedTab) {
                TransactionListView(transactions: wallet.transactions)
                    .tag(0)
                
                AddressListView(addresses: wallet.addresses)
                    .tag(1)
                
                UTXOListView(utxos: wallet.utxos)
                    .tag(2)
            }
            .tabViewStyle(.page(indexDisplayMode: .never))
        }
        .navigationTitle(wallet.label)
        .navigationBarTitleDisplayMode(.inline)
        .sheet(isPresented: $showReceiveAddress) {
            ReceiveAddressView(wallet: wallet)
        }
        .sheet(isPresented: $showSendTransaction) {
            SendTransactionView(wallet: wallet)
        }
        .task {
            await walletService.loadWallet(wallet)
        }
    }
}

struct BalanceCardView: View {
    let wallet: HDWallet
    
    var body: some View {
        VStack(spacing: 12) {
            Text("Total Balance")
                .font(.subheadline)
                .foregroundColor(.secondary)
            
            Text(formatBalance(wallet.totalBalance))
                .font(.system(size: 36, weight: .bold, design: .rounded))
            
            HStack(spacing: 20) {
                VStack(spacing: 4) {
                    Text("Confirmed")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(formatBalance(wallet.confirmedBalance))
                        .font(.subheadline)
                        .fontWeight(.medium)
                }
                
                Divider()
                    .frame(height: 30)
                
                VStack(spacing: 4) {
                    Text("Unconfirmed")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(formatBalance(wallet.unconfirmedBalance))
                        .font(.subheadline)
                        .fontWeight(.medium)
                }
            }
        }
        .padding()
        .background(Color(UIColor.secondarySystemBackground))
        .cornerRadius(12)
    }
    
    private func formatBalance(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
}

struct TransactionListView: View {
    let transactions: [HDTransaction]
    
    var body: some View {
        if transactions.isEmpty {
            ContentUnavailableView(
                "No Transactions",
                systemImage: "list.bullet.rectangle",
                description: Text("Transactions will appear here")
            )
        } else {
            List(transactions.sorted(by: { $0.timestamp > $1.timestamp })) { transaction in
                TransactionRowView(transaction: transaction)
            }
            .listStyle(.plain)
        }
    }
}

struct TransactionRowView: View {
    let transaction: HDTransaction
    
    var body: some View {
        HStack {
            Image(systemName: transaction.amount < 0 ? "arrow.up.circle" : "arrow.down.circle")
                .font(.title2)
                .foregroundColor(transaction.amount < 0 ? .red : .green)
            
            VStack(alignment: .leading, spacing: 4) {
                Text(transaction.type.capitalized)
                    .font(.subheadline)
                    .fontWeight(.medium)
                
                Text(transaction.timestamp, style: .date)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            VStack(alignment: .trailing, spacing: 4) {
                Text(formatAmount(transaction.amount))
                    .font(.subheadline)
                    .fontWeight(.medium)
                    .foregroundColor(transaction.amount < 0 ? .red : .green)
                
                if transaction.isPending {
                    Text("Pending")
                        .font(.caption)
                        .foregroundColor(.orange)
                } else {
                    Text("\(transaction.confirmations) conf")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(.vertical, 4)
    }
    
    private func formatAmount(_ amount: Int64) -> String {
        let dash = Double(abs(amount)) / 100_000_000.0
        let sign = amount < 0 ? "-" : "+"
        return "\(sign)\(String(format: "%.8f", dash))"
    }
}

struct AddressListView: View {
    let addresses: [HDAddress]
    
    var body: some View {
        if addresses.isEmpty {
            ContentUnavailableView(
                "No Addresses",
                systemImage: "qrcode",
                description: Text("Addresses will appear here")
            )
        } else {
            List(addresses.sorted(by: { $0.index < $1.index })) { address in
                AddressRowView(address: address)
            }
            .listStyle(.plain)
        }
    }
}

struct AddressRowView: View {
    let address: HDAddress
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text("Address #\(address.index)")
                    .font(.subheadline)
                    .fontWeight(.medium)
                
                Spacer()
                
                if address.isUsed {
                    Label("Used", systemImage: "checkmark.circle.fill")
                        .font(.caption)
                        .foregroundColor(.green)
                }
            }
            
            Text(address.address)
                .font(.system(.caption, design: .monospaced))
                .foregroundColor(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .padding(.vertical, 4)
    }
}

struct UTXOListView: View {
    let utxos: [HDUTXO]
    
    var body: some View {
        if utxos.isEmpty {
            ContentUnavailableView(
                "No UTXOs",
                systemImage: "bitcoinsign.circle",
                description: Text("Unspent outputs will appear here")
            )
        } else {
            List(utxos) { utxo in
                UTXORowView(utxo: utxo)
            }
            .listStyle(.plain)
        }
    }
}

struct UTXORowView: View {
    let utxo: HDUTXO
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(formatAmount(utxo.amount))
                    .font(.subheadline)
                    .fontWeight(.medium)
                
                Spacer()
                
                if utxo.isConfirmed {
                    Label("Confirmed", systemImage: "checkmark.circle.fill")
                        .font(.caption)
                        .foregroundColor(.green)
                } else {
                    Label("Unconfirmed", systemImage: "clock")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }
            
            Text("\(utxo.txid):\(utxo.outputIndex)")
                .font(.system(.caption, design: .monospaced))
                .foregroundColor(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .padding(.vertical, 4)
    }
    
    private func formatAmount(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
}