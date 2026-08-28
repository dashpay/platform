import SwiftUI
import SwiftData
import SwiftDashSDK
import UIKit

// MARK: - Shared Helpers

private struct FieldRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack {
            Text(label).foregroundColor(.secondary)
            Spacer()
            Text(value).lineLimit(1).truncationMode(.middle).textSelection(.enabled)
        }
    }
}

private func hexString(_ data: Data) -> String {
    data.map { String(format: "%02x", $0) }.joined()
}

/// Human label for a stored public key, keyed on its byte length — the
/// curve is fixed by the width (ECDSA 33 / BLS 48 / Ed25519 32),
/// matching the Rust-side `KeyTypeTagFFI` discriminant.
private func publicKeyTypeLabel(byteCount: Int) -> String {
    switch byteCount {
    case 33: return "ECDSA Public Key"
    case 48: return "BLS Public Key"
    case 32: return "Ed25519 Public Key"
    default: return "Public Key"
    }
}

/// Render an owning `PersistentWallet` for one-line display on
/// the storage-record detail screens. Priority: explicit wallet
/// name → `"<short hex>…"` of the `walletId` → "None" for
/// detached rows. Kept file-private so the identity and
/// (future) other relationship-carrying storage views share the
/// same presentation.
private func walletLabel(_ wallet: PersistentWallet?) -> String {
    guard let wallet else { return "None" }
    if let name = wallet.name, !name.isEmpty {
        return name
    }
    let hex = wallet.walletId.prefix(4)
        .map { String(format: "%02x", $0) }
        .joined()
    return hex.isEmpty ? "None" : "\(hex)…"
}

private func dateString(_ date: Date?) -> String {
    AppDate.formatted(optional: date)
}

private func jsonString(_ data: Data?) -> String? {
    guard let data = data,
          let json = try? JSONSerialization.jsonObject(with: data),
          let pretty = try? JSONSerialization.data(withJSONObject: json, options: [.prettyPrinted, .sortedKeys]),
          let str = String(data: pretty, encoding: .utf8) else { return nil }
    return str
}

// MARK: - PersistentIdentity

struct IdentityStorageDetailView: View {
    let record: PersistentIdentity

    var body: some View {
        Form {
            Section {
                // Surface the operational identity view from the
                // storage explorer — the StorageExplorer page is
                // metadata-only; the live view (balance, top-up,
                // documents, etc.) lives in IdentityDetailView.
                NavigationLink {
                    IdentityDetailView(identityId: record.identityId)
                } label: {
                    HStack {
                        Text("View Identity")
                        Spacer()
                        Image(systemName: "arrow.right")
                            .foregroundColor(.secondary)
                    }
                }
            }
            Section("Core") {
                FieldRow(label: "ID (Base58)", value: record.identityIdBase58)
                FieldRow(label: "ID (Hex)", value: record.identityIdString)
                FieldRow(label: "Balance", value: record.formattedBalance)
                FieldRow(label: "Revision", value: "\(record.revision)")
                FieldRow(label: "Is Local", value: record.isLocal ? "Yes" : "No")
                FieldRow(label: "Network", value: record.network.displayName)
                // `identityIndex` is the DIP-9 index the owning
                // wallet registered this identity at. Only
                // meaningful when `wallet != nil`; shown as "—"
                // otherwise so the row is consistent for orphaned
                // identities that predate the wallet-relationship
                // wiring.
                FieldRow(
                    label: "Identity Index",
                    value: record.wallet != nil ? "\(record.identityIndex)" : "—"
                )
            }
            Section("Names") {
                FieldRow(label: "Alias", value: record.alias ?? "None")
                FieldRow(label: "DPNS Name", value: record.dpnsName ?? "None")
                FieldRow(label: "Main DPNS Name", value: record.mainDpnsName ?? "None")
            }
            Section("Keys") {
                FieldRow(label: "Owner Key", value: record.ownerPrivateKeyIdentifier != nil ? "Present" : "Not set")
                FieldRow(label: "Voting Key", value: record.votingPrivateKeyIdentifier != nil ? "Present" : "Not set")
                FieldRow(label: "Payout Key", value: record.payoutPrivateKeyIdentifier != nil ? "Present" : "Not set")
            }
            Section("Relationships") {
                // Owning wallet via the `@Relationship`. Prefer
                // the user-visible name; fall back to a short hex
                // fingerprint of the wallet id; fall back to
                // "None" for orphaned identities.
                FieldRow(
                    label: "Wallet",
                    value: walletLabel(record.wallet)
                )
                FieldRow(label: "Public Keys", value: "\(record.publicKeys.count)")
                FieldRow(label: "Documents", value: "\(record.documents.count)")
                FieldRow(label: "Token Balances", value: "\(record.tokenBalances.count)")
                FieldRow(label: "Owned Data Contracts", value: "\(record.ownedDataContracts.count)")
                // DashPay / DPNS relationships added with the
                // contact-request and profile changesets. Surface
                // counts here; the dedicated storage-explorer
                // sections own the per-row drill-down.
                FieldRow(label: "DPNS Names", value: "\(record.dpnsNames.count)")
                FieldRow(
                    label: "DashPay Profile",
                    value: record.dashpayProfile != nil ? "Present" : "None"
                )
                FieldRow(
                    label: "Contact Requests",
                    value: "\(record.contactRequests.count)"
                )
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
                FieldRow(label: "Synced", value: dateString(record.lastSyncedAt))
            }
        }
        .navigationTitle("Identity")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentDPNSName

/// Detail view for one cached DPNS-label row. Surfaces every stored
/// field plus a navigation link back to the owning identity so the
/// explorer can hop between the parent identity and its individual
/// labels.
struct DPNSNameStorageDetailView: View {
    let record: PersistentDPNSName

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Label", value: record.label)
                FieldRow(label: "Normalized Label", value: record.normalizedLabel)
                FieldRow(label: "Parent Domain", value: record.parentDomainName)
                FieldRow(
                    label: "Normalized Parent Domain",
                    value: record.normalizedParentDomainName
                )
                FieldRow(label: "Network", value: record.network.displayName)
            }
            Section("Status") {
                FieldRow(label: "Currently Owned", value: record.isOwned ? "Yes" : "No")
                // `acquiredAt` is Unix-millis from
                // `DpnsNameInfo.acquired_at`. Zero when the FFI
                // changeset didn't carry a timestamp (legacy rows
                // before the field was wired through).
                FieldRow(
                    label: "Acquired At (ms)",
                    value: record.acquiredAt == 0 ? "—" : "\(record.acquiredAt)"
                )
                if record.acquiredAt > 0 {
                    let date = Date(
                        timeIntervalSince1970: TimeInterval(record.acquiredAt) / 1000.0
                    )
                    FieldRow(label: "Acquired", value: dateString(date))
                }
            }
            Section("Marketplace") {
                FieldRow(label: "Document ID", value: record.documentIdBase58 ?? "—")
                FieldRow(
                    label: "Sale Status",
                    value: record.saleStatus.map(DpnsMarketplaceUI.status) ?? "Not tracked"
                )
                FieldRow(
                    label: "Price",
                    value: record.listedPriceCredits.map(DpnsMarketplaceUI.price) ?? "Not listed"
                )
                FieldRow(label: "Counterparty", value: record.counterpartyIdBase58 ?? "—")
                FieldRow(
                    label: "Document Created (ms)",
                    value: record.documentCreatedAtMs.map { String($0) } ?? "—"
                )
                FieldRow(
                    label: "Document Updated (ms)",
                    value: record.documentUpdatedAtMs.map { String($0) } ?? "—"
                )
                FieldRow(
                    label: "Document Transferred (ms)",
                    value: record.documentTransferredAtMs.map { String($0) } ?? "—"
                )
                FieldRow(
                    label: "Marketplace Synced (ms)",
                    value: record.marketplaceUpdatedAt == 0
                        ? "—" : String(record.marketplaceUpdatedAt)
                )
            }
            Section("Relationships") {
                NavigationLink(destination: IdentityStorageDetailView(record: record.identity)) {
                    FieldRow(
                        label: "Owner Identity",
                        value: record.identity.identityIdBase58
                    )
                }
                FieldRow(label: "Owner ID (Hex)", value: record.identity.identityIdString)
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("DPNS Name")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentDashpayProfile

/// Detail view for one cached DashPay profile row. Mirrors every
/// stored profile field; optional ones render as "—" when nil so
/// the field stays visible (rather than disappearing) and partial
/// profiles are obvious in the explorer.
struct DashpayProfileStorageDetailView: View {
    let record: PersistentDashpayProfile

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Display Name", value: record.displayName ?? "—")
                FieldRow(label: "Public Message", value: record.publicMessage ?? "—")
                // `bio` is reserved on the row for forwards-compat
                // with future DashPay contract revisions; v3 doesn't
                // populate it. Surface anyway so the column isn't
                // invisible if a later contract lights it up.
                FieldRow(label: "Bio", value: record.bio ?? "—")
                FieldRow(label: "Network", value: record.network.displayName)
            }
            Section("Avatar") {
                FieldRow(label: "URL", value: record.avatarUrl ?? "—")
                FieldRow(
                    label: "Hash (32 B)",
                    value: record.avatarHash.map { hexString($0) } ?? "—"
                )
                FieldRow(
                    label: "Fingerprint (8 B)",
                    value: record.avatarFingerprint.map { hexString($0) } ?? "—"
                )
            }
            Section("Relationships") {
                NavigationLink(destination: IdentityStorageDetailView(record: record.identity)) {
                    FieldRow(
                        label: "Owner Identity",
                        value: record.identity.identityIdBase58
                    )
                }
                FieldRow(label: "Owner ID (Hex)", value: record.identity.identityIdString)
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("DashPay Profile")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentDashpayContactProfile

/// Detail view for one cached contact profile — a counterparty's DashPay
/// profile as seen by an owner identity. One row per (owner, contact).
/// Optional fields render as "—" when nil so partial profiles stay visible.
struct DashpayContactProfileStorageDetailView: View {
    let record: PersistentDashpayContactProfile

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Display Name", value: record.displayName ?? "—")
                FieldRow(label: "Public Message", value: record.publicMessage ?? "—")
                FieldRow(label: "Bio", value: record.bio ?? "—")
                FieldRow(label: "Network", value: record.network.displayName)
            }
            Section("Avatar") {
                FieldRow(label: "URL", value: record.avatarUrl ?? "—")
                FieldRow(
                    label: "Hash (32 B)",
                    value: record.avatarHash.map { hexString($0) } ?? "—"
                )
                FieldRow(
                    label: "Fingerprint (8 B)",
                    value: record.avatarFingerprint.map { hexString($0) } ?? "—"
                )
            }
            Section("Relationships") {
                NavigationLink(destination: IdentityStorageDetailView(record: record.owner)) {
                    FieldRow(
                        label: "Owner Identity",
                        value: record.owner.identityIdBase58
                    )
                }
                FieldRow(label: "Owner ID (Hex)", value: hexString(record.ownerIdentityId))
                FieldRow(label: "Contact ID (Hex)", value: hexString(record.contactIdentityId))
            }
            Section("Timestamps") {
                FieldRow(label: "Checked At (ms)", value: String(record.checkedAtMs))
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Contact Profile")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentDashpayPayment

/// Detail view for one DashPay payment-history row. Read-only dump
/// of every column the persister bridge writes, mirroring the other
/// storage detail views.
struct DashpayPaymentStorageDetailView: View {
    let record: PersistentDashpayPayment

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(
                    label: "Direction",
                    value: record.direction == .sent ? "Sent" : "Received"
                )
                FieldRow(label: "Status", value: statusText)
                FieldRow(
                    label: "Amount",
                    value: String(format: "%.8f DASH", Double(record.amountDuffs) / 100_000_000)
                )
                FieldRow(label: "Amount (duffs)", value: "\(record.amountDuffs)")
                FieldRow(label: "Network", value: record.network.displayName)
                FieldRow(label: "Memo", value: record.memo ?? "—")
            }
            Section("Transaction") {
                FieldRow(label: "Txid", value: record.txid)
            }
            Section("Identities") {
                FieldRow(label: "Owner", value: record.ownerIdentityId.map { String(format: "%02x", $0) }.joined())
                FieldRow(
                    label: "Counterparty",
                    value: record.counterpartyIdentityId.map { String(format: "%02x", $0) }.joined()
                )
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: AppDate.formatted(record.createdAt, dateStyle: .abbreviated, timeStyle: .standard))
                FieldRow(label: "Updated", value: AppDate.formatted(record.lastUpdated, dateStyle: .abbreviated, timeStyle: .standard))
            }
        }
        .navigationTitle("DashPay Payment")
        .navigationBarTitleDisplayMode(.inline)
    }

    private var statusText: String {
        switch record.status {
        case .pending: return "Pending"
        case .confirmed: return "Confirmed"
        case .failed: return "Failed"
        }
    }
}

// MARK: - PersistentInvitation

/// Human label for a `PersistentInvitation.statusRaw` discriminant
/// (0 = Created, 1 = Claimed, 2 = Reclaimed). Shared with the list view;
/// an unmapped value renders as "Unknown (n)" rather than being hidden.
func invitationStatusLabel(_ raw: Int) -> String {
    switch raw {
    case 0: return "Created"
    case 1: return "Claimed"
    case 2: return "Reclaimed"
    default: return "Unknown (\(raw))"
    }
}

/// Detail view for one created DashPay invitation (DIP-13). Read-only dump
/// of every column the persister bridge writes, mirroring the other storage
/// detail views. Note there is no secret column — the one-time voucher key
/// is never stored.
struct InvitationStorageDetailView: View {
    let record: PersistentInvitation

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Status", value: invitationStatusLabel(record.statusRaw))
                FieldRow(
                    label: "Amount",
                    value: String(format: "%.8f DASH", Double(record.amountDuffs) / 100_000_000)
                )
                FieldRow(label: "Amount (duffs)", value: "\(record.amountDuffs)")
                FieldRow(label: "Funding index", value: "\(record.fundingIndexRaw)")
                FieldRow(label: "Has inviter", value: record.hasInviter ? "Yes" : "No")
            }
            Section("Outpoint") {
                FieldRow(label: "Outpoint", value: record.outPointHex)
                FieldRow(
                    label: "Raw outpoint",
                    value: record.rawOutPoint.map { String(format: "%02x", $0) }.joined()
                )
            }
            Section("Wallet") {
                FieldRow(
                    label: "Wallet id",
                    value: record.walletId.map { String(format: "%02x", $0) }.joined()
                )
            }
            Section("Timestamps") {
                FieldRow(label: "Expiry (unix)", value: "\(record.expiryUnix)")
                FieldRow(label: "Created (unix)", value: "\(record.createdAtSecs)")
                FieldRow(label: "Created", value: AppDate.formatted(record.createdAt, dateStyle: .abbreviated, timeStyle: .standard))
                FieldRow(label: "Updated", value: AppDate.formatted(record.updatedAt, dateStyle: .abbreviated, timeStyle: .standard))
            }
        }
        .navigationTitle("Invitation")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentDashpayIgnoredSender

/// Detail view for one DashPay ignored sender (per-sender mute,
/// local-only). Read-only dump of every column, mirroring the other
/// storage detail views.
struct DashpayIgnoredSenderStorageDetailView: View {
    let record: PersistentDashpayIgnoredSender

    var body: some View {
        Form {
            Section("Suppression key") {
                FieldRow(label: "Owner", value: record.ownerIdentityId.toHexString())
                FieldRow(label: "Ignored sender", value: record.ignoredSenderId.toHexString())
                FieldRow(label: "Network", value: record.network.displayName)
            }
            Section("Audit") {
                FieldRow(label: "Ignored", value: AppDate.formatted(record.ignoredAt, dateStyle: .abbreviated, timeStyle: .standard))
            }
        }
        .navigationTitle("Ignored Sender")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentDashpayContactRequest

/// Detail view for one DashPay contact-request row. Surfaces every
/// payload field plus the relationship pair (owner / contact). The
/// `ownerIdentityId` denorm shadow is presented in the relationships
/// section with a note — it's redundant with `owner.identityId` but
/// query-friendly.
struct DashpayContactRequestStorageDetailView: View {
    let record: PersistentDashpayContactRequest

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(
                    label: "Direction",
                    value: record.isOutgoing ? "Outgoing" : "Incoming"
                )
                FieldRow(label: "Network", value: record.network.displayName)
                FieldRow(label: "Sender Key Index", value: "\(record.senderKeyIndex)")
                FieldRow(label: "Recipient Key Index", value: "\(record.recipientKeyIndex)")
                FieldRow(label: "Account Reference", value: "\(record.accountReference)")
                FieldRow(
                    label: "Core Height Created At",
                    value: "\(record.coreHeightCreatedAt)"
                )
                FieldRow(
                    label: "Created At (ms)",
                    value: record.createdAtMillis == 0
                        ? "—"
                        : "\(record.createdAtMillis)"
                )
            }
            Section("Payload") {
                FieldRow(
                    label: "Encrypted Public Key",
                    value: "\(record.encryptedPublicKey.count) bytes"
                )
                FieldRow(
                    label: "Encrypted Account Label",
                    value: record.encryptedAccountLabel.map { "\($0.count) bytes" } ?? "—"
                )
                FieldRow(
                    label: "Account Label (decrypted)",
                    value: record.contactAccountLabel ?? "—"
                )
                FieldRow(
                    label: "Auto-Accept Proof",
                    value: record.autoAcceptProof.map { "\($0.count) bytes" } ?? "—"
                )
            }
            Section("Relationships") {
                NavigationLink(destination: IdentityStorageDetailView(record: record.owner)) {
                    FieldRow(
                        label: "Owner Identity",
                        value: record.owner.identityIdBase58
                    )
                }
                FieldRow(
                    label: "Owner ID (Hex, denorm)",
                    value: hexString(record.ownerIdentityId)
                )
                FieldRow(
                    label: "Contact ID (Hex)",
                    value: hexString(record.contactIdentityId)
                )
            }
            Section("Timestamps") {
                if record.createdAtMillis > 0 {
                    let date = Date(
                        timeIntervalSince1970: TimeInterval(record.createdAtMillis) / 1000.0
                    )
                    FieldRow(label: "Document Created", value: dateString(date))
                }
                FieldRow(label: "Row Created", value: dateString(record.createdAt))
                FieldRow(label: "Row Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle(record.isOutgoing ? "Outgoing Contact Request" : "Incoming Contact Request")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentDocument

struct DocumentStorageDetailView: View {
    let record: PersistentDocument

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Document ID", value: record.documentId)
                FieldRow(label: "Type", value: record.documentType)
                FieldRow(label: "Display Title", value: record.displayTitle)
                FieldRow(label: "Revision", value: "\(record.revision)")
                FieldRow(label: "Contract ID", value: record.contractId)
                FieldRow(label: "Owner ID", value: record.ownerId)
                FieldRow(label: "Network", value: record.network.displayName)
                FieldRow(label: "Deleted", value: record.isDeleted ? "Yes" : "No")
            }
            Section("Block Heights") {
                FieldRow(
                    label: "Created (Platform)",
                    value: record.createdAtBlockHeight.map { "\($0)" } ?? "—"
                )
                FieldRow(
                    label: "Updated (Platform)",
                    value: record.updatedAtBlockHeight.map { "\($0)" } ?? "—"
                )
                FieldRow(
                    label: "Transferred (Platform)",
                    value: record.transferredAtBlockHeight.map { "\($0)" } ?? "—"
                )
                FieldRow(
                    label: "Created (Core)",
                    value: record.createdAtCoreBlockHeight.map { "\($0)" } ?? "—"
                )
                FieldRow(
                    label: "Updated (Core)",
                    value: record.updatedAtCoreBlockHeight.map { "\($0)" } ?? "—"
                )
                FieldRow(
                    label: "Transferred (Core)",
                    value: record.transferredAtCoreBlockHeight.map { "\($0)" } ?? "—"
                )
            }
            Section("Relationships") {
                if let docType = record.documentType_relation {
                    NavigationLink(destination: DocumentTypeStorageDetailView(record: docType)) {
                        FieldRow(label: "Document Type", value: docType.name)
                    }
                } else {
                    FieldRow(label: "Document Type", value: "Not linked")
                }
                if let contract = record.dataContract {
                    NavigationLink(destination: DataContractStorageDetailView(record: contract)) {
                        FieldRow(label: "Data Contract", value: contract.name)
                    }
                } else {
                    FieldRow(label: "Data Contract", value: "Not linked")
                }
                // `ownerIdentity` is only populated for documents whose
                // owner happens to also be a local identity. Most
                // Platform-fetched documents will have nil here.
                if let owner = record.ownerIdentity {
                    NavigationLink(destination: IdentityStorageDetailView(record: owner)) {
                        FieldRow(label: "Owner Identity", value: owner.identityIdBase58)
                    }
                } else {
                    FieldRow(label: "Owner Identity", value: "Not local")
                }
            }
            Section("Timestamps") {
                // `createdAt` / `updatedAt` are the platform-side
                // document timestamps; `localCreatedAt` / `localUpdatedAt`
                // are when this row entered / changed in the local
                // SwiftData store. They diverge whenever the row was
                // back-filled or refreshed from a remote fetch.
                FieldRow(label: "Created (Platform)", value: dateString(record.createdAt))
                FieldRow(label: "Updated (Platform)", value: dateString(record.updatedAt))
                FieldRow(label: "Transferred (Platform)", value: dateString(record.transferredAt))
                FieldRow(label: "Local Created", value: dateString(record.localCreatedAt))
                FieldRow(label: "Local Updated", value: dateString(record.localUpdatedAt))
            }
            Section("Payload") {
                FieldRow(label: "Data Size", value: "\(record.data.count) bytes")
                if let json = jsonString(record.data) {
                    Text(json)
                        .font(.system(.caption, design: .monospaced))
                        .textSelection(.enabled)
                }
            }
        }
        .navigationTitle("Document")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentDataContract

struct DataContractStorageDetailView: View {
    let record: PersistentDataContract

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "ID (Base58)", value: record.idBase58)
                FieldRow(label: "Name", value: record.name)
                FieldRow(label: "Version", value: record.version.map { "\($0)" } ?? "None")
                FieldRow(label: "Owner (Base58)", value: record.ownerIdBase58 ?? "None")
                FieldRow(label: "Network", value: record.network.displayName)
                FieldRow(label: "Has Tokens", value: record.hasTokens ? "Yes" : "No")
                FieldRow(label: "Description", value: record.contractDescription ?? "—")
                FieldRow(
                    label: "Schema Defs",
                    value: record.schemaDefs.map { "\($0)" } ?? "—"
                )
            }
            Section("Flags") {
                FieldRow(label: "Can Be Deleted", value: record.canBeDeleted ? "Yes" : "No")
                FieldRow(label: "Read Only", value: record.readonly ? "Yes" : "No")
                FieldRow(label: "Keeps History", value: record.keepsHistory ? "Yes" : "No")
            }
            Section("Document Defaults") {
                // Contract-level fallbacks applied to document types
                // that don't override the corresponding flag. Surfaced
                // separately from the contract-wide flags above
                // because they govern docs, not the contract itself.
                FieldRow(
                    label: "Docs Keep History",
                    value: record.documentsKeepHistoryContractDefault ? "Yes" : "No"
                )
                FieldRow(
                    label: "Docs Mutable",
                    value: record.documentsMutableContractDefault ? "Yes" : "No"
                )
                FieldRow(
                    label: "Docs Can Be Deleted",
                    value: record.documentsCanBeDeletedContractDefault ? "Yes" : "No"
                )
            }
            Section("Keywords") {
                FieldRow(label: "Count", value: "\(record.keywordRelations.count)")
                if !record.keywords.isEmpty {
                    FieldRow(label: "Values", value: record.keywords.joined(separator: ", "))
                }
            }
            Section("Relationships") {
                FieldRow(label: "Document Types", value: "\(record.documentTypes?.count ?? 0)")
                FieldRow(label: "Tokens", value: "\(record.tokens?.count ?? 0)")
                FieldRow(label: "Documents", value: "\(record.documents.count)")
                if let owner = record.ownerIdentity {
                    NavigationLink(destination: IdentityStorageDetailView(record: owner)) {
                        FieldRow(label: "Owner Identity", value: owner.identityIdBase58)
                    }
                } else {
                    FieldRow(label: "Owner Identity", value: "Not local")
                }
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
                FieldRow(label: "Accessed", value: dateString(record.lastAccessedAt))
                FieldRow(label: "Synced", value: dateString(record.lastSyncedAt))
            }
            Section("Serialized Blobs") {
                FieldRow(
                    label: "Contract (JSON)",
                    value: "\(record.serializedContract.count) bytes"
                )
                FieldRow(
                    label: "Binary (CBOR)",
                    value: record.binarySerialization.map { "\($0.count) bytes" } ?? "—"
                )
                FieldRow(label: "Schema Data", value: "\(record.schemaData.count) bytes")
                FieldRow(
                    label: "Document Types Data",
                    value: "\(record.documentTypesData.count) bytes"
                )
                FieldRow(
                    label: "Tokens Data",
                    value: record.tokensData.map { "\($0.count) bytes" } ?? "—"
                )
                FieldRow(
                    label: "Groups Data",
                    value: record.groupsData.map { "\($0.count) bytes" } ?? "—"
                )
            }
        }
        .navigationTitle("Data Contract")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentPublicKey

struct PublicKeyStorageDetailView: View {
    let record: PersistentPublicKey

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Key ID", value: "\(record.keyId)")
                // Stored as the raw `String(rawValue)`; project the
                // human-readable name when the value parses to a known
                // enum case so the row shows e.g. "Authentication (0)".
                FieldRow(label: "Purpose", value: purposeDisplay)
                FieldRow(label: "Security Level", value: securityLevelDisplay)
                FieldRow(label: "Key Type", value: keyTypeDisplay)
                FieldRow(label: "Read Only", value: record.readOnly ? "Yes" : "No")
                FieldRow(label: "Disabled At", value: record.disabledAt.map { "\($0)" } ?? "No")
                FieldRow(label: "Identity ID (Base58)", value: record.identityId)
            }
            Section("Data") {
                FieldRow(label: "Public Key", value: hexString(record.publicKeyData))
                if let bounds = record.contractBounds, !bounds.isEmpty {
                    FieldRow(label: "Contract Bounds", value: "\(bounds.count)")
                    ForEach(Array(bounds.enumerated()), id: \.offset) { _, contractId in
                        FieldRow(label: "Contract", value: contractId.toBase58String())
                    }
                } else {
                    FieldRow(label: "Contract Bounds", value: "None")
                }
                // Surface the keychain identifier itself rather than a
                // bare presence/absence flag — it's load-bearing for
                // debugging the privkey<->pubkey wiring (the row links
                // by string identifier, not foreign-key).
                FieldRow(
                    label: "Private Key Keychain ID",
                    value: record.privateKeyKeychainIdentifier ?? "None"
                )
            }
            Section("Relationships") {
                if let identity = record.identity {
                    NavigationLink(destination: IdentityStorageDetailView(record: identity)) {
                        FieldRow(label: "Identity", value: identity.identityIdBase58)
                    }
                } else {
                    FieldRow(label: "Identity", value: "None")
                }
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Accessed", value: dateString(record.lastAccessed))
            }
        }
        .navigationTitle("Public Key")
        .navigationBarTitleDisplayMode(.inline)
    }

    private var purposeDisplay: String {
        if let p = record.purposeEnum { return "\(p.name) (\(record.purpose))" }
        return record.purpose
    }

    private var securityLevelDisplay: String {
        if let s = record.securityLevelEnum { return "\(s.name) (\(record.securityLevel))" }
        return record.securityLevel
    }

    private var keyTypeDisplay: String {
        if let t = record.keyTypeEnum { return "\(t.name) (\(record.keyType))" }
        return record.keyType
    }
}

// MARK: - PersistentToken

/// Compact one-row summary of a `ChangeControlRules` value: shows the
/// authorized + admin role pair, with a trailing tag for any of the
/// three `*Allowed` toggles that flip away from their defaults. Used
/// across every change-rule slot on the token detail view so the
/// section stays scannable.
private struct ChangeControlRulesRow: View {
    let label: String
    let rules: ChangeControlRules?

    var body: some View {
        if let rules = rules {
            FieldRow(label: label, value: format(rules))
        } else {
            FieldRow(label: label, value: "Not set")
        }
    }

    private func format(_ rules: ChangeControlRules) -> String {
        var parts: [String] = []
        parts.append("auth=\(rules.authorizedToMakeChange)")
        parts.append("admin=\(rules.adminActionTakers)")
        if rules.changingAuthorizedActionTakersToNoOneAllowed { parts.append("auth→none") }
        if rules.changingAdminActionTakersToNoOneAllowed { parts.append("admin→none") }
        if rules.selfChangingAdminActionTakersAllowed { parts.append("self-admin") }
        return parts.joined(separator: " · ")
    }
}

struct TokenStorageDetailView: View {
    let record: PersistentToken

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "ID", value: hexString(record.id))
                FieldRow(label: "Contract (Base58)", value: record.contractIdBase58)
                FieldRow(label: "Name", value: record.name)
                FieldRow(label: "Display Name", value: record.displayName)
                FieldRow(label: "Position", value: "\(record.position)")
                FieldRow(label: "Decimals", value: "\(record.decimals)")
                FieldRow(label: "Base Supply", value: record.formattedBaseSupply)
                FieldRow(label: "Max Supply", value: record.maxSupply ?? "Unlimited")
                FieldRow(label: "Description", value: record.tokenDescription ?? "—")
            }
            Section("Status") {
                FieldRow(label: "Paused", value: record.isPaused ? "Yes" : "No")
                FieldRow(
                    label: "Allow Transfer to Frozen",
                    value: record.allowTransferToFrozenBalance ? "Yes" : "No"
                )
            }
            Section("Localization") {
                let locs = record.localizations ?? [:]
                FieldRow(label: "Languages", value: "\(locs.count)")
                ForEach(locs.keys.sorted(), id: \.self) { lang in
                    if let loc = locs[lang] {
                        FieldRow(
                            label: lang,
                            value: "\(loc.singularForm) / \(loc.pluralForm)"
                        )
                    }
                }
            }
            Section("History Rules") {
                FieldRow(label: "Transfers", value: record.keepsTransferHistory ? "Yes" : "No")
                FieldRow(label: "Freezing", value: record.keepsFreezingHistory ? "Yes" : "No")
                FieldRow(label: "Minting", value: record.keepsMintingHistory ? "Yes" : "No")
                FieldRow(label: "Burning", value: record.keepsBurningHistory ? "Yes" : "No")
                FieldRow(
                    label: "Direct Pricing",
                    value: record.keepsDirectPricingHistory ? "Yes" : "No"
                )
                FieldRow(
                    label: "Direct Purchase",
                    value: record.keepsDirectPurchaseHistory ? "Yes" : "No"
                )
            }
            Section("Change Control Rules") {
                ChangeControlRulesRow(label: "Conventions", rules: record.conventionsChangeRules)
                ChangeControlRulesRow(label: "Max Supply", rules: record.maxSupplyChangeRules)
                ChangeControlRulesRow(label: "Manual Mint", rules: record.manualMintingRules)
                ChangeControlRulesRow(label: "Manual Burn", rules: record.manualBurningRules)
                ChangeControlRulesRow(label: "Freeze", rules: record.freezeRules)
                ChangeControlRulesRow(label: "Unfreeze", rules: record.unfreezeRules)
                ChangeControlRulesRow(
                    label: "Destroy Frozen",
                    rules: record.destroyFrozenFundsRules
                )
                ChangeControlRulesRow(label: "Emergency", rules: record.emergencyActionRules)
                ChangeControlRulesRow(label: "Trade Mode", rules: record.tradeModeChangeRules)
            }
            Section("Distribution") {
                // `perpetualDistribution` and `preProgrammedDistribution`
                // are typed Codable structs — surface the headline
                // fields per slot so a misconfigured token shows up
                // here rather than vanishing behind a presence flag.
                if let perp = record.perpetualDistribution {
                    FieldRow(label: "Perpetual", value: "Configured")
                    FieldRow(label: "  Recipient", value: perp.distributionRecipient)
                    FieldRow(label: "  Enabled", value: perp.enabled ? "Yes" : "No")
                    FieldRow(label: "  Last", value: dateString(perp.lastDistributionTime))
                    FieldRow(label: "  Next", value: dateString(perp.nextDistributionTime))
                } else {
                    FieldRow(label: "Perpetual", value: "Not configured")
                }
                if let prog = record.preProgrammedDistribution {
                    FieldRow(label: "Pre-Programmed", value: "Configured")
                    FieldRow(label: "  Schedule Events", value: "\(prog.distributionSchedule.count)")
                    FieldRow(label: "  Current Index", value: "\(prog.currentEventIndex)")
                    FieldRow(label: "  Total Distributed", value: prog.totalDistributed)
                    FieldRow(label: "  Remaining", value: prog.remainingToDistribute)
                    FieldRow(label: "  Active", value: prog.isActive ? "Yes" : "No")
                    FieldRow(label: "  Paused", value: prog.isPaused ? "Yes" : "No")
                    FieldRow(label: "  Completed", value: prog.isCompleted ? "Yes" : "No")
                } else {
                    FieldRow(label: "Pre-Programmed", value: "Not configured")
                }
                FieldRow(
                    label: "Destination Identity",
                    value: record.newTokensDestinationIdentityBase58 ?? "Not set"
                )
                FieldRow(
                    label: "Choose Mint Destination",
                    value: record.mintingAllowChoosingDestination ? "Yes" : "No"
                )
            }
            if let dcr = record.distributionChangeRules {
                Section("Distribution Change Rules") {
                    ChangeControlRulesRow(
                        label: "Perpetual",
                        rules: dcr.perpetualDistributionRules
                    )
                    ChangeControlRulesRow(
                        label: "Destination",
                        rules: dcr.newTokensDestinationIdentityRules
                    )
                    ChangeControlRulesRow(
                        label: "Choose Destination",
                        rules: dcr.mintingAllowChoosingDestinationRules
                    )
                    ChangeControlRulesRow(
                        label: "Direct Purchase Pricing",
                        rules: dcr.changeDirectPurchasePricingRules
                    )
                }
            }
            Section("Marketplace") {
                FieldRow(label: "Trade Mode", value: record.tradeMode.displayName)
                FieldRow(label: "Tradeable", value: record.isTradeable ? "Yes" : "No")
            }
            Section("Main Control Group") {
                FieldRow(
                    label: "Position",
                    value: record.mainControlGroupPosition.map { "\($0)" } ?? "—"
                )
                FieldRow(
                    label: "Can Be Modified",
                    value: record.mainControlGroupCanBeModified ?? "—"
                )
            }
            Section("Relationships") {
                if let contract = record.dataContract {
                    NavigationLink(destination: DataContractStorageDetailView(record: contract)) {
                        FieldRow(label: "Data Contract", value: contract.name)
                    }
                } else {
                    FieldRow(label: "Data Contract", value: "None")
                }
                FieldRow(label: "Balances", value: "\(record.balances?.count ?? 0)")
                FieldRow(label: "History Events", value: "\(record.historyEvents?.count ?? 0)")
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdatedAt))
            }
        }
        .navigationTitle("Token")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentTokenBalance

struct TokenBalanceStorageDetailView: View {
    let record: PersistentTokenBalance

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Token ID", value: record.tokenId)
                FieldRow(label: "Identity ID", value: hexString(record.identityId))
                FieldRow(label: "Balance", value: "\(record.balance)")
                FieldRow(label: "Display Balance", value: record.displayBalance)
                FieldRow(label: "Frozen", value: record.frozen ? "Yes" : "No")
                FieldRow(label: "Network", value: record.network.displayName)
            }
            Section("Token Info") {
                FieldRow(label: "Name", value: record.tokenName ?? "None")
                FieldRow(label: "Symbol", value: record.tokenSymbol ?? "None")
                FieldRow(label: "Decimals", value: record.tokenDecimals.map { "\($0)" } ?? "None")
            }
            Section("Relationships") {
                if let identity = record.identity {
                    NavigationLink(destination: IdentityStorageDetailView(record: identity)) {
                        FieldRow(label: "Identity", value: identity.identityIdBase58)
                    }
                } else {
                    FieldRow(label: "Identity", value: "None")
                }
                if let token = record.token {
                    NavigationLink(destination: TokenStorageDetailView(record: token)) {
                        FieldRow(label: "Token", value: token.name)
                    }
                } else {
                    FieldRow(label: "Token", value: "None")
                }
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
                FieldRow(label: "Synced", value: dateString(record.lastSyncedAt))
            }
        }
        .navigationTitle("Token Balance")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentTokenHistoryEvent

struct TokenHistoryStorageDetailView: View {
    let record: PersistentTokenHistoryEvent

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Event ID", value: record.id.uuidString)
                FieldRow(label: "Event Type", value: record.eventType)
                FieldRow(label: "Display", value: record.displayTitle)
                FieldRow(
                    label: "Transaction ID",
                    value: record.transactionId.map { hexString($0) } ?? "None"
                )
                FieldRow(
                    label: "Block Height",
                    value: record.blockHeight.map { "\($0)" } ?? "None"
                )
                FieldRow(
                    label: "Core Block Height",
                    value: record.coreBlockHeight.map { "\($0)" } ?? "None"
                )
                FieldRow(label: "Amount", value: record.amount ?? "None")
                FieldRow(label: "Description", value: record.eventDescription ?? "—")
            }
            Section("Parties") {
                FieldRow(
                    label: "From",
                    value: record.fromIdentity.map { hexString($0) } ?? "None"
                )
                FieldRow(
                    label: "To",
                    value: record.toIdentity.map { hexString($0) } ?? "None"
                )
                FieldRow(label: "Performed By", value: hexString(record.performedByIdentity))
            }
            Section("Balance") {
                FieldRow(label: "Before", value: record.balanceBefore ?? "None")
                FieldRow(label: "After", value: record.balanceAfter ?? "None")
            }
            // Optional event-type-specific payload (e.g. distribution
            // recipient breakdown, emergency-action params). Render as
            // pretty JSON when decodable; size only when not.
            if let blob = record.additionalDataJSON {
                Section("Additional Data") {
                    if let json = jsonString(blob) {
                        Text(json)
                            .font(.system(.caption, design: .monospaced))
                            .textSelection(.enabled)
                    } else {
                        FieldRow(label: "Raw", value: "\(blob.count) bytes")
                    }
                }
            }
            Section("Relationships") {
                if let token = record.token {
                    NavigationLink(destination: TokenStorageDetailView(record: token)) {
                        FieldRow(label: "Token", value: token.name)
                    }
                } else {
                    FieldRow(label: "Token", value: "None")
                }
            }
            Section("Timestamps") {
                FieldRow(label: "Event", value: dateString(record.eventTimestamp))
                FieldRow(label: "Created", value: dateString(record.createdAt))
            }
        }
        .navigationTitle("Token History Event")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentDocumentType

struct DocumentTypeStorageDetailView: View {
    let record: PersistentDocumentType

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Name", value: record.name)
                FieldRow(label: "Contract (Base58)", value: record.contractIdBase58)
                FieldRow(label: "Security Level", value: "\(record.securityLevel)")
                FieldRow(label: "Trade Mode", value: "\(record.tradeMode)")
                FieldRow(
                    label: "Creation Restriction Mode",
                    value: "\(record.creationRestrictionMode)"
                )
            }
            Section("Flags") {
                FieldRow(label: "Keeps History", value: record.documentsKeepHistory ? "Yes" : "No")
                FieldRow(label: "Mutable", value: record.documentsMutable ? "Yes" : "No")
                FieldRow(label: "Can Be Deleted", value: record.documentsCanBeDeleted ? "Yes" : "No")
                FieldRow(label: "Transferable", value: record.documentsTransferable ? "Yes" : "No")
                FieldRow(
                    label: "Requires Encryption Key",
                    value: record.requiresIdentityEncryptionBoundedKey ? "Yes" : "No"
                )
                FieldRow(
                    label: "Requires Decryption Key",
                    value: record.requiresIdentityDecryptionBoundedKey ? "Yes" : "No"
                )
            }
            Section("Schema") {
                // The schema and properties JSON blobs are stored as
                // raw bytes; surface sizes and required fields.
                FieldRow(label: "Schema Size", value: "\(record.schemaJSON.count) bytes")
                FieldRow(
                    label: "Properties Size",
                    value: "\(record.propertiesJSON.count) bytes"
                )
                if let req = record.requiredFieldsJSON {
                    FieldRow(label: "Required Fields Size", value: "\(req.count) bytes")
                }
                if let fields = record.requiredFields, !fields.isEmpty {
                    FieldRow(label: "Required Fields", value: fields.joined(separator: ", "))
                }
            }
            Section("Relationships") {
                if let contract = record.dataContract {
                    NavigationLink(destination: DataContractStorageDetailView(record: contract)) {
                        FieldRow(label: "Data Contract", value: contract.name)
                    }
                } else {
                    FieldRow(label: "Data Contract", value: "None")
                }
                FieldRow(label: "Properties", value: "\(record.propertiesList?.count ?? 0)")
                FieldRow(label: "Indices", value: "\(record.indices?.count ?? 0)")
                FieldRow(label: "Documents", value: "\(record.documentCount)")
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Accessed", value: dateString(record.lastAccessedAt))
            }
        }
        .navigationTitle("Document Type")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentIndex

struct IndexStorageDetailView: View {
    let record: PersistentIndex

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Name", value: record.name)
                FieldRow(label: "Document Type", value: record.documentTypeName)
                FieldRow(label: "Contract ID (Hex)", value: hexString(record.contractId))
                FieldRow(label: "Unique", value: record.unique ? "Yes" : "No")
                FieldRow(label: "Null Searchable", value: record.nullSearchable ? "Yes" : "No")
                FieldRow(label: "Contested", value: record.contested ? "Yes" : "No")
            }
            // Protocol v14 index keywords - rows appear only when set, so
            // pre-v14 indexes render exactly as before
            if record.countable != nil || record.summable != nil || record.terminal != nil
                || record.preallocated || record.timeRangeJSON != nil {
                Section("Axes & Storage Mode") {
                    if let countable = record.countable {
                        FieldRow(label: "Countable", value: countable)
                    }
                    if record.rangeCountable {
                        FieldRow(label: "Range Countable", value: "Yes")
                    }
                    if let summable = record.summable {
                        FieldRow(label: "Summable", value: summable)
                    }
                    if record.rangeSummable {
                        FieldRow(label: "Range Summable", value: "Yes")
                    }
                    if record.rankedCountable {
                        FieldRow(label: "Ranked by Count", value: "Yes")
                    }
                    if record.rankedSummable {
                        FieldRow(label: "Ranked by Sum", value: "Yes")
                    }
                    if record.rankedAverageable {
                        FieldRow(label: "Ranked by Average", value: "Yes")
                    }
                    if let terminal = record.terminal {
                        FieldRow(label: "Terminal", value: terminal)
                    }
                    if record.preallocated {
                        FieldRow(label: "Preallocated", value: "Yes")
                    }
                    if let timeRange = record.timeRange,
                       let range = timeRange["range"] as? Int,
                       let step = timeRange["step"] as? Int {
                        FieldRow(label: "Time Range", value: "\(range)s windows every \(step)s")
                    }
                }
            }
            if let props = record.properties, !props.isEmpty {
                Section("Properties") {
                    ForEach(props, id: \.self) { prop in
                        Text(prop).font(.system(.caption, design: .monospaced))
                    }
                }
            }
            // `contestedDetailsJSON` is only populated when
            // `contested == true`. Render the parsed payload
            // pretty-printed; fall back to the raw size if the JSON
            // bytes don't decode.
            if let blob = record.contestedDetailsJSON {
                Section("Contested Details") {
                    if let json = jsonString(blob) {
                        Text(json)
                            .font(.system(.caption, design: .monospaced))
                            .textSelection(.enabled)
                    } else {
                        FieldRow(label: "Raw", value: "\(blob.count) bytes")
                    }
                }
            }
            Section("Relationships") {
                if let docType = record.documentType {
                    NavigationLink(destination: DocumentTypeStorageDetailView(record: docType)) {
                        FieldRow(label: "Document Type", value: docType.name)
                    }
                } else {
                    FieldRow(label: "Document Type", value: "None")
                }
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
            }
        }
        .navigationTitle("Index")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentProperty

struct PropertyStorageDetailView: View {
    let record: PersistentProperty

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Name", value: record.name)
                FieldRow(label: "Type", value: record.type)
                FieldRow(label: "Document Type", value: record.documentTypeName)
                FieldRow(label: "Contract ID (Hex)", value: hexString(record.contractId))
                FieldRow(label: "Required", value: record.isRequired ? "Yes" : "No")
                FieldRow(label: "Transient", value: record.transient ? "Yes" : "No")
                FieldRow(label: "Byte Array", value: record.byteArray ? "Yes" : "No")
                FieldRow(label: "Description", value: record.fieldDescription ?? "—")
            }
            Section("Constraints") {
                FieldRow(label: "Format", value: record.format ?? "—")
                FieldRow(label: "Content Media Type", value: record.contentMediaType ?? "—")
                FieldRow(label: "Pattern", value: record.pattern ?? "—")
                FieldRow(
                    label: "Min Length",
                    value: record.minLength.map { "\($0)" } ?? "—"
                )
                FieldRow(
                    label: "Max Length",
                    value: record.maxLength.map { "\($0)" } ?? "—"
                )
                FieldRow(
                    label: "Min Items",
                    value: record.minItems.map { "\($0)" } ?? "—"
                )
                FieldRow(
                    label: "Max Items",
                    value: record.maxItems.map { "\($0)" } ?? "—"
                )
                FieldRow(
                    label: "Min Value",
                    value: record.minValue.map { "\($0)" } ?? "—"
                )
                FieldRow(
                    label: "Max Value",
                    value: record.maxValue.map { "\($0)" } ?? "—"
                )
            }
            Section("Relationships") {
                if let docType = record.documentType {
                    NavigationLink(destination: DocumentTypeStorageDetailView(record: docType)) {
                        FieldRow(label: "Document Type", value: docType.name)
                    }
                } else {
                    FieldRow(label: "Document Type", value: "None")
                }
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
            }
        }
        .navigationTitle("Property")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentKeyword

struct KeywordStorageDetailView: View {
    let record: PersistentKeyword

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Keyword", value: record.keyword)
                // `id` is the composite `"<contractId>_<keyword>"` row
                // key. Surfaced for the storage explorer because it's
                // load-bearing (uniqueness pivot) even though it's
                // derived from the other two fields.
                FieldRow(label: "Row ID", value: record.id)
                FieldRow(label: "Contract ID (Base58)", value: record.contractId)
            }
            Section("Relationships") {
                if let contract = record.dataContract {
                    NavigationLink(destination: DataContractStorageDetailView(record: contract)) {
                        FieldRow(label: "Data Contract", value: contract.name)
                    }
                } else {
                    FieldRow(label: "Data Contract", value: "None")
                }
            }
        }
        .navigationTitle("Keyword")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentPlatformAddressesSyncState

struct PlatformAddressesSyncStateStorageDetailView: View {
    let record: PersistentPlatformAddressesSyncState

    private var blockDate: Date? {
        record.syncTimestamp > 0
            ? Date(timeIntervalSince1970: TimeInterval(record.syncTimestamp))
            : nil
    }

    var body: some View {
        Form {
            Section("Scope") {
                // `walletId` is the 32-byte unique scope key for this
                // sync-state row. The current persistence layer writes a
                // network-scoped key (one row per network) rather than a
                // concrete wallet id, but the column name is preserved
                // for schema compatibility — see model header doc.
                FieldRow(label: "Scope Key (Hex)", value: hexString(record.walletId))
            }
            Section("Sync Watermark") {
                FieldRow(label: "Network", value: record.network.displayName)
                FieldRow(label: "Sync Height", value: "\(record.syncHeight)")
                FieldRow(label: "Sync Timestamp", value: "\(record.syncTimestamp)")
                if let date = blockDate {
                    FieldRow(
                        label: "Local Time",
                        value: AppDate.formatted(date, dateStyle: .abbreviated, timeStyle: .standard)
                    )
                    FieldRow(label: "UTC", value: {
                        let f = DateFormatter.posixGregorian()
                        f.dateFormat = "yyyy-MM-dd HH:mm:ss"
                        f.timeZone = TimeZone(identifier: "UTC")
                        return f.string(from: date) + " UTC"
                    }())
                }
                FieldRow(label: "Last Known Recent Block", value: record.lastKnownRecentBlock > 0
                    ? "\(record.lastKnownRecentBlock)"
                    : "0 (no recent address activity)")
            }
            Section("Timestamps") {
                FieldRow(label: "Record Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Platform Addresses Sync State")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentPlatformAddress

struct PlatformAddressDetailView: View {
    let record: PersistentPlatformAddress

    var body: some View {
        Form {
            Section("Address") {
                FieldRow(label: "Address", value: record.address)
                FieldRow(
                    label: "Type",
                    value: record.addressType == 0 ? "P2PKH" : "P2SH"
                )
                FieldRow(label: "Hash", value: hexString(record.addressHash))
                FieldRow(label: "Account Index", value: "\(record.accountIndex)")
                FieldRow(label: "Index", value: "\(record.addressIndex)")
                FieldRow(label: "Derivation Path", value: record.derivationPath)
                FieldRow(label: "Used", value: record.isUsed ? "Yes" : "No")
            }
            Section("Public Key") {
                FieldRow(
                    label: publicKeyTypeLabel(byteCount: record.publicKey.count),
                    value: record.publicKey.isEmpty
                        ? "—"
                        : record.publicKey.map { String(format: "%02x", $0) }.joined()
                )
            }
            Section("Balance / Activity") {
                FieldRow(label: "Balance", value: "\(record.balance) credits")
                FieldRow(label: "Nonce", value: "\(record.nonce)")
                FieldRow(
                    label: "First Seen Height",
                    value: record.firstSeenHeight == 0 ? "—" : "\(record.firstSeenHeight)"
                )
                FieldRow(
                    label: "Last Seen Height",
                    value: record.lastSeenHeight == 0 ? "—" : "\(record.lastSeenHeight)"
                )
            }
            Section("Ownership") {
                FieldRow(label: "Wallet ID", value: hexString(record.walletId))
            }
            Section("Relationships") {
                if let account = record.account {
                    NavigationLink(destination: AccountStorageDetailView(record: account)) {
                        FieldRow(label: "Account", value: account.accountTypeName)
                    }
                } else {
                    FieldRow(label: "Account", value: "Not linked")
                }
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Platform Address")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentWallet

struct WalletStorageDetailView: View {
    let record: PersistentWallet

    /// `lastSynced` is stored as Unix-seconds (`UInt64`). Render the
    /// canonical date when non-zero so it matches the other
    /// timestamp surfaces; "—" when the wallet has never synced.
    private var lastSyncedDate: Date? {
        record.lastSynced > 0
            ? Date(timeIntervalSince1970: TimeInterval(record.lastSynced))
            : nil
    }

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Wallet ID", value: hexString(record.walletId))
                FieldRow(label: "Network", value: record.network?.displayName ?? "Unknown")
                FieldRow(label: "Name", value: record.name ?? "None")
                FieldRow(label: "Birth Height", value: "\(record.birthHeight)")
                FieldRow(label: "Synced Height", value: "\(record.syncedHeight)")
                FieldRow(label: "Imported", value: record.isImported ? "Yes" : "No")
            }
            Section("Relationships") {
                FieldRow(label: "Accounts", value: "\(record.accounts.count)")
                // Inverse of `PersistentIdentity.wallet`. Surfaces
                // how many identities are currently anchored to
                // this wallet so the storage explorer shows the
                // mapping from both sides.
                FieldRow(label: "Identities", value: "\(record.identities.count)")
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
                FieldRow(label: "Last Synced", value: dateString(lastSyncedDate))
            }
        }
        .navigationTitle("Wallet")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentAccount

struct AccountStorageDetailView: View {
    let record: PersistentAccount

    /// Base58check-encoded xpub/tpub for this account, derived from
    /// the stored ExtendedPubKey bytes. `nil` when the bytes are
    /// missing (account not yet hydrated) or decode fails.
    private var accountXpubString: String? {
        guard let bytes = record.accountExtendedPubKeyBytes, !bytes.isEmpty else {
            return nil
        }
        return PlatformWalletManager.accountExtendedPubKeyString(bytes: bytes)
    }

    /// Distinct transactions this account participates in: union of
    /// every TXO's creating tx (`transaction`) and spending tx
    /// (`spendingTransaction`). Mirrors the AccountDetailView helper
    /// — `PersistentTransaction` no longer carries an account link,
    /// so the per-account set has to be derived on read. Walks the
    /// address pool because `PersistentAccount.outputs` is gone;
    /// the canonical account → TXO path is now
    /// `coreAddresses.flatMap(\.txos)`.
    private var distinctTransactionCount: Int {
        var seen: Set<Data> = []
        for address in record.coreAddresses {
            for txo in address.txos {
                if let tx = txo.transaction { seen.insert(tx.txid) }
                if let spending = txo.spendingTransaction { seen.insert(spending.txid) }
            }
        }
        return seen.count
    }

    /// Total TXO count for the account, summed across the address
    /// pool. Cheap because the pool is bounded (~gap limit).
    private var txoCount: Int {
        record.coreAddresses.reduce(0) { $0 + $1.txos.count }
    }

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Type", value: record.accountTypeName)
                FieldRow(label: "Type ID", value: "\(record.accountType)")
                FieldRow(label: "Index", value: "\(record.accountIndex)")
                FieldRow(
                    label: "Extended Public Key",
                    value: accountXpubString ?? "—"
                )
            }
            Section("Variant Disambiguators") {
                // Account-identity disambiguators carried on every
                // row: `standardTag` distinguishes BIP44 (0) from
                // BIP32 (1) for Standard accounts; `registrationIndex`
                // is the IdentityTopUp registration index;
                // `keyClass` is the PlatformPayment key class;
                // `userIdentityId` / `friendIdentityId` populate
                // for the DashPay account variants. Only the
                // disambiguators meaningful for the current
                // `accountType` are populated; others are zero or
                // empty by construction.
                FieldRow(label: "Standard Tag", value: "\(record.standardTag)")
                FieldRow(
                    label: "Registration Index",
                    value: "\(record.registrationIndex)"
                )
                FieldRow(label: "Key Class", value: "\(record.keyClass)")
                FieldRow(
                    label: "User Identity ID",
                    value: record.userIdentityId.isEmpty
                        ? "—"
                        : hexString(record.userIdentityId)
                )
                FieldRow(
                    label: "Friend Identity ID",
                    value: record.friendIdentityId.isEmpty
                        ? "—"
                        : hexString(record.friendIdentityId)
                )
            }
            Section("Balance") {
                FieldRow(label: "Confirmed", value: "\(record.balanceConfirmed)")
                FieldRow(label: "Unconfirmed", value: "\(record.balanceUnconfirmed)")
            }
            Section("Address Pools") {
                FieldRow(label: "External Highest Used", value: "\(record.externalHighestUsed)")
                FieldRow(label: "Internal Highest Used", value: "\(record.internalHighestUsed)")
            }
            Section("Relationships") {
                // Per-account transaction count = union of creating
                // and spending txs across this account's TXOs.
                // `PersistentTransaction` is no longer
                // account-scoped, so this has to be derived in Swift.
                FieldRow(label: "Transactions", value: "\(distinctTransactionCount)")
                FieldRow(label: "TXOs", value: "\(txoCount)")
                FieldRow(label: "Core Addresses", value: "\(record.coreAddresses.count)")
                FieldRow(
                    label: "Platform Addresses",
                    value: "\(record.platformAddresses.count)"
                )
                FieldRow(
                    label: "Wallet",
                    value: walletLabel(record.wallet)
                )
            }
            ForEach(addressSections(), id: \.0) { poolName, addresses in
                Section("\(poolName) Addresses (\(addresses.count))") {
                    ForEach(addresses) { addr in
                        NavigationLink(destination: CoreAddressDetailView(record: addr)) {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(addr.address)
                                    .font(.system(.caption, design: .monospaced))
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                                HStack(spacing: 8) {
                                    Text("Index \(addr.addressIndex)")
                                    if addr.isUsed {
                                        Text("• used")
                                    }
                                    if addr.balance > 0 {
                                        Text("• \(addr.balance)")
                                    }
                                }
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            }
                        }
                    }
                }
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Account")
        .navigationBarTitleDisplayMode(.inline)
    }

    /// Group the account's addresses by pool-type tag and present in
    /// a stable order: External, Internal, Additional, Additional
    /// (Hardened). Empty sections are skipped. Matches
    /// `PersistentCoreAddress.poolTypeName` (tags 2/3 are the on-demand
    /// "Additional" pools; no Rust "Absent" jargon).
    private func addressSections() -> [(String, [PersistentCoreAddress])] {
        let grouped = Dictionary(grouping: record.coreAddresses) { $0.poolTypeTag }
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
}

// MARK: - PersistentCoreAddress

struct CoreAddressDetailView: View {
    let record: PersistentCoreAddress

    @EnvironmentObject private var walletManager: PlatformWalletManager

    /// The revealed key material, held only after the user confirms.
    /// `nil` keeps the section in its "View Private Key" gated state.
    @State private var privateKey: ManagedPlatformWallet.CoreAddressPrivateKey?
    @State private var showRevealConfirm = false
    @State private var isRevealing = false
    @State private var revealError: String?
    /// Label of the row whose value was just copied, for a transient
    /// "Copied" confirmation.
    @State private var copiedLabel: String?

    var body: some View {
        Form {
            Section("Address") {
                FieldRow(label: "Address", value: record.address)
                FieldRow(label: "Pool", value: record.poolTypeName)
                FieldRow(label: "Index", value: "\(record.addressIndex)")
                FieldRow(label: "Derivation Path", value: record.derivationPath)
                FieldRow(label: "Used", value: record.isUsed ? "Yes" : "No")
            }
            Section("Public Key") {
                FieldRow(
                    label: publicKeyTypeLabel(byteCount: record.publicKey.count),
                    value: record.publicKey.isEmpty
                        ? "—"
                        : record.publicKey.map { String(format: "%02x", $0) }.joined()
                )
            }
            privateKeySection
            Section("Balance / Activity") {
                FieldRow(label: "Balance", value: "\(record.balance)")
                FieldRow(
                    label: "First Seen Height",
                    value: record.firstSeenHeight == 0 ? "—" : "\(record.firstSeenHeight)"
                )
                FieldRow(
                    label: "Last Seen Height",
                    value: record.lastSeenHeight == 0 ? "—" : "\(record.lastSeenHeight)"
                )
            }
            Section("Relationships") {
                FieldRow(label: "Account", value: record.account?.accountTypeName ?? "—")
                FieldRow(label: "TXOs", value: "\(record.txos.count)")
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Address")
        .navigationBarTitleDisplayMode(.inline)
    }

    /// Reveal-gated private-key section. Before reveal it shows a single
    /// "View Private Key" button that pops a confirmation dialog (this is
    /// a developer example app, so a plain confirm — no biometrics — is
    /// enough). After the user confirms, the derived hex + WIF are shown
    /// monospaced with tap-to-copy.
    @ViewBuilder
    private var privateKeySection: some View {
        Section("Private Key") {
            if let key = privateKey {
                copyableKeyRow(label: "Hex", value: key.hex)
                copyableKeyRow(label: "WIF", value: key.wif)
                Text("Anyone with this key controls this address's funds. Never share it.")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            } else {
                Button {
                    showRevealConfirm = true
                } label: {
                    HStack {
                        Image(systemName: "key.fill")
                        Text(isRevealing ? "Revealing…" : "View Private Key")
                    }
                }
                .disabled(isRevealing)

                if let revealError {
                    Text(revealError)
                        .font(.caption)
                        .foregroundColor(.red)
                }
            }
        }
        .confirmationDialog(
            "Reveal Private Key?",
            isPresented: $showRevealConfirm,
            titleVisibility: .visible
        ) {
            Button("Reveal Private Key", role: .destructive) { reveal() }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("The private key grants full control of this address's funds. Only reveal it somewhere private.")
        }
    }

    /// One monospaced key row (hex or WIF) with tap-to-copy and a
    /// transient "Copied" confirmation.
    @ViewBuilder
    private func copyableKeyRow(label: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(label).foregroundColor(.secondary)
                Spacer()
                if copiedLabel == label {
                    Label("Copied", systemImage: "checkmark")
                        .font(.caption2)
                        .foregroundColor(.green)
                } else {
                    Image(systemName: "doc.on.doc")
                        .font(.caption)
                        .foregroundColor(.accentColor)
                }
            }
            Text(value)
                .font(.system(.footnote, design: .monospaced))
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .contentShape(Rectangle())
        .onTapGesture { copy(value, label: label) }
    }

    /// Look up the owning wallet and ask Rust to derive this address's
    /// private key. All derivation happens on the Rust side; the mnemonic
    /// is pulled on demand via the resolver and never enters Swift.
    private func reveal() {
        guard let walletId = record.account?.wallet.walletId else {
            revealError = "This address is not linked to a wallet."
            return
        }
        guard let wallet = walletManager.wallet(for: walletId) else {
            revealError = "The owning wallet is not loaded."
            return
        }
        isRevealing = true
        revealError = nil
        // Off the main thread: the synchronous FFI's resolver reads the
        // iOS Keychain, which can stall. Mirrors
        // `AccountDetailView.revealPrivateKey(index:)`.
        Task {
            do {
                let key = try wallet.coreAddressPrivateKey(address: record.address)
                await MainActor.run {
                    privateKey = key
                    isRevealing = false
                }
            } catch {
                await MainActor.run {
                    revealError = error.localizedDescription
                    isRevealing = false
                }
            }
        }
    }

    private func copy(_ value: String, label: String) {
        // This copies a raw private key / WIF to the system-wide
        // pasteboard, which other apps and clipboard managers can read and
        // Universal Clipboard syncs across devices. Set a short expiry so
        // the secret doesn't linger there indefinitely. Fine for this demo
        // app; a production wallet should avoid clipboard export of secrets
        // (or gate it far more tightly).
        UIPasteboard.general.setItems(
            [["public.utf8-plain-text": value]],
            options: [.expirationDate: Date().addingTimeInterval(60)]
        )
        copiedLabel = label
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
            if copiedLabel == label { copiedLabel = nil }
        }
    }
}

// MARK: - PersistentTransaction

struct TransactionStorageDetailView: View {
    let record: PersistentTransaction

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "TXID", value: record.txidHex)
                FieldRow(label: "Direction", value: record.displayDirection)
                FieldRow(label: "Type", value: record.transactionType)
                FieldRow(label: "Net Amount", value: record.formattedAmount)
                if let fee = record.fee {
                    FieldRow(label: "Fee", value: "\(fee) duffs")
                }
            }
            Section("Block") {
                FieldRow(label: "Context", value: record.contextName)
                FieldRow(label: "Height", value: "\(record.blockHeight)")
                FieldRow(label: "Timestamp", value: "\(record.blockTimestamp)")
                if let hash = record.blockHash {
                    FieldRow(label: "Block Hash", value: hexString(hash))
                }
            }
            Section("Metadata") {
                FieldRow(label: "Label", value: record.label.isEmpty ? "None" : record.label)
                FieldRow(label: "First Seen", value: "\(record.firstSeen)")
                FieldRow(label: "TX Size", value: "\(record.transactionData.count) bytes")
            }
            // Per-output drill-downs. Each row navigates to the
            // owning `PersistentTxo` so the address / spent state /
            // wallet linkage of that single output is one tap away.
            // Sorted by `vout` so the order matches the on-chain
            // serialization. The vout column is left-aligned and
            // monospaced so columns line up across rows.
            Section {
                if record.outputs.isEmpty {
                    Text("None")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    ForEach(record.outputs.sorted { $0.vout < $1.vout }) { txo in
                        NavigationLink(destination: TxoStorageDetailView(record: txo)) {
                            txoRowLabel(txo, indexLabel: "vout \(txo.vout)")
                        }
                    }
                }
            } header: {
                Text("Outputs (\(record.outputs.count))")
            }

            // Per-input drill-downs. Each input is the
            // `PersistentTxo` of the *previous* output that this tx
            // consumed — tapping it surfaces where the funds came
            // from (address, originating tx, amount). Rows are
            // ordered by `spendingInputIndex` (the canonical vin
            // position captured when the spend was reconciled), so
            // row N matches input N in the serialized transaction.
            // The fallback ordering (`outpointHex`) only kicks in
            // for legacy rows that predate the column or for rows
            // whose pending-input resolution didn't run with an
            // index — both rare edge cases that drop to the bottom
            // of the list with a sentinel "prev" label.
            Section {
                if record.inputs.isEmpty {
                    Text("None")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    let orderedInputs = record.inputs.sorted { lhs, rhs in
                        switch (lhs.spendingInputIndex, rhs.spendingInputIndex) {
                        case let (.some(l), .some(r)): return l < r
                        case (.some, .none): return true
                        case (.none, .some): return false
                        case (.none, .none): return lhs.outpointHex < rhs.outpointHex
                        }
                    }
                    ForEach(orderedInputs) { txo in
                        NavigationLink(destination: TxoStorageDetailView(record: txo)) {
                            txoRowLabel(
                                txo,
                                indexLabel: txo.spendingInputIndex.map { "vin \($0)" } ?? "prev"
                            )
                        }
                    }
                }
            } header: {
                Text("Inputs (\(record.inputs.count))")
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Transaction")
        .navigationBarTitleDisplayMode(.inline)
    }

    /// Two-line row label for an input / output cell. Top line:
    /// `<vout-or-prev>  <amount>  <spent-pill>`. Bottom line: the
    /// canonical block-explorer outpoint string —
    /// `<display-order txid hex>:<vout>` via `PersistentTxo.outpointHex`,
    /// which is what users paste into DashScan / mempool explorers —
    /// followed by the address when present. Address line stays
    /// truncated-middle so a long Base58 doesn't push the right edge
    /// off the screen. (If you ever need the raw 36-byte outpoint
    /// hex instead, use `hexString(txo.outpoint)` — the
    /// `outpointHex` accessor flips byte order on the txid half.)
    @ViewBuilder
    private func txoRowLabel(
        _ txo: PersistentTxo,
        indexLabel: String
    ) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: 6) {
                Text(indexLabel)
                    .font(.caption2.monospaced())
                    .foregroundColor(.secondary)
                Text(txo.formattedAmount)
                    .font(.caption)
                if txo.isSpent {
                    Text("spent")
                        .font(.caption2)
                        .padding(.horizontal, 5)
                        .padding(.vertical, 1)
                        .background(Color.red.opacity(0.15))
                        .foregroundColor(.red)
                        .clipShape(Capsule())
                }
                if txo.isCoinbase {
                    Text("coinbase")
                        .font(.caption2)
                        .padding(.horizontal, 5)
                        .padding(.vertical, 1)
                        .background(Color.orange.opacity(0.15))
                        .foregroundColor(.orange)
                        .clipShape(Capsule())
                }
                if txo.isInstantLocked {
                    Text("IS")
                        .font(.caption2)
                        .padding(.horizontal, 5)
                        .padding(.vertical, 1)
                        .background(Color.green.opacity(0.15))
                        .foregroundColor(.green)
                        .clipShape(Capsule())
                }
                Spacer()
            }
            Text(txo.outpointHex)
                .font(.caption2.monospaced())
                .foregroundColor(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
            if !txo.address.isEmpty {
                Text(txo.address)
                    .font(.caption2.monospaced())
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
        }
        .padding(.vertical, 2)
    }
}

// MARK: - PersistentTxo

struct TxoStorageDetailView: View {
    let record: PersistentTxo

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Outpoint", value: record.outpointHex)
                FieldRow(label: "TXID", value: record.txidHex)
                FieldRow(label: "Vout", value: "\(record.vout)")
                FieldRow(label: "Amount", value: record.formattedAmount)
            }
            Section("Status") {
                FieldRow(label: "Height", value: "\(record.height)")
                FieldRow(label: "Confirmed", value: record.isConfirmed ? "Yes" : "No")
                FieldRow(label: "InstantLocked", value: record.isInstantLocked ? "Yes" : "No")
                FieldRow(label: "Coinbase", value: record.isCoinbase ? "Yes" : "No")
                FieldRow(label: "Locked", value: record.isLocked ? "Yes" : "No")
                FieldRow(label: "Spent", value: record.isSpent ? "Yes" : "No")
            }
            Section("Relationships") {
                // Address: tappable when the `coreAddress` link
                // exists (navigates to the address detail), plain
                // text fallback otherwise. The Base58Check string
                // is the authoritative identifier in either case;
                // the link just makes the address-row drill-down
                // one tap away when we have it.
                if let coreAddress = record.coreAddress {
                    NavigationLink(destination: CoreAddressDetailView(record: coreAddress)) {
                        FieldRow(label: "Address", value: record.address)
                    }
                } else {
                    FieldRow(label: "Address", value: record.address)
                }
                // Prefer the canonical `coreAddress.account` path;
                // fall back to the one-way `account` field for TXOs
                // whose address row hasn't been linked yet.
                FieldRow(
                    label: "Account",
                    value: (record.coreAddress?.account ?? record.account)?.accountTypeName ?? "—"
                )
                FieldRow(label: "Wallet ID", value: record.walletId.isEmpty ? "—" : hexString(record.walletId))
                FieldRow(
                    label: "Created By",
                    value: record.transaction?.txidHex ?? "—"
                )
                FieldRow(
                    label: "Spent By",
                    value: record.spendingTransaction?.txidHex ?? "—"
                )
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("TXO")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentPendingInput

struct PendingInputStorageDetailView: View {
    let record: PersistentPendingInput

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Outpoint", value: outpointHex(record.outpoint))
                FieldRow(label: "Input Index", value: "\(record.inputIndex)")
                // Display order matches the canonical block-explorer
                // form (byte-reversed from on-disk wire order) — same
                // convention `PersistentTransaction.txidHex` uses.
                FieldRow(
                    label: "Spending TXID",
                    value: record.spendingTxid.reversed()
                        .map { String(format: "%02x", $0) }
                        .joined()
                )
                FieldRow(label: "Wallet ID", value: record.walletId.isEmpty ? "—" : hexString(record.walletId))
            }
            Section("Relationships") {
                if let spending = record.spendingTransaction {
                    NavigationLink(destination: TransactionStorageDetailView(record: spending)) {
                        FieldRow(label: "Spending Transaction", value: spending.txidHex)
                    }
                } else {
                    // The pending row's parent transaction may not have
                    // faulted in (rare — the cascade-delete relationship
                    // keeps them in lockstep, but the field is optional
                    // for SwiftData's brief-window tolerance). Surface
                    // explicitly so reviewers can spot the orphan.
                    FieldRow(label: "Spending Transaction", value: "— (unlinked)")
                }
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
            }
            Section {
                Text(
                    "A pending input lives here until its previous-output "
                    + "PersistentTxo arrives. On `upsertUtxo`, the matching "
                    + "row is consumed: the new TXO is marked spent, "
                    + "linked to this row's spendingTransaction, and the "
                    + "pending entry is deleted in one pass."
                )
                .font(.caption2)
                .foregroundColor(.secondary)
            }
        }
        .navigationTitle("Pending Input")
        .navigationBarTitleDisplayMode(.inline)
    }

    /// 36-byte outpoint as `<txid hex (display order)>:<vout>`.
    /// Duplicates the helper in the list view rather than threading
    /// it through a shared file — the function is small and the two
    /// surfaces don't otherwise collaborate.
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

struct WalletManagerMetadataStorageDetailView: View {
    let record: PersistentWalletManagerMetadata

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Network", value: record.network.displayName)
                FieldRow(label: "Combined Sync Height", value: "\(record.combinedSyncHeight)")
                FieldRow(label: "Wallet Count", value: "\(record.walletCount)")
                if let hash = record.combinedSyncBlockHash {
                    FieldRow(label: "Block Hash", value: hexString(hash))
                }
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Manager Metadata")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentMasternode

struct MasternodeStorageDetailView: View {
    let record: PersistentMasternode

    var body: some View {
        Form {
            Section("Identity") {
                FieldRow(label: "Wallet ID", value: hexString(record.walletId))
                FieldRow(label: "proTxHash", value: record.proTxHashHex)
                FieldRow(label: "Registration Txid", value: hexString(record.registrationTxid))
                FieldRow(label: "Type", value: record.typeName)
                FieldRow(label: "Status", value: record.statusName)
            }
            Section("Service") {
                FieldRow(label: "Service Address", value: record.serviceAddress ?? "—")
            }
            Section("Keys") {
                FieldRow(
                    label: "Owner Key Hash",
                    value: record.ownerKeyHash.map(hexString) ?? "—"
                )
                FieldRow(
                    label: "Voting Key Hash",
                    value: record.votingKeyHash.map(hexString) ?? "—"
                )
                FieldRow(label: "Owner Address", value: record.ownerAddress ?? "—")
                FieldRow(label: "Voting Address", value: record.votingAddress ?? "—")
            }
            Section("Collateral") {
                FieldRow(
                    label: "Collateral Txid",
                    value: record.collateralTxid.map(hexString) ?? "—"
                )
                FieldRow(label: "Collateral Vout", value: "\(record.collateralVout)")
            }
            Section("Aggregation") {
                FieldRow(label: "Has Registration", value: record.hasRegistration ? "Yes" : "No")
                FieldRow(label: "Registration Height", value: "\(record.registrationHeight)")
                FieldRow(label: "Tx Count", value: "\(record.txCount)")
                FieldRow(label: "Order Index", value: "\(record.orderIndex)")
                FieldRow(label: "Type Index", value: "\(record.typeIndex)")
            }
            Section("Revocation") {
                FieldRow(label: "Revoked", value: record.revoked ? "Yes" : "No")
                FieldRow(label: "Revocation Reason", value: "\(record.revocationReason)")
                FieldRow(label: "Status Raw", value: "\(record.statusRaw)")
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle(record.displayTitle)
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentTrackedMasternode

struct TrackedMasternodeStorageDetailView: View {
    let record: PersistentTrackedMasternode

    var body: some View {
        Form {
            Section("Identity") {
                FieldRow(label: "Network", value: record.network?.displayName ?? "raw \(record.networkRaw)")
                FieldRow(label: "proTxHash (wire)", value: hexString(record.proTxHash))
                FieldRow(label: "Label", value: record.label ?? "—")
                FieldRow(
                    label: "Added",
                    value: dateString(Date(timeIntervalSince1970: TimeInterval(record.addedAt))))
            }
            Section("Snapshot") {
                // Opaque, Rust-owned document (PUBLIC material only) —
                // shown verbatim; only Rust interprets it.
                Text(record.snapshotJSON)
                    .font(.system(.caption2, design: .monospaced))
                    .textSelection(.enabled)
            }
        }
        .navigationTitle(record.label ?? "Tracked Masternode")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentShieldedNote

struct ShieldedNoteStorageDetailView: View {
    let record: PersistentShieldedNote

    var body: some View {
        Form {
            Section("Identity") {
                FieldRow(label: "Wallet ID", value: hexString(record.walletId))
                FieldRow(label: "Account Index", value: "\(record.accountIndex)")
                FieldRow(label: "Position", value: "\(record.position)")
            }
            Section("Commitment") {
                FieldRow(label: "cmx", value: hexString(record.cmx))
                FieldRow(label: "Nullifier", value: hexString(record.nullifier))
            }
            Section("State") {
                FieldRow(label: "Block Height", value: "\(record.blockHeight)")
                FieldRow(label: "Spent", value: record.isSpent ? "Yes" : "No")
                FieldRow(label: "Value", value: "\(record.value) credits")
            }
            Section("Note Bytes") {
                Text(hexString(record.noteData))
                    .font(.system(.caption2, design: .monospaced))
                    .textSelection(.enabled)
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Shielded Note")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentShieldedOutgoingNote

struct ShieldedOutgoingNoteStorageDetailView: View {
    let record: PersistentShieldedOutgoingNote

    var body: some View {
        Form {
            Section("Identity") {
                FieldRow(label: "Wallet ID", value: hexString(record.walletId))
                FieldRow(label: "Account Index", value: "\(record.accountIndex)")
            }
            Section("Commitment") {
                FieldRow(label: "cmx", value: hexString(record.cmx))
            }
            Section("Send") {
                FieldRow(label: "Recipient", value: hexString(record.recipient))
                FieldRow(label: "Value", value: "\(record.value) credits")
                FieldRow(label: "Block Height", value: "\(record.blockHeight)")
            }
            Section("Memo") {
                if record.memo.isEmpty {
                    Text("(empty)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Text(hexString(record.memo))
                        .font(.system(.caption2, design: .monospaced))
                        .textSelection(.enabled)
                }
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Shielded Sent Note")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentShieldedActivity

struct ShieldedActivityStorageDetailView: View {
    let record: PersistentShieldedActivity

    var body: some View {
        Form {
            Section("Identity") {
                FieldRow(label: "Wallet ID", value: hexString(record.walletId))
                FieldRow(label: "Account Index", value: "\(record.accountIndex)")
                FieldRow(label: "Entry ID", value: hexString(record.entryId))
            }
            Section("Classification") {
                FieldRow(label: "Kind Tag", value: kindDisplay(record.kindTag))
                FieldRow(label: "Direction", value: directionDisplay(record.direction))
                FieldRow(label: "Status", value: statusDisplay(record.status))
            }
            Section("Amounts") {
                FieldRow(label: "Amount", value: "\(record.amount) credits")
                FieldRow(
                    label: "Fee",
                    value: record.hasFee ? "\(record.fee) credits" : "(unknown)"
                )
                FieldRow(
                    label: "Block Height",
                    value: record.hasBlockHeight ? "\(record.blockHeight)" : "(pending)"
                )
            }
            Section("Linkage") {
                if !record.identityId.isEmpty {
                    FieldRow(label: "Identity ID", value: hexString(record.identityId))
                }
                if !record.counterparty.isEmpty {
                    FieldRow(label: "Counterparty", value: hexString(record.counterparty))
                }
                FieldRow(label: "Note cmxs", value: "\(record.noteCmxs.count / 32)")
                FieldRow(label: "Spent Nullifiers", value: "\(record.spentNullifiers.count / 32)")
            }
            Section("Memo") {
                if record.memo.isEmpty {
                    Text("(empty)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Text(hexString(record.memo))
                        .font(.system(.caption2, design: .monospaced))
                        .textSelection(.enabled)
                }
            }
            Section("Timestamps") {
                FieldRow(label: "Created (ms)", value: "\(record.createdAtMs)")
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Shielded Activity")
        .navigationBarTitleDisplayMode(.inline)
    }

    private func kindDisplay(_ tag: Int) -> String {
        let name: String
        switch tag {
        case 0: name = "Shield"
        case 1: name = "ShieldFromAssetLock"
        case 2: name = "Received"
        case 3: name = "Sent"
        case 4: name = "Unshield"
        case 5: name = "Withdrawal"
        case 6: name = "IdentityCreate"
        case 7: name = "ShieldedSpend"
        default: return "Unknown(\(tag))"
        }
        return "\(name) (\(tag))"
    }

    private func directionDisplay(_ raw: Int) -> String {
        let name: String
        switch raw {
        case 0: name = "In"
        case 1: name = "Out"
        case 2: name = "Self"
        default: return "Unknown(\(raw))"
        }
        return "\(name) (\(raw))"
    }

    private func statusDisplay(_ raw: Int) -> String {
        let name: String
        switch raw {
        case 0: name = "Pending"
        case 1: name = "Confirmed"
        case 2: name = "Failed"
        default: return "Unknown(\(raw))"
        }
        return "\(name) (\(raw))"
    }
}

// MARK: - PersistentShieldedSyncState

struct ShieldedSyncStateStorageDetailView: View {
    let record: PersistentShieldedSyncState

    var body: some View {
        Form {
            Section("Identity") {
                FieldRow(label: "Wallet ID", value: hexString(record.walletId))
                FieldRow(label: "Account Index", value: "\(record.accountIndex)")
            }
            Section("Sync") {
                FieldRow(label: "Last Synced Index", value: "\(record.lastSyncedIndex)")
            }
            Section("Timestamps") {
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Shielded Sync State")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentAssetLock

struct AssetLockStorageDetailView: View {
    let record: PersistentAssetLock

    /// Candidate identity rows at this asset lock's
    /// `identityIndex`. Filtered down to the strict
    /// `(walletId, identityIndex)` match in `linkedIdentity` —
    /// using the predicate alone would miss legacy rows that
    /// don't yet have the `wallet` relationship populated.
    @Query private var candidateIdentities: [PersistentIdentity]

    /// Wallet this asset lock belongs to. Filtered by walletId so
    /// the bech32m HRP picker on the Recipient section reads the
    /// correct network. `@Query` is reactive; if the wallet row
    /// vanishes (e.g. wallet deletion), the helper falls back to
    /// testnet HRP rather than crashing.
    @Query private var owningWallets: [PersistentWallet]

    init(record: PersistentAssetLock) {
        self.record = record
        // `PersistentAssetLock.identityIndexRaw` is `Int32` (the
        // changeset FFI uses i32 to match the upstream tracked
        // type), but `PersistentIdentity.identityIndex` is `UInt32`
        // (the DIP-9 slot is unsigned). Bridge with a `UInt32`
        // cast captured in Swift before predicate construction —
        // SwiftData's `#Predicate` macro doesn't allow inline
        // conversions inside the closure body.
        let identityIndex = UInt32(bitPattern: record.identityIndexRaw)
        _candidateIdentities = Query(
            filter: #Predicate<PersistentIdentity> { identity in
                identity.identityIndex == identityIndex
            }
        )
        let walletId = record.walletId
        _owningWallets = Query(
            filter: #Predicate<PersistentWallet> { wallet in
                wallet.walletId == walletId
            }
        )
    }

    /// Resolve the identity row this asset lock points at. Strict
    /// `(walletId, identityIndex)` match preferred; legacy rows
    /// that lack the `wallet` relationship fall back to a plain
    /// `identityIndex` match (single candidate only — multiple
    /// orphaned candidates at the same index are ambiguous and we
    /// don't guess).
    private var linkedIdentity: PersistentIdentity? {
        if let strict = candidateIdentities.first(where: {
            $0.wallet?.walletId == record.walletId
        }) {
            return strict
        }
        let orphaned = candidateIdentities.filter { $0.wallet == nil }
        return orphaned.count == 1 ? orphaned.first : nil
    }

    var body: some View {
        Form {
            Section("Asset Lock") {
                FieldRow(label: "Outpoint", value: record.outPointHex)
                FieldRow(label: "Status", value: record.statusLabel)
                FieldRow(label: "Funding Type", value: fundingTypeLabel(record.fundingTypeRaw))
                FieldRow(label: "Identity Index", value: "\(record.identityIndexRaw)")
                FieldRow(label: "Amount (duffs)", value: "\(record.amountDuffs)")
                FieldRow(label: "Wallet ID", value: hexString(record.walletId))
            }
            if isAddressFunding {
                // Address-funding section: show the recipient
                // platform address when Swift stamped it after a
                // successful `fundFromAssetLock`. `nil` on rows
                // that pre-date this column or whose funding hasn't
                // completed yet — communicate either case
                // explicitly so the explorer entry is self-
                // describing.
                Section("Recipient Platform Address") {
                    if let hash = record.recipientPlatformAddressHash {
                        FieldRow(label: "Hash", value: hexString(hash))
                        FieldRow(
                            label: "Address Type",
                            value: addressTypeLabel(record.recipientPlatformAddressType)
                        )
                        if let encoded = bech32mPlatformAddress(
                            hash: hash,
                            addressType: record.recipientPlatformAddressType
                        ) {
                            FieldRow(label: "Bech32m", value: encoded)
                        }
                    } else if record.statusRaw == 4 {
                        FieldRow(
                            label: "Recipient",
                            value: "— (pre-this-commit row)"
                        )
                    } else {
                        FieldRow(
                            label: "Recipient",
                            value: "— (funding not yet completed)"
                        )
                    }
                }
            }
            if isIdentityFunding {
                // Identity section is always shown for identity-
                // funding asset locks (Registration / TopUp). If the
                // linked identity is in SwiftData, drill-down link;
                // otherwise surface the current registration status
                // so partial / in-flight asset locks aren't silently
                // hidden.
                Section("Identity") {
                    if let identity = linkedIdentity {
                        // Static row — punted on navigation. Pushing
                        // `IdentityDetailView` from this nested
                        // Settings → Storage path hung the main
                        // thread on iOS 26 and burned a session
                        // chasing the cause. Tap-to-copy `Text` is
                        // good enough for an explorer surface; the
                        // operational identity view is reachable
                        // from the Identities tab.
                        Text(identity.identityIdBase58)
                            .font(.system(.caption, design: .monospaced))
                            .foregroundColor(.primary)
                            .lineLimit(1)
                            .truncationMode(.middle)
                            .textSelection(.enabled)
                    } else {
                        // No matching identity row yet. Either the
                        // asset lock is still pre-finality
                        // (statusRaw 0/1) and the registration
                        // hasn't been submitted, or it's IS/CL-
                        // locked (2/3) but the IdentityCreate
                        // transition failed or wasn't submitted —
                        // surface either case with the current
                        // status so the entry is self-explanatory.
                        FieldRow(
                            label: pendingLabel(record.statusRaw),
                            value: record.statusLabel
                        )
                    }
                }
            }
            Section("Bytes") {
                FieldRow(label: "Transaction Bytes", value: "\(record.transactionBytes.count) bytes")
                FieldRow(
                    label: "Proof Bytes",
                    value: record.proofBytes.map { "\($0.count) bytes" } ?? "—"
                )
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.updatedAt))
            }
        }
        .navigationTitle("Asset Lock")
        .navigationBarTitleDisplayMode(.inline)
    }

    private func fundingTypeLabel(_ raw: Int) -> String {
        switch raw {
        case 0: return "IdentityRegistration"
        case 1: return "IdentityTopUp"
        case 2: return "IdentityTopUpNotBound"
        case 3: return "IdentityInvitation"
        case 4: return "AssetLockAddressTopUp"
        case 5: return "AssetLockShieldedAddressTopUp"
        default: return "Unknown(\(raw))"
        }
    }

    /// True when this asset lock funded an identity at a specific
    /// `(walletId, identityIndex)` slot — i.e. registration or
    /// top-up. The other funding types don't deterministically
    /// resolve to a single identity on this wallet.
    private var isIdentityFunding: Bool {
        record.fundingTypeRaw == 0 || record.fundingTypeRaw == 1
    }

    /// True when this asset lock funded a platform address via
    /// `AddressFundingFromAssetLockTransition` (`fundingTypeRaw == 4`).
    /// The Recipient Platform Address section shows the destination
    /// hash + bech32m encoding when set.
    private var isAddressFunding: Bool {
        record.fundingTypeRaw == 4
    }

    /// Render the recipient address-type byte as a human label.
    /// 0 = P2PKH (the only shape the wallet generates today),
    /// 1 = P2SH (reserved). Defensive for future-shape support.
    private func addressTypeLabel(_ raw: UInt8?) -> String {
        switch raw {
        case 0: return "P2PKH"
        case 1: return "P2SH"
        case .some(let v): return "Unknown(\(v))"
        case .none: return "—"
        }
    }

    /// Encode the recipient hash as a DIP-0018 bech32m platform
    /// address. Returns `nil` for unsupported shapes (so the row
    /// silently falls back to the hex display) and on any encoder
    /// failure.
    ///
    /// HRP selection follows DIP-0018 — `dash` on mainnet, `tdash`
    /// on every other network. We pull the network from the
    /// matching wallet row when available; absent that we default
    /// to testnet which is the common case in this example app.
    private func bech32mPlatformAddress(
        hash: Data,
        addressType: UInt8?
    ) -> String? {
        guard hash.count == 20 else { return nil }
        // Bech32m type byte: 0xb0 for P2PKH, 0x80 for P2SH (per
        // DIP-0018). Note these differ from the storage discriminant
        // (0 / 1) — same conversion the Rust side does in
        // `PlatformAddress::to_bech32m_string` /
        // `from_bech32m_string`.
        let typeByte: UInt8
        switch addressType {
        case 0: typeByte = 0xb0
        case 1: typeByte = 0x80
        default: return nil
        }
        var payload5: [UInt8] = []
        let payload8: [UInt8] = [typeByte] + Array(hash)
        // Convert 8-bit → 5-bit groups. Bech32m payloads carry
        // 5-bit "data" symbols.
        guard convertBits(payload8, fromBits: 8, toBits: 5, pad: true, out: &payload5) else {
            return nil
        }
        let hrp = networkHRP()
        return bech32mEncode(hrp: hrp, data: payload5)
    }

    /// Determine the network HRP for the wallet that owns this
    /// asset lock. Reads from the matching `PersistentWallet`'s
    /// `network` field per DIP-0018 (`dash` on mainnet, `tdash`
    /// everywhere else). Falls back to testnet only when the
    /// owning wallet row can't be resolved (deleted wallet, legacy
    /// row without the relationship populated) — that case is
    /// already non-functional so the fallback string is
    /// inconsequential.
    private func networkHRP() -> String {
        guard let wallet = owningWallets.first, let network = wallet.network else {
            return "tdash"
        }
        switch network {
        case .mainnet: return "dash"
        default: return "tdash"
        }
    }
}

// MARK: - Bech32m helpers

/// Standard bech32 / bech32m bit-conversion. Inputs are unsigned
/// integers in `fromBits`-bit groups; outputs are unsigned
/// integers in `toBits`-bit groups. Returns false on overflow
/// (which never happens for the 8→5 case we use here).
private func convertBits(
    _ data: [UInt8],
    fromBits: Int,
    toBits: Int,
    pad: Bool,
    out: inout [UInt8]
) -> Bool {
    var acc: UInt32 = 0
    var bits: UInt32 = 0
    let maxv: UInt32 = (1 << toBits) - 1
    for value in data {
        let v = UInt32(value)
        if (v >> fromBits) != 0 { return false }
        acc = (acc << fromBits) | v
        bits += UInt32(fromBits)
        while bits >= toBits {
            bits -= UInt32(toBits)
            out.append(UInt8((acc >> bits) & maxv))
        }
    }
    if pad {
        if bits > 0 {
            out.append(UInt8((acc << (UInt32(toBits) - bits)) & maxv))
        }
    } else if bits >= fromBits || (acc << (UInt32(toBits) - bits)) & maxv != 0 {
        return false
    }
    return true
}

/// Encode a bech32m string (BIP-350). The checksum constant is the
/// BIP-350 0x2bc830a3 vs bech32's 1; everything else matches.
private func bech32mEncode(hrp: String, data: [UInt8]) -> String {
    let charset = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
    var combined = data
    combined.append(contentsOf: bech32mCreateChecksum(hrp: hrp, data: data))
    let charsetArr = Array(charset)
    var result = hrp + "1"
    for v in combined {
        result.append(charsetArr[Int(v)])
    }
    return result
}

private func bech32mCreateChecksum(hrp: String, data: [UInt8]) -> [UInt8] {
    var values: [UInt8] = bech32mHRPExpand(hrp)
    values.append(contentsOf: data)
    values.append(contentsOf: [0, 0, 0, 0, 0, 0])
    let mod = bech32mPolymod(values) ^ 0x2bc830a3
    var out: [UInt8] = []
    for i in 0..<6 {
        out.append(UInt8((mod >> (5 * (5 - i))) & 31))
    }
    return out
}

private func bech32mHRPExpand(_ hrp: String) -> [UInt8] {
    var ret: [UInt8] = []
    for c in hrp.utf8 { ret.append(UInt8(c >> 5)) }
    ret.append(0)
    for c in hrp.utf8 { ret.append(UInt8(c & 31)) }
    return ret
}

private func bech32mPolymod(_ values: [UInt8]) -> UInt32 {
    let gen: [UInt32] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3]
    var chk: UInt32 = 1
    for v in values {
        let top = chk >> 25
        chk = ((chk & 0x1ffffff) << 5) ^ UInt32(v)
        for i in 0..<5 {
            if (top >> i) & 1 != 0 {
                chk ^= gen[i]
            }
        }
    }
    return chk
}

private extension AssetLockStorageDetailView {

    /// Label for the pending row shown when no identity row has
    /// been persisted for this slot yet. Communicates whether the
    /// lock is mid-flight (still on its way to finality) versus
    /// IS/CL-locked but the IdentityCreate transition never
    /// completed.
    private func pendingLabel(_ raw: Int) -> String {
        switch raw {
        case 0, 1: return "In progress"
        case 2, 3: return "Pending (unused)"
        default: return "Pending"
        }
    }
}

// MARK: - PersistentShieldedViewingKey

struct ShieldedViewingKeyStorageDetailView: View {
    let record: PersistentShieldedViewingKey

    var body: some View {
        Form {
            Section("Identity") {
                FieldRow(label: "Wallet ID", value: hexString(record.walletId))
                FieldRow(label: "Account Index", value: "\(record.accountIndex)")
            }
            Section("Viewing Key") {
                // Viewing-grade only (cannot spend), but still key
                // material — the full 96-byte FVK is intentionally
                // rendered for QA inspection, matching how the
                // explorer shows other derived public-key batches.
                FieldRow(label: "FVK Length", value: "\(record.fvkBytes.count) bytes")
                FieldRow(label: "FVK (hex)", value: hexString(record.fvkBytes))
            }
            Section("Timestamps") {
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Shielded Viewing Key")
        .navigationBarTitleDisplayMode(.inline)
    }
}
