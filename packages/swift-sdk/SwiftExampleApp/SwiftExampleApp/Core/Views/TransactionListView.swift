import SwiftUI
import SwiftData
import SwiftDashSDK

struct TransactionListView: View {
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    let wallet: HDWallet

    @State private var transactions: [WalletTransaction] = []
    @State private var selectedTransaction: WalletTransaction?

    private var sortedTransactions: [WalletTransaction] {
        transactions.sorted { $0.timestamp > $1.timestamp }
    }

    var body: some View {
        ZStack {
            List {
                ForEach(sortedTransactions, id: \.txid) { transaction in
                    Button {
                        selectedTransaction = transaction
                    } label: {
                        TransactionRowView(transaction: transaction)
                    }
                    .buttonStyle(.plain)
                }
            }
            .listStyle(.insetGrouped)
        }
        .navigationTitle("Transactions")
        .navigationBarTitleDisplayMode(.inline)
        .sheet(item: $selectedTransaction) { transaction in
            TransactionDetailView(transaction: transaction)
        }
        .task {
            loadTransactions()
        }
        .refreshable {
            loadTransactions()
        }
    }

    private func loadTransactions() {
      self.transactions = walletService.walletManager.getTransactions(for: wallet)
    }
}

// MARK: - Transaction Row View

struct TransactionRowView: View {
    let transaction: WalletTransaction

    private var typeIcon: String {
        switch transaction.netAmount {
        case let amount where amount > 0:
            return "arrow.down.circle.fill"
        case let amount where amount < 0:
            return "arrow.up.circle.fill"
        default:
            return "arrow.triangle.2.circlepath"
        }
    }
    
    private var typeColor: Color {
        switch transaction.netAmount {
        case let amount where amount > 0:
            return .green
        case let amount where amount < 0:
            return .red
        default:
            return .blue
        }
    }


    @ViewBuilder
    private var confirmationBadge: some View {
        if !transaction.isConfirmed {
            HStack(spacing: 4) {
                Image(systemName: "clock")
                    .font(.caption2)
                Text("Pending")
                    .font(.caption2)
            }
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(Color.orange.opacity(0.2))
            .foregroundColor(.orange)
            .cornerRadius(4)
        } else {
            HStack(spacing: 4) {
                Image(systemName: "checkmark.circle.fill")
                    .font(.caption2)
                Text("Confirmed")
                    .font(.caption2)
            }
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(Color.green.opacity(0.2))
            .foregroundColor(.green)
            .cornerRadius(4)
        }
    }

    var body: some View {
        HStack(spacing: 12) {
            // Type icon
            Image(systemName: typeIcon)
                .font(.title2)
                .foregroundColor(typeColor)
                .frame(width: 40)

            VStack(alignment: .leading, spacing: 4) {
                // Transaction ID (truncated) and timestamp
                HStack {
                    Text(transaction.truncatedTxid)
                        .font(.system(.subheadline, design: .monospaced))
                        .foregroundColor(.primary)
    
                        Spacer()
                        
                    Text(transaction.date, style: .relative)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                    
                // confirmation and amount
                HStack {
                  confirmationBadge
                  
                  Spacer()
                  
                  VStack(alignment: .trailing, spacing: 2) {
                      Text(transaction.formattedAmount)
                          .font(.headline)
                          .foregroundColor(typeColor)
      
                      if let fee = transaction.formattedFee, transaction.netAmount < 0 {
                          Text("Fee: \(fee)")
                              .font(.caption2)
                              .foregroundColor(.secondary)
                      }
                  }
                }
            }
        }
        .padding(.vertical, 4)
    }
}

