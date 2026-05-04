import SwiftUI
import SwiftData
import SwiftDashSDK

struct TransactionListView: View {
    /// Per-wallet transaction list. Queries `PersistentTxo` flat by
    /// the denormalized `walletId` column and resolves distinct
    /// creating-or-spending `PersistentTransaction`s in the body —
    /// same union `WalletDetailView`'s count uses.
    ///
    /// Reached via value-based navigation (see
    /// `WalletsContentView`'s `.navigationDestination` modifiers).
    /// Closure-based `NavigationLink { Destination }` is unusable on
    /// iOS 26 here — the eager destination construction stalls the
    /// push when the destination has any meaningful `init` or
    /// `@Query`. Value-based push only constructs the destination
    /// at navigate time.
    let walletId: Data
    /// Membership: which txids belong to this wallet, via the
    /// denormalized `walletId` on TXOs.
    @Query private var walletTxos: [PersistentTxo]
    @Query private var transactionObservation: [PersistentTransaction]
    @State private var selectedTransaction: PersistentTransaction?

    init(walletId: Data) {
        self.walletId = walletId
        let descriptor = FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        _walletTxos = Query(descriptor)
    }

    private var transactions: [PersistentTransaction] {
        _ = transactionObservation // keep the subscription alive
        var seen: Set<Data> = []
        var result: [PersistentTransaction] = []
        for txo in walletTxos {
            if let tx = txo.transaction, seen.insert(tx.txid).inserted {
                result.append(tx)
            }
            if let spending = txo.spendingTransaction, seen.insert(spending.txid).inserted {
                result.append(spending)
            }
        }
        return result.sorted { lhs, rhs in
            if (lhs.context == 0) != (rhs.context == 0) {
                return lhs.context == 0
            }
            return lhs.firstSeen > rhs.firstSeen
        }
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
        // direction: 0=incoming, 1=outgoing, 2=internal, 3=coinJoin
        switch transaction.direction {
        case 0: return "arrow.down.circle.fill"
        case 1: return "arrow.up.circle.fill"
        case 2: return "arrow.triangle.2.circlepath"
        case 3: return "shuffle.circle.fill"
        default: return "questionmark.circle"
        }
    }

    private var typeColor: Color {
        switch transaction.direction {
        case 0: return .green
        case 1, 2: return .red
        case 3: return .blue
        default: return .secondary
        }
    }

    private var isConfirmed: Bool {
        // context: 0=mempool, 1=instantSend, 2=inBlock, 3=inChainLockedBlock
        transaction.context >= 2
    }

    private var truncatedTxid: String {
        let txid = transaction.txidHex
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
