import SwiftUI
import SwiftDashSDK

struct TransactionDetailView: View {
    let transaction: PersistentTransaction
    /// Override amount for asset-lock txs. The wallet's `netAmount`
    /// shows ~0 for these (credit output is structurally self-owned),
    /// so the list view passes the linked
    /// `PersistentAssetLock.amountDuffs`. `nil` for non-asset-lock
    /// rows OR consumed asset locks whose tracking row was cleaned
    /// up after successful identity registration.
    var assetLockAmountDuffs: Int64? = nil
    @Environment(\.dismiss) private var dismiss
    @State private var showCopiedAlert = false

    /// Amount label rendered prominently at the top of the sheet.
    /// Same precedence rule as the row: asset-lock duffs when we
    /// have them, else an explicit "amount unknown" label for the
    /// historical-asset-lock case (rather than the misleading
    /// `+0.00000000 DASH` from `transaction.formattedAmount`).
    /// `nil` for a payload-only provider special tx — a ProRegTx
    /// observed via the owner/voting keys moves no wallet balance,
    /// and `+0.00000000 DASH` reads as a broken zero-value receive.
    private var displayAmount: String? {
        if transaction.isAssetLock {
            if let duffs = assetLockAmountDuffs {
                let dash = Double(duffs) / 100_000_000.0
                return String(format: "-%.8f DASH", dash)
            }
            return "Asset Lock (amount unknown)"
        }
        if transaction.isProviderSpecial && transaction.netAmount == 0 {
            return nil
        }
        return transaction.formattedAmount
    }

    private var typeDescription: String {
        if transaction.isAssetLock { return "Asset Lock" }
        if transaction.isAssetUnlock { return "Asset Unlock" }
        if let name = transaction.providerSpecialName { return name }
        switch transaction.netAmount {
        case let amount where amount > 0:
            return "Received"
        case let amount where amount < 0:
            return "Sent"
        default:
            return "Self-Transfer"
        }
    }

    private var typeIcon: String {
        if transaction.isAssetLock { return "lock.fill" }
        if transaction.isAssetUnlock { return "lock.open.fill" }
        if transaction.isProviderSpecial { return "server.rack" }
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
        if transaction.isAssetLock || transaction.isAssetUnlock {
            return .purple
        }
        if transaction.isProviderSpecial {
            return .orange
        }
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
        transaction.context >= 2
    }

    private var transactionDate: Date {
        Date(timeIntervalSince1970: TimeInterval(transaction.firstSeen))
    }

    private var blockHashHex: String? {
        guard let bh = transaction.blockHash, !bh.isEmpty else { return nil }
        return bh.map { String(format: "%02x", $0) }.joined()
    }

    private var formattedFee: String? {
        guard let fee = transaction.fee else { return nil }
        let dash = Double(fee) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }

    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 24) {
                    // Header with amount
                    VStack(spacing: 8) {
                        Image(systemName: typeIcon)
                            .font(.system(size: 50))
                            .foregroundColor(typeColor)

                        Text(typeDescription)
                            .font(.headline)
                            .foregroundColor(.secondary)

                        if let displayAmount {
                            Text(displayAmount)
                                .font(.system(size: 32, weight: .bold, design: .rounded))
                                .foregroundColor(typeColor)
                        }
                    }
                    .padding(.top, 20)

                    // Transaction Details
                    VStack(spacing: 16) {
                        TransactionDetailRow(
                            label: "Status",
                            value: isConfirmed ? "Confirmed" : "Pending"
                        )

                        TransactionDetailRow(
                            label: "Date",
                            value: formatDate(transactionDate)
                        )

                        if transaction.blockHeight != 0 {
                            TransactionDetailRow(
                                label: "Block Height",
                                value: "\(transaction.blockHeight)"
                            )
                        }

                        if let fee = formattedFee, transaction.netAmount < 0 {
                            TransactionDetailRow(
                                label: "Network Fee",
                                value: fee
                            )
                        }

                        // Transaction ID
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Transaction ID")
                                .font(.caption)
                                .foregroundColor(.secondary)

                            Button {
                                copyToClipboard(transaction.txidHex)
                            } label: {
                                HStack {
                                    Text(transaction.txidHex)
                                        .font(.system(.footnote, design: .monospaced))
                                        .foregroundColor(.primary)
                                        .lineLimit(nil)
                                        .fixedSize(horizontal: false, vertical: true)

                                    Spacer()

                                    Image(systemName: "doc.on.doc")
                                        .font(.caption)
                                        .foregroundColor(.blue)
                                }
                                .padding()
                                .background(Color(UIColor.secondarySystemBackground))
                                .cornerRadius(8)
                            }
                        }

                        // Block Hash (if available)
                        if let blockHash = blockHashHex {
                            VStack(alignment: .leading, spacing: 8) {
                                Text("Block Hash")
                                    .font(.caption)
                                    .foregroundColor(.secondary)

                                Button {
                                    copyToClipboard(blockHash)
                                } label: {
                                    HStack {
                                        Text(blockHash)
                                            .font(.system(.footnote, design: .monospaced))
                                            .foregroundColor(.primary)
                                            .lineLimit(nil)
                                            .fixedSize(horizontal: false, vertical: true)

                                        Spacer()

                                        Image(systemName: "doc.on.doc")
                                            .font(.caption)
                                            .foregroundColor(.blue)
                                    }
                                    .padding()
                                    .background(Color(UIColor.secondarySystemBackground))
                                    .cornerRadius(8)
                                }
                            }
                        }
                    }
                    .padding(.horizontal)
                }
            }
            .navigationTitle("Transaction Details")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
        }
        .overlay(alignment: .top) {
            if showCopiedAlert {
                HStack {
                    Image(systemName: "checkmark.circle.fill")
                        .foregroundColor(.green)
                    Text("Copied to clipboard")
                        .font(.subheadline)
                }
                .padding()
                .background(Color(UIColor.systemBackground))
                .cornerRadius(10)
                .shadow(radius: 10)
                .padding(.top, 50)
                .transition(.move(edge: .top).combined(with: .opacity))
            }
        }
    }

    private func formatDate(_ date: Date) -> String {
        let formatter = DateFormatter.gregorian()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }

    private func copyToClipboard(_ text: String) {
        UIPasteboard.general.string = text

        withAnimation {
            showCopiedAlert = true
        }

        // Hide alert after 2 seconds
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
            withAnimation {
                showCopiedAlert = false
            }
        }
    }
}

// MARK: - Detail Row

struct TransactionDetailRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack {
            Text(label)
                .font(.subheadline)
                .foregroundColor(.secondary)

            Spacer()

            Text(value)
                .font(.subheadline)
                .fontWeight(.medium)
                .foregroundColor(.primary)
        }
        .padding(.horizontal)
    }
}
