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
    @StateObject private var loader = ModerationPagedLoader<ContractDocumentRemovalEntry, String>()

    var body: some View {
        List {
            Section {
                TextField("Document IDs, comma-separated", text: $documentIdsInput)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .font(.system(.body, design: .monospaced))
                    .accessibilityIdentifier("contractModeration.removals.documentIds")
                    .onChange(of: documentIdsInput) { _, _ in draftChanged() }
                Button("Look Up These Documents") {
                    Task { await lookUp() }
                }
                .disabled(parsedIds.isEmpty)
                .accessibilityIdentifier("contractModeration.removals.lookup")
                if lookedUpIds != nil || !loader.loadedOnce {
                    Button("Show Every Record") {
                        Task { await reload() }
                    }
                    .accessibilityIdentifier("contractModeration.removals.showAll")
                }
            } header: {
                Text("Look Up")
            } footer: {
                Text("A document with no record is left out of the answer.")
            }

            Section(lookedUpIds == nil ? "Records" : "Records of the Documents Named") {
                if let lookedUpIds {
                    ModerationFieldRow(
                        label: "Looked up",
                        value: lookedUpIds.joined(separator: ", "),
                        monospaced: true
                    )
                    .accessibilityIdentifier("contractModeration.removals.submittedIds")
                }
                if let errorMessage = loader.errorMessage {
                    ModerationMessageRow(
                        kind: .error,
                        text: errorMessage,
                        identifier: "contractModeration.removals.error"
                    )
                }
                if !loader.loadedOnce && !loader.isLoading && loader.errorMessage == nil {
                    ModerationMessageRow(
                        kind: .empty,
                        text: "Look up the ids typed above, or show every record.",
                        identifier: "contractModeration.removals.idle"
                    )
                }
                if loader.items.isEmpty {
                    if loader.isLoading {
                        ModerationMessageRow(
                            kind: .loading,
                            text: "Loading records…",
                            identifier: "contractModeration.removals.loading"
                        )
                    } else if loader.loadedOnce && loader.errorMessage == nil {
                        ModerationMessageRow(
                            kind: .empty,
                            text: "No records.",
                            identifier: "contractModeration.removals.empty"
                        )
                    }
                }
                ForEach(loader.items) { removal in
                    ContractDocumentRemovalRow(removal: removal)
                        .accessibilityIdentifier("contractModeration.removals.row.\(removal.documentId)")
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
                    .accessibilityIdentifier("contractModeration.removals.loadMore")
                }
            }
        }
        .navigationTitle(documentTypeName)
        .navigationBarTitleDisplayMode(.inline)
        .refreshable { await refresh() }
        .task { await reload() }
    }

    private var parsedIds: [String] {
        documentIdsInput
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
    }

    /// The ids typed changed: the lookup shown, and one still running, no
    /// longer answer them, so they are cleared until the next look up.
    private func draftChanged() {
        guard lookedUpIds != nil else { return }
        lookedUpIds = nil
        loader.clear()
    }

    /// Read again what is shown: the ids looked up, or the pages.
    private func refresh() async {
        if let lookedUpIds {
            await read(ids: lookedUpIds)
        } else {
            await reload()
        }
    }

    /// Every record, a page at a time.
    private func reload() async {
        lookedUpIds = nil
        guard let sdk = appState.sdk else {
            loader.fail("SDK not initialized")
            return
        }
        let contractId = contractId
        let documentTypeName = documentTypeName
        let pageSize = Self.pageSize
        await loader.reload { startAfter in
            try await sdk.contractDocumentRemovals(
                contractId: contractId,
                documentTypeName: documentTypeName,
                startAfter: startAfter,
                limit: pageSize
            )
            .map { ModerationPage(items: $0.removals, nextCursor: $0.nextStartAfter) }
        }
    }

    private func lookUp() async {
        await read(ids: parsedIds)
    }

    /// The records of `ids` alone; a read by ids has no page after it.
    private func read(ids: [String]) async {
        lookedUpIds = ids
        guard let sdk = appState.sdk else {
            loader.fail("SDK not initialized")
            return
        }
        let contractId = contractId
        let documentTypeName = documentTypeName
        await loader.reload { _ in
            try await sdk.contractDocumentRemovals(
                contractId: contractId,
                documentTypeName: documentTypeName,
                documentIds: ids
            )
            .map { ModerationPage(items: $0.removals, nextCursor: $0.nextStartAfter) }
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
