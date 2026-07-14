import SwiftUI
import SwiftData
import SwiftDashSDK

// MARK: - PersistentIdentity

struct IdentityStorageListView: View {
    let network: Network
    @Query(sort: \PersistentIdentity.lastUpdated, order: .reverse)
    private var records: [PersistentIdentity]

    private var filtered: [PersistentIdentity] {
        records.filter { $0.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
            NavigationLink(destination: IdentityStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.dpnsName ?? record.alias ?? record.identityIdBase58)
                        .font(.body).lineLimit(1)
                    Text(record.formattedBalance)
                        .font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Identities (\(visible.count))")
        .overlay { if visible.isEmpty { ContentUnavailableView("No Records", systemImage: "person.crop.circle") } }
    }
}

// MARK: - PersistentDocument

struct DocumentStorageListView: View {
    let network: Network
    @Query(sort: \PersistentDocument.localUpdatedAt, order: .reverse)
    private var records: [PersistentDocument]

    private var filtered: [PersistentDocument] {
        records.filter { $0.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
            NavigationLink(destination: DocumentStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.displayTitle).font(.body).lineLimit(1)
                    Text(record.documentType).font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Documents (\(visible.count))")
        .overlay { if visible.isEmpty { ContentUnavailableView("No Records", systemImage: "doc.text") } }
    }
}

// MARK: - PersistentDataContract

struct DataContractStorageListView: View {
    let network: Network
    @Query(sort: \PersistentDataContract.lastAccessedAt, order: .reverse)
    private var records: [PersistentDataContract]

    private var filtered: [PersistentDataContract] {
        records.filter { $0.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
            NavigationLink(destination: DataContractStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.name).font(.body).lineLimit(1)
                    Text(record.idBase58).font(.caption).foregroundColor(.secondary).lineLimit(1).truncationMode(.middle)
                }
            }
        }
        .navigationTitle("Data Contracts (\(visible.count))")
        .overlay { if visible.isEmpty { ContentUnavailableView("No Records", systemImage: "doc.plaintext") } }
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
    let network: Network
    @Environment(\.modelContext) private var modelContext
    @Query(sort: \PersistentIdentity.identityIndex)
    private var identities: [PersistentIdentity]

    @Query(sort: \PersistentPublicKey.createdAt, order: .reverse)
    private var allKeys: [PersistentPublicKey]

    @Query private var hdWallets: [PersistentWallet]

    /// Identities scoped to the active network. The keys, wallet
    /// labels, and orphan section all derive from this.
    private var scopedIdentities: [PersistentIdentity] {
        identities.filter { $0.networkRaw == network.rawValue }
    }

    /// Keys whose owning identity is on the active network. Orphan
    /// keys (no parent identity) drop out of the per-network view
    /// entirely — duplicating them across mainnet / testnet / devnet
    /// / regtest would re-introduce the cross-network leakage this
    /// PR removes and would also disagree with
    /// `StorageExplorerView.loadCounts()` (which evaluates
    /// `$0.identity?.networkRaw == raw` and excludes nil identities
    /// the same way). If global orphan-key diagnostics are needed
    /// later they belong on a separate, network-agnostic surface.
    private var scopedKeys: [PersistentPublicKey] {
        allKeys.filter { key in
            guard let identity = key.identity else { return false }
            return identity.networkRaw == network.rawValue
        }
    }

    /// Key targeted by a pending Remove swipe. Non-nil presents the
    /// confirmation dialog; holding the reference lets the dialog
    /// show `Key N` without re-fetching.
    @State private var keyPendingRemoval: PersistentPublicKey?

    var body: some View {
        let scoped = scopedKeys
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
        .navigationTitle("Public Keys (\(scoped.count))")
        .overlay {
            if scoped.isEmpty {
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
        let grouped = Dictionary(grouping: scopedIdentities) { $0.wallet?.walletId ?? Data() }
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
    /// deleted. `scopedKeys` already strips them from the
    /// per-network view, so this collection is always empty in the
    /// current explorer; the section render below short-circuits on
    /// `isEmpty`. Kept as a one-liner so a future global
    /// orphan-diagnostics surface can reuse it.
    private var orphanKeys: [PersistentPublicKey] {
        scopedKeys.filter { $0.identity == nil }
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
    let network: Network
    @Query(sort: \PersistentDPNSName.acquiredAt, order: .reverse)
    private var records: [PersistentDPNSName]

    private var filtered: [PersistentDPNSName] {
        records.filter { $0.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
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
        .navigationTitle("DPNS Names (\(visible.count))")
        .overlay {
            if visible.isEmpty {
                ContentUnavailableView("No Records", systemImage: "at")
            }
        }
    }
}

// MARK: - PersistentDashpayProfile

/// Storage-explorer list of every cached DashPay profile. One row
/// per (network, identity). Newest profile update first.
struct DashpayProfileStorageListView: View {
    let network: Network
    @Query(sort: \PersistentDashpayProfile.lastUpdated, order: .reverse)
    private var records: [PersistentDashpayProfile]

    private var filtered: [PersistentDashpayProfile] {
        records.filter { $0.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
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
        .navigationTitle("DashPay Profiles (\(visible.count))")
        .overlay {
            if visible.isEmpty {
                ContentUnavailableView("No Records", systemImage: "person.text.rectangle")
            }
        }
    }
}

// MARK: - PersistentDashpayContactProfile

/// Storage-explorer list of every cached contact profile (a counterparty's
/// DashPay profile). One row per (owner, contact). Newest update first.
struct DashpayContactProfileStorageListView: View {
    let network: Network
    @Query(sort: \PersistentDashpayContactProfile.lastUpdated, order: .reverse)
    private var records: [PersistentDashpayContactProfile]

    private var filtered: [PersistentDashpayContactProfile] {
        records.filter { $0.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
            NavigationLink(destination: DashpayContactProfileStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.displayName ?? "(no display name)")
                        .font(.body).lineLimit(1)
                    Text(record.contactIdentityId.toHexString())
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
            }
        }
        .navigationTitle("Contact Profiles (\(visible.count))")
        .overlay {
            if visible.isEmpty {
                ContentUnavailableView("No Records", systemImage: "person.crop.circle")
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
    let network: Network
    @Query private var records: [PersistentDashpayContactRequest]

    private var scoped: [PersistentDashpayContactRequest] {
        records.filter { $0.networkRaw == network.rawValue }
    }

    private var outgoing: [PersistentDashpayContactRequest] {
        scoped.filter { $0.isOutgoing }
            .sorted { $0.createdAtMillis > $1.createdAtMillis }
    }

    private var incoming: [PersistentDashpayContactRequest] {
        scoped.filter { !$0.isOutgoing }
            .sorted { $0.createdAtMillis > $1.createdAtMillis }
    }

    var body: some View {
        let visible = scoped
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
        .navigationTitle("Contact Requests (\(visible.count))")
        .overlay {
            if visible.isEmpty {
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

// MARK: - PersistentDashpayPayment

struct DashpayPaymentStorageListView: View {
    let network: Network
    @Query(sort: \PersistentDashpayPayment.createdAt, order: .reverse)
    private var records: [PersistentDashpayPayment]

    private var scoped: [PersistentDashpayPayment] {
        records.filter { $0.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = scoped
        List(visible) { record in
            NavigationLink(destination: DashpayPaymentStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text(record.direction == .sent ? "Sent" : "Received")
                            .font(.body)
                        Spacer()
                        Text(String(format: "%.8f DASH", Double(record.amountDuffs) / 100_000_000))
                            .font(.system(.caption, design: .monospaced))
                    }
                    Text(record.txid)
                        .font(.system(.caption2, design: .monospaced))
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
            }
        }
        .navigationTitle("DashPay Payments (\(visible.count))")
        .overlay {
            if visible.isEmpty {
                ContentUnavailableView(
                    "No Records",
                    systemImage: "arrow.left.arrow.right.circle"
                )
            }
        }
    }
}

// MARK: - PersistentInvitation

struct InvitationStorageListView: View {
    let network: Network
    @Query(sort: [SortDescriptor(\PersistentInvitation.createdAtSecs, order: .reverse)])
    private var records: [PersistentInvitation]

    @Query private var allWallets: [PersistentWallet]

    private var walletIdsOnNetwork: Set<Data> {
        Set(allWallets.lazy
            .filter { $0.networkRaw == network.rawValue }
            .map(\.walletId))
    }

    private var scopedRecords: [PersistentInvitation] {
        let ids = walletIdsOnNetwork
        return records.filter { ids.contains($0.walletId) }
    }

    var body: some View {
        let visible = scopedRecords
        List(visible) { record in
            NavigationLink(destination: InvitationStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.outPointHex)
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(1).truncationMode(.middle)
                    HStack(spacing: 8) {
                        Text(invitationStatusLabel(record.statusRaw))
                            .font(.caption2)
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(String(format: "%.8f DASH", Double(record.amountDuffs) / 100_000_000))
                            .font(.system(.caption2, design: .monospaced))
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
        .navigationTitle("Sent Invitations (\(visible.count))")
        .overlay {
            if visible.isEmpty {
                ContentUnavailableView(
                    "No Invitations",
                    systemImage: "paperplane"
                )
            }
        }
    }
}

// MARK: - PersistentDashpayIgnoredSender

struct DashpayIgnoredSenderStorageListView: View {
    let network: Network
    @Query(sort: \PersistentDashpayIgnoredSender.ignoredAt, order: .reverse)
    private var records: [PersistentDashpayIgnoredSender]

    private var scoped: [PersistentDashpayIgnoredSender] {
        records.filter { $0.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = scoped
        List(visible) { record in
            NavigationLink(destination: DashpayIgnoredSenderStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text("ignored sender")
                            .font(.body)
                        Spacer()
                        Text(record.ignoredAt, style: .date)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    Text(record.ignoredSenderId.toHexString())
                        .font(.system(.caption2, design: .monospaced))
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
            }
        }
        .navigationTitle("Ignored Senders (\(visible.count))")
        .overlay {
            if visible.isEmpty {
                ContentUnavailableView(
                    "No Records",
                    systemImage: "person.crop.circle.badge.xmark"
                )
            }
        }
    }
}

// MARK: - PersistentToken

struct TokenStorageListView: View {
    let network: Network
    @Query(sort: \PersistentToken.name)
    private var records: [PersistentToken]

    /// Tokens trace their network through the parent contract; the
    /// token row itself doesn't store a `networkRaw` column.
    private var filtered: [PersistentToken] {
        records.filter { $0.dataContract?.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
            NavigationLink(destination: TokenStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.name).font(.body).lineLimit(1)
                    Text(record.formattedBaseSupply).font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Tokens (\(visible.count))")
        .overlay { if visible.isEmpty { ContentUnavailableView("No Records", systemImage: "circle.hexagongrid") } }
    }
}

// MARK: - PersistentTokenBalance

struct TokenBalanceStorageListView: View {
    let network: Network
    @Query(sort: \PersistentTokenBalance.lastUpdated, order: .reverse)
    private var records: [PersistentTokenBalance]

    private var filtered: [PersistentTokenBalance] {
        records.filter { $0.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
            NavigationLink(destination: TokenBalanceStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.tokenName ?? record.tokenId).font(.body).lineLimit(1)
                    Text(record.displayBalance).font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Token Balances (\(visible.count))")
        .overlay { if visible.isEmpty { ContentUnavailableView("No Records", systemImage: "banknote") } }
    }
}

// MARK: - PersistentTokenHistoryEvent

struct TokenHistoryStorageListView: View {
    let network: Network
    @Query(sort: \PersistentTokenHistoryEvent.createdAt, order: .reverse)
    private var records: [PersistentTokenHistoryEvent]

    /// Token-history rows trace their network through `token →
    /// dataContract.networkRaw`. Both relationships are optional so
    /// any orphan row drops out of the explorer.
    private var filtered: [PersistentTokenHistoryEvent] {
        records.filter { $0.token?.dataContract?.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
            NavigationLink(destination: TokenHistoryStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.displayTitle).font(.body).lineLimit(1)
                    Text(AppDate.formatted(record.eventTimestamp, dateStyle: .abbreviated, timeStyle: .omitted))
                        .font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Token History (\(visible.count))")
        .overlay { if visible.isEmpty { ContentUnavailableView("No Records", systemImage: "clock.arrow.circlepath") } }
    }
}

// MARK: - PersistentDocumentType

struct DocumentTypeStorageListView: View {
    let network: Network
    @Query(sort: \PersistentDocumentType.name)
    private var records: [PersistentDocumentType]

    private var filtered: [PersistentDocumentType] {
        records.filter { $0.dataContract?.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
            NavigationLink(destination: DocumentTypeStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.name).font(.body).lineLimit(1)
                    Text(record.contractIdBase58).font(.caption).foregroundColor(.secondary).lineLimit(1).truncationMode(.middle)
                }
            }
        }
        .navigationTitle("Document Types (\(visible.count))")
        .overlay { if visible.isEmpty { ContentUnavailableView("No Records", systemImage: "list.bullet.rectangle") } }
    }
}

// MARK: - PersistentIndex

struct IndexStorageListView: View {
    let network: Network
    @Query(sort: \PersistentIndex.name)
    private var records: [PersistentIndex]

    /// Indices live two relationships deep: documentType → dataContract.
    /// Orphan rows (broken chain) drop out — they have no network to
    /// attribute them to.
    private var filtered: [PersistentIndex] {
        records.filter { $0.documentType?.dataContract?.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
            NavigationLink(destination: IndexStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.name).font(.body).lineLimit(1)
                    Text(record.documentTypeName).font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Indices (\(visible.count))")
        .overlay { if visible.isEmpty { ContentUnavailableView("No Records", systemImage: "tablecells") } }
    }
}

// MARK: - PersistentProperty

struct PropertyStorageListView: View {
    let network: Network
    @Query(sort: \PersistentProperty.name)
    private var records: [PersistentProperty]

    private var filtered: [PersistentProperty] {
        records.filter { $0.documentType?.dataContract?.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
            NavigationLink(destination: PropertyStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.name).font(.body).lineLimit(1)
                    Text(record.type).font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Properties (\(visible.count))")
        .overlay { if visible.isEmpty { ContentUnavailableView("No Records", systemImage: "slider.horizontal.3") } }
    }
}

// MARK: - PersistentKeyword

struct KeywordStorageListView: View {
    let network: Network
    @Query(sort: \PersistentKeyword.keyword)
    private var records: [PersistentKeyword]

    private var filtered: [PersistentKeyword] {
        records.filter { $0.dataContract?.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
            NavigationLink(destination: KeywordStorageDetailView(record: record)) {
                Text(record.keyword).font(.body)
            }
        }
        .navigationTitle("Keywords (\(visible.count))")
        .overlay { if visible.isEmpty { ContentUnavailableView("No Records", systemImage: "tag") } }
    }
}

// MARK: - PersistentPlatformAddressesSyncState

struct PlatformAddressesSyncStateStorageListView: View {
    let network: Network
    @Query(sort: \PersistentPlatformAddressesSyncState.lastUpdated, order: .reverse)
    private var records: [PersistentPlatformAddressesSyncState]

    private var filtered: [PersistentPlatformAddressesSyncState] {
        records.filter { $0.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
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
        .navigationTitle("Platform Addresses Sync State (\(visible.count))")
        .overlay { if visible.isEmpty { ContentUnavailableView("No Records", systemImage: "arrow.triangle.2.circlepath") } }
    }
}

// MARK: - PersistentWallet

struct WalletStorageListView: View {
    let network: Network
    @Query(sort: \PersistentWallet.lastUpdated, order: .reverse)
    private var records: [PersistentWallet]

    /// `networkRaw` is optional on this model: a row can predate the
    /// persister's network-fill step. Strict filter — `nil` rows
    /// don't match any active network and are hidden until the
    /// persister fills them in.
    private var filtered: [PersistentWallet] {
        records.filter { $0.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
            NavigationLink(destination: WalletStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.name ?? record.walletId.map { String(format: "%02x", $0) }.prefix(16).joined())
                        .font(.body).lineLimit(1)
                    Text("\(record.network?.displayName ?? "Unknown") · height \(record.syncedHeight)")
                        .font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Wallets (\(visible.count))")
        .overlay { if visible.isEmpty { ContentUnavailableView("No Records", systemImage: "wallet.pass") } }
    }
}

// MARK: - PersistentAccount

struct AccountStorageListView: View {
    let network: Network
    /// Query the wallet side of the `@Relationship` so the list groups
    /// by wallet. Accounts come through `wallet.accounts`, which
    /// updates reactively under SwiftData.
    @Query(sort: \PersistentWallet.createdAt, order: .reverse)
    private var wallets: [PersistentWallet]

    /// Wallets on the active network. Accounts inherit the wallet's
    /// network — there is no per-account `networkRaw` column.
    private var scopedWallets: [PersistentWallet] {
        wallets.filter { $0.networkRaw == network.rawValue }
    }

    private var totalAccountCount: Int {
        scopedWallets.reduce(0) { $0 + $1.accounts.count }
    }

    var body: some View {
        List {
            ForEach(scopedWallets) { wallet in
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
    let network: Network
    @Query(sort: \PersistentTransaction.firstSeen, order: .reverse)
    private var records: [PersistentTransaction]

    /// Wallets on the active network. Transactions don't carry a
    /// `networkRaw` column; we match a tx to a network by checking
    /// whether *any* of its TXOs (input or output) carry a
    /// `walletId` that resolves to a wallet on this network.
    @Query private var allWallets: [PersistentWallet]

    private var walletIdsOnNetwork: Set<Data> {
        Set(allWallets.lazy
            .filter { $0.networkRaw == network.rawValue }
            .map(\.walletId))
    }

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

    /// Records whose at-least-one TXO points at a wallet on the
    /// active network. The base set the search/direction/type
    /// filters then narrow further. Counts on the title row are
    /// against this set, not `records`, so they reflect the
    /// network-scoped universe rather than the cross-network store.
    private var networkScopedRecords: [PersistentTransaction] {
        let ids = walletIdsOnNetwork
        return records.filter { tx in
            for txo in tx.outputs where ids.contains(txo.walletId) { return true }
            for txo in tx.inputs where ids.contains(txo.walletId) { return true }
            return false
        }
    }

    private var filteredRecords: [PersistentTransaction] {
        let trimmed = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        let needle = trimmed.lowercased()
        return networkScopedRecords.filter { record in
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
        let scoped = networkScopedRecords
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
            if !scoped.isEmpty && visible.isEmpty {
                Section {
                    ContentUnavailableView(
                        "No matching transactions",
                        systemImage: "magnifyingglass",
                        description: Text("Adjust the search / direction / type filters")
                    )
                }
            }
            ForEach(visible) { record in
                transactionRow(record)
            }
        }
        .navigationTitle(
            (directionFilter == .all && typeFilter == nil && searchText.isEmpty)
                ? "Transactions (\(scoped.count))"
                : "Transactions (\(visible.count) / \(scoped.count))"
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
            if scoped.isEmpty {
                ContentUnavailableView(
                    "No Records",
                    systemImage: "arrow.left.arrow.right.circle"
                )
            }
        }
    }

    /// Shared row builder for both sections ("Identity Funding" and
    /// "Transactions"). Uses [`displayDirection`] so asset-lock rows
    /// read as "Asset Lock" rather than the structurally-correct-but-
    /// misleading "Internal" label.
    @ViewBuilder
    private func transactionRow(_ record: PersistentTransaction) -> some View {
        NavigationLink(destination: TransactionStorageDetailView(record: record)) {
            VStack(alignment: .leading, spacing: 4) {
                Text(record.txidHex)
                    .font(.system(.caption, design: .monospaced))
                    .lineLimit(1).truncationMode(.middle)
                HStack(spacing: 8) {
                    Text(record.displayDirection).font(.caption)
                    // Only surface the type label when it isn't the
                    // default Classic — saves a line on the most-common
                    // row shape. Asset-lock rows also get the badge so
                    // the visual stays consistent if the user filters
                    // away the Identity Funding section header.
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
    let network: Network
    /// Every Core-chain address record. PlatformPayment (DIP-17)
    /// addresses now live in their own `PersistentPlatformAddress`
    /// store, so no filtering is needed here.
    @Query(sort: [SortDescriptor(\PersistentCoreAddress.addressIndex)])
    private var records: [PersistentCoreAddress]

    /// Network filter. CoreAddress doesn't have a `networkRaw`
    /// column; the network is owned by the parent wallet via
    /// `account.wallet.networkRaw`. Rows whose `account` link is
    /// nil have no resolvable network and drop out — they're
    /// recovered by the persister on next sync.
    private var scopedRecords: [PersistentCoreAddress] {
        records.filter { $0.account?.wallet.networkRaw == network.rawValue }
    }

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
        guard !trimmed.isEmpty else { return scopedRecords }
        let needle = trimmed.lowercased()
        return scopedRecords.filter { record in
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
            if scopedRecords.isEmpty {
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
    let network: Network
    @Query(sort: [SortDescriptor(\PersistentPlatformAddress.addressIndex)])
    private var records: [PersistentPlatformAddress]

    /// Wallet-id set used for the fallback path when a row's
    /// `account` join hasn't been hydrated yet.
    @Query private var allWallets: [PersistentWallet]

    private var walletIdsOnNetwork: Set<Data> {
        Set(allWallets.lazy
            .filter { $0.networkRaw == network.rawValue }
            .map(\.walletId))
    }

    private var scopedRecords: [PersistentPlatformAddress] {
        let raw = network.rawValue
        let ids = walletIdsOnNetwork
        return records.filter { entry in
            if let walletRaw = entry.account?.wallet.networkRaw {
                return walletRaw == raw
            }
            return ids.contains(entry.walletId)
        }
    }

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
        let grouped = Dictionary(grouping: scopedRecords) { record -> GroupKey in
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
        let visible = scopedRecords
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
        .navigationTitle("Platform Addresses (\(visible.count))")
        .overlay {
            if visible.isEmpty {
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
    let network: Network
    /// Sort by block height descending — newest at the top, mempool
    /// (`height == 0`) ahead of confirmed entries. The previous
    /// `createdAt` sort meant rows were ordered by when they happened
    /// to flush into SwiftData rather than chain order, which fights
    /// the mental model when scanning for a specific block range.
    @Query(sort: [SortDescriptor(\PersistentTxo.height, order: .reverse)])
    private var records: [PersistentTxo]

    /// Wallets on the active network. Used to scope the TXO list by
    /// the denormalized `walletId` column — every TXO carries one,
    /// so this is a straight `Set` lookup rather than an optional
    /// chain through the `account` relationship.
    @Query private var allWallets: [PersistentWallet]

    private var walletIdsOnNetwork: Set<Data> {
        Set(allWallets.lazy
            .filter { $0.networkRaw == network.rawValue }
            .map(\.walletId))
    }

    private var scopedRecords: [PersistentTxo] {
        let ids = walletIdsOnNetwork
        return records.filter { ids.contains($0.walletId) }
    }

    /// Spent / unspent filter. `.all` shows the full set (matches the
    /// previous behavior); `.unspent` and `.spent` narrow to the
    /// matching `isSpent` value. Toggled via a segmented Picker
    /// pinned to the top of the list so the choice survives scroll
    /// position.
    @State private var filter: SpentFilter = .all

    private var filteredRecords: [PersistentTxo] {
        let scoped = scopedRecords
        switch filter {
        case .all: return scoped
        case .unspent: return scoped.filter { !$0.isSpent }
        case .spent: return scoped.filter { $0.isSpent }
        }
    }

    var body: some View {
        let scoped = scopedRecords
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
            if !scoped.isEmpty && visible.isEmpty {
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
            ? "TXOs (\(scoped.count))"
            : "TXOs (\(visible.count) / \(scoped.count))")
        // Overlay only the no-records-at-all case; the
        // filter-narrowed empty state lives inline above so the
        // Picker stays reachable.
        .overlay {
            if scoped.isEmpty {
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
    let network: Network
    @Query(sort: [SortDescriptor(\PersistentPendingInput.createdAt, order: .reverse)])
    private var records: [PersistentPendingInput]

    @Query private var allWallets: [PersistentWallet]

    private var walletIdsOnNetwork: Set<Data> {
        Set(allWallets.lazy
            .filter { $0.networkRaw == network.rawValue }
            .map(\.walletId))
    }

    private var scopedRecords: [PersistentPendingInput] {
        let ids = walletIdsOnNetwork
        return records.filter { ids.contains($0.walletId) }
    }

    var body: some View {
        let visible = scopedRecords
        List(visible) { record in
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
        .navigationTitle("Pending Inputs (\(visible.count))")
        .overlay {
            if visible.isEmpty {
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

// MARK: - PersistentAssetLock

/// Storage explorer surface for tracked asset locks. SwiftData-backed
/// — every row is upserted by the Rust-side `on_persist_asset_locks_fn`
/// callback as the lock advances through Built / Broadcast /
/// InstantSendLocked / ChainLocked, and deleted when the registration
/// consumes it. The same rows also drive `RegistrationProgressView`
/// (iter 3 part 2).
struct AssetLockStorageListView: View {
    let network: Network
    @Query(sort: [SortDescriptor(\PersistentAssetLock.updatedAt, order: .reverse)])
    private var records: [PersistentAssetLock]

    @Query private var allWallets: [PersistentWallet]

    private var walletIdsOnNetwork: Set<Data> {
        Set(allWallets.lazy
            .filter { $0.networkRaw == network.rawValue }
            .map(\.walletId))
    }

    private var scopedRecords: [PersistentAssetLock] {
        let ids = walletIdsOnNetwork
        return records.filter { ids.contains($0.walletId) }
    }

    var body: some View {
        let visible = scopedRecords
        List(visible) { record in
            NavigationLink(destination: AssetLockStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.outPointHex)
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(1).truncationMode(.middle)
                    HStack(spacing: 8) {
                        Text(record.statusLabel)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                        Spacer()
                        Text("identity #\(record.identityIndexRaw)")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                        Text(record.updatedAt, style: .relative)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
        .navigationTitle("Asset Locks (\(visible.count))")
        .overlay {
            if visible.isEmpty {
                ContentUnavailableView(
                    "No Asset Locks",
                    systemImage: "lock.shield"
                )
            }
        }
    }
}

// MARK: - PersistentMasternode

/// Masternode entities aggregated by Rust from a wallet's provider
/// special transactions. Scoped to the active network via the
/// `walletId`→wallet join (masternodes carry no `networkRaw` column),
/// sorted by the stable cross-type registration order.
struct MasternodeStorageListView: View {
    let network: Network
    @Query(sort: [SortDescriptor(\PersistentMasternode.orderIndex)])
    private var records: [PersistentMasternode]

    @Query private var allWallets: [PersistentWallet]

    private var walletIdsOnNetwork: Set<Data> {
        Set(allWallets.lazy
            .filter { $0.networkRaw == network.rawValue }
            .map(\.walletId))
    }

    private var scopedRecords: [PersistentMasternode] {
        let ids = walletIdsOnNetwork
        return records.filter { ids.contains($0.walletId) }
    }

    var body: some View {
        let visible = scopedRecords
        List(visible) { record in
            NavigationLink(destination: MasternodeStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 8) {
                        Text(record.displayTitle)
                            .font(.body)
                        Spacer()
                        Text(record.statusName)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                    Text(record.serviceAddress ?? "—")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                    Text(record.proTxHashShort)
                        .font(.system(.caption2, design: .monospaced))
                        .foregroundColor(.secondary)
                        .lineLimit(1).truncationMode(.middle)
                }
            }
        }
        .navigationTitle("Masternodes (\(visible.count))")
        .overlay {
            if visible.isEmpty {
                ContentUnavailableView(
                    "No Masternodes",
                    systemImage: "server.rack"
                )
            }
        }
    }
}

// MARK: - PersistentWalletManagerMetadata

struct WalletManagerMetadataStorageListView: View {
    let network: Network
    @Query(sort: \PersistentWalletManagerMetadata.lastUpdated, order: .reverse)
    private var records: [PersistentWalletManagerMetadata]

    private var filtered: [PersistentWalletManagerMetadata] {
        records.filter { $0.networkRaw == network.rawValue }
    }

    var body: some View {
        let visible = filtered
        List(visible) { record in
            NavigationLink(destination: WalletManagerMetadataStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.network.displayName).font(.body)
                    Text("Height \(record.combinedSyncHeight) · \(record.walletCount) wallets")
                        .font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Manager Metadata (\(visible.count))")
        .overlay { if visible.isEmpty { ContentUnavailableView("No Records", systemImage: "gearshape.2") } }
    }
}

// MARK: - PersistentShieldedNote

/// Filter enum local to this view — mirrors the private one
/// inside `TxoStorageListView`. Both views need the same
/// "all / unspent / spent" segmented control; duplicating two
/// lines beats hoisting the private type to file scope and
/// touching the existing TXO view.
private enum ShieldedSpentFilter: CaseIterable, Hashable {
    case all, unspent, spent

    var title: String {
        switch self {
        case .all: return "All"
        case .unspent: return "Unspent"
        case .spent: return "Spent"
        }
    }
}

/// Read-only browser for the per-(wallet, account) decrypted
/// shielded notes the persister mirrors out of
/// `ShieldedChangeSet`. Scoped by the active network via the
/// denormalized `walletId` column on each row — same trick
/// `TxoStorageListView` uses.
struct ShieldedNoteStorageListView: View {
    let network: Network

    /// Sort by block height (newest first), then position so
    /// rows from the same block stay deterministic.
    @Query(
        sort: [
            SortDescriptor(\PersistentShieldedNote.blockHeight, order: .reverse),
            SortDescriptor(\PersistentShieldedNote.position),
        ]
    )
    private var records: [PersistentShieldedNote]

    @Query private var allWallets: [PersistentWallet]

    private var walletIdsOnNetwork: Set<Data> {
        Set(allWallets.lazy
            .filter { $0.networkRaw == network.rawValue }
            .map(\.walletId))
    }

    private var scopedRecords: [PersistentShieldedNote] {
        let ids = walletIdsOnNetwork
        return records.filter { ids.contains($0.walletId) }
    }

    @State private var filter: ShieldedSpentFilter = .all

    private var filteredRecords: [PersistentShieldedNote] {
        let scoped = scopedRecords
        switch filter {
        case .all: return scoped
        case .unspent: return scoped.filter { !$0.isSpent }
        case .spent: return scoped.filter { $0.isSpent }
        }
    }

    var body: some View {
        let scoped = scopedRecords
        let visible = filteredRecords
        List {
            Section {
                Picker("Filter", selection: $filter) {
                    ForEach(ShieldedSpentFilter.allCases, id: \.self) { f in
                        Text(f.title).tag(f)
                    }
                }
                .pickerStyle(.segmented)
            }
            if !scoped.isEmpty && visible.isEmpty {
                Section {
                    ContentUnavailableView(
                        "No \(filter.title) Notes",
                        systemImage: "lock.shield"
                    )
                }
            }
            ForEach(visible) { record in
                NavigationLink(destination: ShieldedNoteStorageDetailView(record: record)) {
                    VStack(alignment: .leading, spacing: 4) {
                        HStack(spacing: 8) {
                            Text("acct \(record.accountIndex)")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            Text("pos \(record.position)")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            if record.blockHeight > 0 {
                                Text("h \(record.blockHeight)")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                            }
                            Spacer()
                            if record.isSpent {
                                Text("spent")
                                    .font(.caption2)
                                    .foregroundColor(.red)
                            }
                        }
                        Text("\(record.value) credits")
                            .font(.caption)
                        Text(record.nullifier.prefix(8).map { String(format: "%02x", $0) }.joined())
                            .font(.system(.caption2, design: .monospaced))
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
        .navigationTitle("Shielded Notes (\(visible.count))")
        .overlay {
            if scoped.isEmpty {
                ContentUnavailableView("No Notes", systemImage: "lock.shield")
            }
        }
    }
}

// MARK: - PersistentShieldedOutgoingNote

/// Read-only browser for the per-(wallet, account) OVK-recovered
/// outgoing (sent) shielded notes the persister mirrors out of
/// `ShieldedChangeSet::outgoing_notes`. Scoped by the active network
/// via the denormalized `walletId` column — same trick
/// `ShieldedNoteStorageListView` uses.
struct ShieldedOutgoingNoteStorageListView: View {
    let network: Network

    /// Sort by block height (newest first), then account index so
    /// rows from the same block stay deterministic. `cmx` is `Data`
    /// (not Comparable), so it can't be a sort key.
    @Query(
        sort: [
            SortDescriptor(\PersistentShieldedOutgoingNote.blockHeight, order: .reverse),
            SortDescriptor(\PersistentShieldedOutgoingNote.accountIndex),
        ]
    )
    private var records: [PersistentShieldedOutgoingNote]

    @Query private var allWallets: [PersistentWallet]

    private var walletIdsOnNetwork: Set<Data> {
        Set(allWallets.lazy
            .filter { $0.networkRaw == network.rawValue }
            .map(\.walletId))
    }

    private var scopedRecords: [PersistentShieldedOutgoingNote] {
        let ids = walletIdsOnNetwork
        return records.filter { ids.contains($0.walletId) }
    }

    var body: some View {
        let visible = scopedRecords
        List {
            ForEach(visible) { record in
                NavigationLink(
                    destination: ShieldedOutgoingNoteStorageDetailView(record: record)
                ) {
                    VStack(alignment: .leading, spacing: 4) {
                        HStack(spacing: 8) {
                            Text("acct \(record.accountIndex)")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            if record.blockHeight > 0 {
                                Text("h \(record.blockHeight)")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                            }
                            Spacer()
                        }
                        Text("\(record.value) credits")
                            .font(.caption)
                        Text(record.cmx.prefix(8).map { String(format: "%02x", $0) }.joined())
                            .font(.system(.caption2, design: .monospaced))
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
        .navigationTitle("Shielded Sent Notes (\(visible.count))")
        .overlay {
            if visible.isEmpty {
                ContentUnavailableView("No Sent Notes", systemImage: "paperplane")
            }
        }
    }
}

// MARK: - PersistentShieldedActivity

/// Read-only browser for the derived shielded activity log the
/// persister mirrors out of `ShieldedChangeSet::activity_entries`.
/// Scoped by the active network via the denormalized `walletId`
/// column — same trick `ShieldedNoteStorageListView` uses.
struct ShieldedActivityStorageListView: View {
    let network: Network

    /// Sort by block height (newest first), then account index so rows
    /// from the same block stay deterministic. `entryId` is `Data`
    /// (not Comparable), so it can't be a sort key.
    @Query(
        sort: [
            SortDescriptor(\PersistentShieldedActivity.blockHeight, order: .reverse),
            SortDescriptor(\PersistentShieldedActivity.accountIndex),
        ]
    )
    private var records: [PersistentShieldedActivity]

    @Query private var allWallets: [PersistentWallet]

    private var walletIdsOnNetwork: Set<Data> {
        Set(allWallets.lazy
            .filter { $0.networkRaw == network.rawValue }
            .map(\.walletId))
    }

    private var scopedRecords: [PersistentShieldedActivity] {
        let ids = walletIdsOnNetwork
        return records.filter { ids.contains($0.walletId) }
    }

    var body: some View {
        let visible = scopedRecords
        List {
            ForEach(visible) { record in
                NavigationLink(
                    destination: ShieldedActivityStorageDetailView(record: record)
                ) {
                    VStack(alignment: .leading, spacing: 4) {
                        HStack(spacing: 8) {
                            Text("kind \(record.kindTag)")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            Text("status \(record.status)")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            if record.hasBlockHeight {
                                Text("h \(record.blockHeight)")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                            }
                            Spacer()
                        }
                        Text("\(record.amount) credits")
                            .font(.caption)
                        Text(record.entryId.prefix(8).map { String(format: "%02x", $0) }.joined())
                            .font(.system(.caption2, design: .monospaced))
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
        .navigationTitle("Shielded Activity (\(visible.count))")
        .overlay {
            if visible.isEmpty {
                ContentUnavailableView("No Activity", systemImage: "clock.arrow.circlepath")
            }
        }
    }
}

// MARK: - PersistentShieldedSyncState

struct ShieldedSyncStateStorageListView: View {
    let network: Network

    // SwiftData's `SortDescriptor` doesn't accept `Data` fields
    // (Data isn't Comparable), so sort only by `accountIndex`
    // and let the wallet-id grouping fall out of insertion
    // order — there are at most a handful of rows per device.
    @Query(sort: [SortDescriptor(\PersistentShieldedSyncState.accountIndex)])
    private var records: [PersistentShieldedSyncState]

    @Query private var allWallets: [PersistentWallet]

    private var walletIdsOnNetwork: Set<Data> {
        Set(allWallets.lazy
            .filter { $0.networkRaw == network.rawValue }
            .map(\.walletId))
    }

    private var scopedRecords: [PersistentShieldedSyncState] {
        let ids = walletIdsOnNetwork
        return records.filter { ids.contains($0.walletId) }
    }

    var body: some View {
        let visible = scopedRecords
        List {
            ForEach(visible) { record in
                NavigationLink(destination: ShieldedSyncStateStorageDetailView(record: record)) {
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text(
                                record.walletId.prefix(4)
                                    .map { String(format: "%02x", $0) }.joined()
                            )
                            .font(.system(.caption2, design: .monospaced))
                            Text("acct \(record.accountIndex)")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            Spacer()
                        }
                        Text("synced index: \(record.lastSyncedIndex)")
                            .font(.caption)
                    }
                }
            }
        }
        .navigationTitle("Shielded Sync State (\(visible.count))")
        .overlay {
            if visible.isEmpty {
                ContentUnavailableView("No Sync States", systemImage: "arrow.triangle.2.circlepath")
            }
        }
    }
}
