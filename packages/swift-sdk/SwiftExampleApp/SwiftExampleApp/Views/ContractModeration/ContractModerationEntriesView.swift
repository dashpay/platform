import SwiftUI
import SwiftDashSDK

/// One moderation list of a contract, a page at a time, in identity id
/// order. "Load More" passes the cursor the last page returned; a page
/// without one was the last.
struct ContractModerationEntriesView: View {
    @EnvironmentObject var appState: AppState
    @EnvironmentObject var offers: ContractModerationOfferStore
    /// The contract and the connection it was opened on: this screen reads
    /// only through that connection, and the hub hides it once replaced.
    let scope: ModerationScope
    private var contractId: String { scope.contractId }
    let list: ContractModerationList

    /// Entries per page.
    private static let pageSize: UInt32 = 20

    @StateObject private var loader = ModerationPagedLoader<ContractModerationEntry, String>()
    /// The identity a moderation sheet is open for.
    @State private var moderating: ModerationTarget?

    var body: some View {
        List {
            if let errorMessage = loader.errorMessage {
                ModerationMessageRow(
                    kind: .error,
                    text: errorMessage,
                    identifier: "contractModeration.entries.error"
                )
            }

            Section(list.title) {
                if loader.items.isEmpty {
                    if loader.isLoading {
                        ModerationMessageRow(
                            kind: .loading,
                            text: "Loading entries…",
                            identifier: "contractModeration.entries.loading"
                        )
                    } else if loader.loadedOnce && loader.errorMessage == nil {
                        ModerationMessageRow(
                            kind: .empty,
                            text: "Nobody is on this list.",
                            identifier: "contractModeration.entries.empty"
                        )
                    }
                }
                ForEach(loader.items) { entry in
                    ContractModerationEntryRow(entry: entry)
                    // Only for an identity of a loaded wallet Rust named a
                    // moderator of this contract's lists
                    if !offers.userModerators.isEmpty {
                        Button("Moderate…") {
                            moderating = ModerationTarget(identityId: entry.identityId)
                        }
                        .font(.caption)
                        .accessibilityIdentifier("contractModeration.entries.moderate.\(entry.identityId)")
                    }
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
                    .accessibilityIdentifier("contractModeration.entries.loadMore")
                }
            }
        }
        .navigationTitle(list.title)
        .navigationBarTitleDisplayMode(.inline)
        .refreshable { await reload() }
        .task { await reload() }
        .sheet(item: $moderating) { target in
            ContractUserModerationSheet(scope: scope, target: target) {
                Task { await reload() }
            }
            .environmentObject(offers)
        }
        // A replaced SDK: drop what was read through the old one and read
        // nothing through the new one; the hub's gate shows the notice instead
        .onChange(of: appState.sdkGeneration) { _, _ in loader.clear() }
    }

    private func reload() async {
        guard let connection = appState.moderationConnection(in: scope) else {
            loader.fail("SDK not initialized")
            return
        }
        let sdk = connection.sdk
        let appState = appState
        let contractId = contractId
        let list = list
        let pageSize = Self.pageSize
        await loader.reload({ startAfter in
            try await sdk.contractModerationEntries(
                contractId: contractId,
                list: list,
                startAfter: startAfter,
                limit: pageSize
            )
            .map { ModerationPage(items: $0.entries, nextCursor: $0.nextStartAfter) }
        }, connectionIsCurrent: { appState.isCurrent(connection) })
    }
}

/// One entry: the identity, why it is on the list, and the end of a
/// suspension or, for a warning list entry, every warning it carries.
private struct ContractModerationEntryRow: View {
    let entry: ContractModerationEntry

    private var identifier: String { "contractModeration.entries.row.\(entry.identityId)" }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(entry.identityId)
                .font(.system(.caption, design: .monospaced))
                .lineLimit(1)
                .truncationMode(.middle)
                .textSelection(.enabled)
                .accessibilityIdentifier(identifier)
            if let untilMs = entry.untilMs {
                ModerationFieldRow(label: "Suspended until", value: moderationDate(ms: untilMs))
            }
            if let warnings = entry.warnings, !warnings.isEmpty {
                ModerationWarningsView(warnings: warnings, identifierPrefix: identifier)
            } else {
                ModerationReasonView(reason: entry.reason)
            }
        }
        .padding(.vertical, 2)
    }
}
