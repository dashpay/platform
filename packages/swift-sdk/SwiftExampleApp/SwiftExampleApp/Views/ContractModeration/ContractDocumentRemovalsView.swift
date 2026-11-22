import SwiftUI
import SwiftDashSDK

/// The removal records one document type of a contract keeps: a page at a
/// time, or the records of the document ids typed in. Whether the type
/// keeps records is the node's to answer.
struct ContractDocumentRemovalsView: View {
    @EnvironmentObject var appState: AppState
    let contractId: String
    let documentTypeName: String

    /// Records per page.
    private static let pageSize: UInt32 = 20

    @State private var documentIdsInput = ""
    /// The ids the shown records were looked up by; `nil` while paging.
    @State private var lookedUpIds: [String]?
    @State private var removals: [ContractDocumentRemovalEntry] = []
    @State private var nextStartAfter: String?
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var loadedOnce = false

    var body: some View {
        List {
            Section {
                TextField("Document IDs, comma-separated", text: $documentIdsInput)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .font(.system(.body, design: .monospaced))
                    .accessibilityIdentifier("contractModeration.removals.documentIds")
                Button("Look Up These Documents") {
                    Task { await lookUp() }
                }
                .disabled(parsedIds.isEmpty || isLoading)
                .accessibilityIdentifier("contractModeration.removals.lookup")
                if lookedUpIds != nil {
                    Button("Show Every Record") {
                        Task { await reload() }
                    }
                    .disabled(isLoading)
                    .accessibilityIdentifier("contractModeration.removals.showAll")
                }
            } header: {
                Text("Look Up")
            } footer: {
                Text("A document with no record is left out of the answer.")
            }

            if let errorMessage {
                ModerationMessageRow(
                    kind: .error,
                    text: errorMessage,
                    identifier: "contractModeration.removals.error"
                )
            }

            Section(lookedUpIds == nil ? "Records" : "Records of the Documents Named") {
                if removals.isEmpty {
                    if isLoading {
                        ModerationMessageRow(
                            kind: .loading,
                            text: "Loading records…",
                            identifier: "contractModeration.removals.loading"
                        )
                    } else if loadedOnce && errorMessage == nil {
                        ModerationMessageRow(
                            kind: .empty,
                            text: "No records.",
                            identifier: "contractModeration.removals.empty"
                        )
                    }
                }
                ForEach(removals) { removal in
                    ContractDocumentRemovalRow(removal: removal)
                        .accessibilityIdentifier("contractModeration.removals.row.\(removal.documentId)")
                }
                if let nextStartAfter, !removals.isEmpty {
                    Button {
                        Task { await loadPage(startAfter: nextStartAfter) }
                    } label: {
                        HStack {
                            if isLoading { ProgressView().controlSize(.small) }
                            Text("Load More")
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .disabled(isLoading)
                    .accessibilityIdentifier("contractModeration.removals.loadMore")
                }
            }
        }
        .navigationTitle(documentTypeName)
        .navigationBarTitleDisplayMode(.inline)
        .refreshable { await reload() }
        .task { await reload() }
    }

    private var parsedIds: [String] {
        documentIdsInput
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
    }

    private func reload() async {
        lookedUpIds = nil
        removals = []
        nextStartAfter = nil
        await loadPage(startAfter: nil)
    }

    private func loadPage(startAfter: String?) async {
        await read {
            try await $0.contractDocumentRemovals(
                contractId: contractId,
                documentTypeName: documentTypeName,
                startAfter: startAfter,
                limit: Self.pageSize
            )
        }
    }

    private func lookUp() async {
        let ids = parsedIds
        lookedUpIds = ids
        removals = []
        nextStartAfter = nil
        await read {
            try await $0.contractDocumentRemovals(
                contractId: contractId,
                documentTypeName: documentTypeName,
                documentIds: ids
            )
        }
    }

    private func read(_ fetch: (SDK) async throws -> ContractDocumentRemovalsPage?) async {
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
            let page = try await fetch(sdk)
            removals.append(contentsOf: page?.removals ?? [])
            nextStartAfter = page?.nextStartAfter
        } catch {
            errorMessage = moderationErrorText(error)
        }
    }
}

/// One removal record: who removed what, why and when, whether it was
/// restored, and the values the record keeps.
private struct ContractDocumentRemovalRow: View {
    let removal: ContractDocumentRemovalEntry

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(removal.documentId)
                .font(.system(.caption, design: .monospaced))
                .lineLimit(1)
                .truncationMode(.middle)
                .textSelection(.enabled)
            ModerationFieldRow(label: "Owner", value: removal.documentOwnerId, monospaced: true)
            ModerationFieldRow(label: "Removed by", value: removal.moderatorId, monospaced: true)
            ModerationFieldRow(label: "Removed", value: moderationDate(ms: removal.removedAtMs))
            ModerationReasonView(reason: removal.reason)
            if let restoredBy = removal.restoredBy {
                ModerationFieldRow(label: "Restored by", value: restoredBy, monospaced: true)
            }
            if let restoredAtMs = removal.restoredAtMs {
                ModerationFieldRow(label: "Restored", value: moderationDate(ms: restoredAtMs))
            }
            ForEach(removal.keptFields.keys.sorted(), id: \.self) { path in
                ModerationFieldRow(label: path, value: removal.keptFields[path]?.displayText ?? "")
            }
            ModerationFieldRow(label: "Hash", value: removal.documentHash, monospaced: true)
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }
}
