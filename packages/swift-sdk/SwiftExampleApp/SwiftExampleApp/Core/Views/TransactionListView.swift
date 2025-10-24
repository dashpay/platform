import SwiftUI
import SwiftData
import SwiftDashSDK

struct TransactionListView: View {
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    let wallet: HDWallet

    @State private var transactions: [WalletTransaction] = []
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var showError = false
    @State private var selectedTransaction: WalletTransaction?

    private var sortedTransactions: [WalletTransaction] {
        transactions.sorted { $0.timestamp > $1.timestamp }
    }

    var body: some View {
        ZStack {
            if isLoading {
                ProgressView("Loading transactions...")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if transactions.isEmpty {
                emptyStateView
            } else {
                transactionsList
            }
        }
        .navigationTitle("Transactions")
        .navigationBarTitleDisplayMode(.inline)
        .alert("Error", isPresented: $showError) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(errorMessage ?? "Unknown error occurred")
        }
        .sheet(item: $selectedTransaction) { transaction in
            TransactionDetailView(transaction: transaction)
        }
        .task {
            await loadTransactions()
        }
        .refreshable {
            await loadTransactions()
        }
    }

    private var emptyStateView: some View {
        VStack(spacing: 16) {
            Image(systemName: "doc.text.magnifyingglass")
                .font(.system(size: 60))
                .foregroundColor(.gray)

            Text("No Transactions Yet")
                .font(.headline)

            Text("Transactions will appear here once you send or receive Dash")
                .font(.caption)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var transactionsList: some View {
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

    private func loadTransactions() async {
        isLoading = true
        defer { isLoading = false }

        do {
            // Get wallet manager
            guard let walletManager = walletService.walletManager else {
                throw NSError(domain: "TransactionListView", code: 1,
                            userInfo: [NSLocalizedDescriptionKey: "Wallet manager not initialized"])
            }

            // Get transactions from the wallet manager
            let fetchedTransactions = try await walletManager.getTransactions(for: wallet)

            await MainActor.run {
                self.transactions = fetchedTransactions
            }
        } catch {
            await MainActor.run {
                self.errorMessage = error.localizedDescription
                self.showError = true
            }
        }
    }
}

// MARK: - Transaction Row View

struct TransactionRowView: View {
    let transaction: WalletTransaction

    private var typeIcon: String {
        switch transaction.type {
        case "received":
            return "arrow.down.circle.fill"
        case "sent":
            return "arrow.up.circle.fill"
        case "self":
            return "arrow.triangle.2.circlepath"
        default:
            return "questionmark.circle"
        }
    }

    private var typeColor: Color {
        switch transaction.type {
        case "received":
            return .green
        case "sent":
            return .red
        case "self":
            return .blue
        default:
            return .gray
        }
    }

    @ViewBuilder
    private var confirmationBadge: some View {
        if transaction.confirmations == 0 {
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
            } else if transaction.confirmations < 6 {
                HStack(spacing: 4) {
                    Image(systemName: "checkmark.circle")
                        .font(.caption2)
                    Text("\(transaction.confirmations)")
                        .font(.caption2)
                }
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(Color.blue.opacity(0.2))
                .foregroundColor(.blue)
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
                // Transaction ID (truncated)
                Text(transaction.truncatedTxid)
                    .font(.system(.subheadline, design: .monospaced))
                    .foregroundColor(.primary)

                // Date and confirmation
                HStack(spacing: 8) {
                    Text(transaction.date, style: .relative)
                        .font(.caption)
                        .foregroundColor(.secondary)

                    confirmationBadge
                }
            }

            Spacer()

            // Amount
            VStack(alignment: .trailing, spacing: 2) {
                Text(transaction.formattedAmount)
                    .font(.headline)
                    .foregroundColor(typeColor)

                if let fee = transaction.formattedFee, transaction.type == "sent" {
                    Text("Fee: \(fee)")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(.vertical, 4)
    }
}

