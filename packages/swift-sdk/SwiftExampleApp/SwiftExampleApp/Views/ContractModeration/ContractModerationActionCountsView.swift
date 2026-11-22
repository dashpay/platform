import SwiftUI
import SwiftDashSDK

/// How many counted moderation actions each member of an elected contract's
/// seated team signed since the moderators pot was last claimed or settled.
struct ContractModerationActionCountsView: View {
    @EnvironmentObject var appState: AppState
    let contractId: String

    @StateObject private var loader = ModerationSingleLoader<ContractModerationActionCounts?>()
    private var load: ModerationLoad<ContractModerationActionCounts?> { loader.load }

    var body: some View {
        List {
            Section {
                content
            } header: {
                Text("Counts Since the Last Claim or Settlement")
            } footer: {
                Text("Bans, suspensions, warnings and document deletions since the moderators pot was last claimed or settled. A settlement (a membership change of the team) resets the counts even when it pays nothing out. A member that did not act since has no count.")
            }
            Section {
                Button("Reload") {
                    Task { await reload() }
                }
                .disabled(isLoading)
                .accessibilityIdentifier("contractModeration.actionCounts.reload")
            }
        }
        .navigationTitle("Action Counts")
        .navigationBarTitleDisplayMode(.inline)
        .refreshable { await reload() }
        .task { await reload() }
    }

    @ViewBuilder
    private var content: some View {
        switch load {
        case .idle, .loading:
            ModerationMessageRow(
                kind: .loading,
                text: "Loading counts…",
                identifier: "contractModeration.actionCounts.loading"
            )
        case .failed(let message):
            ModerationMessageRow(
                kind: .error,
                text: message,
                identifier: "contractModeration.actionCounts.error"
            )
        case .loaded(let answer):
            let counts = answer?.counts ?? []
            if counts.isEmpty {
                ModerationMessageRow(
                    kind: .empty,
                    text: "No member acted since the last claim or settlement.",
                    identifier: "contractModeration.actionCounts.empty"
                )
            }
            ForEach(counts) { count in
                ModerationFieldRow(label: count.identityId, value: "\(count.count)", monospaced: true)
                    .accessibilityElement(children: .combine)
                    .accessibilityIdentifier("contractModeration.actionCounts.row.\(count.identityId)")
            }
        }
    }

    private var isLoading: Bool {
        if case .loading = load { return true }
        return false
    }

    private func reload() async {
        guard let sdk = appState.sdk else {
            loader.fail("SDK not initialized")
            return
        }
        let contractId = contractId
        await loader.reload { try await sdk.contractModerationActionCounts(contractId: contractId) }
    }
}
