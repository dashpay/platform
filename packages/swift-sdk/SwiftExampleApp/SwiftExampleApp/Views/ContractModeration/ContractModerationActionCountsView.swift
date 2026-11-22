import SwiftUI
import SwiftDashSDK

/// How many counted moderation actions each member of an elected contract's
/// seated team signed since the moderators pot was last paid out.
struct ContractModerationActionCountsView: View {
    @EnvironmentObject var appState: AppState
    let contractId: String

    @State private var load: ModerationLoad<ContractModerationActionCounts?> = .idle

    var body: some View {
        List {
            Section {
                content
            } header: {
                Text("Counts Since the Last Payout")
            } footer: {
                Text("Bans, suspensions, warnings and document deletions. A member that did not act since has no count.")
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
                    text: "No member acted since the last payout.",
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
            load = .failed("SDK not initialized")
            return
        }
        load = .loading
        do {
            load = .loaded(try await sdk.contractModerationActionCounts(contractId: contractId))
        } catch {
            load = .failed(moderationErrorText(error))
        }
    }
}
