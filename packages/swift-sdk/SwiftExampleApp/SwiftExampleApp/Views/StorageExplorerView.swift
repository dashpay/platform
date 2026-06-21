import SwiftUI
import SwiftData
import SwiftDashSDK

struct StorageExplorerView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext
    @State private var counts: [String: Int] = [:]

    /// Active network the explorer is filtering by. The storage layer
    /// keeps every network's rows (so switching back to testnet
    /// re-surfaces what was synced there), but the UI only shows the
    /// rows tied to the network currently selected on `AppState`.
    private var network: Network { appState.currentNetwork }

    var body: some View {
        List {
            Section {
                HStack {
                    Label("Network", systemImage: "network")
                    Spacer()
                    Text(network.displayName)
                        .foregroundColor(.secondary)
                }
                .font(.callout)
            }
            modelRow("Identities", icon: "person.crop.circle", type: PersistentIdentity.self) {
                IdentityStorageListView(network: network)
            }
            // Identity-relationship caches: cascade-owned by
            // `PersistentIdentity`, surfaced as their own explorer
            // sections so the row counts and per-row drill-downs
            // are visible without going through the parent identity.
            modelRow("DPNS Names", icon: "at", type: PersistentDPNSName.self) {
                DPNSNameStorageListView(network: network)
            }
            modelRow(
                "DashPay Profiles",
                icon: "person.text.rectangle",
                type: PersistentDashpayProfile.self
            ) {
                DashpayProfileStorageListView(network: network)
            }
            modelRow(
                "Contact Requests",
                icon: "person.crop.circle.badge.plus",
                type: PersistentDashpayContactRequest.self
            ) {
                DashpayContactRequestStorageListView(network: network)
            }
            modelRow(
                "Contact Profiles",
                icon: "person.crop.circle",
                type: PersistentDashpayContactProfile.self
            ) {
                DashpayContactProfileStorageListView(network: network)
            }
            modelRow(
                "DashPay Payments",
                icon: "arrow.left.arrow.right.circle",
                type: PersistentDashpayPayment.self
            ) {
                DashpayPaymentStorageListView(network: network)
            }
            modelRow(
                "Ignored Senders",
                icon: "person.crop.circle.badge.xmark",
                type: PersistentDashpayIgnoredSender.self
            ) {
                DashpayIgnoredSenderStorageListView(network: network)
            }
            modelRow("Documents", icon: "doc.text", type: PersistentDocument.self) {
                DocumentStorageListView(network: network)
            }
            modelRow("Data Contracts", icon: "doc.plaintext", type: PersistentDataContract.self) {
                DataContractStorageListView(network: network)
            }
            modelRow("Public Keys", icon: "key", type: PersistentPublicKey.self) {
                PublicKeyStorageListView(network: network)
            }
            modelRow("Tokens", icon: "circle.hexagongrid", type: PersistentToken.self) {
                TokenStorageListView(network: network)
            }
            modelRow("Token Balances", icon: "banknote", type: PersistentTokenBalance.self) {
                TokenBalanceStorageListView(network: network)
            }
            modelRow("Token History", icon: "clock.arrow.circlepath", type: PersistentTokenHistoryEvent.self) {
                TokenHistoryStorageListView(network: network)
            }
            modelRow("Document Types", icon: "list.bullet.rectangle", type: PersistentDocumentType.self) {
                DocumentTypeStorageListView(network: network)
            }
            modelRow("Indices", icon: "tablecells", type: PersistentIndex.self) {
                IndexStorageListView(network: network)
            }
            modelRow("Properties", icon: "slider.horizontal.3", type: PersistentProperty.self) {
                PropertyStorageListView(network: network)
            }
            modelRow("Keywords", icon: "tag", type: PersistentKeyword.self) {
                KeywordStorageListView(network: network)
            }
            modelCountRow(
                "Platform Addresses",
                icon: "creditcard",
                countKey: platformAddressesCountKey
            ) {
                PlatformAddressStorageListView(network: network)
            }
            modelRow("Platform Addresses Sync State", icon: "arrow.triangle.2.circlepath", type: PersistentPlatformAddressesSyncState.self) {
                PlatformAddressesSyncStateStorageListView(network: network)
            }
            modelRow("Wallets", icon: "wallet.pass", type: PersistentWallet.self) {
                WalletStorageListView(network: network)
            }
            modelRow("Accounts", icon: "person.2", type: PersistentAccount.self) {
                AccountStorageListView(network: network)
            }
            modelCountRow(
                "Core Addresses",
                icon: "square.and.pencil",
                countKey: coreAddressesCountKey
            ) {
                CoreAddressStorageListView(network: network)
            }
            modelRow("Transactions", icon: "arrow.left.arrow.right.circle", type: PersistentTransaction.self) {
                TransactionStorageListView(network: network)
            }
            modelRow("TXOs", icon: "bitcoinsign.circle", type: PersistentTxo.self) {
                TxoStorageListView(network: network)
            }
            modelRow("Pending Inputs", icon: "hourglass", type: PersistentPendingInput.self) {
                PendingInputStorageListView(network: network)
            }
            modelRow("Asset Locks", icon: "lock.shield", type: PersistentAssetLock.self) {
                AssetLockStorageListView(network: network)
            }
            modelRow("Manager Metadata", icon: "gearshape.2", type: PersistentWalletManagerMetadata.self) {
                WalletManagerMetadataStorageListView(network: network)
            }
            modelRow("Shielded Notes", icon: "lock.shield", type: PersistentShieldedNote.self) {
                ShieldedNoteStorageListView(network: network)
            }
            modelRow(
                "Shielded Sent Notes",
                icon: "paperplane",
                type: PersistentShieldedOutgoingNote.self
            ) {
                ShieldedOutgoingNoteStorageListView(network: network)
            }
            modelRow(
                "Shielded Sync State",
                icon: "arrow.triangle.2.circlepath",
                type: PersistentShieldedSyncState.self
            ) {
                ShieldedSyncStateStorageListView(network: network)
            }
            modelRow(
                "Shielded Activity",
                icon: "clock.arrow.circlepath",
                type: PersistentShieldedActivity.self
            ) {
                ShieldedActivityStorageListView(network: network)
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
        // Re-count whenever the user flips networks while the
        // explorer is on screen. Without this the row counts would
        // stay pinned to the network that was active on `onAppear`.
        .onChange(of: appState.currentNetwork) { _, _ in
            loadCounts()
        }
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
        let raw = network.rawValue

        // Wallet-id set for the active network. Used to count
        // transaction-side rows that don't carry a `networkRaw` field
        // and instead trace back to a wallet via the
        // `walletId`/`account` denorms.
        let walletsOnNetwork: Set<Data> = {
            let descriptor = FetchDescriptor<PersistentWallet>(
                predicate: #Predicate { $0.networkRaw == raw }
            )
            let fetched = (try? modelContext.fetch(descriptor)) ?? []
            return Set(fetched.map(\.walletId))
        }()

        func directCount<T: PersistentModel>(
            _ type: T.Type,
            predicate: Predicate<T>
        ) {
            let key = String(describing: type)
            counts[key] = (try? modelContext.fetchCount(
                FetchDescriptor<T>(predicate: predicate)
            )) ?? 0
        }

        func filteredCount<T: PersistentModel>(
            _ type: T.Type,
            matches: (T) -> Bool
        ) {
            let key = String(describing: type)
            let all = (try? modelContext.fetch(FetchDescriptor<T>())) ?? []
            counts[key] = all.lazy.filter(matches).count
        }

        // Models with a direct `networkRaw` column — predicate-friendly,
        // no in-memory pass needed.
        directCount(PersistentIdentity.self, predicate: #Predicate { $0.networkRaw == raw })
        directCount(PersistentDPNSName.self, predicate: #Predicate { $0.networkRaw == raw })
        directCount(PersistentDashpayProfile.self, predicate: #Predicate { $0.networkRaw == raw })
        directCount(PersistentDashpayContactRequest.self, predicate: #Predicate { $0.networkRaw == raw })
        directCount(PersistentDashpayContactProfile.self, predicate: #Predicate { $0.networkRaw == raw })
        directCount(PersistentDashpayPayment.self, predicate: #Predicate { $0.networkRaw == raw })
        directCount(PersistentDashpayIgnoredSender.self, predicate: #Predicate { $0.networkRaw == raw })
        directCount(PersistentDocument.self, predicate: #Predicate { $0.networkRaw == raw })
        directCount(PersistentDataContract.self, predicate: #Predicate { $0.networkRaw == raw })
        directCount(PersistentTokenBalance.self, predicate: #Predicate { $0.networkRaw == raw })
        directCount(PersistentPlatformAddressesSyncState.self, predicate: #Predicate { $0.networkRaw == raw })
        directCount(PersistentWallet.self, predicate: #Predicate { $0.networkRaw == raw })
        directCount(PersistentWalletManagerMetadata.self, predicate: #Predicate { $0.networkRaw == raw })

        // Models that derive their network through a relationship —
        // SwiftData's predicate compiler can't follow optional
        // chains into these without crashing, so we count in memory.
        filteredCount(PersistentToken.self) { $0.dataContract?.networkRaw == raw }
        filteredCount(PersistentTokenHistoryEvent.self) {
            $0.token?.dataContract?.networkRaw == raw
        }
        filteredCount(PersistentDocumentType.self) { $0.dataContract?.networkRaw == raw }
        filteredCount(PersistentIndex.self) {
            $0.documentType?.dataContract?.networkRaw == raw
        }
        filteredCount(PersistentProperty.self) {
            $0.documentType?.dataContract?.networkRaw == raw
        }
        filteredCount(PersistentKeyword.self) { $0.dataContract?.networkRaw == raw }
        filteredCount(PersistentPublicKey.self) { $0.identity?.networkRaw == raw }
        filteredCount(PersistentAccount.self) { $0.wallet.networkRaw == raw }
        filteredCount(PersistentTransaction.self) { tx in
            for txo in tx.outputs where walletsOnNetwork.contains(txo.walletId) {
                return true
            }
            for txo in tx.inputs where walletsOnNetwork.contains(txo.walletId) {
                return true
            }
            return false
        }
        filteredCount(PersistentTxo.self) { walletsOnNetwork.contains($0.walletId) }
        filteredCount(PersistentPendingInput.self) {
            walletsOnNetwork.contains($0.walletId)
        }
        filteredCount(PersistentShieldedNote.self) {
            walletsOnNetwork.contains($0.walletId)
        }
        filteredCount(PersistentShieldedOutgoingNote.self) {
            walletsOnNetwork.contains($0.walletId)
        }
        filteredCount(PersistentShieldedSyncState.self) {
            walletsOnNetwork.contains($0.walletId)
        }
        filteredCount(PersistentShieldedActivity.self) {
            walletsOnNetwork.contains($0.walletId)
        }
        filteredCount(PersistentAssetLock.self) {
            walletsOnNetwork.contains($0.walletId)
        }

        // Core / Platform addresses partition the same family of
        // tables by account type, so they need their own counts.
        let coreAddresses = (try? modelContext.fetch(
            FetchDescriptor<PersistentCoreAddress>()
        )) ?? []
        counts[coreAddressesCountKey] = coreAddresses.lazy
            .filter { $0.account?.wallet.networkRaw == raw }
            .count

        let platformAddresses = (try? modelContext.fetch(
            FetchDescriptor<PersistentPlatformAddress>()
        )) ?? []
        counts[platformAddressesCountKey] = platformAddresses.lazy
            .filter { entry in
                if let raw2 = entry.account?.wallet.networkRaw {
                    return raw2 == raw
                }
                // Fallback: address row was persisted before the
                // account relationship caught up — match through the
                // denormalized `walletId`.
                return walletsOnNetwork.contains(entry.walletId)
            }
            .count
    }
}
