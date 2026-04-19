import SwiftUI
import SwiftData
import SwiftDashSDK

// MARK: - PersistentIdentity

struct IdentityStorageListView: View {
    @Query(sort: \PersistentIdentity.lastUpdated, order: .reverse)
    private var records: [PersistentIdentity]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: IdentityStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.dpnsName ?? record.alias ?? record.identityIdBase58)
                        .font(.body).lineLimit(1)
                    Text(record.formattedBalance)
                        .font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Identities (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "person.crop.circle") } }
    }
}

// MARK: - PersistentDocument

struct DocumentStorageListView: View {
    @Query(sort: \PersistentDocument.localUpdatedAt, order: .reverse)
    private var records: [PersistentDocument]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: DocumentStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.displayTitle).font(.body).lineLimit(1)
                    Text(record.documentType).font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Documents (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "doc.text") } }
    }
}

// MARK: - PersistentDataContract

struct DataContractStorageListView: View {
    @Query(sort: \PersistentDataContract.lastAccessedAt, order: .reverse)
    private var records: [PersistentDataContract]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: DataContractStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.name).font(.body).lineLimit(1)
                    Text(record.idBase58).font(.caption).foregroundColor(.secondary).lineLimit(1).truncationMode(.middle)
                }
            }
        }
        .navigationTitle("Data Contracts (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "doc.plaintext") } }
    }
}

// MARK: - PersistentPublicKey

struct PublicKeyStorageListView: View {
    @Query(sort: \PersistentPublicKey.createdAt, order: .reverse)
    private var records: [PersistentPublicKey]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: PublicKeyStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Key \(record.keyId)").font(.body)
                    Text("\(record.purpose) / \(record.securityLevel)")
                        .font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Public Keys (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "key") } }
    }
}

// MARK: - PersistentToken

struct TokenStorageListView: View {
    @Query(sort: \PersistentToken.name)
    private var records: [PersistentToken]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: TokenStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.name).font(.body).lineLimit(1)
                    Text(record.formattedBaseSupply).font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Tokens (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "circle.hexagongrid") } }
    }
}

// MARK: - PersistentTokenBalance

struct TokenBalanceStorageListView: View {
    @Query(sort: \PersistentTokenBalance.lastUpdated, order: .reverse)
    private var records: [PersistentTokenBalance]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: TokenBalanceStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.tokenName ?? record.tokenId).font(.body).lineLimit(1)
                    Text(record.displayBalance).font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Token Balances (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "banknote") } }
    }
}

// MARK: - PersistentTokenHistoryEvent

struct TokenHistoryStorageListView: View {
    @Query(sort: \PersistentTokenHistoryEvent.createdAt, order: .reverse)
    private var records: [PersistentTokenHistoryEvent]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: TokenHistoryStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.displayTitle).font(.body).lineLimit(1)
                    Text(record.eventTimestamp, style: .date).font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Token History (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "clock.arrow.circlepath") } }
    }
}

// MARK: - PersistentDocumentType

struct DocumentTypeStorageListView: View {
    @Query(sort: \PersistentDocumentType.name)
    private var records: [PersistentDocumentType]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: DocumentTypeStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.name).font(.body).lineLimit(1)
                    Text(record.contractIdBase58).font(.caption).foregroundColor(.secondary).lineLimit(1).truncationMode(.middle)
                }
            }
        }
        .navigationTitle("Document Types (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "list.bullet.rectangle") } }
    }
}

// MARK: - PersistentIndex

struct IndexStorageListView: View {
    @Query(sort: \PersistentIndex.name)
    private var records: [PersistentIndex]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: IndexStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.name).font(.body).lineLimit(1)
                    Text(record.documentTypeName).font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Indices (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "tablecells") } }
    }
}

// MARK: - PersistentProperty

struct PropertyStorageListView: View {
    @Query(sort: \PersistentProperty.name)
    private var records: [PersistentProperty]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: PropertyStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.name).font(.body).lineLimit(1)
                    Text(record.type).font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Properties (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "slider.horizontal.3") } }
    }
}

// MARK: - PersistentKeyword

struct KeywordStorageListView: View {
    @Query(sort: \PersistentKeyword.keyword)
    private var records: [PersistentKeyword]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: KeywordStorageDetailView(record: record)) {
                Text(record.keyword).font(.body)
            }
        }
        .navigationTitle("Keywords (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "tag") } }
    }
}

// MARK: - PersistentSyncState

struct SyncStateStorageListView: View {
    @Query(sort: \PersistentSyncState.lastUpdated, order: .reverse)
    private var records: [PersistentSyncState]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: SyncStateStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.network.capitalized)
                        .font(.body)
                    Text("Height \(record.syncHeight)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text("Updated \(record.lastUpdated, style: .relative)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Sync State (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "arrow.triangle.2.circlepath") } }
    }
}

// MARK: - PersistentWallet

struct WalletStorageListView: View {
    @Query(sort: \PersistentWallet.lastUpdated, order: .reverse)
    private var records: [PersistentWallet]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: WalletStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.name ?? record.walletId.map { String(format: "%02x", $0) }.prefix(16).joined())
                        .font(.body).lineLimit(1)
                    Text("\(record.network) · height \(record.syncedHeight)")
                        .font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Wallets (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "wallet.pass") } }
    }
}

// MARK: - PersistentAccount

struct AccountStorageListView: View {
    /// Query the wallet side of the `@Relationship` so the list groups
    /// by wallet. Accounts come through `wallet.accounts`, which
    /// updates reactively under SwiftData.
    @Query(sort: \PersistentWallet.createdAt, order: .reverse)
    private var wallets: [PersistentWallet]

    /// Catch any accounts whose `wallet` inverse is nil (shouldn't
    /// happen in steady state — the write path always links them —
    /// but shown so the explorer doesn't silently hide them).
    @Query(sort: \PersistentAccount.createdAt, order: .reverse)
    private var allAccounts: [PersistentAccount]

    private var orphanAccounts: [PersistentAccount] {
        allAccounts.filter { $0.wallet == nil }
    }

    private var totalAccountCount: Int {
        wallets.reduce(0) { $0 + $1.accounts.count } + orphanAccounts.count
    }

    var body: some View {
        List {
            ForEach(wallets) { wallet in
                Section(header: Text(walletHeader(for: wallet))) {
                    let sorted = sortedAccounts(wallet.accounts)
                    if sorted.isEmpty {
                        Text("No accounts").font(.caption).foregroundColor(.secondary)
                    } else {
                        ForEach(sorted) { account in
                            NavigationLink(destination: AccountStorageDetailView(record: account)) {
                                accountRow(account)
                            }
                        }
                    }
                }
            }
            if !orphanAccounts.isEmpty {
                Section(header: Text("Unlinked")) {
                    ForEach(orphanAccounts) { account in
                        NavigationLink(destination: AccountStorageDetailView(record: account)) {
                            accountRow(account)
                        }
                    }
                }
            }
        }
        .navigationTitle("Accounts (\(totalAccountCount))")
        .overlay {
            if totalAccountCount == 0 {
                ContentUnavailableView("No Records", systemImage: "person.2")
            }
        }
    }

    /// "{name}" when the wallet has one, else "Wallet {short-id}". The
    /// network is appended when available to distinguish the same
    /// mnemonic on different networks.
    private func walletHeader(for wallet: PersistentWallet) -> String {
        let id = wallet.walletId.prefix(4).map { String(format: "%02x", $0) }.joined()
        let label = wallet.name ?? "Wallet \(id)…"
        return "\(label) (\(wallet.network))"
    }

    /// Same ordering used in the load-path emit — stable across runs.
    private func sortedAccounts(_ accounts: [PersistentAccount]) -> [PersistentAccount] {
        accounts.sorted {
            ($0.accountType, $0.accountIndex, $0.registrationIndex, $0.keyClass)
                < ($1.accountType, $1.accountIndex, $1.registrationIndex, $1.keyClass)
        }
    }

    @ViewBuilder
    private func accountRow(_ record: PersistentAccount) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(record.accountTypeName).font(.body).lineLimit(1)
            Text(
                "Index \(record.accountIndex) · "
                    + "\(record.transactions.count) txs · "
                    + "\(record.utxos.count) utxos"
            )
            .font(.caption).foregroundColor(.secondary)
        }
    }
}

// MARK: - PersistentTransaction

struct TransactionStorageListView: View {
    @Query(sort: \PersistentTransaction.firstSeen, order: .reverse)
    private var records: [PersistentTransaction]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: TransactionStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.txid)
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(1).truncationMode(.middle)
                    HStack {
                        Text(record.directionName).font(.caption)
                        Spacer()
                        Text(record.formattedAmount)
                            .font(.caption).foregroundColor(record.netAmount >= 0 ? .green : .red)
                    }
                }
            }
        }
        .navigationTitle("Transactions (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "arrow.left.arrow.right.circle") } }
    }
}

// MARK: - PersistentCoreAddress

struct CoreAddressStorageListView: View {
    /// Every Core-chain address record. PlatformPayment (DIP-17)
    /// addresses now live in their own `PersistentPlatformAddress`
    /// store, so no filtering is needed here.
    @Query(sort: [SortDescriptor(\PersistentCoreAddress.addressIndex)])
    private var records: [PersistentCoreAddress]

    /// Composite key identifying one (wallet, account) bucket. All
    /// pools (External / Internal / Absent / Absent Hardened) for a
    /// given account collapse into a single section — the pool name
    /// rides on the address row instead of the header. `standardTag`
    /// is part of the key because a Standard account at index 0 can
    /// coexist in both BIP44 (tag 0) and BIP32 (tag 1) forms for the
    /// same wallet, and they should render as distinct sections.
    private struct GroupKey: Hashable, Comparable {
        let walletId: Data
        let walletLabel: String
        let accountType: UInt32
        let accountIndex: UInt32
        let standardTag: UInt8
        let accountLabel: String

        static func < (lhs: Self, rhs: Self) -> Bool {
            if lhs.walletId != rhs.walletId {
                return lhs.walletId.lexicographicallyPrecedes(rhs.walletId)
            }
            if lhs.accountType != rhs.accountType { return lhs.accountType < rhs.accountType }
            if lhs.accountIndex != rhs.accountIndex { return lhs.accountIndex < rhs.accountIndex }
            return lhs.standardTag < rhs.standardTag
        }
    }

    /// Group addresses by (wallet, account). Addresses within a group
    /// are sorted by (pool tag, derivation index) so external pool
    /// entries come first, followed by internal, followed by any
    /// absent-pool entries — each in index order.
    private var groups: [(GroupKey, [PersistentCoreAddress])] {
        let grouped = Dictionary(grouping: records) { record -> GroupKey in
            let account = record.account
            let wallet = account?.wallet
            return GroupKey(
                walletId: wallet?.walletId ?? Data(),
                walletLabel: walletLabel(for: wallet),
                accountType: account?.accountType ?? 0,
                accountIndex: account?.accountIndex ?? 0,
                standardTag: account?.standardTag ?? 0,
                accountLabel: account?.accountTypeName ?? "Unknown"
            )
        }
        return grouped
            .map { entry in
                let sorted = entry.value.sorted { lhs, rhs in
                    if lhs.poolTypeTag != rhs.poolTypeTag {
                        return lhs.poolTypeTag < rhs.poolTypeTag
                    }
                    return lhs.addressIndex < rhs.addressIndex
                }
                return (entry.key, sorted)
            }
            .sorted { $0.0 < $1.0 }
    }

    var body: some View {
        List {
            ForEach(Array(groups.enumerated()), id: \.offset) { _, pair in
                let (key, addresses) = pair
                Section(header: Text(sectionTitle(for: key))) {
                    ForEach(addresses) { record in
                        NavigationLink(destination: CoreAddressDetailView(record: record)) {
                            addressRow(record)
                        }
                    }
                }
            }
        }
        .navigationTitle("Core Addresses (\(records.count))")
        .overlay {
            if records.isEmpty {
                ContentUnavailableView(
                    "No Records",
                    systemImage: "square.and.pencil"
                )
            }
        }
    }

    /// Header format: `"WalletName · AccountName #N"`. `#N` is dropped
    /// for non-indexed account types (identity registration, provider
    /// keys, etc.) so the title doesn't dangle a stray `#0`.
    private func sectionTitle(for key: GroupKey) -> String {
        let accountPart = hasMeaningfulIndex(for: key.accountType)
            ? "\(key.accountLabel) #\(key.accountIndex)"
            : key.accountLabel
        return "\(key.walletLabel) · \(accountPart)"
    }

    /// Account types whose `accountIndex` carries real meaning (BIP44
    /// account 0/1/2, CoinJoin index, DashPay per-contact index,
    /// PlatformPayment account). Singleton account types (identity
    /// registration, provider keys, etc.) always have index 0 and
    /// showing `#0` in the header just adds noise.
    private func hasMeaningfulIndex(for typeTag: UInt32) -> Bool {
        switch typeTag {
        case 0, 1, 3, 12, 13, 14: return true
        default: return false
        }
    }

    private func walletLabel(for wallet: PersistentWallet?) -> String {
        guard let wallet = wallet else { return "Unknown Wallet" }
        if let name = wallet.name, !name.isEmpty { return name }
        let prefix = wallet.walletId.prefix(4).map { String(format: "%02x", $0) }.joined()
        return "Wallet \(prefix)…"
    }

    private func addressRow(_ record: PersistentCoreAddress) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(record.address)
                .font(.system(.caption, design: .monospaced))
                .lineLimit(1).truncationMode(.middle)
            HStack(spacing: 8) {
                Text(record.poolTypeName)
                Text("• #\(record.addressIndex)")
                if record.isUsed { Text("• used") }
                if record.balance > 0 { Text("• \(record.balance)") }
            }
            .font(.caption2)
            .foregroundColor(.secondary)
        }
    }
}

// MARK: - PersistentPlatformAddress

/// List view for DIP-17 PlatformPayment addresses. Queries the
/// dedicated `PersistentPlatformAddress` store (populated by the
/// address-emit path for type-14 accounts, refreshed by BLAST
/// sync).
struct PlatformAddressStorageListView: View {
    @Query(sort: [SortDescriptor(\PersistentPlatformAddress.addressIndex)])
    private var records: [PersistentPlatformAddress]

    private struct GroupKey: Hashable, Comparable {
        let walletId: Data
        let walletLabel: String
        let accountIndex: UInt32
        let accountLabel: String
        let keyClass: UInt32

        static func < (lhs: Self, rhs: Self) -> Bool {
            if lhs.walletId != rhs.walletId {
                return lhs.walletId.lexicographicallyPrecedes(rhs.walletId)
            }
            if lhs.accountIndex != rhs.accountIndex { return lhs.accountIndex < rhs.accountIndex }
            return lhs.keyClass < rhs.keyClass
        }
    }

    private var groups: [(GroupKey, [PersistentPlatformAddress])] {
        let grouped = Dictionary(grouping: records) { record -> GroupKey in
            let account = record.account
            let wallet = account?.wallet
            return GroupKey(
                walletId: wallet?.walletId ?? record.walletId,
                walletLabel: walletLabel(for: wallet, fallbackId: record.walletId),
                accountIndex: account?.accountIndex ?? record.accountIndex,
                accountLabel: account?.accountTypeName ?? "Platform Payment",
                keyClass: account?.keyClass ?? 0
            )
        }
        return grouped
            .map { ($0.key, $0.value.sorted { $0.addressIndex < $1.addressIndex }) }
            .sorted { $0.0 < $1.0 }
    }

    var body: some View {
        List {
            ForEach(Array(groups.enumerated()), id: \.offset) { _, pair in
                let (key, addresses) = pair
                Section(header: Text(sectionTitle(for: key))) {
                    ForEach(addresses) { record in
                        NavigationLink(destination: PlatformAddressDetailView(record: record)) {
                            addressRow(record)
                        }
                    }
                }
            }
        }
        .navigationTitle("Platform Addresses (\(records.count))")
        .overlay {
            if records.isEmpty {
                ContentUnavailableView(
                    "No Records",
                    systemImage: "creditcard"
                )
            }
        }
    }

    /// Header format: `"WalletName · Platform Payment #N · key class K"`.
    /// `keyClass` is elided when zero (the common default) to reduce
    /// visual noise.
    private func sectionTitle(for key: GroupKey) -> String {
        var title = "\(key.walletLabel) · \(key.accountLabel) #\(key.accountIndex)"
        if key.keyClass != 0 {
            title += " · key class \(key.keyClass)"
        }
        return title
    }

    private func walletLabel(for wallet: PersistentWallet?, fallbackId: Data) -> String {
        if let wallet = wallet {
            if let name = wallet.name, !name.isEmpty { return name }
            let prefix = wallet.walletId.prefix(4).map { String(format: "%02x", $0) }.joined()
            return "Wallet \(prefix)…"
        }
        let prefix = fallbackId.prefix(4).map { String(format: "%02x", $0) }.joined()
        return prefix.isEmpty ? "Unknown Wallet" : "Wallet \(prefix)…"
    }

    private func addressRow(_ record: PersistentPlatformAddress) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(record.address)
                .font(.system(.caption, design: .monospaced))
                .lineLimit(1).truncationMode(.middle)
            HStack(spacing: 8) {
                Text("#\(record.addressIndex)")
                if record.isUsed { Text("• used") }
                if record.balance > 0 { Text("• \(record.balance)") }
            }
            .font(.caption2)
            .foregroundColor(.secondary)
        }
    }
}

// MARK: - PersistentUtxo

struct UtxoStorageListView: View {
    @Query(sort: \PersistentUtxo.createdAt, order: .reverse)
    private var records: [PersistentUtxo]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: UtxoStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.outpoint)
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(1).truncationMode(.middle)
                    HStack {
                        Text(record.formattedAmount).font(.caption)
                        Spacer()
                        if record.isSpent {
                            Text("Spent").font(.caption2).foregroundColor(.red)
                        } else {
                            Text("Unspent").font(.caption2).foregroundColor(.green)
                        }
                    }
                }
            }
        }
        .navigationTitle("UTXOs (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "bitcoinsign.circle") } }
    }
}

// MARK: - PersistentWalletManagerMetadata

struct WalletManagerMetadataStorageListView: View {
    @Query(sort: \PersistentWalletManagerMetadata.lastUpdated, order: .reverse)
    private var records: [PersistentWalletManagerMetadata]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: WalletManagerMetadataStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.network).font(.body)
                    Text("Height \(record.combinedSyncHeight) · \(record.walletCount) wallets")
                        .font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Manager Metadata (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "gearshape.2") } }
    }
}
