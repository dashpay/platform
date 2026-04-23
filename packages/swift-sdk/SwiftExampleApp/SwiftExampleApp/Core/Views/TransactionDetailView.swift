import SwiftUI
import SwiftDashSDK

struct TransactionDetailView: View {
    let transaction: PersistentTransaction
    @Environment(\.dismiss) private var dismiss
    @State private var showCopiedAlert = false

    private var typeDescription: String {
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

                        Text(transaction.formattedAmount)
                            .font(.system(size: 32, weight: .bold, design: .rounded))
                            .foregroundColor(typeColor)
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
                                copyToClipboard(transaction.txid)
                            } label: {
                                HStack {
                                    Text(transaction.txid)
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
