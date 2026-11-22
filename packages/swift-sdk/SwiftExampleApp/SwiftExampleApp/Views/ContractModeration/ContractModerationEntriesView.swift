import SwiftUI
import SwiftDashSDK

/// One moderation list of a contract, a page at a time, in identity id
/// order. "Load More" passes the cursor the last page returned; a page
/// without one was the last.
struct ContractModerationEntriesView: View {
    @EnvironmentObject var appState: AppState
    let contractId: String
    let list: ContractModerationList

    /// Entries per page.
    private static let pageSize: UInt32 = 20

    @State private var entries: [ContractModerationEntry] = []
    @State private var nextStartAfter: String?
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var loadedOnce = false

    var body: some View {
        List {
            if let errorMessage {
                ModerationMessageRow(
                    kind: .error,
                    text: errorMessage,
                    identifier: "contractModeration.entries.error"
                )
            }

            Section(list.title) {
                if entries.isEmpty {
                    if isLoading {
                        ModerationMessageRow(
                            kind: .loading,
                            text: "Loading entries…",
                            identifier: "contractModeration.entries.loading"
                        )
                    } else if loadedOnce && errorMessage == nil {
                        ModerationMessageRow(
                            kind: .empty,
                            text: "Nobody is on this list.",
                            identifier: "contractModeration.entries.empty"
                        )
                    }
                }
                ForEach(entries) { entry in
                    ContractModerationEntryRow(entry: entry)
                        .accessibilityIdentifier("contractModeration.entries.row.\(entry.identityId)")
                }
                if let nextStartAfter, !entries.isEmpty {
                    Button {
                        Task { await load(startAfter: nextStartAfter) }
                    } label: {
                        HStack {
                            if isLoading { ProgressView().controlSize(.small) }
                            Text("Load More")
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .disabled(isLoading)
                    .accessibilityIdentifier("contractModeration.entries.loadMore")
                }
            }
        }
        .navigationTitle(list.title)
        .navigationBarTitleDisplayMode(.inline)
        .refreshable { await reload() }
        .task { await reload() }
    }

    private func reload() async {
        entries = []
        nextStartAfter = nil
        await load(startAfter: nil)
    }

    private func load(startAfter: String?) async {
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
        do {
            let page = try await sdk.contractModerationEntries(
                contractId: contractId,
                list: list,
                startAfter: startAfter,
                limit: Self.pageSize
            )
            entries.append(contentsOf: page?.entries ?? [])
            nextStartAfter = page?.nextStartAfter
        } catch {
            errorMessage = moderationErrorText(error)
        }
    }
}

/// One entry: the identity, why it is on the list, and the end of a
/// suspension or the warnings it carries.
private struct ContractModerationEntryRow: View {
    let entry: ContractModerationEntry

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(entry.identityId)
                .font(.system(.caption, design: .monospaced))
                .lineLimit(1)
                .truncationMode(.middle)
                .textSelection(.enabled)
            if let untilMs = entry.untilMs {
                ModerationFieldRow(label: "Suspended until", value: moderationDate(ms: untilMs))
            }
            ModerationReasonView(reason: entry.reason)
            if let warnings = entry.warnings, warnings.count > 1 {
                Text("\(warnings.count) warnings")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }
}
