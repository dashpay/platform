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
    /// The contract and the connection it was opened on: this screen reads
    /// only through that connection, and the hub hides it once replaced.
    let scope: ModerationScope
    private var contractId: String { scope.contractId }

    /// Actions per page.
    private static let pageSize: UInt32 = 20

    @State private var status: ContractTeamActionStatus = .active
    /// The status the rows shown were listed under, which their signers are
    /// read with.
    @State private var listedStatus: ContractTeamActionStatus = .active
    /// Advanced by every read started (a status picked, a refresh), so a read
    /// queued before a newer one is not sent.
    @State private var readToken = 0
    /// The connection the rows shown were read through; their drill-down reads
    /// signers through it, and only while it is current.
    @State private var listedConnection: ModerationConnection?
    @StateObject private var loader =
        ModerationPagedLoader<ContractTeamActionEntry, ContractTeamActionsCursor>()

    var body: some View {
        List {
            Section {
                Picker("Status", selection: Binding(get: { status }, set: { select($0) })) {
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
                    NavigationLink(value: ContractModerationRoute.teamAction(
                        action,
                        listedStatus,
                        connection: listedConnection?.generation ?? -1
                    )) {
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
        .refreshable { await reload(status: status, token: begin(status: status)) }
        .task { await reload(status: status, token: begin(status: status)) }
        // A replaced SDK: drop what was read through the old one and read
        // nothing through the new one; the hub's gate shows the notice instead
        .onChange(of: appState.sdkGeneration) { _, _ in loader.clear() }
    }

    /// A status picked: in this main-actor turn the selection changes, the
    /// token advances and the loader drops the rows, cursor and error shown
    /// and any answer still in flight; then the read of that status is queued.
    private func select(_ newStatus: ContractTeamActionStatus) {
        guard newStatus != status else { return }
        status = newStatus
        let token = begin(status: newStatus)
        Task { await reload(status: newStatus, token: token) }
    }

    /// The synchronous part of a read of `status`; returns its token.
    private func begin(status: ContractTeamActionStatus) -> Int {
        readToken += 1
        listedStatus = status
        listedConnection = appState.moderationConnection(in: scope)
        loader.begin()
        return readToken
    }

    /// Reads the actions of `status`, the status captured when the read
    /// began, unless a newer selection superseded it before it ran.
    private func reload(status: ContractTeamActionStatus, token: Int) async {
        guard token == readToken else { return }
        guard let connection = listedConnection, appState.isCurrent(connection) else {
            loader.fail("SDK not initialized")
            return
        }
        let sdk = connection.sdk
        let appState = appState
        let contractId = contractId
        let pageSize = Self.pageSize
        await loader.reload({ cursor in
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
        }, connectionIsCurrent: { appState.isCurrent(connection) })
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
