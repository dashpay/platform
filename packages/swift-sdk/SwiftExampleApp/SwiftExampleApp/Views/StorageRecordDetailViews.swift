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

private func dateString(_ date: Date?) -> String {
    guard let date = date else { return "None" }
    return date.formatted(date: .abbreviated, time: .shortened)
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
                FieldRow(label: "Network", value: record.network)
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
                FieldRow(label: "Public Keys", value: "\(record.publicKeys.count)")
                FieldRow(label: "Documents", value: "\(record.documents.count)")
                FieldRow(label: "Token Balances", value: "\(record.tokenBalances.count)")
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

// MARK: - PersistentDocument

struct DocumentStorageDetailView: View {
    let record: PersistentDocument

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Document ID", value: record.documentId)
                FieldRow(label: "Type", value: record.documentType)
                FieldRow(label: "Revision", value: "\(record.revision)")
                FieldRow(label: "Contract ID", value: record.contractId)
                FieldRow(label: "Owner ID", value: record.ownerId)
                FieldRow(label: "Network", value: record.network)
                FieldRow(label: "Deleted", value: record.isDeleted ? "Yes" : "No")
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.localCreatedAt))
                FieldRow(label: "Updated", value: dateString(record.localUpdatedAt))
            }
            if let json = jsonString(record.data) {
                Section("Data") {
                    Text(json).font(.system(.caption, design: .monospaced)).textSelection(.enabled)
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
                FieldRow(label: "Version", value: "\(record.version)")
                FieldRow(label: "Owner (Base58)", value: record.ownerIdBase58 ?? "None")
                FieldRow(label: "Network", value: record.network)
                FieldRow(label: "Has Tokens", value: record.hasTokens ? "Yes" : "No")
            }
            Section("Flags") {
                FieldRow(label: "Can Be Deleted", value: record.canBeDeleted ? "Yes" : "No")
                FieldRow(label: "Read Only", value: record.readonly ? "Yes" : "No")
                FieldRow(label: "Keeps History", value: record.keepsHistory ? "Yes" : "No")
            }
            Section("Relationships") {
                FieldRow(label: "Document Types", value: "\(record.documentTypes?.count ?? 0)")
                FieldRow(label: "Tokens", value: "\(record.tokens?.count ?? 0)")
                FieldRow(label: "Documents", value: "\(record.documents.count)")
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
                FieldRow(label: "Accessed", value: dateString(record.lastAccessedAt))
                FieldRow(label: "Synced", value: dateString(record.lastSyncedAt))
            }
            Section("Serialized") {
                FieldRow(label: "Contract Size", value: "\(record.serializedContract.count) bytes")
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
                FieldRow(label: "Purpose", value: record.purpose)
                FieldRow(label: "Security Level", value: record.securityLevel)
                FieldRow(label: "Key Type", value: record.keyType)
                FieldRow(label: "Read Only", value: record.readOnly ? "Yes" : "No")
                FieldRow(label: "Disabled At", value: record.disabledAt.map { "\($0)" } ?? "No")
            }
            Section("Data") {
                FieldRow(label: "Public Key", value: hexString(record.publicKeyData))
                FieldRow(label: "Private Key", value: record.hasPrivateKeyIdentifier ? "Present" : "Not set")
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Accessed", value: dateString(record.lastAccessed))
            }
        }
        .navigationTitle("Public Key")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentToken

struct TokenStorageDetailView: View {
    let record: PersistentToken

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "ID", value: hexString(record.id))
                FieldRow(label: "Contract (Base58)", value: record.contractIdBase58)
                FieldRow(label: "Name", value: record.name)
                FieldRow(label: "Position", value: "\(record.position)")
                FieldRow(label: "Decimals", value: "\(record.decimals)")
                FieldRow(label: "Base Supply", value: record.formattedBaseSupply)
                FieldRow(label: "Paused", value: record.isPaused ? "Yes" : "No")
            }
            Section("Relationships") {
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
                FieldRow(label: "Frozen", value: record.frozen ? "Yes" : "No")
                FieldRow(label: "Network", value: record.network)
            }
            Section("Token Info") {
                FieldRow(label: "Name", value: record.tokenName ?? "None")
                FieldRow(label: "Symbol", value: record.tokenSymbol ?? "None")
                FieldRow(label: "Decimals", value: record.tokenDecimals.map { "\($0)" } ?? "None")
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
                FieldRow(label: "Event Type", value: record.eventType)
                FieldRow(label: "Transaction ID", value: record.transactionId.map { hexString($0) } ?? "None")
                FieldRow(label: "Block Height", value: "\(record.blockHeight)")
                FieldRow(label: "Amount", value: "\(record.amount)")
            }
            Section("Parties") {
                FieldRow(label: "From", value: record.fromIdentity.map { hexString($0) } ?? "None")
                FieldRow(label: "To", value: record.toIdentity.map { hexString($0) } ?? "None")
                FieldRow(label: "Performed By", value: hexString(record.performedByIdentity))
            }
            Section("Balance") {
                FieldRow(label: "Before", value: "\(record.balanceBefore)")
                FieldRow(label: "After", value: "\(record.balanceAfter)")
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
            }
            Section("Flags") {
                FieldRow(label: "Keeps History", value: record.documentsKeepHistory ? "Yes" : "No")
                FieldRow(label: "Mutable", value: record.documentsMutable ? "Yes" : "No")
                FieldRow(label: "Can Be Deleted", value: record.documentsCanBeDeleted ? "Yes" : "No")
            }
            Section("Relationships") {
                FieldRow(label: "Properties", value: "\(record.propertiesList?.count ?? 0)")
                FieldRow(label: "Indices", value: "\(record.indices?.count ?? 0)")
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
                FieldRow(label: "Required", value: record.isRequired ? "Yes" : "No")
            }
            Section("Constraints") {
                if let v = record.minLength { FieldRow(label: "Min Length", value: "\(v)") }
                if let v = record.maxLength { FieldRow(label: "Max Length", value: "\(v)") }
                if let v = record.pattern { FieldRow(label: "Pattern", value: v) }
                if let v = record.format { FieldRow(label: "Format", value: v) }
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
                if let contract = record.dataContract {
                    FieldRow(label: "Contract", value: contract.name)
                }
            }
        }
        .navigationTitle("Keyword")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentSyncState

struct SyncStateStorageDetailView: View {
    let record: PersistentSyncState

    private var blockDate: Date? {
        record.syncTimestamp > 0
            ? Date(timeIntervalSince1970: TimeInterval(record.syncTimestamp))
            : nil
    }

    var body: some View {
        Form {
            Section("Sync Watermark") {
                FieldRow(label: "Sync Height", value: "\(record.syncHeight)")
                FieldRow(label: "Sync Timestamp", value: "\(record.syncTimestamp)")
                if let date = blockDate {
                    FieldRow(label: "Local Time", value: date.formatted(date: .abbreviated, time: .standard))
                    FieldRow(label: "UTC", value: {
                        let f = DateFormatter()
                        f.dateFormat = "yyyy-MM-dd HH:mm:ss"
                        f.timeZone = TimeZone(identifier: "UTC")
                        return f.string(from: date) + " UTC"
                    }())
                }
                FieldRow(label: "Last Known Recent Block", value: record.lastKnownRecentBlock > 0
                    ? "\(record.lastKnownRecentBlock)"
                    : "0 (no recent address activity)")
                FieldRow(label: "Wallet ID", value: hexString(record.walletId))
            }
            Section("Timestamps") {
                FieldRow(label: "Record Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Sync State")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentAddressBalance

struct AddressBalanceStorageDetailView: View {
    let record: PersistentAddressBalance

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Address Type", value: record.addressType == 0 ? "P2PKH" : "P2SH")
                FieldRow(label: "Address Hash", value: hexString(record.addressHash))
                FieldRow(label: "Balance", value: "\(record.balance) credits")
                FieldRow(label: "Wallet ID", value: hexString(record.walletId))
            }
            Section("Timestamps") {
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Address Balance")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentWallet

struct WalletStorageDetailView: View {
    let record: PersistentWallet

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Wallet ID", value: hexString(record.walletId))
                FieldRow(label: "Network", value: record.network)
                FieldRow(label: "Name", value: record.name ?? "None")
                FieldRow(label: "Birth Height", value: "\(record.birthHeight)")
                FieldRow(label: "Synced Height", value: "\(record.syncedHeight)")
            }
            Section("Balance") {
                FieldRow(label: "Confirmed", value: "\(record.balanceConfirmed)")
                FieldRow(label: "Unconfirmed", value: "\(record.balanceUnconfirmed)")
                FieldRow(label: "Immature", value: "\(record.balanceImmature)")
                FieldRow(label: "Locked", value: "\(record.balanceLocked)")
            }
            Section("Relationships") {
                FieldRow(label: "Accounts", value: "\(record.accounts.count)")
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Wallet")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentAccount

struct AccountStorageDetailView: View {
    let record: PersistentAccount

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Type", value: record.accountTypeName)
                FieldRow(label: "Type ID", value: "\(record.accountType)")
                FieldRow(label: "Index", value: "\(record.accountIndex)")
                FieldRow(label: "Watch Only", value: record.isWatchOnly ? "Yes" : "No")
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
                FieldRow(label: "Transactions", value: "\(record.transactions.count)")
                FieldRow(label: "UTXOs", value: "\(record.utxos.count)")
                FieldRow(label: "Wallet", value: record.wallet?.name ?? record.wallet.map { hexString($0.walletId) } ?? "None")
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Account")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentTransaction

struct TransactionStorageDetailView: View {
    let record: PersistentTransaction

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "TXID", value: record.txid)
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
                if let size = record.transactionData?.count {
                    FieldRow(label: "TX Size", value: "\(size) bytes")
                }
            }
            Section("Relationships") {
                FieldRow(label: "Account", value: record.account?.accountTypeName ?? "None")
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("Transaction")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentUtxo

struct UtxoStorageDetailView: View {
    let record: PersistentUtxo

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Outpoint", value: record.outpoint)
                FieldRow(label: "TXID", value: record.txid)
                FieldRow(label: "Vout", value: "\(record.vout)")
                FieldRow(label: "Amount", value: record.formattedAmount)
                FieldRow(label: "Address", value: record.address)
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
                FieldRow(label: "Account", value: record.account?.accountTypeName ?? "None")
            }
            Section("Timestamps") {
                FieldRow(label: "Created", value: dateString(record.createdAt))
                FieldRow(label: "Updated", value: dateString(record.lastUpdated))
            }
        }
        .navigationTitle("UTXO")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - PersistentWalletManagerMetadata

struct WalletManagerMetadataStorageDetailView: View {
    let record: PersistentWalletManagerMetadata

    var body: some View {
        Form {
            Section("Core") {
                FieldRow(label: "Network", value: record.network)
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
