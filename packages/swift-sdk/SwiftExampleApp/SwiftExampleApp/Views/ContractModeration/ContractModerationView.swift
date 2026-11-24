import SwiftUI
import SwiftData
import SwiftDashSDK

/// Contract moderation and fee pots (protocol version 14) for one contract:
/// its fee pots, its banlist, suspension list and warning list, one
/// identity's status, the removal records of its document types, its team
/// actions and the moderation action counts.
///
/// Every read is offered whatever the contract declares: which lists it
/// keeps, which types keep removal records and whether it is elected are
/// the node's to answer, and its refusal is shown on the screen that asked.
///
/// The moderations (state transition type 24) are offered only to the
/// identities of the loaded wallets Rust names, and only what Rust lists for
/// each (``ContractModerationOfferStore``), on the screens below: an identity
/// from a list row or the status screen, a document from "Moderate
/// Documents", a restore from a removal record, an approval from a team
/// action.
struct ContractModerationView: View {
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    /// The store the hub reads identities from and signs against. Passed on
    /// to every screen of the hub so they all use the same one.
    @Environment(\.modelContext) private var modelContext
    @Query private var identities: [PersistentIdentity]
    @StateObject private var offers = ContractModerationOfferStore()

    /// The contract, and the connection current when it was opened: the hub
    /// and every screen under it read and show only while that connection is
    /// still the app's.
    let scope: ModerationScope
    /// The contract's document types, offered for removal records.
    let documentTypeNames: [String]

    /// The contract's id, base58.
    private var contractId: String { scope.contractId }

    var body: some View {
        ModerationScopeGate(scope: scope) { hub }
            .environmentObject(offers)
            .navigationTitle("Moderation")
            .navigationBarTitleDisplayMode(.inline)
            .navigationDestination(for: ContractModerationRoute.self) { route in
                ModerationScopeGate(scope: scope) { destination(for: route) }
                    .environmentObject(offers)
                    .environment(\.modelContext, modelContext)
            }
            .task(id: scope) { await reloadOffer() }
            // A replaced SDK: the answer was read through the old connection
            .onChange(of: appState.sdkGeneration) { _, _ in offers.clear() }
    }

    /// Ask every loaded wallet on the current network what its identities
    /// may do on this contract.
    private func reloadOffer() async {
        guard appState.moderationConnection(in: scope) != nil else {
            offers.clear()
            return
        }
        let network = appState.currentNetwork
        let walletIds = Set(
            identities
                .filter { $0.network == network }
                .compactMap { $0.wallet?.walletId }
        )
        await offers.reload(
            contractId: contractId,
            walletIds: walletIds.sorted(by: { $0.lexicographicallyPrecedes($1) }),
            walletManager: walletManager
        )
    }

    @ViewBuilder
    private var moderateSection: some View {
        Section {
            if offers.isLoading {
                ModerationMessageRow(
                    kind: .loading,
                    text: "Asking who may moderate…",
                    identifier: "contractModeration.offer.loading"
                )
            } else if let error = offers.errorMessage {
                ModerationMessageRow(
                    kind: .error,
                    text: "Could not ask who may moderate: \(error)",
                    identifier: "contractModeration.offer.error"
                )
            } else if offers.loadedOnce && offers.noLinkedWallet {
                ModerationMessageRow(
                    kind: .empty,
                    text: "No wallet linked to an identity on this network was found in this app, so no wallet was asked who may moderate this contract.",
                    identifier: "contractModeration.offer.noWallet"
                )
            } else if offers.nobodyModerates {
                ModerationMessageRow(
                    kind: .empty,
                    text: "No identity of a loaded wallet moderates this contract.",
                    identifier: "contractModeration.offer.none"
                )
            }
            if !offers.isLoading && offers.loadedOnce && offers.unloadedWalletCount > 0 {
                ModerationMessageRow(
                    kind: .error,
                    text: unloadedWalletsText(offers.unloadedWalletCount),
                    identifier: "contractModeration.offer.unloadedWallets"
                )
            }
            ForEach(offers.moderators) { moderator in
                VStack(alignment: .leading, spacing: 2) {
                    ModerationFieldRow(label: moderator.offer.role.title, value: label(forIdentity: moderator.identityIdBase58), monospaced: true)
                    if !moderator.offer.userActions.isEmpty {
                        Text(moderator.offer.userActions.map(\.title).joined(separator: ", "))
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("contractModeration.offer.moderator.\(moderator.identityIdBase58)")
            }
            ForEach(offers.documentTypeNames, id: \.self) { name in
                NavigationLink(value: ContractModerationRoute.document(documentTypeName: name)) {
                    Label("Moderate \(name) Documents", systemImage: "doc.badge.gearshape")
                }
                .accessibilityIdentifier("contractModeration.documentLink.\(name)")
            }
            Button("Ask Again") {
                Task { await reloadOffer() }
            }
            .disabled(offers.isLoading)
            .accessibilityIdentifier("contractModeration.offer.reload")
        } header: {
            Text("Moderate")
        } footer: {
            Text("The identities of the loaded wallets that moderate this contract, as Rust read its declaration and seated team. Identities are moderated from a list row or the identity status screen; Platform still refuses what the target, document or team action does not admit.")
        }
    }

    /// The note for wallets that hold identities on this network but are not
    /// loaded, so they were not asked.
    private func unloadedWalletsText(_ count: Int) -> String {
        count == 1
            ? "1 wallet holding identities on this network is not loaded and was not asked."
            : "\(count) wallets holding identities on this network are not loaded and were not asked."
    }

    /// The alias of a local identity, or its base58 id.
    private func label(forIdentity base58: String) -> String {
        identities.first { $0.identityIdBase58 == base58 }?.alias ?? base58
    }

    private var hub: some View {
        List {
            ContractFeePotsSection(scope: scope)

            moderateSection

            Section {
                ForEach(ContractModerationList.allCases) { list in
                    NavigationLink(value: ContractModerationRoute.list(list)) {
                        Label(list.title, systemImage: list.systemImage)
                    }
                    .accessibilityIdentifier("contractModeration.listLink.\(list.rawValue)")
                }
                NavigationLink(value: ContractModerationRoute.status) {
                    Label("Look Up an Identity", systemImage: "person.crop.circle.badge.questionmark")
                }
                .accessibilityIdentifier("contractModeration.statusLink")
            } header: {
                Text("Moderation Lists")
            } footer: {
                Text("A list the contract does not keep is refused by the node.")
            }

            Section {
                if documentTypeNames.isEmpty {
                    ModerationMessageRow(
                        kind: .empty,
                        text: "This contract has no document types.",
                        identifier: "contractModeration.removalsEmpty"
                    )
                }
                ForEach(documentTypeNames, id: \.self) { name in
                    NavigationLink(value: ContractModerationRoute.removals(documentTypeName: name)) {
                        Label(name, systemImage: "trash.slash")
                    }
                    .accessibilityIdentifier("contractModeration.removalsLink.\(name)")
                }
            } header: {
                Text("Removal Records")
            } footer: {
                Text("Records of documents a moderator deleted. A type that keeps none is refused by the node.")
            }

            Section {
                NavigationLink(value: ContractModerationRoute.teamActions) {
                    Label("Team Actions", systemImage: "person.3.sequence")
                }
                .accessibilityIdentifier("contractModeration.teamActionsLink")
                NavigationLink(value: ContractModerationRoute.actionCounts) {
                    Label("Moderation Action Counts", systemImage: "number")
                }
                .accessibilityIdentifier("contractModeration.actionCountsLink")
            } header: {
                Text("Moderation Team")
            } footer: {
                Text("Only an elected contract has a seated team; the node refuses any other. Team actions are kept only when at least one document type sets moderatorAbilities.deleteSettled: otherwise the node refuses them, which differs from an empty list.")
            }
        }
    }

    @ViewBuilder
    private func destination(for route: ContractModerationRoute) -> some View {
        switch route {
        case .list(let list):
            ContractModerationEntriesView(scope: scope, list: list)
        case .status:
            ContractModerationStatusView(scope: scope)
        case .removals(let documentTypeName):
            ContractDocumentRemovalsView(scope: scope, documentTypeName: documentTypeName)
        case .teamActions:
            ContractTeamActionsView(scope: scope)
        case .teamAction(let action, let status, let connection):
            ContractTeamActionDetailView(
                scope: scope,
                action: action,
                status: status,
                connection: connection
            )
        case .actionCounts:
            ContractModerationActionCountsView(scope: scope)
        case .document(let documentTypeName):
            ContractDocumentModerationView(scope: scope, documentTypeName: documentTypeName)
        }
    }
}

extension ContractModerationList {
    /// The list's name on screen.
    var title: String {
        switch self {
        case .banlist: return "Banlist"
        case .suspensions: return "Suspensions"
        case .warnings: return "Warnings"
        }
    }

    var systemImage: String {
        switch self {
        case .banlist: return "nosign"
        case .suspensions: return "pause.circle"
        case .warnings: return "exclamationmark.bubble"
        }
    }
}
