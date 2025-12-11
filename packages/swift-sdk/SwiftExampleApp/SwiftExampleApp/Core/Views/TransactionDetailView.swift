import SwiftUI
import SwiftDashSDK

struct TransactionDetailView: View {
    let transaction: WalletTransaction
    @Environment(\.dismiss) private var dismiss
    @State private var showCopiedAlert = false

    private var typeDescription: String {
        switch transaction.type {
        case "received":
            return "Received"
        case "sent":
            return "Sent"
        case "self":
            return "Self-Transfer"
        default:
            return "Unknown"
        }
    }

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
                            value: transaction.confirmations == 0 ? "Pending" :
                                   transaction.confirmations < 6 ? "\(transaction.confirmations) confirmations" :
                                   "Confirmed"
                        )

                        TransactionDetailRow(
                            label: "Date",
                            value: formatDate(transaction.date)
                        )

                        if let height = transaction.height {
                            TransactionDetailRow(
                                label: "Block Height",
                                value: "\(height)"
                            )
                        }

                        if let fee = transaction.formattedFee, transaction.type == "sent" {
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
                        if let blockHash = transaction.blockHash {
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
        let formatter = DateFormatter()
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

// MARK: - Preview

#Preview {
    TransactionDetailView(
        transaction: WalletTransaction(
            txid: "abc123def456abc123def456abc123def456abc123def456abc123def456abc1",
            netAmount: 50000000,
            height: 12345,
            blockHash: "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f",
            timestamp: UInt64(Date().timeIntervalSince1970),
            fee: 226,
            confirmations: 6,
            type: "received",
            isOurs: false
        )
    )
}
