import SwiftUI
import SwiftDashSDK

/// Contract moderation and fee pots (protocol version 14) for one contract:
/// its fee pots, its banlist, suspension list and warning list, one
/// identity's status, the removal records of its document types, its team
/// actions and the moderation action counts.
///
/// Every choice is offered whatever the contract declares: which lists it
/// keeps, which types keep removal records and whether it is elected are
/// the node's to answer, and its refusal is shown on the screen that asked.
struct ContractModerationView: View {
    /// The contract's id, base58.
    let contractId: String
    /// The contract's document types, offered for removal records.
    let documentTypeNames: [String]

    var body: some View {
        List {
            ContractFeePotsSection(contractId: contractId)

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
        .navigationTitle("Moderation")
        .navigationBarTitleDisplayMode(.inline)
        .navigationDestination(for: ContractModerationRoute.self) { route in
            destination(for: route)
        }
    }

    @ViewBuilder
    private func destination(for route: ContractModerationRoute) -> some View {
        switch route {
        case .list(let list):
            ContractModerationEntriesView(contractId: contractId, list: list)
        case .status:
            ContractModerationStatusView(contractId: contractId)
        case .removals(let documentTypeName):
            ContractDocumentRemovalsView(contractId: contractId, documentTypeName: documentTypeName)
        case .teamActions:
            ContractTeamActionsView(contractId: contractId)
        case .teamAction(let action, let status):
            ContractTeamActionDetailView(contractId: contractId, action: action, status: status)
        case .actionCounts:
            ContractModerationActionCountsView(contractId: contractId)
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
