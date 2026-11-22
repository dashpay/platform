import SwiftUI
import SwiftDashSDK

/// The two fee pots of a contract: the balance of each and its last explicit
/// claim.
struct ContractFeePotsSection: View {
    @EnvironmentObject var appState: AppState
    /// The contract and the connection it was opened on: this screen reads
    /// only through that connection, and the hub hides it once replaced.
    let scope: ModerationScope
    private var contractId: String { scope.contractId }

    @StateObject private var loader = ModerationSingleLoader<ContractFeePots?>()
    private var load: ModerationLoad<ContractFeePots?> { loader.load }

    var body: some View {
        Section {
            switch load {
            case .idle, .loading:
                ModerationMessageRow(
                    kind: .loading,
                    text: "Loading fee pots…",
                    identifier: "contractModeration.feePots.loading"
                )
            case .failed(let message):
                ModerationMessageRow(
                    kind: .error,
                    text: message,
                    identifier: "contractModeration.feePots.error"
                )
            case .loaded(nil):
                ModerationMessageRow(
                    kind: .empty,
                    text: "No answer for this contract.",
                    identifier: "contractModeration.feePots.empty"
                )
            case .loaded(let pots?):
                FeePotRow(title: "Owner Pot", pot: pots.owner)
                    .accessibilityIdentifier("contractModeration.feePots.owner")
                FeePotRow(title: "Moderators Pot", pot: pots.moderators)
                    .accessibilityIdentifier("contractModeration.feePots.moderators")
            }
            Button("Reload") {
                Task { await reload() }
            }
            .disabled(isLoading)
            .accessibilityIdentifier("contractModeration.feePots.reload")
        } header: {
            Text("Fee Pots")
        } footer: {
            Text("Each pot's balance from the contract's document action fees. The last claim is the last explicit claim, at most one per epoch; a membership change of an elected team can also pay out the moderators pot without changing it.")
        }
        .task(id: contractId) {
            await reload()
        }
        // A replaced SDK: drop what was read through the old one and read
        // nothing through the new one; the hub's gate shows the notice instead
        .onChange(of: appState.sdkGeneration) { _, _ in loader.clear() }
    }

    private var isLoading: Bool {
        if case .loading = load { return true }
        return false
    }

    private func reload() async {
        guard let connection = appState.moderationConnection(in: scope) else {
            loader.fail("SDK not initialized")
            return
        }
        let sdk = connection.sdk
        let appState = appState
        let contractId = contractId
        await loader.reload(
            { try await sdk.contractFeePots(contractId: contractId) },
            connectionIsCurrent: { appState.isCurrent(connection) }
        )
    }
}

/// One pot: its balance and, once it was claimed, the last explicit claim.
private struct FeePotRow: View {
    let title: String
    let pot: ContractFeePotState

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.headline)
            ModerationFieldRow(label: "Balance (credits)", value: "\(pot.credits)")
            if let epoch = pot.lastClaimEpoch {
                ModerationFieldRow(label: "Last claim epoch", value: "\(epoch)")
                if let timeMs = pot.lastClaimTimeMs {
                    ModerationFieldRow(label: "Last claim time", value: moderationDate(ms: timeMs))
                }
                if let claimant = pot.lastClaimantId {
                    ModerationFieldRow(label: "Last claimant", value: claimant, monospaced: true)
                }
            } else {
                Text("Never claimed")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }
}
