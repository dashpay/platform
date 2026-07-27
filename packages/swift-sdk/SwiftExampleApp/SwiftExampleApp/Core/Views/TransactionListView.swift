import SwiftUI
import SwiftData
import SwiftDashSDK

struct TransactionListView: View {
    /// Per-wallet transaction list. Queries `PersistentTxo` flat by
    /// the denormalized `walletId` column and resolves distinct
    /// creating-or-spending `PersistentTransaction`s in the body,
    /// then unions in each account's payload-only
    /// `involvedTransactions` (special txs that matched an account by
    /// payload and produced no TXO, so the TXO join misses them) —
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
    /// This wallet's accounts, for the payload-only
    /// `involvedTransactions` union below — special txs that matched an
    /// account by payload with no TXO in the wallet, invisible to the
    /// `walletTxos` join.
    @Query private var walletAccounts: [PersistentAccount]
    @Query private var transactionObservation: [PersistentTransaction]
    /// Per-wallet asset-lock rows. Used to look up the *locked* amount
    /// for each asset-lock tx — `PersistentTransaction.netAmount` is
    /// the wallet's input-vs-output diff, which sees the credit
    /// output as "to-self" and reports ~0 for asset locks. The
    /// `amountDuffs` on the asset-lock row is the actual L1 burn.
    @Query private var assetLocks: [PersistentAssetLock]
    /// This wallet's owning identities. The DashPay payment / contact
    /// join below must be scoped to these — two identities in one store
    /// (A pays B) each persist a row for the same txid, and joining by
    /// txid alone would let B's incoming payment resolve to A's `.sent`
    /// row (or the wrong counterparty name). `ownerIdentityIds` narrows
    /// the join to this wallet's identities, leaving at most one payment
    /// row per txid.
    @Query private var walletIdentities: [PersistentIdentity]
    /// DashPay payments + the contact-name sources, to label a tx that is a
    /// DashPay payment with who it's with (+ memo). Scoped to this wallet's
    /// identities in the joins below.
    @Query private var dashpayPayments: [PersistentDashpayPayment]
    @Query private var contactProfiles: [PersistentDashpayContactProfile]
    @Query private var contactRequests: [PersistentDashpayContactRequest]
    @State private var selectedTransaction: PersistentTransaction?

    init(walletId: Data) {
        self.walletId = walletId
        let txoDescriptor = FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        _walletTxos = Query(txoDescriptor)
        _walletAccounts = Query(
            filter: #Predicate<PersistentAccount> { $0.wallet.walletId == walletId }
        )
        let assetLockDescriptor = FetchDescriptor<PersistentAssetLock>(
            predicate: PersistentAssetLock.predicate(walletId: walletId)
        )
        _assetLocks = Query(assetLockDescriptor)
        _walletIdentities = Query(
            filter: #Predicate<PersistentIdentity> { $0.wallet?.walletId == walletId }
        )
    }

    /// The owning identity ids of this wallet — the scope for the
    /// DashPay payment / contact joins below.
    private var ownerIdentityIds: Set<Data> {
        Set(walletIdentities.map(\.identityId))
    }

    /// Lookup `txid (display-order hex) → total asset-lock amount in
    /// duffs`, built once per @Query invalidation so the row's amount
    /// label can swap in the real funded amount without re-fetching.
    ///
    /// `PersistentAssetLock.outPointHex` is `"<txidHex>:<vout>"`. A
    /// single funding tx can carry multiple credit outputs (DIP-0027
    /// allows up to 255), each becoming its own `PersistentAssetLock`
    /// row at a distinct `vout`. We sum amounts across all rows
    /// sharing a txid so the list shows the *total* DASH burned by
    /// that funding tx, not whichever row SwiftData happened to
    /// enumerate last.
    private var assetLockAmountByTxid: [String: Int64] {
        var map: [String: Int64] = [:]
        for lock in assetLocks {
            let parts = lock.outPointHex.split(separator: ":", maxSplits: 1)
            guard let txidHex = parts.first.map(String.init) else { continue }
            map[txidHex, default: 0] += lock.amountDuffs
        }
        return map
    }

    /// `txid (display-order hex) → DashPay payment`, so a row can tell whether
    /// the tx it's rendering is a DashPay payment (and to whom). Scoped to
    /// this wallet's identities so a txid another identity in the same store
    /// also recorded (A pays B) can't resolve to the wrong direction/row.
    private var dashpayPaymentByTxid: [String: PersistentDashpayPayment] {
        let owners = ownerIdentityIds
        let scoped = dashpayPayments.filter { owners.contains($0.ownerIdentityId) }
        return Dictionary(scoped.map { ($0.txid, $0) }, uniquingKeysWith: { first, _ in first })
    }

    /// Resolve a counterparty identity id → best display name (alias > profile
    /// > DPNS > truncated id), reusing the shared DashPay name helper. Scoped
    /// to this wallet's identities so a contact row owned by a sibling
    /// identity can't leak the wrong alias/name.
    private func dashpayCounterpartyName(for contactId: Data) -> String {
        let owners = ownerIdentityIds
        let alias = contactRequests.first {
            owners.contains($0.ownerIdentityId)
                && $0.contactIdentityId == contactId
                && ($0.contactAlias?.isEmpty == false)
        }?.contactAlias
        let profileName = contactProfiles.first {
            owners.contains($0.ownerIdentityId) && $0.contactIdentityId == contactId
        }?.displayName
        return dashPayContactDisplayName(
            contactId: contactId,
            alias: alias,
            profileDisplayName: profileName,
            dpnsLabel: nil
        )
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
        // Payload-only involvement: special txs that matched one of
        // this wallet's accounts by payload (provider owner / voting
        // key address) with no TXO, so the `walletTxos` join above
        // never sees them. De-dup by txid against the funded set.
        for account in walletAccounts {
            for tx in account.involvedTransactions where seen.insert(tx.txid).inserted {
                result.append(tx)
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
            TransactionDetailView(
                transaction: transaction,
                assetLockAmountDuffs: assetLockAmountByTxid[transaction.txidHex]
            )
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
        let assetLockAmounts = assetLockAmountByTxid
        let payments = dashpayPaymentByTxid
        return List(transactions) { transaction in
            let payment = payments[transaction.txidHex]
            Button {
                selectedTransaction = transaction
            } label: {
                TransactionRowView(
                    transaction: transaction,
                    assetLockAmountDuffs: assetLockAmounts[transaction.txidHex],
                    dashpayPayment: payment,
                    dashpayCounterpartyName: payment.map {
                        dashpayCounterpartyName(for: $0.counterpartyIdentityId)
                    }
                )
            }
            .buttonStyle(.plain)
        }
        .listStyle(.insetGrouped)
    }
}

// MARK: - Transaction Row View

struct TransactionRowView: View {
    let transaction: PersistentTransaction
    /// Override amount displayed for asset-lock rows. The wallet's
    /// `netAmount` shows ~0 for these (credit output is structurally
    /// self-owned), so the list view passes the linked
    /// `PersistentAssetLock.amountDuffs` — the actual L1 DASH burned
    /// to mint platform credits. `nil` for non-asset-lock rows or
    /// when no matching row was found.
    var assetLockAmountDuffs: Int64? = nil
    /// The DashPay payment this tx belongs to, if any — joined by `txid` in
    /// `TransactionListView`. When set, the row shows the contact context
    /// (who + memo + a DashPay badge) instead of a bare txid; a DashPay
    /// payment is otherwise an ordinary `standard` core tx, indistinguishable
    /// from a plain send.
    var dashpayPayment: PersistentDashpayPayment? = nil
    /// The counterparty's resolved display name (alias / profile / DPNS), or
    /// nil to fall through to a truncated identity id.
    var dashpayCounterpartyName: String? = nil

    private var isDashPay: Bool { dashpayPayment != nil }

    private var typeIcon: String {
        // A DashPay payment is a person-to-person send/receive — mark it with
        // a contact glyph so it reads differently from a raw on-chain tx.
        if isDashPay { return "person.crop.circle.fill" }
        // Asset-lock / asset-unlock txs override direction-based icons
        // since the `direction` classifier reports `Internal` (the
        // credit output is structurally self-owned), but the intent
        // is L1→L2 / L2→L1 credit conversion — neither "send" nor
        // "receive" applies cleanly.
        if transaction.isAssetLock { return "lock.fill" }
        if transaction.isAssetUnlock { return "lock.open.fill" }
        // Provider special txs (ProRegTx / ProUp*Tx) also classify as
        // `Internal` — the wallet just sees its own owner/voting/payout
        // keys in the payload — so the self-transfer arrows would lie.
        if transaction.isProviderSpecial { return "server.rack" }
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
        // DashPay rows are indigo — a third axis distinct from the
        // green/red send-receive and the purple asset-lock rows.
        if isDashPay { return .indigo }
        // Asset-lock txs render purple — distinct from the red
        // outgoing / green incoming axis so the user can scan the
        // list and immediately spot identity-funding rows.
        if transaction.isAssetLock || transaction.isAssetUnlock {
            return .purple
        }
        // Provider special txs get their own axis too — orange, so a
        // masternode registration doesn't scan as a red "sent" row.
        if transaction.isProviderSpecial {
            return .orange
        }
        switch transaction.direction {
        case 0: return .green
        case 1, 2: return .red
        case 3: return .blue
        default: return .secondary
        }
    }

    /// Primary label: the contact context for a DashPay payment, else the
    /// truncated txid.
    private var primaryText: String {
        guard let payment = dashpayPayment else { return truncatedTxid }
        let verb = payment.direction == .sent ? "Sent to" : "Received from"
        return "\(verb) \(dashpayCounterpartyName ?? "contact")"
    }

    @ViewBuilder
    private var dashPayBadge: some View {
        HStack(spacing: 4) {
            Image(systemName: "person.2.fill")
                .font(.caption2)
            Text("DashPay")
                .font(.caption2)
        }
        .padding(.horizontal, 6)
        .padding(.vertical, 2)
        .background(Color.indigo.opacity(0.2))
        .foregroundColor(.indigo)
        .cornerRadius(4)
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
                // DashPay payment → contact context; otherwise the txid.
                HStack {
                    Text(primaryText)
                        .font(isDashPay ? .subheadline : .system(.subheadline, design: .monospaced))
                        .foregroundColor(.primary)
                        .lineLimit(1)

                    Spacer()

                    Text(transactionDate, style: .relative)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                // DashPay memo (if the sender attached one).
                if let memo = dashpayPayment?.memo,
                    !memo.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    Text(memo)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }

                // confirmation and amount
                HStack {
                    confirmationBadge

                    if isDashPay {
                        dashPayBadge
                    }

                    Spacer()

                    VStack(alignment: .trailing, spacing: 2) {
                        Text(displayAmount)
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

    /// Amount label for the row. For asset-lock txs we substitute
    /// the linked `PersistentAssetLock.amountDuffs` (the L1 DASH
    /// actually burned to mint platform credits); the wallet's
    /// `netAmount` is ~0 for these because the credit output is a
    /// self-owned address. Rendered as a negative (DASH leaving L1).
    ///
    /// If we know the row is an asset lock but the linked
    /// `PersistentAssetLock` is missing (e.g. a historical record
    /// from before the `Consumed`-status retention change shipped),
    /// we render "Asset Lock (amount unknown)" instead of falling
    /// through to `transaction.formattedAmount` — that would say
    /// `+0.00000000 DASH`, which is misleading for a row the user
    /// can see was a funding tx.
    private var displayAmount: String {
        if transaction.isAssetLock {
            if let duffs = assetLockAmountDuffs {
                let dash = Double(duffs) / 100_000_000.0
                return String(format: "-%.8f DASH", dash)
            }
            return "Asset Lock (amount unknown)"
        }
        // A payload-only provider special tx moves no wallet balance;
        // `+0.00000000 DASH` reads as a broken zero-value receive, so
        // put the tx kind in the amount slot instead. A provider tx
        // that DOES move value (e.g. this wallet funded the collateral)
        // falls through and shows the real signed amount.
        if transaction.isProviderSpecial && transaction.netAmount == 0 {
            return transaction.providerSpecialName ?? transaction.transactionType
        }
        return transaction.formattedAmount
    }
}
