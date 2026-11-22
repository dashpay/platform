import SwiftUI
import SwiftDashSDK

/// The actions an elected contract's seated moderation team votes on,
/// active or closed, a page at a time in action id order. Each drills into
/// ``ContractTeamActionDetailView`` for its event and signers.
struct ContractTeamActionsView: View {
    @EnvironmentObject var appState: AppState
    let contractId: String

    /// Actions per page.
    private static let pageSize: UInt32 = 20

    @State private var status: ContractTeamActionStatus = .active
    @State private var actions: [ContractTeamActionEntry] = []
    @State private var nextStartAtActionId: String?
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var loadedOnce = false
    /// Bumped on every fresh load so a page that arrives late for the other
    /// status is dropped instead of mixed in.
    @State private var generation = 0

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

            if let errorMessage {
                ModerationMessageRow(
                    kind: .error,
                    text: errorMessage,
                    identifier: "contractModeration.teamActions.error"
                )
            }

            Section("Actions") {
                if actions.isEmpty {
                    if isLoading {
                        ModerationMessageRow(
                            kind: .loading,
                            text: "Loading actions…",
                            identifier: "contractModeration.teamActions.loading"
                        )
                    } else if loadedOnce && errorMessage == nil {
                        ModerationMessageRow(
                            kind: .empty,
                            text: "No actions.",
                            identifier: "contractModeration.teamActions.empty"
                        )
                    }
                }
                ForEach(actions) { action in
                    NavigationLink(value: ContractModerationRoute.teamAction(action, status)) {
                        ContractTeamActionRow(action: action)
                    }
                    .accessibilityIdentifier("contractModeration.teamActions.row.\(action.actionId)")
                }
                if let nextStartAtActionId, !actions.isEmpty {
                    Button {
                        Task { await load(startAt: nextStartAtActionId, generation: generation) }
                    } label: {
                        HStack {
                            if isLoading { ProgressView().controlSize(.small) }
                            Text("Load More")
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .disabled(isLoading)
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
        generation += 1
        actions = []
        nextStartAtActionId = nil
        errorMessage = nil
        await load(startAt: nil, generation: generation)
    }

    private func load(startAt: String?, generation: Int) async {
        guard let sdk = appState.sdk else {
            errorMessage = "SDK not initialized"
            return
        }
        isLoading = true
        errorMessage = nil
        defer {
            isLoading = false
            loadedOnce = true
        }
        let status = status
        do {
            // The page's cursor is the action to start after, so it is not
            // included.
            let page = try await sdk.contractTeamActions(
                contractId: contractId,
                status: status,
                startAtActionId: startAt,
                startAtActionIdIncluded: false,
                limit: Self.pageSize
            )
            guard generation == self.generation else { return }
            actions.append(contentsOf: page?.actions ?? [])
            nextStartAtActionId = page?.nextStartAtActionId
        } catch {
            guard generation == self.generation else { return }
            errorMessage = moderationErrorText(error)
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
