import SwiftUI
import SwiftDashSDK

/// Where the next page of team actions starts: the action and whether the
/// page includes it, both as the last page returned them.
struct ContractTeamActionsCursor: Sendable, Equatable {
    let actionId: String
    let included: Bool
}

/// The actions an elected contract's seated moderation team votes on,
/// active or closed, a page at a time in action id order. Each drills into
/// ``ContractTeamActionDetailView`` for its event and signers.
struct ContractTeamActionsView: View {
    @EnvironmentObject var appState: AppState
    let contractId: String

    /// Actions per page.
    private static let pageSize: UInt32 = 20

    @State private var status: ContractTeamActionStatus = .active
    /// The status the rows shown were listed under, which their signers are
    /// read with.
    @State private var listedStatus: ContractTeamActionStatus = .active
    @StateObject private var loader =
        ModerationPagedLoader<ContractTeamActionEntry, ContractTeamActionsCursor>()

    var body: some View {
        List {
            Section {
                Picker("Status", selection: $status) {
                    Text("Active").tag(ContractTeamActionStatus.active)
                    Text("Closed").tag(ContractTeamActionStatus.closed)
                }
                .pickerStyle(.segmented)
                .accessibilityIdentifier("contractModeration.teamActions.status")
            } footer: {
                Text(status == .active
                     ? "Actions still gathering approvals."
                     : "Actions whose approvals met the rule and ran.")
            }

            if let errorMessage = loader.errorMessage {
                ModerationMessageRow(
                    kind: .error,
                    text: errorMessage,
                    identifier: "contractModeration.teamActions.error"
                )
            }

            Section("Actions") {
                if loader.items.isEmpty {
                    if loader.isLoading {
                        ModerationMessageRow(
                            kind: .loading,
                            text: "Loading actions…",
                            identifier: "contractModeration.teamActions.loading"
                        )
                    } else if loader.loadedOnce && loader.errorMessage == nil {
                        ModerationMessageRow(
                            kind: .empty,
                            text: "No actions.",
                            identifier: "contractModeration.teamActions.empty"
                        )
                    }
                }
                ForEach(loader.items) { action in
                    NavigationLink(value: ContractModerationRoute.teamAction(action, listedStatus)) {
                        ContractTeamActionRow(action: action)
                    }
                    .accessibilityIdentifier("contractModeration.teamActions.row.\(action.actionId)")
                }
                if loader.nextCursor != nil, !loader.items.isEmpty {
                    Button {
                        Task { await loader.loadMore() }
                    } label: {
                        HStack {
                            if loader.isLoading { ProgressView().controlSize(.small) }
                            Text("Load More")
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .disabled(loader.isLoading)
                    .accessibilityIdentifier("contractModeration.teamActions.loadMore")
                }
            }
        }
        .navigationTitle("Team Actions")
        .navigationBarTitleDisplayMode(.inline)
        .refreshable { await reload() }
        .task(id: status) { await reload() }
    }

    private func reload() async {
        guard let sdk = appState.sdk else {
            loader.fail("SDK not initialized")
            return
        }
        let contractId = contractId
        let status = status
        let pageSize = Self.pageSize
        listedStatus = status
        await loader.reload { cursor in
            // The cursor goes back as the last page returned it
            try await sdk.contractTeamActions(
                contractId: contractId,
                status: status,
                startAtActionId: cursor?.actionId,
                startAtActionIdIncluded: cursor?.included ?? false,
                limit: pageSize
            )
            .map { page in
                ModerationPage(
                    items: page.actions,
                    nextCursor: page.nextStartAtActionId.map {
                        ContractTeamActionsCursor(
                            actionId: $0,
                            included: page.nextStartAtActionIdIncluded ?? false
                        )
                    }
                )
            }
        }
    }
}

/// One team action in the listing.
private struct ContractTeamActionRow: View {
    let action: ContractTeamActionEntry

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            switch action.event {
            case .deleteSettledDocument(let event):
                Text("Delete \(event.documentTypeName) document")
                    .font(.headline)
                Text(event.documentId)
                    .font(.system(.caption2, design: .monospaced))
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            HStack(spacing: 12) {
                Label("\(action.approvalCount)", systemImage: "checkmark.seal")
                Text(moderationDate(ms: action.proposedAtMs))
            }
            .font(.caption)
            .foregroundColor(.secondary)
        }
        .padding(.vertical, 2)
    }
}
