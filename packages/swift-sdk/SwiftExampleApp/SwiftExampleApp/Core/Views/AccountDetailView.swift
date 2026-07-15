import SwiftUI
import SwiftDashSDK
import SwiftData
import UIKit

// MARK: - Account Detail View
struct AccountDetailView: View {
    @EnvironmentObject var appUIState: AppUIState
    @EnvironmentObject var walletManager: PlatformWalletManager
    let wallet: PersistentWallet
    let account: PersistentAccount

    /// Masternodes for the provider-key-address usage subtitle. Matched
    /// by base58 address (Rust-encoded owner/voting address ↔ the
    /// account's persisted address), so no key hashing happens in Swift.
    @Query private var allMasternodes: [PersistentMasternode]

    @State private var errorMessage: String?
    @State private var copiedText: String?
    @State private var showingPrivateKey: String?
    @State private var privateKeyToShow: (hex: String, wif: String)?
    @State private var showingPINPrompt = false
    @State private var pinInput = ""

    // MARK: Provider derived-keys state
    /// The #0..#19 keys derived from a provider account's extended
    /// public key. Empty until loaded — eagerly for operator/BLS on
    /// appear, straight from the persisted batch for platform-node/Ed25519
    /// wallets registered after pre-derivation shipped, and lazily behind
    /// a button for older platform-node wallets (see `derivedKeysCard`).
    @State private var derivedKeys: [ManagedPlatformWallet.ProviderDerivedKey] = []
    @State private var derivedKeysLoaded = false
    @State private var isLoadingDerivedKeys = false
    @State private var derivedKeysError: String?
    /// Per-index revealed private-key hex, populated only after the user
    /// confirms a reveal for that row.
    @State private var revealedPrivateKeys: [UInt32: String] = [:]
    /// Which row's reveal confirmation dialog is open (`nil` = none).
    @State private var revealConfirmIndex: UInt32?
    /// Which row is mid-reveal, to disable buttons and show progress.
    @State private var revealingIndex: UInt32?
    /// Copy-key of the derived-keys row just copied, for a transient
    /// "Copied" confirmation.
    @State private var derivedCopiedKey: String?

    /// The transaction sheet currently presented from the Transactions
    /// card (`nil` = none). Mirrors `TransactionListView`'s pattern.
    @State private var selectedTransaction: PersistentTransaction?

    /// Distinct transactions this account participates in, as display
    /// rows: the **union** of the TXO-derived set (every TXO's creating
    /// tx + spending tx) and the payload-only `involvedTransactions`
    /// join. Lives here rather than on the model because
    /// `PersistentTransaction` is not account-scoped (a single tx can
    /// produce outputs into multiple accounts), so the per-account set
    /// is derived on demand.
    ///
    /// The TXO walk covers funded involvement (canonical path:
    /// `coreAddresses.flatMap(\.txos)`); `involvedTransactions` adds the
    /// payload-only special txs that produced no TXO here and are
    /// therefore invisible to that walk. De-dup is by `txid` so a tx
    /// that is both funded and payload-linked appears once.
    ///
    /// Sorted to match `TransactionListView`: unconfirmed (context 0)
    /// first, then by `firstSeen` descending.
    private var distinctTransactions: [PersistentTransaction] {
        var seen: Set<Data> = []
        var result: [PersistentTransaction] = []
        func add(_ tx: PersistentTransaction) {
            if seen.insert(tx.txid).inserted { result.append(tx) }
        }
        for address in account.coreAddresses {
            for txo in address.txos {
                if let tx = txo.transaction { add(tx) }
                if let spending = txo.spendingTransaction { add(spending) }
            }
        }
        for tx in account.involvedTransactions { add(tx) }
        return result.sorted { lhs, rhs in
            if (lhs.context == 0) != (rhs.context == 0) {
                return lhs.context == 0
            }
            return lhs.firstSeen > rhs.firstSeen
        }
    }

    private var distinctTransactionCount: Int {
        distinctTransactions.count
    }

    /// Total TXO count for the account, summed across address
    /// buckets. Avoids materializing a single big array since the
    /// address pool is bounded (~gap limit) and txos-per-address
    /// is small.
    private var txoCount: Int {
        account.coreAddresses.reduce(0) { $0 + $1.txos.count }
    }

    var body: some View {
        ScrollView {
            if let error = errorMessage {
                ContentUnavailableView(
                    "Failed to Load Details",
                    systemImage: "exclamationmark.triangle",
                    description: Text(error)
                )
            } else {
                VStack(alignment: .leading, spacing: 20) {
                    accountOverviewCard()

                    if isProviderKeyAccount {
                        // Provider operator (BLS) / platform-node (EdDSA)
                        // key accounts hold key material, not on-chain
                        // addresses or a balance — surface the extended
                        // public key instead of an empty address pool.
                        extendedPublicKeyCard()
                        // ...and the per-index keys derived from it (the
                        // actual operator / platform-node keys).
                        derivedKeysCard()
                        // Operator-key accounts hold no TXOs, but a
                        // ProRegTx can still match them payload-only
                        // (provider owner / voting key address), so
                        // surface those txs here too.
                        transactionsCard()
                    } else {
                        if shouldShowBalance {
                            balanceCard()
                        }

                        poolSummaryCard()

                        if account.accountType == 14 {
                            // PlatformPayment accounts keep their address
                            // list in `platformAddresses`, with no
                            // external/internal pool split.
                            let sorted = account.platformAddresses.sorted {
                                $0.addressIndex < $1.addressIndex
                            }
                            if sorted.isEmpty {
                                emptyAddressesCard()
                            } else {
                                platformAddressListCard(addresses: sorted)
                            }
                        } else {
                            ForEach(addressSections(), id: \.0) { name, addresses in
                                addressListCard(
                                    name: name,
                                    systemImage: poolIcon(for: name),
                                    addresses: addresses
                                )
                            }

                            if account.coreAddresses.isEmpty {
                                emptyAddressesCard()
                            }
                        }

                        transactionsCard()
                    }
                }
                .padding()
            }
        }
        .navigationTitle(account.accountTypeName)
        .navigationBarTitleDisplayMode(.large)
        .sheet(item: $selectedTransaction) { transaction in
            TransactionDetailView(transaction: transaction)
        }
        .sheet(isPresented: $showingPINPrompt) {
            PINPromptView(
                pinInput: $pinInput,
                isPresented: $showingPINPrompt,
                onSubmit: {
                    Task {
                        await derivePrivateKeyWithPIN()
                        pinInput = ""
                    }
                }
            )
        }
        .onAppear { appUIState.showWalletsSyncDetails = false }
    }

    // MARK: - Cards

    private func accountOverviewCard() -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("Account Information", systemImage: "info.circle.fill")
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Type:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text(account.accountTypeName)
                        .fontWeight(.medium)
                }

                HStack {
                    Text("Index:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("#\(account.accountIndex)")
                        .font(.system(.body, design: .monospaced))
                }

                HStack {
                    Text("Network:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text(wallet.network?.displayName ?? "Unknown")
                        .fontWeight(.medium)
                }
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    private func balanceCard() -> some View {
        if account.accountType == 14 {
            return AnyView(platformBalanceCard())
        }

        let balances = walletManager.accountBalances(for: wallet.walletId)
        let match = balances.first { b in
            UInt32(b.typeTag) == account.accountType &&
            b.standardTag == account.standardTag &&
            b.index == account.accountIndex
        }
        let confirmed = match?.confirmed ?? 0
        let unconfirmed = match?.unconfirmed ?? 0

        return AnyView(VStack(alignment: .leading, spacing: 12) {
            Label("Balance", systemImage: "bitcoinsign.circle.fill")
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            HStack(spacing: 20) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Confirmed")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(formatBalance(confirmed))
                        .font(.title3)
                        .fontWeight(.semibold)
                }

                Spacer()

                if unconfirmed > 0 {
                    VStack(alignment: .trailing, spacing: 4) {
                        Text("Pending")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text(formatBalance(unconfirmed))
                            .font(.title3)
                            .fontWeight(.semibold)
                            .foregroundColor(.orange)
                    }
                }
            }

            Divider()

            HStack {
                Text("Total Balance")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Text(formatBalance(confirmed + unconfirmed))
                    .font(.headline)
                    .fontWeight(.bold)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2))
    }

    private func platformBalanceCard() -> some View {
        let total = account.platformAddresses.reduce(0) { $0 + $1.balance }
        return VStack(alignment: .leading, spacing: 12) {
            Label("Balance", systemImage: "creditcard")
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            HStack {
                Text("Platform Credits")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Text(formatCredits(total))
                    .font(.title3)
                    .fontWeight(.semibold)
            }

            HStack {
                Text("Raw Credits")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Text("\(total)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    private func poolSummaryCard() -> some View {
        // DIP-17 PlatformPayment pools are flat — there's no
        // external/internal split, no on-chain transactions, and no
        // UTXOs. Surface only address-pool cardinality instead.
        if account.accountType == 14 {
            return AnyView(platformPoolSummaryCard())
        }

        let externalCount = account.coreAddresses.filter { $0.poolTypeTag == 0 }.count
        let internalCount = account.coreAddresses.filter { $0.poolTypeTag == 1 }.count
        return AnyView(VStack(alignment: .leading, spacing: 12) {
            Label("Address Pool", systemImage: "square.stack.3d.up.fill")
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            if externalCount > 0 {
                HStack {
                    Text("Pool Size (External):")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("\(externalCount)")
                        .fontWeight(.medium)
                }
            }
            if internalCount > 0 {
                HStack {
                    Text("Pool Size (Internal):")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("\(internalCount)")
                        .fontWeight(.medium)
                }
            }
            HStack {
                Text("Highest Used (External):")
                    .foregroundColor(.secondary)
                Spacer()
                Text(account.externalHighestUsed >= 0
                     ? "\(account.externalHighestUsed)" : "—")
                    .fontWeight(.medium)
            }
            HStack {
                Text("Highest Used (Internal):")
                    .foregroundColor(.secondary)
                Spacer()
                Text(account.internalHighestUsed >= 0
                     ? "\(account.internalHighestUsed)" : "—")
                    .fontWeight(.medium)
            }
            HStack {
                Text("Transactions:")
                    .foregroundColor(.secondary)
                Spacer()
                // Per-account transaction count = distinct creating
                // and spending txs across this account's TXOs. The
                // per-tx wallet/account columns are gone; the union
                // is computed in Swift each time the card renders.
                Text("\(distinctTransactionCount)")
                    .fontWeight(.medium)
            }
            HStack {
                Text("TXOs:")
                    .foregroundColor(.secondary)
                Spacer()
                Text("\(txoCount)")
                    .fontWeight(.medium)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2))
    }

    /// Platform-payment-specific pool summary. Shows total vs used
    /// addresses plus the highest-seen index, and omits external /
    /// internal / transactions / UTXOs (not meaningful for DIP-17).
    private func platformPoolSummaryCard() -> some View {
        let total = account.platformAddresses.count
        let used = account.platformAddresses.filter { $0.isUsed }.count
        let highest = account.platformAddresses
            .filter { $0.isUsed }
            .map { $0.addressIndex }
            .max()
        return VStack(alignment: .leading, spacing: 12) {
            Label("Address Pool", systemImage: "square.stack.3d.up.fill")
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            HStack {
                Text("Total Addresses:")
                    .foregroundColor(.secondary)
                Spacer()
                Text("\(total)")
                    .fontWeight(.medium)
            }
            HStack {
                Text("Addresses Used:")
                    .foregroundColor(.secondary)
                Spacer()
                Text("\(used)")
                    .fontWeight(.medium)
            }
            HStack {
                Text("Highest Used:")
                    .foregroundColor(.secondary)
                Spacer()
                Text(highest.map { "\($0)" } ?? "—")
                    .fontWeight(.medium)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    /// Lists this account's distinct transactions (see
    /// `distinctTransactions`) — the union of TXO-derived and
    /// payload-only involvement — each tappable to open its detail
    /// sheet. Rendered in both the provider-key and address-pool
    /// branches since operator-key accounts can be payload-matched
    /// too; skipped for PlatformPayment (tag 14), which has no core
    /// txs, and hidden entirely when the list is empty.
    @ViewBuilder
    private func transactionsCard() -> some View {
        let txs = distinctTransactions
        if account.accountType != 14, !txs.isEmpty {
            VStack(alignment: .leading, spacing: 12) {
                Label("Transactions (\(txs.count))", systemImage: "list.bullet.rectangle")
                    .font(.headline)
                    .foregroundColor(.primary)

                Divider()

                ForEach(Array(txs.enumerated()), id: \.element.txid) { idx, tx in
                    Button {
                        selectedTransaction = tx
                    } label: {
                        TransactionRowView(transaction: tx)
                    }
                    .buttonStyle(.plain)
                    if idx < txs.count - 1 {
                        Divider()
                    }
                }
            }
            .padding()
            .background(Color(.systemBackground))
            .cornerRadius(12)
            .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
        }
    }

    /// Group this account's persisted addresses by pool tag, in a
    /// stable display order (External → Internal → Additional →
    /// Additional Hardened). Empty pools are skipped. Names come from
    /// `PersistentCoreAddress.poolTypeName` so all address lists share
    /// one taxonomy (tags 2/3 are the on-demand "Additional" pools where
    /// provider keys live — no Rust "Absent" jargon).
    private func addressSections() -> [(String, [PersistentCoreAddress])] {
        let grouped = Dictionary(grouping: account.coreAddresses) { $0.poolTypeTag }
        let order: [(UInt8, String)] = [
            (0, "External"),
            (1, "Internal"),
            (2, "Additional"),
            (3, "Additional (Hardened)"),
        ]
        return order.compactMap { tag, name in
            guard let bucket = grouped[tag], !bucket.isEmpty else { return nil }
            let sorted = bucket.sorted { $0.addressIndex < $1.addressIndex }
            return (name, sorted)
        }
    }

    private func addressListCard(
        name: String,
        systemImage: String,
        addresses: [PersistentCoreAddress]
    ) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("\(name) Addresses (\(addresses.count))", systemImage: systemImage)
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            ForEach(Array(addresses.enumerated()), id: \.element.id) { idx, addr in
                NavigationLink(destination: CoreAddressDetailView(record: addr)) {
                    addressRow(addr)
                }
                .buttonStyle(.plain)
                if idx < addresses.count - 1 {
                    Divider()
                }
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    /// DIP-17 Platform Payment address list — single section (no
    /// external/internal split). Rows link to `PlatformAddressDetailView`
    /// so all the platform-specific fields (bech32m string, nonce,
    /// hash, etc.) are reachable from here.
    private func platformAddressListCard(
        addresses: [PersistentPlatformAddress]
    ) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(
                "Platform Addresses (\(addresses.count))",
                systemImage: "creditcard"
            )
            .font(.headline)
            .foregroundColor(.primary)

            Divider()

            ForEach(Array(addresses.enumerated()), id: \.element.id) { idx, addr in
                NavigationLink(destination: PlatformAddressDetailView(record: addr)) {
                    platformAddressRow(addr)
                }
                .buttonStyle(.plain)
                if idx < addresses.count - 1 {
                    Divider()
                }
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    private func platformAddressRow(_ addr: PersistentPlatformAddress) -> some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(addr.address)
                    .font(.system(.caption, design: .monospaced))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .foregroundColor(.primary)
                HStack(spacing: 6) {
                    Text("#\(addr.addressIndex)")
                    if addr.isUsed { Text("• used") }
                    if addr.balance > 0 {
                        Text("• \(addr.balance) credits")
                    }
                }
                .font(.caption2)
                .foregroundColor(.secondary)
            }
            Spacer()
            Image(systemName: "chevron.right")
                .foregroundColor(.secondary)
                .font(.caption)
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
    }

    private func addressRow(_ addr: PersistentCoreAddress) -> some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(addr.address)
                    .font(.system(.caption, design: .monospaced))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .foregroundColor(.primary)
                HStack(spacing: 6) {
                    Text("#\(addr.addressIndex)")
                    if let usage = masternodeUsage(for: addr) {
                        Text("• \(usage)")
                    } else if addr.isUsed {
                        Text("• used")
                    }
                    if addr.balance > 0 {
                        Text("• \(formatBalance(addr.balance))")
                    }
                }
                .font(.caption2)
                .foregroundColor(.secondary)
            }
            Spacer()
            Image(systemName: "chevron.right")
                .foregroundColor(.secondary)
                .font(.caption)
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
    }

    /// Masternode-usage subtitle for a provider key address. Joins the
    /// address (base58) to the matching masternode field — the "join by key
    /// hash" without any Swift key hashing:
    ///   * owner / voting (accountType 9 / 8) ↔ `owner/votingAddress`
    ///     (real on-chain addresses), and
    ///   * operator / platform-node (accountType 10 / 11) ↔
    ///     `operator/platformNodeAddress` (base58 pseudo-addresses Rust
    ///     encoded from the payload key — these keys have no on-chain
    ///     address, so this is the only way to surface their usage).
    /// `nil` for non-provider-key accounts or unmatched addresses, so the
    /// caller falls back to "used".
    private func masternodeUsage(for addr: PersistentCoreAddress) -> String? {
        // 8 ProviderVotingKeys, 9 ProviderOwnerKeys, 10 ProviderOperatorKeys,
        // 11 ProviderPlatformKeys.
        let addressMatches: (PersistentMasternode) -> Bool
        switch account.accountType {
        case 8, 9:
            addressMatches = {
                $0.ownerAddress == addr.address || $0.votingAddress == addr.address
            }
        case 10:
            addressMatches = { $0.operatorPseudoAddress == addr.address }
        case 11:
            addressMatches = { $0.platformNodeAddress == addr.address }
        default:
            return nil
        }

        let wid = wallet.walletId
        let matches = allMasternodes.filter { $0.walletId == wid && addressMatches($0) }
        guard let first = matches.min(by: { $0.orderIndex < $1.orderIndex }) else {
            return nil
        }
        let count = matches.count
        let times = count == 1 ? "once" : (count == 2 ? "twice" : "\(count) times")
        if let ip = first.serviceAddress {
            return "used \(times) at \(ip) · \(first.displayTitle)"
        }
        return "used \(times) on \(first.displayTitle)"
    }

    private func emptyAddressesCard() -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Label("Addresses", systemImage: "info.circle")
                .font(.headline)
                .foregroundColor(.primary)
            Text("No addresses have been persisted for this account yet. They land here after the wallet is (re)created via `PlatformWalletManager`.")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding()
        .background(Color(.secondarySystemBackground))
        .cornerRadius(12)
    }

    /// Extended-public-key card for provider key-material accounts
    /// (`ProviderOperatorKeys` = BLS, `ProviderPlatformKeys` = EdDSA).
    /// These accounts derive masternode / platform-node keys and hold
    /// no on-chain addresses or balance, so the detail view surfaces
    /// the persisted extended public key hex instead of an address
    /// pool. The bytes are the bincode-encoded extended BLS / Ed25519
    /// public key the persister stored on `accountExtendedPubKeyBytes`.
    private func extendedPublicKeyCard() -> some View {
        let hex = account.accountExtendedPubKeyBytes?
            .map { String(format: "%02x", $0) }
            .joined() ?? ""
        return VStack(alignment: .leading, spacing: 12) {
            Label("Extended Public Key", systemImage: "key.horizontal.fill")
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            if hex.isEmpty {
                Text("No extended public key has been persisted for this account yet. It lands here after the wallet is (re)created via `PlatformWalletManager`.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                Text(account.accountType == 10
                     ? "Derives BLS operator keys for masternode operation. No on-chain addresses or balance."
                     : "Derives Ed25519 platform node keys. No on-chain addresses or balance.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Text(hex)
                    .font(.system(.caption2, design: .monospaced))
                    .foregroundColor(.primary)
                    .textSelection(.enabled)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    // MARK: - Provider derived keys

    /// The per-index keys a provider account actually derives from its
    /// extended public key: BLS operator keys (tag 10) or Ed25519
    /// platform-node keys (tag 11). Shows #0..#19, each with its public
    /// key (and, for platform nodes, the 20-byte node id that a ProRegTx
    /// carries) plus a confirm-gated private-key reveal.
    ///
    /// Loading policy follows the curve asymmetry: operator (BLS) public
    /// keys derive from the account xpub with no mnemonic, so they load
    /// eagerly on appear. Platform-node (Ed25519, SLIP-10 hardened-only)
    /// keys need the seed even for their public key — but the batch is
    /// pre-derived at registration and persisted, so they render straight
    /// from the account row with no keychain prompt. Only wallets created
    /// before pre-derivation shipped fall back to the lazy "Load Keys"
    /// button.
    private func derivedKeysCard() -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("Derived Keys", systemImage: "key.fill")
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            if derivedKeysLoaded {
                ForEach(Array(derivedKeys.enumerated()), id: \.element.index) { idx, key in
                    derivedKeyRow(key)
                    if idx < derivedKeys.count - 1 {
                        Divider()
                    }
                }
            } else if account.accountType == 11 {
                // Ed25519 platform-node keys — gated behind an explicit
                // unlock so no resolver / keychain read fires on nav.
                Button {
                    loadDerivedKeys()
                } label: {
                    HStack {
                        if isLoadingDerivedKeys {
                            ProgressView()
                        } else {
                            Image(systemName: "lock.open")
                        }
                        Text(isLoadingDerivedKeys ? "Loading…" : "Load Keys")
                    }
                }
                .disabled(isLoadingDerivedKeys)

                Text("Platform-node keys derive on the Ed25519 curve (hardened-only), so listing them needs the wallet mnemonic. Tap to unlock.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                // Operator (BLS) — loads on appear; brief placeholder.
                ProgressView()
                    .frame(maxWidth: .infinity, alignment: .center)
            }

            if let derivedKeysError {
                Text(derivedKeysError)
                    .font(.caption)
                    .foregroundColor(.red)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
        .onAppear {
            guard !derivedKeysLoaded, !isLoadingDerivedKeys else { return }
            // Platform-node (tag 11) keys pre-derived at registration are
            // persisted on the account row — render them straight from
            // persistence with no resolver / keychain read. Falls through
            // to the "Load Keys" button only for wallets created before
            // the batch was persisted.
            if account.accountType == 11, !persistedPlatformNodeKeys.isEmpty {
                derivedKeys = persistedPlatformNodeKeys
                derivedKeysLoaded = true
                return
            }
            // Operator (tag 10) public keys need no resolver — load them
            // eagerly. A tag-11 account with no persisted batch waits for
            // the button.
            if account.accountType == 10 {
                loadDerivedKeys()
            }
        }
    }

    /// The persisted platform-node keys mapped into the
    /// `ProviderDerivedKey` display shape (public key + node id as hex),
    /// read straight from this account's typed `PersistentCoreAddress`
    /// rows — the Ed25519 entries (`keyType == 2`) populated at wallet
    /// registration. Empty for non-platform-node accounts and for wallets
    /// created before those rows were persisted (those use the
    /// resolver-based "Load Keys" fallback). The node id is recomputed
    /// from the public key via the pure Rust bridge; private keys stay
    /// `nil` here — a reveal re-derives per index through
    /// `providerKeyAtIndex`.
    private var persistedPlatformNodeKeys: [ManagedPlatformWallet.ProviderDerivedKey] {
        account.coreAddresses
            .filter { $0.keyType == 2 && !$0.publicKey.isEmpty }
            .sorted { $0.addressIndex < $1.addressIndex }
            .map { addr in
                let nodeIdHex = ManagedPlatformWallet
                    .platformNodeId(fromEd25519PublicKey: addr.publicKey)
                    .map { hexString($0) }
                return ManagedPlatformWallet.ProviderDerivedKey(
                    index: addr.addressIndex,
                    publicKeyHex: hexString(addr.publicKey),
                    // Platform-node keys are Ed25519 — no legacy BLS form.
                    legacyPublicKeyHex: nil,
                    nodeIdHex: nodeIdHex,
                    privateKeyHex: nil
                )
            }
    }

    /// Lowercase hex of raw bytes — matches the Rust FFI's hex encoding
    /// so persisted rows render identically to resolver-derived ones.
    private func hexString(_ data: Data) -> String {
        data.map { String(format: "%02x", $0) }.joined()
    }

    /// One derived-key row: index header, public key, optional node id,
    /// and a confirm-gated private-key reveal. All value rows are
    /// monospaced, middle-truncated, and tap-to-copy.
    @ViewBuilder
    private func derivedKeyRow(_ key: ManagedPlatformWallet.ProviderDerivedKey) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Key #\(key.index)")
                .font(.subheadline)
                .fontWeight(.semibold)

            // Public key in BOTH encodings (hex + base64) so it can be
            // cross-checked against tools like dashwallet-ios that render
            // Ed25519 / BLS keys in base64. Base64 is a second rendering of
            // the same Rust-emitted bytes — never re-derived here.
            derivedValueRowBothEncodings(
                label: account.accountType == 10 ? "BLS Public Key" : "Ed25519 Public Key",
                hex: key.publicKeyHex,
                copyKey: "\(key.index)-pub"
            )

            // The same BLS G1 point in Dash's legacy serialization, shown
            // only for operator (BLS) keys. Ed25519 platform-node keys have
            // no legacy form, so `legacyPublicKeyHex` is `nil` there and the
            // rows are skipped. Rust emits both encodings — never derived here.
            if let legacy = key.legacyPublicKeyHex {
                derivedValueRowBothEncodings(
                    label: "BLS Public Key (Legacy)",
                    hex: legacy,
                    copyKey: "\(key.index)-pub-legacy"
                )
            }

            if let nodeId = key.nodeIdHex {
                // Node ids are conventionally rendered in hex — no base64
                // sibling.
                derivedValueRow(
                    label: "Platform Node ID",
                    value: nodeId,
                    copyKey: "\(key.index)-node"
                )
            }

            if let priv = revealedPrivateKeys[key.index] {
                derivedValueRowBothEncodings(
                    label: "Private Key",
                    hex: priv,
                    copyKey: "\(key.index)-priv"
                )
                // Ed25519 platform-node key in dashmate's "Enter Ed25519
                // node key" format: base64 of the 64-byte `priv(32) ‖
                // pub(32)` blob its validator accepts (base64-decode →
                // length 64 → split at 32 → pub matches priv). Built purely
                // by decoding the two hex strings the FFI already displays
                // and concatenating the bytes — no crypto, no byte
                // reinterpretation. Only for the Ed25519 platform-node
                // account, and only when both halves are present.
                if account.accountType == 11,
                    let tenderdash = tenderdashNodeKeyBase64(
                        privateHex: priv,
                        publicHex: key.publicKeyHex
                    )
                {
                    derivedValueRow(
                        label: "Tenderdash Node Key (Base64)",
                        value: tenderdash,
                        copyKey: "\(key.index)-tenderdash"
                    )
                }
            } else {
                Button {
                    revealConfirmIndex = key.index
                } label: {
                    HStack(spacing: 4) {
                        Image(systemName: "key.fill")
                        Text(revealingIndex == key.index ? "Revealing…" : "View Private Key")
                    }
                    .font(.caption)
                }
                .disabled(revealingIndex != nil)
            }
        }
        .confirmationDialog(
            "Reveal Private Key?",
            isPresented: revealDialogBinding(for: key.index),
            titleVisibility: .visible
        ) {
            Button("Reveal Private Key", role: .destructive) {
                revealPrivateKey(index: key.index)
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("The private key grants full control of this key. Only reveal it somewhere private.")
        }
    }

    /// Render `hex` in BOTH encodings: a "… (Hex)" row and, when the hex
    /// decodes, a "… (Base64)" sibling — the same bytes rendered a second
    /// way, so a value can be cross-checked against tools (e.g.
    /// dashwallet-ios) that display Ed25519 / BLS keys in base64. Both use
    /// the standard tap-to-copy value-row styling; the base64 row's copy
    /// key / accessibility id gets a `-b64` suffix.
    @ViewBuilder
    private func derivedValueRowBothEncodings(label: String, hex: String, copyKey: String) -> some View {
        derivedValueRow(label: "\(label) (Hex)", value: hex, copyKey: copyKey)
        if let b64 = base64(fromHex: hex) {
            derivedValueRow(label: "\(label) (Base64)", value: b64, copyKey: "\(copyKey)-b64")
        }
    }

    /// Re-render a lowercase-hex string as base64 — the same bytes in a
    /// second display encoding only (no crypto, no byte reinterpretation).
    /// `nil` only when `hex` isn't valid hex.
    private func base64(fromHex hex: String) -> String? {
        Data(hexString: hex)?.base64EncodedString()
    }

    /// The Ed25519 platform-node key in dashmate's "Enter Ed25519 node key"
    /// format: base64 of the 64-byte `priv(32) ‖ pub(32)` concatenation.
    /// dashmate's validator base64-decodes, requires length 64, splits at
    /// 32, and checks the public half matches the private. Built purely by
    /// decoding the two hex strings the FFI already displays and joining the
    /// bytes — no crypto, no byte reinterpretation. `nil` unless both halves
    /// decode to exactly 32 bytes.
    private func tenderdashNodeKeyBase64(privateHex: String, publicHex: String) -> String? {
        guard let priv = Data(hexString: privateHex), priv.count == 32,
            let pub = Data(hexString: publicHex), pub.count == 32
        else { return nil }
        return (priv + pub).base64EncodedString()
    }

    /// One monospaced, middle-truncated value row with tap-to-copy and a
    /// transient "Copied" confirmation keyed by `copyKey`. The `copyKey`
    /// doubles as the row's accessibility identifier (unique per row —
    /// e.g. `"0-pub"`, `"0-pub-b64"`).
    @ViewBuilder
    private func derivedValueRow(label: String, value: String, copyKey: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack {
                Text(label)
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                if derivedCopiedKey == copyKey {
                    Label("Copied", systemImage: "checkmark")
                        .font(.caption2)
                        .foregroundColor(.green)
                } else {
                    Image(systemName: "doc.on.doc")
                        .font(.caption2)
                        .foregroundColor(.accentColor)
                }
            }
            Text(value)
                .font(.system(.caption2, design: .monospaced))
                .lineLimit(1)
                .truncationMode(.middle)
                .foregroundColor(.primary)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .contentShape(Rectangle())
        .onTapGesture { copyDerived(value, copyKey: copyKey) }
        .accessibilityIdentifier(copyKey)
    }

    private func revealDialogBinding(for index: UInt32) -> Binding<Bool> {
        Binding(
            get: { revealConfirmIndex == index },
            set: { open in
                if !open, revealConfirmIndex == index { revealConfirmIndex = nil }
            }
        )
    }

    /// Derive the #0..#19 public keys for this provider account. All
    /// derivation happens on the Rust side (one FFI call per index); for
    /// platform-node accounts the mnemonic is pulled on demand via the
    /// resolver and never enters Swift. Only reached for operator (BLS)
    /// accounts and legacy platform-node wallets with no persisted batch.
    private func loadDerivedKeys() {
        guard let managed = walletManager.wallet(for: wallet.walletId) else {
            derivedKeysError = "The owning wallet is not loaded."
            return
        }
        guard let kind = ManagedPlatformWallet.ProviderKeyKind(
            rawValue: UInt8(account.accountType)
        ) else {
            derivedKeysError = "Unsupported provider account type."
            return
        }
        isLoadingDerivedKeys = true
        derivedKeysError = nil
        Task {
            do {
                var keys: [ManagedPlatformWallet.ProviderDerivedKey] = []
                // Matches `PLATFORM_NODE_KEY_PREDERIVE_COUNT` on the Rust
                // side so the resolver fallback lists the same window the
                // persisted batch would have shown.
                for index in UInt32(0)..<20 {
                    keys.append(
                        try managed.providerKeyAtIndex(
                            kind: kind,
                            index: index,
                            includePrivate: false
                        )
                    )
                }
                let loaded = keys
                await MainActor.run {
                    derivedKeys = loaded
                    derivedKeysLoaded = true
                    isLoadingDerivedKeys = false
                }
            } catch {
                await MainActor.run {
                    derivedKeysError = error.localizedDescription
                    isLoadingDerivedKeys = false
                }
            }
        }
    }

    /// Reveal the private key for one derived-key row. Re-derives at that
    /// index with `includePrivate: true`; Rust returns the raw scalar
    /// hex (BLS / Ed25519 keys have no WIF).
    private func revealPrivateKey(index: UInt32) {
        guard let managed = walletManager.wallet(for: wallet.walletId) else {
            derivedKeysError = "The owning wallet is not loaded."
            return
        }
        guard let kind = ManagedPlatformWallet.ProviderKeyKind(
            rawValue: UInt8(account.accountType)
        ) else { return }
        revealingIndex = index
        derivedKeysError = nil
        Task {
            do {
                let key = try managed.providerKeyAtIndex(
                    kind: kind,
                    index: index,
                    includePrivate: true
                )
                let hex = key.privateKeyHex
                await MainActor.run {
                    if let hex { revealedPrivateKeys[index] = hex }
                    revealingIndex = nil
                }
            } catch {
                await MainActor.run {
                    derivedKeysError = error.localizedDescription
                    revealingIndex = nil
                }
            }
        }
    }

    private func copyDerived(_ value: String, copyKey: String) {
        UIPasteboard.general.string = value
        derivedCopiedKey = copyKey
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
            if derivedCopiedKey == copyKey { derivedCopiedKey = nil }
        }
    }

    private func poolIcon(for name: String) -> String {
        switch name {
        case "External": return "arrow.down.circle"
        case "Internal": return "arrow.triangle.2.circlepath"
        default: return "square.stack"
        }
    }

    // MARK: - Helpers

    private var shouldShowBalance: Bool {
        switch account.accountType {
        case 0, 1, 14: return true
        default: return false
        }
    }

    /// Provider operator (BLS, tag 10) / platform-node (EdDSA, tag 11)
    /// key-material accounts. They hold key material, not addresses.
    private var isProviderKeyAccount: Bool {
        account.accountType == 10 || account.accountType == 11
    }

    private func formatBalance(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."
        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return "\(formatted) DASH"
        }
        return String(format: "%.8f DASH", dash)
    }

    /// Credits (1e11 per DASH) → DASH string. Used for the
    /// PlatformPayment balance row.
    private func formatCredits(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000_000.0
        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."
        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return "\(formatted) DASH"
        }
        return String(format: "%.8f DASH", dash)
    }

    private func derivePrivateKeyWithPIN() async {
        // TODO(platform-wallet): needs new FFI for WIF derivation via
        // PlatformWalletManager. For now, surface a stubbed error.
        await MainActor.run {
            errorMessage = "Private key derivation is not yet available through the new PlatformWalletManager."
        }
    }
}

// MARK: - PIN Prompt View

struct PINPromptView: View {
    @Binding var pinInput: String
    @Binding var isPresented: Bool
    let onSubmit: () -> Void

    var body: some View {
        NavigationView {
            VStack(spacing: 20) {
                Text("Enter Wallet PIN")
                    .font(.title2)
                    .fontWeight(.semibold)

                Text("Your PIN is required to access private keys")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.center)

                SecureField("PIN", text: $pinInput)
                    .textFieldStyle(.roundedBorder)
                    .keyboardType(.numberPad)
                    .padding(.horizontal)

                HStack(spacing: 20) {
                    Button("Cancel") {
                        pinInput = ""
                        isPresented = false
                    }
                    .buttonStyle(.bordered)

                    Button("Unlock") {
                        onSubmit()
                        isPresented = false
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(pinInput.isEmpty)
                }

                Spacer()
            }
            .padding()
            .navigationBarHidden(true)
        }
    }
}
