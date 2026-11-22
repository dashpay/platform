import SwiftUI
import SwiftDashSDK

/// The two fee pots of a contract: the credits in each and its last claim.
struct ContractFeePotsSection: View {
    @EnvironmentObject var appState: AppState
    let contractId: String

    @State private var load: ModerationLoad<ContractFeePots?> = .idle

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
            Text("What the contract's document action fees collected. A pot is paid out at most once per epoch.")
        }
        .task(id: contractId) {
            await reload()
        }
    }

    private var isLoading: Bool {
        if case .loading = load { return true }
        return false
    }

    private func reload() async {
        guard let sdk = appState.sdk else {
            load = .failed("SDK not initialized")
            return
        }
        load = .loading
        do {
            load = .loaded(try await sdk.contractFeePots(contractId: contractId))
        } catch {
            load = .failed(moderationErrorText(error))
        }
    }
}

/// One pot: its credits and, once it was paid out, the last claim.
private struct FeePotRow: View {
    let title: String
    let pot: ContractFeePotState

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.headline)
            ModerationFieldRow(label: "Credits", value: "\(pot.credits)")
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
