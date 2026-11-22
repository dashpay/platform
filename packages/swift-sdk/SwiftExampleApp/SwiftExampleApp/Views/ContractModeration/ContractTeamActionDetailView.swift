import SwiftUI
import SwiftDashSDK

/// One team action: who proposed it, what it does, and who approved it.
struct ContractTeamActionDetailView: View {
    @EnvironmentObject var appState: AppState
    let contractId: String
    let action: ContractTeamActionEntry
    /// The status the action was listed under, which its signers are read
    /// with.
    let status: ContractTeamActionStatus

    @State private var signers: ModerationLoad<ContractTeamActionSigners?> = .idle

    var body: some View {
        List {
            Section("Action") {
                ModerationFieldRow(label: "Action ID", value: action.actionId, monospaced: true)
                ModerationFieldRow(label: "Status", value: status == .active ? "Active" : "Closed")
                ModerationFieldRow(label: "Proposer", value: action.proposerId, monospaced: true)
                ModerationFieldRow(label: "Proposed", value: moderationDate(ms: action.proposedAtMs))
                ModerationFieldRow(label: "Approvals", value: "\(action.approvalCount)")
            }

            Section("Event") {
                eventView
                    .accessibilityElement(children: .combine)
                    .accessibilityIdentifier("contractModeration.teamAction.event")
            }

            Section {
                signersView
                Button("Reload Signers") {
                    Task { await loadSigners() }
                }
                .disabled(isLoadingSigners)
                .accessibilityIdentifier("contractModeration.teamAction.signers.reload")
            } header: {
                Text("Signers")
            } footer: {
                Text("An active action's approval count is an upper bound; the signers are who approved it.")
            }
        }
        .navigationTitle("Team Action")
        .navigationBarTitleDisplayMode(.inline)
        .task { await loadSigners() }
    }

    @ViewBuilder
    private var eventView: some View {
        switch action.event {
        case .deleteSettledDocument(let event):
            VStack(alignment: .leading, spacing: 4) {
                Text("Delete a settled document")
                    .font(.headline)
                ModerationFieldRow(label: "Document type", value: event.documentTypeName)
                ModerationFieldRow(label: "Document", value: event.documentId, monospaced: true)
                ModerationFieldRow(
                    label: "Last modified",
                    value: moderationDate(ms: event.documentLastModifiedAtMs)
                )
                if let revision = event.documentRevision {
                    ModerationFieldRow(label: "Revision", value: "\(revision)")
                }
                ModerationReasonView(reason: event.reason)
            }
        }
    }

    @ViewBuilder
    private var signersView: some View {
        switch signers {
        case .idle, .loading:
            ModerationMessageRow(
                kind: .loading,
                text: "Loading signers…",
                identifier: "contractModeration.teamAction.signers.loading"
            )
        case .failed(let message):
            ModerationMessageRow(
                kind: .error,
                text: message,
                identifier: "contractModeration.teamAction.signers.error"
            )
        case .loaded(let answer):
            let signerIds = answer?.signerIds ?? []
            if signerIds.isEmpty {
                ModerationMessageRow(
                    kind: .empty,
                    text: "No signers: the contract holds no such action with this status.",
                    identifier: "contractModeration.teamAction.signers.empty"
                )
            }
            ForEach(signerIds, id: \.self) { signerId in
                Text(signerId)
                    .font(.system(.caption, design: .monospaced))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .textSelection(.enabled)
                    .accessibilityIdentifier("contractModeration.teamAction.signers.row.\(signerId)")
            }
        }
    }

    private var isLoadingSigners: Bool {
        if case .loading = signers { return true }
        return false
    }

    private func loadSigners() async {
        guard let sdk = appState.sdk else {
            signers = .failed("SDK not initialized")
            return
        }
        signers = .loading
        do {
            signers = .loaded(try await sdk.contractTeamActionSigners(
                contractId: contractId,
                status: status,
                actionId: action.actionId
            ))
        } catch {
            signers = .failed(moderationErrorText(error))
        }
    }
}
