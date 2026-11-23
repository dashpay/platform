import SwiftUI
import SwiftData
import SwiftDashSDK

/// One team action: who proposed it, what it does, and who approved it, and,
/// for an action listed active, an approval (state transition type 24) by
/// each identity of a loaded wallet Rust names a member of the seated team.
/// Whether the approval still counts (already given, the action closed
/// since, the document changed) is Platform's to answer; the proved status
/// says whether more approvals are needed.
struct ContractTeamActionDetailView: View {
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var offers: ContractModerationOfferStore
    @Environment(\.modelContext) private var modelContext
    /// The contract and the connection it was opened on: this screen reads
    /// only through that connection, and the hub hides it once replaced.
    let scope: ModerationScope
    private var contractId: String { scope.contractId }
    let action: ContractTeamActionEntry
    /// The status the action was listed under, which its signers are read
    /// with.
    let status: ContractTeamActionStatus
    /// The generation of the connection the listing was read through
    /// (``AppState/sdkGeneration``): the signers are read through that SDK,
    /// and not at all once it was replaced.
    let connection: Int

    @StateObject private var loader = ModerationSingleLoader<ContractTeamActionSigners?>()
    private var signers: ModerationLoad<ContractTeamActionSigners?> { loader.load }

    @State private var approvingAs: String?
    @State private var approval: ContractTeamActionSignature?
    @State private var approvalError: String?

    var body: some View {
        List {
            Section {
                ModerationFieldRow(label: "Action ID", value: action.actionId, monospaced: true)
                ModerationFieldRow(label: "Status when listed", value: status == .active ? "Active" : "Closed")
                ModerationFieldRow(label: "Proposer", value: action.proposerId, monospaced: true)
                ModerationFieldRow(label: "Proposed", value: moderationDate(ms: action.proposedAtMs))
                ModerationFieldRow(label: "Approvals when listed", value: "\(action.approvalCount)")
            } header: {
                Text("Action")
            } footer: {
                Text("The status and approval count are a snapshot from the listing, taken when the action was listed.")
            }

            Section("Event") {
                eventView
                    .accessibilityElement(children: .combine)
                    .accessibilityIdentifier("contractModeration.teamAction.event")
            }

            if status == .active && !offers.approvers.isEmpty {
                approveSection
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
                Text("The signers are read separately, by the status the action was listed under, possibly at a later block than the listing: an approval since then can show more signers than the count, and an action that has closed since shows no active signers. An active action's count is also an upper bound, as a member who left the team keeps their approval in it until a later approval drops it.")
            }
        }
        .navigationTitle("Team Action")
        .navigationBarTitleDisplayMode(.inline)
        .task { await loadSigners() }
        // A replaced SDK: the whole detail is a snapshot of the old connection,
        // so the hub's gate shows only the notice in its place
        .onChange(of: appState.sdkGeneration) { _, _ in listingReplaced() }
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

    private var approveSection: some View {
        Section {
            ForEach(offers.approvers) { approver in
                Button {
                    approve(as: approver)
                } label: {
                    HStack {
                        if approvingAs == approver.identityIdBase58 { ProgressView().controlSize(.small) }
                        Text("Approve as \(approver.offer.role.title) \(approver.identityIdBase58.prefix(8))…")
                    }
                    .frame(maxWidth: .infinity)
                }
                .disabled(approvingAs != nil)
                .accessibilityIdentifier("contractModeration.teamAction.approve.\(approver.identityIdBase58)")
            }
            if let approvalError {
                ModerationMessageRow(
                    kind: .error,
                    text: approvalError,
                    identifier: "contractModeration.teamAction.approveError"
                )
            }
            if let approval {
                ModerationFieldRow(
                    label: "Status after the approval",
                    value: approval.status == .closed
                        ? "Closed: the action ran"
                        : "Active: more approvals are needed"
                )
                .accessibilityIdentifier("contractModeration.teamAction.approveResult")
            }
        } header: {
            Text("Approve")
        } footer: {
            Text("Shown for each identity of a loaded wallet on the seated team.")
        }
    }

    private func approve(as approver: ContractModerationOfferStore.Moderator) {
        guard let wallet = walletManager.wallet(for: approver.walletId),
              let moderatorId = Data.identifier(fromBase58: approver.identityIdBase58),
              let contractIdBytes = Data.identifier(fromBase58: contractId),
              let actionIdBytes = Data.identifier(fromBase58: action.actionId) else {
            approvalError = DocumentActionError.noWallet.localizedDescription
            return
        }
        // Fresh `KeychainSigner` per approval, as `CreateDocumentView` does.
        let signer = KeychainSigner(modelContainer: modelContext.container)
        approvingAs = approver.identityIdBase58
        approvalError = nil
        approval = nil
        Task {
            do {
                approval = try await wallet.approveContractTeamAction(
                    moderatorIdentityId: moderatorId,
                    contractId: contractIdBytes,
                    actionId: actionIdBytes,
                    signer: signer
                )
            } catch {
                approvalError = moderationWriteErrorText(error)
            }
            approvingAs = nil
            await loadSigners()
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
        guard let listing = appState.moderationConnection(in: scope), listing.generation == connection else {
            listingReplaced()
            return
        }
        let sdk = listing.sdk
        let appState = appState
        let contractId = contractId
        let status = status
        let actionId = action.actionId
        await loader.reload(
            {
                try await sdk.contractTeamActionSigners(
                    contractId: contractId,
                    status: status,
                    actionId: actionId
                )
            },
            connectionIsCurrent: { appState.isCurrent(listing) }
        )
    }

    /// The SDK the listing was read through was replaced: its action may not
    /// exist on the new one, so nothing is read and the signers are cleared.
    private func listingReplaced() {
        loader.fail("The connection changed since this action was listed. Go back and list the actions again.")
    }
}
