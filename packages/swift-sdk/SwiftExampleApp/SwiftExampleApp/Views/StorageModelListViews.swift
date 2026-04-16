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

// MARK: - PersistentAddressBalance

struct AddressBalanceStorageListView: View {
    @Query(sort: \PersistentAddressBalance.lastUpdated, order: .reverse)
    private var records: [PersistentAddressBalance]

    var body: some View {
        List(records) { record in
            NavigationLink(destination: AddressBalanceStorageDetailView(record: record)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(record.addressHash.map { String(format: "%02x", $0) }.joined())
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(1).truncationMode(.middle)
                    Text("\(record.balance) credits")
                        .font(.caption).foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Address Balances (\(records.count))")
        .overlay { if records.isEmpty { ContentUnavailableView("No Records", systemImage: "creditcard") } }
    }
}
