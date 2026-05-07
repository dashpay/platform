import SwiftUI
import SwiftData
import SwiftDashSDK

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
                    label: "Bytes (hex)",
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
                FieldRow(label: "Wallet", value: record.wallet.name ?? hexString(record.wallet.walletId))
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
    /// a stable order: External, Internal, Absent, Absent (Hardened).
    /// Empty sections are skipped.
    private func addressSections() -> [(String, [PersistentCoreAddress])] {
        let grouped = Dictionary(grouping: record.coreAddresses) { $0.poolTypeTag }
        let order: [(UInt8, String)] = [
            (0, "External"),
            (1, "Internal"),
            (2, "Absent"),
            (3, "Absent (Hardened)"),
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
                    label: "Bytes (hex)",
                    value: record.publicKey.isEmpty
                        ? "—"
                        : record.publicKey.map { String(format: "%02x", $0) }.joined()
                )
            }
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
}

// MARK: - PersistentTransaction

struct TransactionStorageDetailView: View {
    let record: PersistentTransaction

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "TXID", value: record.txidHex)
                FieldRow(label: "Direction", value: record.directionName)
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
            Section("Nullifier Checkpoint") {
                FieldRow(label: "Present", value: record.hasNullifierCheckpoint ? "Yes" : "No")
                if record.hasNullifierCheckpoint {
                    FieldRow(label: "Height", value: "\(record.nullifierCheckpointHeight)")
                    FieldRow(label: "Timestamp", value: "\(record.nullifierCheckpointTimestamp)")
                }
            }
            Section("Timestamps") {
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Shielded Sync State")
        .navigationBarTitleDisplayMode(.inline)
    }
}
