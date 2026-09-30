import SwiftUI
import SwiftDashSDK

struct TransactionDetailView: View {
    let transaction: PersistentTransaction
    var walletId: Data? = nil
    /// `nil` while this wallet's amount is unresolved — the same state the
    /// amount label shows as "Amount unavailable", so fee and amount agree.
    private var netAmount: Int64? { transaction.displayNetAmount(for: walletId) }
    private var direction: UInt32 { transaction.displayDirectionCode(for: walletId) }
    /// Asset-lock payload funding amount, excluding the Core transaction fee.
    var assetLockAmountDuffs: Int64? = nil
    @Environment(\.dismiss) private var dismiss
    @State private var showCopiedAlert = false

    /// Show the lock's funding amount, or the wallet's Core value movement for ordinary transactions.
    private var displayAmount: String? {
        if transaction.isAssetLock {
            if let duffs = assetLockAmountDuffs {
                let dash = Double(duffs) / 100_000_000.0
                return String(format: "-%.8f DASH", dash)
            }
            return "Asset Lock (amount unknown)"
        }
        if transaction.isProviderSpecial && netAmount == 0 {
            return nil
        }
        return transaction.displayFormattedAmount(for: walletId)
    }

    private var typeDescription: String {
        // Special kinds (asset lock/unlock, provider txs) take their
        // label from the model so it can't drift from the list rows.
        if transaction.isAssetLock || transaction.isAssetUnlock
            || transaction.isProviderSpecial {
            return transaction.displayDirection
        }
        switch direction {
        case CoreDirectionCode.incoming:
            return "Received"
        case CoreDirectionCode.outgoing:
            return "Sent"
        case CoreDirectionCode.coinJoin:
            return "CoinJoin"
        default:
            return "Self-Transfer"
        }
    }

    private var typeIcon: String {
        if transaction.isAssetLock { return "lock.fill" }
        if transaction.isAssetUnlock { return "lock.open.fill" }
        if transaction.isProviderSpecial { return "server.rack" }
        return TransactionDirectionStyle.icon(for: direction)
    }

    private var typeColor: Color {
        if transaction.isAssetLock || transaction.isAssetUnlock {
            return .purple
        }
        if transaction.isProviderSpecial {
            return .orange
        }
        return TransactionDirectionStyle.color(for: direction)
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

    /// Masternode registration / service-update details, shown only for
    /// ProRegTx / ProUpServTx rows. All fields come pre-parsed from the
    /// Rust FFI (`PersistentTransaction.provider*`); this view only
    /// renders them. Broken out so `body`'s type-check stays cheap.
    @ViewBuilder
    private var masternodeSection: some View {
        if transaction.isProviderRegistration || transaction.isProviderUpdateService {
            VStack(alignment: .leading, spacing: 16) {
                Text(transaction.isProviderRegistration
                    ? "Masternode Registration"
                    : "Masternode Service Update")
                    .font(.headline)
                    .frame(maxWidth: .infinity, alignment: .leading)

                if let service = transaction.providerServiceAddress {
                    TransactionDetailRow(label: "Service", value: service)
                }
                if let proTxHash = transaction.providerProTxHashHex {
                    copyableHashRow(title: "Pro Tx Hash", value: proTxHash)
                }
                if let collateral = transaction.providerCollateralDisplay {
                    copyableHashRow(title: "Collateral Outpoint", value: collateral)
                }
                if let ownerKeyHash = transaction.providerOwnerKeyHashHex {
                    copyableHashRow(title: "Owner Key Hash", value: ownerKeyHash)
                }
                if let votingKeyHash = transaction.providerVotingKeyHashHex {
                    copyableHashRow(title: "Voting Key Hash", value: votingKeyHash)
                }
            }
        }
    }

    /// Caption + monospaced, tap-to-copy value block — same styling as
    /// the Transaction ID / Block Hash rows.
    @ViewBuilder
    private func copyableHashRow(title: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)

            Button {
                copyToClipboard(value)
            } label: {
                HStack {
                    Text(value)
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

                        if let fee = formattedFee, let amount = netAmount, amount < 0 {
                            TransactionDetailRow(
                                label: "Network Fee",
                                value: fee
                            )
                        }

                        masternodeSection

                        // Transaction ID
                        copyableHashRow(title: "Transaction ID", value: transaction.txidHex)

                        // Block Hash (if available)
                        if let blockHash = blockHashHex {
                            copyableHashRow(title: "Block Hash", value: blockHash)
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
