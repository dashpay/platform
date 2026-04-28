import SwiftUI
import SwiftData
import SwiftDashSDK

struct TransactionListView: View {
    let wallet: PersistentWallet

    @Query private var transactions: [PersistentTransaction]
    @State private var selectedTransaction: PersistentTransaction?

    init(wallet: PersistentWallet) {
        self.wallet = wallet
        let walletId = wallet.walletId
        // Use the denormalized `PersistentTransaction.walletId`
        // column rather than chaining `tx.account?.wallet?.walletId`.
        // SwiftData's predicate compiler can't lower a double
        // optional-relationship chain to SQLite and crashes with
        // `Unsupported function expression TERNARY(...).walletId`.
        //
        // `propertiesToFetch` matters: without it, a wallet with
        // ~1.5k transactions stalls the navigation push for several
        // seconds because SwiftData hydrates the full row — including
        // the `transactionData` raw-bytes blob — for every match on
        // the main thread before the List can render. Restricting
        // the fetch to the columns the row actually reads
        // (`TransactionRowView` only touches txid / netAmount /
        // firstSeen / context / fee, plus walletId for the predicate)
        // keeps SQLite on an index-only scan against the
        // `(walletId, firstSeen)` compound index and skips the blob.
        var descriptor = FetchDescriptor<PersistentTransaction>(
            predicate: #Predicate { tx in tx.walletId == walletId },
            sortBy: [SortDescriptor(\PersistentTransaction.firstSeen, order: .reverse)]
        )
        descriptor.propertiesToFetch = [
            \.txid,
            \.netAmount,
            \.firstSeen,
            \.context,
            \.fee,
            \.walletId,
        ]
        _transactions = Query(descriptor)
    }

    var body: some View {
        ZStack {
            if transactions.isEmpty {
                emptyStateView
            } else {
                transactionsList
            }
        }
        .navigationTitle("Transactions")
        .navigationBarTitleDisplayMode(.inline)
        .sheet(item: $selectedTransaction) { transaction in
            TransactionDetailView(transaction: transaction)
        }
    }

    private var emptyStateView: some View {
        VStack(spacing: 16) {
            Image(systemName: "doc.text.magnifyingglass")
                .font(.system(size: 60))
                .foregroundColor(.gray)

            Text("No transactions found.")
                .font(.headline)

            Text("Transactions will appear here once you send or receive Dash")
                .font(.caption)
                .foregroundColor(.gray)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var transactionsList: some View {
        // Use SwiftData's built-in PersistentIdentifier as the row
        // identity (via `List(transactions)`) instead of `id: \.txid`.
        // The Storage Explorer's transaction list uses the same shape
        // and renders instantly; keying on `txid` forces SwiftUI to
        // read the unique-string column from every row for diffing
        // even when SwiftData already has a stable identity for free.
        List(transactions) { transaction in
            Button {
                selectedTransaction = transaction
            } label: {
                TransactionRowView(transaction: transaction)
            }
            .buttonStyle(.plain)
        }
        .listStyle(.insetGrouped)
    }
}

// MARK: - Transaction Row View

struct TransactionRowView: View {
    let transaction: PersistentTransaction

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

    private var isConfirmed: Bool {
        // context: 0=mempool, 1=instantSend, 2=inBlock, 3=inChainLockedBlock
        transaction.context >= 2
    }

    private var truncatedTxid: String {
        let txid = transaction.txid
        guard txid.count > 16 else { return txid }
        return "\(txid.prefix(8))…\(txid.suffix(8))"
    }

    private var transactionDate: Date {
        Date(timeIntervalSince1970: TimeInterval(transaction.firstSeen))
    }

    @ViewBuilder
    private var confirmationBadge: some View {
        if !isConfirmed {
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
                    Text(truncatedTxid)
                        .font(.system(.subheadline, design: .monospaced))
                        .foregroundColor(.primary)

                    Spacer()

                    Text(transactionDate, style: .relative)
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

                        if let fee = transaction.fee, transaction.netAmount < 0 {
                            Text("Fee: \(formatFee(fee))")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                        }
                    }
                }
            }
        }
        .padding(.vertical, 4)
    }

    private func formatFee(_ fee: UInt64) -> String {
        let dash = Double(fee) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
}
