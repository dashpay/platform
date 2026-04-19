import SwiftUI
import SwiftData
import SwiftDashSDK

struct StorageExplorerView: View {
    @Environment(\.modelContext) private var modelContext
    @State private var counts: [String: Int] = [:]

    var body: some View {
        List {
            modelRow("Identities", icon: "person.crop.circle", type: PersistentIdentity.self) {
                IdentityStorageListView()
            }
            modelRow("Documents", icon: "doc.text", type: PersistentDocument.self) {
                DocumentStorageListView()
            }
            modelRow("Data Contracts", icon: "doc.plaintext", type: PersistentDataContract.self) {
                DataContractStorageListView()
            }
            modelRow("Public Keys", icon: "key", type: PersistentPublicKey.self) {
                PublicKeyStorageListView()
            }
            modelRow("Tokens", icon: "circle.hexagongrid", type: PersistentToken.self) {
                TokenStorageListView()
            }
            modelRow("Token Balances", icon: "banknote", type: PersistentTokenBalance.self) {
                TokenBalanceStorageListView()
            }
            modelRow("Token History", icon: "clock.arrow.circlepath", type: PersistentTokenHistoryEvent.self) {
                TokenHistoryStorageListView()
            }
            modelRow("Document Types", icon: "list.bullet.rectangle", type: PersistentDocumentType.self) {
                DocumentTypeStorageListView()
            }
            modelRow("Indices", icon: "tablecells", type: PersistentIndex.self) {
                IndexStorageListView()
            }
            modelRow("Properties", icon: "slider.horizontal.3", type: PersistentProperty.self) {
                PropertyStorageListView()
            }
            modelRow("Keywords", icon: "tag", type: PersistentKeyword.self) {
                KeywordStorageListView()
            }
            modelCountRow(
                "Platform Addresses",
                icon: "creditcard",
                countKey: platformAddressesCountKey
            ) {
                PlatformAddressStorageListView()
            }
            modelRow("Sync State", icon: "arrow.triangle.2.circlepath", type: PersistentSyncState.self) {
                SyncStateStorageListView()
            }
            modelRow("Wallets", icon: "wallet.pass", type: PersistentWallet.self) {
                WalletStorageListView()
            }
            modelRow("Accounts", icon: "person.2", type: PersistentAccount.self) {
                AccountStorageListView()
            }
            modelCountRow(
                "Core Addresses",
                icon: "square.and.pencil",
                countKey: coreAddressesCountKey
            ) {
                CoreAddressStorageListView()
            }
            modelRow("Transactions", icon: "arrow.left.arrow.right.circle", type: PersistentTransaction.self) {
                TransactionStorageListView()
            }
            modelRow("UTXOs", icon: "bitcoinsign.circle", type: PersistentUtxo.self) {
                UtxoStorageListView()
            }
            modelRow("Manager Metadata", icon: "gearshape.2", type: PersistentWalletManagerMetadata.self) {
                WalletManagerMetadataStorageListView()
            }
        }
        .navigationTitle("Storage Explorer")
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button(action: { loadCounts() }) {
                    Image(systemName: "arrow.clockwise")
                }
            }
        }
        .onAppear { loadCounts() }
    }

    private func modelRow<T: PersistentModel, D: View>(
        _ name: String,
        icon: String,
        type: T.Type,
        @ViewBuilder destination: @escaping () -> D
    ) -> some View {
        NavigationLink(destination: destination()) {
            HStack {
                Label(name, systemImage: icon)
                Spacer()
                Text("\(counts[String(describing: type)] ?? 0)")
                    .foregroundColor(.secondary)
                    .font(.callout)
            }
        }
    }

    /// Row variant for sections whose count isn't a 1:1 match with a
    /// persistent model type — e.g. "Core Addresses" and "Platform
    /// Addresses" both back onto `PersistentCoreAddress` but partition
    /// by `account.accountType`, so they need distinct count keys.
    private func modelCountRow<D: View>(
        _ name: String,
        icon: String,
        countKey: String,
        @ViewBuilder destination: @escaping () -> D
    ) -> some View {
        NavigationLink(destination: destination()) {
            HStack {
                Label(name, systemImage: icon)
                Spacer()
                Text("\(counts[countKey] ?? 0)")
                    .foregroundColor(.secondary)
                    .font(.callout)
            }
        }
    }

    private var platformAddressesCountKey: String { "PlatformAddresses" }
    private var coreAddressesCountKey: String { "CoreAddresses" }

    private func loadCounts() {
        func count<T: PersistentModel>(_ type: T.Type) {
            let key = String(describing: type)
            counts[key] = (try? modelContext.fetchCount(FetchDescriptor<T>())) ?? 0
        }
        count(PersistentIdentity.self)
        count(PersistentDocument.self)
        count(PersistentDataContract.self)
        count(PersistentPublicKey.self)
        count(PersistentToken.self)
        count(PersistentTokenBalance.self)
        count(PersistentTokenHistoryEvent.self)
        count(PersistentDocumentType.self)
        count(PersistentIndex.self)
        count(PersistentProperty.self)
        count(PersistentKeyword.self)
        count(PersistentSyncState.self)
        count(PersistentWallet.self)
        count(PersistentAccount.self)
        count(PersistentTransaction.self)
        count(PersistentUtxo.self)
        count(PersistentWalletManagerMetadata.self)
        // Core and Platform address rows live in separate models now
        // (PersistentCoreAddress vs PersistentPlatformAddress), so
        // counting them is a plain `fetchCount` per model.
        counts[platformAddressesCountKey] =
            (try? modelContext.fetchCount(FetchDescriptor<PersistentPlatformAddress>())) ?? 0
        counts[coreAddressesCountKey] =
            (try? modelContext.fetchCount(FetchDescriptor<PersistentCoreAddress>())) ?? 0
    }
}
