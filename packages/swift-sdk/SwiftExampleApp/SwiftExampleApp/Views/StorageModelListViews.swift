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

/// Storage-explorer list of every `PersistentPublicKey`, grouped by
/// owning wallet + identity. Keys without a parent identity land in
/// a trailing "Unassigned" section so they stay visible but don't
/// pollute the wallet-scoped sections above.
///
/// Grouping pivot: the `PersistentPublicKey.identity` relationship.
/// We drive the top-level order from `PersistentIdentity` sorted by
/// `(walletId, identityIndex)` — that way identity #0 of wallet A
/// shows before identity #1, and unnamed wallets stay clustered.
struct PublicKeyStorageListView: View {
    @Environment(\.modelContext) private var modelContext
    @Query(sort: \PersistentIdentity.identityIndex)
    private var identities: [PersistentIdentity]

    @Query(sort: \PersistentPublicKey.createdAt, order: .reverse)
    private var allKeys: [PersistentPublicKey]

    @Query private var hdWallets: [PersistentWallet]

    /// Key targeted by a pending Remove swipe. Non-nil presents the
    /// confirmation dialog; holding the reference lets the dialog
    /// show `Key N` without re-fetching.
    @State private var keyPendingRemoval: PersistentPublicKey?

    var body: some View {
        List {
            ForEach(walletGroups, id: \.walletId) { group in
                walletSection(group)
            }

            let orphans = orphanKeys
            if !orphans.isEmpty {
                Section("Unassigned") {
                    ForEach(orphans) { key in
                        keyRow(key)
                    }
                }
            }
        }
        .navigationTitle("Public Keys (\(allKeys.count))")
        .overlay {
            if allKeys.isEmpty {
                ContentUnavailableView("No Records", systemImage: "key")
            }
        }
        .confirmationDialog(
            removalDialogTitle,
            isPresented: Binding(
                get: { keyPendingRemoval != nil },
                set: { newValue in
                    if !newValue { keyPendingRemoval = nil }
                }
            ),
            titleVisibility: .visible,
            presenting: keyPendingRemoval
        ) { key in
            Button("Remove from Device", role: .destructive) {
                removePublicKeyLocally(key)
                keyPendingRemoval = nil
            }
            Button("Keep on Device", role: .cancel) {
                keyPendingRemoval = nil
            }
        } message: { _ in
            Text(
                "This only deletes the local copy of this public key and any matching private key stored in the Keychain. It does not change the identity on the Dash Platform network."
            )
        }
    }

    // MARK: Helpers

    /// One visual section per identity, but sections belonging to the
    /// same wallet render back-to-back because `walletGroups` already
    /// clusters them.
    private struct WalletGroup {
        /// `Data()` when the identities aren't tied to a wallet.
        let walletId: Data
        let walletLabel: String?
        let identities: [PersistentIdentity]
    }

    /// Cluster `identities` by `walletId` preserving the
    /// `identityIndex` sort inside each cluster. Wallets sort by
    /// label (alphabetical, case-insensitive); no-wallet identities
    /// sort last so user-owned ones dominate the top of the screen.
    private var walletGroups: [WalletGroup] {
        let grouped = Dictionary(grouping: identities) { $0.wallet?.walletId ?? Data() }
        return grouped
            .map { (walletId, ids) -> WalletGroup in
                WalletGroup(
                    walletId: walletId,
                    walletLabel: walletLabel(for: walletId),
                    identities: ids.sorted { $0.identityIndex < $1.identityIndex }
                )
            }
            .sorted { lhs, rhs in
                // Placeholder (empty Data) → bottom; labelled wallets
                // alpha-sorted; named wallets before id-only ones.
                if lhs.walletId.isEmpty != rhs.walletId.isEmpty {
                    return !lhs.walletId.isEmpty
                }
                let l = lhs.walletLabel ?? walletShort(lhs.walletId)
                let r = rhs.walletLabel ?? walletShort(rhs.walletId)
                return l.localizedCaseInsensitiveCompare(r) == .orderedAscending
            }
    }

    /// Keys whose `identity` relationship is nil — e.g. rows that
    /// predate the changeset wiring or belong to identities since
    /// deleted.
    private var orphanKeys: [PersistentPublicKey] {
        allKeys.filter { $0.identity == nil }
    }

    private func walletLabel(for walletId: Data) -> String? {
        guard !walletId.isEmpty else { return nil }
        return hdWallets.first { $0.walletId == walletId }?.label
    }

    private func walletShort(_ walletId: Data) -> String {
        guard !walletId.isEmpty else { return "(no wallet)" }
        let hex = walletId.prefix(4)
            .map { String(format: "%02x", $0) }
            .joined()
        return "Wallet \(hex)"
    }

    @ViewBuilder
    private func walletSection(_ group: WalletGroup) -> some View {
        ForEach(group.identities, id: \.identityId) { identity in
            Section {
                let keys = identity.publicKeys
                    .sorted { $0.keyId < $1.keyId }
                if keys.isEmpty {
                    Text("No keys")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    ForEach(keys) { key in
                        keyRow(key)
                    }
                }
            } header: {
                identityHeader(for: identity, group: group)
            }
        }
    }

    @ViewBuilder
    private func identityHeader(
        for identity: PersistentIdentity,
        group: WalletGroup
    ) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: 6) {
                Text("#\(identity.identityIndex)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Text(identityDisplayName(identity))
                    .font(.subheadline)
                    .fontWeight(.semibold)
                    .textCase(nil)
            }
            Text(group.walletLabel ?? walletShort(group.walletId))
                .font(.caption2)
                .foregroundColor(.secondary)
                .textCase(nil)
        }
        .padding(.vertical, 2)
    }

    /// Prefer the user-facing main DPNS name, fall back to any DPNS
    /// name, then alias, then a short identity id — avoids the
    /// generic "Identity" placeholder except for truly empty rows.
    private func identityDisplayName(_ identity: PersistentIdentity) -> String {
        if let name = identity.mainDpnsName, !name.isEmpty {
            return name
        }
        if let name = identity.dpnsName, !name.isEmpty {
            return name
        }
        if let alias = identity.alias, !alias.isEmpty {
            return alias
        }
        // Truncated base58 id keeps rows distinguishable when no
        // name has been fetched yet.
        let b58 = identity.identityIdBase58
        guard b58.count > 12 else { return b58 }
        return "\(b58.prefix(6))…\(b58.suffix(6))"
    }

    @ViewBuilder
    private func keyRow(_ record: PersistentPublicKey) -> some View {
        NavigationLink(destination: PublicKeyStorageDetailView(record: record)) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Key \(record.keyId)").font(.body)
                Text("\(record.purpose) / \(record.securityLevel)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        // Same pattern as the identities list: no `role: .destructive`
        // on the swipe button itself (that would animate the row out
        // before the user confirms), and `.tint(.red)` gives us the
        // expected red look. `allowsFullSwipe: false` forces the user
        // through the confirmation dialog.
        .swipeActions(edge: .trailing, allowsFullSwipe: false) {
            Button {
                keyPendingRemoval = record
            } label: {
                Label("Remove", systemImage: "trash")
            }
            .tint(.red)
        }
    }

    private var removalDialogTitle: String {
        guard let key = keyPendingRemoval else {
            return "Remove public key from this device?"
        }
        return "Remove Key \(key.keyId) from this device?"
    }

    /// Local-only removal of a public key row plus any paired
    /// Keychain entry. We delete by the stored
    /// `privateKeyKeychainIdentifier` string (format-agnostic across
    /// the `privkey_<identityHex>_<keyIndex>` and
    /// `identity_privkey.<derivationPath>` formats the app writes).
    /// SwiftData handles the rest: the public-key row disappears,
    /// and the `@Relationship(inverse: \PersistentIdentity.publicKeys)`
    /// inverse removes it from the owning identity's collection.
    private func removePublicKeyLocally(_ key: PersistentPublicKey) {
        if let identifier = key.privateKeyKeychainIdentifier {
            _ = KeychainManager.shared.deleteKeyData(identifier: identifier)
        }
        modelContext.delete(key)
        try? modelContext.save()
    }
}

// MARK: - PersistentDPNSName

/// Storage-explorer list of every confirmed DPNS label across all
/// identities. Newest acquisition first — `acquiredAt` is Unix-millis
/// from `DpnsNameInfo.acquired_at` and zero-valued rows (legacy,
/// un-timestamped) naturally fall to the bottom.
struct DPNSNameStorageListView: View {
    @Query(sort: \PersistentDPNSName.acquiredAt, order: .reverse)
    private var records: [PersistentDPNSName]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: DPNSNameStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("\(record.label).\(record.parentDomainName)")
                        .font(.body).lineLimit(1)
                    Text(record.identity.identityIdBase58)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
            }
        }
        .navigationTitle("DPNS Names (\(records.count))")
        .overlay {
            if records.isEmpty {
                ContentUnavailableView("No Records", systemImage: "at")
            }
        }
    }
}

// MARK: - PersistentDashpayProfile

/// Storage-explorer list of every cached DashPay profile. One row
/// per (network, identity). Newest profile update first.
struct DashpayProfileStorageListView: View {
    @Query(sort: \PersistentDashpayProfile.lastUpdated, order: .reverse)
    private var records: [PersistentDashpayProfile]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: DashpayProfileStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.displayName ?? "(no display name)")
                        .font(.body).lineLimit(1)
                    Text(record.identity.identityIdBase58)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
            }
        }
        .navigationTitle("DashPay Profiles (\(records.count))")
        .overlay {
            if records.isEmpty {
                ContentUnavailableView("No Records", systemImage: "person.text.rectangle")
            }
        }
    }
}

// MARK: - PersistentDashpayContactRequest

/// Storage-explorer list of every DashPay contact-request row.
/// Grouped by direction (Outgoing / Incoming) — `isOutgoing` partitions
/// the rows because the encrypted payload differs per direction (each
/// side seals to the other party's identity key), so the two
/// directions are inherently distinct rows even for the same
/// (owner, contact) pair. Within each section, newest request first
/// (`createdAtMillis` desc; `0` falls to the bottom).
struct DashpayContactRequestStorageListView: View {
    @Query private var records: [PersistentDashpayContactRequest]

    private var outgoing: [PersistentDashpayContactRequest] {
        records.filter { $0.isOutgoing }
            .sorted { $0.createdAtMillis > $1.createdAtMillis }
    }

    private var incoming: [PersistentDashpayContactRequest] {
        records.filter { !$0.isOutgoing }
            .sorted { $0.createdAtMillis > $1.createdAtMillis }
    }

    var body: some View {
        List {
            if !outgoing.isEmpty {
                Section("Outgoing (\(outgoing.count))") {
                    ForEach(outgoing) { record in
                        contactRequestLink(record)
                    }
                }
            }
            if !incoming.isEmpty {
                Section("Incoming (\(incoming.count))") {
                    ForEach(incoming) { record in
                        contactRequestLink(record)
                    }
                }
            }
        }
        .navigationTitle("Contact Requests (\(records.count))")
        .overlay {
            if records.isEmpty {
                ContentUnavailableView(
                    "No Records",
                    systemImage: "person.crop.circle.badge.plus"
                )
            }
        }
    }

    @ViewBuilder
    private func contactRequestLink(
        _ record: PersistentDashpayContactRequest
    ) -> some View {
        NavigationLink(destination: DashpayContactRequestStorageDetailView(record: record)) {
            VStack(alignment: .leading, spacing: 4) {
                Text(shortHex(record.contactIdentityId))
                    .font(.system(.body, design: .monospaced))
                    .lineLimit(1)
                    .truncationMode(.middle)
                Text("from \(shortHex(record.ownerIdentityId))")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
        }
    }

    /// Render a 32-byte identity id as a "<first 4 hex>…<last 4 hex>"
    /// to keep the row concise. Mirrors the truncation pattern other
    /// storage list views use for ids.
    private func shortHex(_ data: Data) -> String {
        guard data.count >= 8 else {
            return data.map { String(format: "%02x", $0) }.joined()
        }
        let head = data.prefix(4).map { String(format: "%02x", $0) }.joined()
        let tail = data.suffix(4).map { String(format: "%02x", $0) }.joined()
        return "\(head)…\(tail)"
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
                    Text(AppDate.formatted(record.eventTimestamp, dateStyle: .abbreviated, timeStyle: .omitted))
                        .font(.caption).foregroundColor(.secondary)
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

// MARK: - PersistentPlatformAddressesSyncState

struct PlatformAddressesSyncStateStorageListView: View {
    @Query(sort: \PersistentPlatformAddressesSyncState.lastUpdated, order: .reverse)
    private var records: [PersistentPlatformAddressesSyncState]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: PlatformAddressesSyncStateStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.network.displayName)
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
        .navigationTitle("Platform Addresses Sync State (\(records.count))")
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
                    Text("\(record.network?.displayName ?? "Unknown") · height \(record.syncedHeight)")
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

    private var totalAccountCount: Int {
        wallets.reduce(0) { $0 + $1.accounts.count }
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
        return "\(label) (\(wallet.network?.displayName ?? "Unknown"))"
    }

    /// Same ordering used in the load-path emit — stable across runs.
    private func sortedAccounts(_ accounts: [PersistentAccount]) -> [PersistentAccount] {
        accounts.sorted {
            ($0.accountType, $0.accountIndex, $0.registrationIndex, $0.keyClass)
                < ($1.accountType, $1.accountIndex, $1.registrationIndex, $1.keyClass)
        }
    }

    /// Per-account transaction count = distinct creating + spending
    /// txs across this account's TXOs. Derived because the direct
    /// `account.transactions` relationship is gone — a single tx can
    /// span multiple accounts and is no longer account-scoped on the
    /// model side. Walks the address pool now that
    /// `PersistentAccount.outputs` is gone; the canonical
    /// account → TXO path is `coreAddresses.flatMap(\.txos)`.
    private func distinctTxCount(_ record: PersistentAccount) -> Int {
        var seen: Set<Data> = []
        for address in record.coreAddresses {
            for txo in address.txos {
                if let tx = txo.transaction { seen.insert(tx.txid) }
                if let spending = txo.spendingTransaction { seen.insert(spending.txid) }
            }
        }
        return seen.count
    }

    /// TXO count per account, summed across the address pool.
    private func txoCount(_ record: PersistentAccount) -> Int {
        record.coreAddresses.reduce(0) { $0 + $1.txos.count }
    }

    @ViewBuilder
    private func accountRow(_ record: PersistentAccount) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(record.accountTypeName).font(.body).lineLimit(1)
            Text(
                "Index \(record.accountIndex) · "
                    + "\(distinctTxCount(record)) txs · "
                    + "\(txoCount(record)) txos"
            )
            .font(.caption).foregroundColor(.secondary)
        }
    }
}

// MARK: - PersistentTransaction

struct TransactionStorageListView: View {
    @Query(sort: \PersistentTransaction.firstSeen, order: .reverse)
    private var records: [PersistentTransaction]

    /// Live-search needle — matches against the canonical block-explorer
    /// `txidHex` (byte-reversed) so a user can paste an explorer link's
    /// id directly. Case-insensitive substring; empty disables the
    /// filter entirely.
    @State private var searchText: String = ""
    /// Direction filter. `PersistentTransaction.direction` carries the
    /// four values exposed by `TransactionDirection` on the Rust side
    /// (`Incoming`/`Outgoing`/`Internal`/`CoinJoin`); `.all` keeps
    /// everything.
    @State private var directionFilter: DirectionFilter = .all
    /// Transaction-type filter. Keyed on `transactionType` strings the
    /// Rust FFI emits via `format!("{:?}", …)` on
    /// `dashcore::TransactionType` (e.g. `"Classic Transaction"`,
    /// `"Asset Lock Transaction"`). The legacy default placeholder
    /// `"Standard"` is also tolerated so older rows still match
    /// "Classic". `nil` means "any type".
    @State private var typeFilter: String? = nil

    /// Pre-defined list of every type the data model exposes. Not
    /// derived from the live record set so that the picker shape stays
    /// stable as the user changes filters and so reviewers can spot a
    /// type that has zero rows (data exists / not exists is itself a
    /// signal). Strings match the FFI-emitted Debug form on
    /// `TransactionType`.
    private static let knownTypes: [String] = [
        "Classic Transaction",
        "Provider Registration Transaction",
        "Provider Update Service Transaction",
        "Provider Update Registrar Transaction",
        "Provider Update Revocation Transaction",
        "Coinbase Transaction",
        "Quorum Commitment Transaction",
        "MNHF Signal Transaction",
        "Asset Lock Transaction",
        "Asset Unlock Transaction",
    ]

    private var filteredRecords: [PersistentTransaction] {
        let trimmed = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        let needle = trimmed.lowercased()
        return records.filter { record in
            // Direction.
            switch directionFilter {
            case .all: break
            case .incoming where record.direction != 0: return false
            case .outgoing where record.direction != 1: return false
            case .internalTx where record.direction != 2: return false
            case .coinjoin where record.direction != 3: return false
            default: break
            }
            // Type. Treat the legacy `"Standard"` placeholder (the
            // default the persister uses when the FFI column is nil)
            // as equivalent to `"Classic Transaction"` so users
            // searching for classic txs still see those rows.
            if let want = typeFilter {
                let actual = record.transactionType
                let normalized = actual == "Standard" ? "Classic Transaction" : actual
                if normalized != want { return false }
            }
            // Search needle.
            if !needle.isEmpty {
                if !record.txidHex.lowercased().contains(needle) { return false }
            }
            return true
        }
    }

    var body: some View {
        let visible = filteredRecords
        List {
            Section {
                Picker("Direction", selection: $directionFilter) {
                    ForEach(DirectionFilter.allCases, id: \.self) { d in
                        Text(d.title).tag(d)
                    }
                }
                .pickerStyle(.segmented)

                // Type selection. 10+ options doesn't fit a segmented
                // picker — use a `Menu` so the row stays single-line.
                Menu {
                    Button("All Types") { typeFilter = nil }
                    Divider()
                    ForEach(Self.knownTypes, id: \.self) { t in
                        Button(displayName(forType: t)) { typeFilter = t }
                    }
                } label: {
                    HStack {
                        Text("Type")
                            .foregroundColor(.primary)
                        Spacer()
                        Text(typeFilter.map(displayName(forType:)) ?? "All")
                            .foregroundColor(.secondary)
                        Image(systemName: "chevron.up.chevron.down")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
            }
            // Render the filter-narrowed empty state INSIDE the List
            // (not as an `.overlay`) so the filter Section above
            // remains tappable. Overlaying ContentUnavailableView on
            // top of the same List that hosts the filter controls
            // makes "no matches" a dead-end — the user can't change
            // direction / type / search to recover. As inline list
            // content, the message scrolls in below the filter row
            // and leaves the controls fully reachable.
            if !records.isEmpty && visible.isEmpty {
                Section {
                    ContentUnavailableView(
                        "No matching transactions",
                        systemImage: "magnifyingglass",
                        description: Text("Adjust the search / direction / type filters")
                    )
                }
            }
            ForEach(visible) { record in
                NavigationLink(destination: TransactionStorageDetailView(record: record)) {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(record.txidHex)
                            .font(.system(.caption, design: .monospaced))
                            .lineLimit(1).truncationMode(.middle)
                        HStack(spacing: 8) {
                            Text(record.directionName).font(.caption)
                            // Only surface the type label when it
                            // isn't the default Classic — saves a
                            // line on the most-common row shape.
                            let normalizedType =
                                record.transactionType == "Standard"
                                    ? "Classic Transaction"
                                    : record.transactionType
                            if normalizedType != "Classic Transaction" {
                                Text(displayName(forType: normalizedType))
                                    .font(.caption2)
                                    .padding(.horizontal, 6)
                                    .padding(.vertical, 2)
                                    .background(Color.purple.opacity(0.15))
                                    .foregroundColor(.purple)
                                    .clipShape(Capsule())
                            }
                            Spacer()
                            Text(record.formattedAmount)
                                .font(.caption)
                                .foregroundColor(record.netAmount >= 0 ? .green : .red)
                        }
                    }
                }
            }
        }
        .navigationTitle(
            (directionFilter == .all && typeFilter == nil && searchText.isEmpty)
                ? "Transactions (\(records.count))"
                : "Transactions (\(visible.count) / \(records.count))"
        )
        .searchable(
            text: $searchText,
            placement: .navigationBarDrawer(displayMode: .always),
            prompt: "Search by tx id (hex)"
        )
        // Only the "no records at all" state uses an overlay — there's
        // nothing to interact with in that case, so blocking the List
        // is fine. The filter-narrowed empty state is rendered as
        // list content above (see the inline Section).
        .overlay {
            if records.isEmpty {
                ContentUnavailableView(
                    "No Records",
                    systemImage: "arrow.left.arrow.right.circle"
                )
            }
        }
    }

    /// Drop the redundant trailing "Transaction" word so the
    /// segmented row + picker stay readable. `"Asset Lock
    /// Transaction"` → `"Asset Lock"`, etc. The full string is what
    /// gets compared against `record.transactionType` — only the
    /// label is shortened.
    private func displayName(forType raw: String) -> String {
        let suffix = " Transaction"
        if raw.hasSuffix(suffix) {
            return String(raw.dropLast(suffix.count))
        }
        return raw
    }

    private enum DirectionFilter: CaseIterable, Hashable {
        case all
        case incoming
        case outgoing
        case internalTx
        case coinjoin

        var title: String {
            switch self {
            case .all: return "All"
            case .incoming: return "In"
            case .outgoing: return "Out"
            case .internalTx: return "Internal"
            case .coinjoin: return "CoinJoin"
            }
        }
    }
}

// MARK: - PersistentCoreAddress

struct CoreAddressStorageListView: View {
    /// Every Core-chain address record. PlatformPayment (DIP-17)
    /// addresses now live in their own `PersistentPlatformAddress`
    /// store, so no filtering is needed here.
    @Query(sort: [SortDescriptor(\PersistentCoreAddress.addressIndex)])
    private var records: [PersistentCoreAddress]

    /// Live-search query. Matches case-insensitively against the
    /// Base58Check address, derivation path, and address index.
    /// Empty string disables the filter.
    @State private var searchText: String = ""

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

    /// Records narrowed by `searchText`. Empty query passes
    /// everything through. Match runs case-insensitively against the
    /// address, derivation path, and stringified `addressIndex` so
    /// the user can paste a Base58Check, type "44'/1'", or just
    /// "/3" to find a specific row. Done before grouping so empty
    /// sections drop out cleanly.
    private var filteredRecords: [PersistentCoreAddress] {
        let trimmed = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return records }
        let needle = trimmed.lowercased()
        return records.filter { record in
            if record.address.lowercased().contains(needle) { return true }
            if record.derivationPath.lowercased().contains(needle) { return true }
            if String(record.addressIndex).contains(needle) { return true }
            return false
        }
    }

    /// Group addresses by (wallet, account). Addresses within a group
    /// are sorted by (pool tag, derivation index) so external pool
    /// entries come first, followed by internal, followed by any
    /// absent-pool entries — each in index order.
    private var groups: [(GroupKey, [PersistentCoreAddress])] {
        let grouped = Dictionary(grouping: filteredRecords) { record -> GroupKey in
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
        .navigationTitle("Core Addresses (\(filteredRecords.count))")
        .searchable(text: $searchText, prompt: "Search address, path, or index")
        .overlay {
            if records.isEmpty {
                ContentUnavailableView(
                    "No Records",
                    systemImage: "square.and.pencil"
                )
            } else if filteredRecords.isEmpty {
                ContentUnavailableView.search(text: searchText)
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

// MARK: - PersistentTxo

struct TxoStorageListView: View {
    /// Sort by block height descending — newest at the top, mempool
    /// (`height == 0`) ahead of confirmed entries. The previous
    /// `createdAt` sort meant rows were ordered by when they happened
    /// to flush into SwiftData rather than chain order, which fights
    /// the mental model when scanning for a specific block range.
    @Query(sort: [SortDescriptor(\PersistentTxo.height, order: .reverse)])
    private var records: [PersistentTxo]

    /// Spent / unspent filter. `.all` shows the full set (matches the
    /// previous behavior); `.unspent` and `.spent` narrow to the
    /// matching `isSpent` value. Toggled via a segmented Picker
    /// pinned to the top of the list so the choice survives scroll
    /// position.
    @State private var filter: SpentFilter = .all

    private var filteredRecords: [PersistentTxo] {
        switch filter {
        case .all: return records
        case .unspent: return records.filter { !$0.isSpent }
        case .spent: return records.filter { $0.isSpent }
        }
    }

    var body: some View {
        let visible = filteredRecords
        List {
            Section {
                Picker("Filter", selection: $filter) {
                    ForEach(SpentFilter.allCases, id: \.self) { f in
                        Text(f.title).tag(f)
                    }
                }
                .pickerStyle(.segmented)
            }
            // Render the filter-narrowed empty state as inline list
            // content rather than as an `.overlay` over the List —
            // see the matching note in `TransactionStorageListView`.
            // Overlay would block the segmented Picker above and
            // dead-end the user.
            if !records.isEmpty && visible.isEmpty {
                Section {
                    ContentUnavailableView(
                        "No \(filter.title) TXOs",
                        systemImage: "bitcoinsign.circle"
                    )
                }
            }
            ForEach(visible) { record in
                NavigationLink(destination: TxoStorageDetailView(record: record)) {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(record.outpointHex)
                            .font(.system(.caption, design: .monospaced))
                            .lineLimit(1).truncationMode(.middle)
                        HStack(spacing: 8) {
                            Text(record.formattedAmount).font(.caption)
                            // `height == 0` is the SPV convention for
                            // "not yet in a block" (mempool /
                            // unconfirmed). Render as a friendly
                            // string instead of a literal "0" so the
                            // distinction reads clearly.
                            Text(record.height == 0 ? "mempool" : "h \(record.height)")
                                .font(.caption2)
                                .foregroundColor(.secondary)
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
        }
        .navigationTitle(filter == .all
            ? "TXOs (\(records.count))"
            : "TXOs (\(visible.count) / \(records.count))")
        // Overlay only the no-records-at-all case; the
        // filter-narrowed empty state lives inline above so the
        // Picker stays reachable.
        .overlay {
            if records.isEmpty {
                ContentUnavailableView("No Records", systemImage: "bitcoinsign.circle")
            }
        }
    }

    private enum SpentFilter: CaseIterable, Hashable {
        case all
        case unspent
        case spent

        var title: String {
            switch self {
            case .all: return "All"
            case .unspent: return "Unspent"
            case .spent: return "Spent"
            }
        }
    }
}

// MARK: - PersistentPendingInput

/// Diagnostic list of every `PersistentPendingInput` row — one per
/// transaction-input outpoint whose previous-output `PersistentTxo`
/// hasn't landed in SwiftData yet. A non-empty list is informational
/// rather than alarming on its own (entries resolve and self-delete
/// when the matching `upsertUtxo` arrives), but a *long-lived*
/// non-zero count points at a real reconciliation gap — addresses
/// that the wallet never derives, or input outpoints whose previous
/// output isn't ours and will never resolve.
struct PendingInputStorageListView: View {
    @Query(sort: [SortDescriptor(\PersistentPendingInput.createdAt, order: .reverse)])
    private var records: [PersistentPendingInput]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: PendingInputStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(outpointHex(record.outpoint))
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(1).truncationMode(.middle)
                    HStack(spacing: 8) {
                        Text("input \(record.inputIndex)")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(record.createdAt, style: .relative)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
        .navigationTitle("Pending Inputs (\(records.count))")
        .overlay {
            if records.isEmpty {
                ContentUnavailableView(
                    "No Pending Inputs",
                    systemImage: "hourglass"
                )
            }
        }
    }

    /// 36-byte outpoint as `<txid hex (display order)>:<vout>`. Mirrors
    /// `PersistentTxo.outpointHex` so the same row is identifiable
    /// across both surfaces.
    private func outpointHex(_ outpoint: Data) -> String {
        guard outpoint.count == 36 else {
            return outpoint.map { String(format: "%02x", $0) }.joined()
        }
        let txid = outpoint.prefix(32)
        let voutBytes = outpoint.suffix(4)
        let vout = voutBytes.withUnsafeBytes { raw in
            raw.load(as: UInt32.self).littleEndian
        }
        let txidHex = txid.reversed().map { String(format: "%02x", $0) }.joined()
        return "\(txidHex):\(vout)"
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
                    Text(record.network.displayName).font(.body)
                    Text("Height \(record.combinedSyncHeight) · \(record.walletCount) wallets")
                        .font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Manager Metadata (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "gearshape.2") } }
    }
}
