import SwiftUI
import SwiftDashSDK

/// The removal records one document type of a contract keeps: a page at a
/// time, or the records of the document ids typed in. Whether the type
/// keeps records is the node's to answer.
struct ContractDocumentRemovalsView: View {
    @EnvironmentObject var appState: AppState
    /// The contract and the connection it was opened on: this screen reads
    /// only through that connection, and the hub hides it once replaced.
    let scope: ModerationScope
    private var contractId: String { scope.contractId }
    let documentTypeName: String

    /// Records per page.
    private static let pageSize: UInt32 = 20

    @State private var documentIdsInput = ""
    /// The filter shown (the ids looked up, `nil` while paging) and the token
    /// of the read last started for it.
    @State private var filter = RemovalsLookupFilter()
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
                    // The ids are taken now, at the tap: an edit made before the
                    // task runs cannot change what this look up sends
                    submit(ids: parsedIds)
                }
                .disabled(parsedIds.isEmpty)
                .accessibilityIdentifier("contractModeration.removals.lookup")
                if lookedUpIds != nil || !loader.loadedOnce {
                    Button("Show Every Record") {
                        submit(ids: nil)
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
        .task { submit(ids: nil) }
        // A replaced SDK: drop what was read through the old one and read
        // nothing through the new one; the hub's gate shows the notice instead
        .onChange(of: appState.sdkGeneration) { _, _ in loader.clear() }
    }

    private var parsedIds: [String] {
        documentIdsInput
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
    }

    /// The ids typed changed: the lookup shown, and one still running, no
    /// longer answer them, so they are cleared until the next look up.
    /// Typed ids matter only to a look up by ids: while every record is read
    /// (`nil` filter), an edit leaves that read alone.
    private func draftChanged() {
        guard filter.draftChanged() else { return }
        loader.clear()
    }

    /// The ids the shown records were looked up by; `nil` while paging.
    private var lookedUpIds: [String]? { filter.ids }

    /// Starts a read of `ids` (`nil` for every record, a page at a time), all
    /// in this main-actor turn: the filter shown changes, the submission
    /// token advances so a read queued earlier is not sent, and the loader
    /// drops the rows, cursor and error shown and anything still in flight.
    /// Only then is the read queued.
    private func submit(ids: [String]?) {
        let token = begin(ids: ids)
        let connection = appState.moderationConnection(in: scope)
        Task { await send(ids: ids, token: token, connection: connection) }
    }

    /// The synchronous part of a read of `ids`; returns its token.
    private func begin(ids: [String]?) -> Int {
        let token = filter.begin(ids: ids)
        loader.begin()
        return token
    }

    /// Sends the read of `ids` unless a newer read or an edit superseded it
    /// before it ran.
    /// The read goes out on `connection`, the SDK current when it began.
    private func send(ids: [String]?, token: Int, connection: ModerationConnection?) async {
        guard filter.isCurrent(token) else { return }
        guard let connection, appState.isCurrent(connection) else {
            loader.fail("SDK not initialized")
            return
        }
        if let ids {
            await read(ids: ids, on: connection)
        } else {
            await reload(on: connection)
        }
    }

    /// Read again what is shown: the ids looked up, or the pages.
    private func refresh() async {
        let ids = lookedUpIds
        let token = begin(ids: ids)
        await send(ids: ids, token: token, connection: appState.moderationConnection(in: scope))
    }

    /// Every record, a page at a time.
    private func reload(on connection: ModerationConnection) async {
        let sdk = connection.sdk
        let appState = appState
        let contractId = contractId
        let documentTypeName = documentTypeName
        let pageSize = Self.pageSize
        await loader.reload({ startAfter in
            try await sdk.contractDocumentRemovals(
                contractId: contractId,
                documentTypeName: documentTypeName,
                startAfter: startAfter,
                limit: pageSize
            )
            .map { ModerationPage(items: $0.removals, nextCursor: $0.nextStartAfter) }
        }, connectionIsCurrent: { appState.isCurrent(connection) })
    }

    /// The records of `ids` alone; a read by ids has no page after it.
    private func read(ids: [String], on connection: ModerationConnection) async {
        let sdk = connection.sdk
        let appState = appState
        let contractId = contractId
        let documentTypeName = documentTypeName
        await loader.reload({ _ in
            try await sdk.contractDocumentRemovals(
                contractId: contractId,
                documentTypeName: documentTypeName,
                documentIds: ids
            )
            .map { ModerationPage(items: $0.removals, nextCursor: $0.nextStartAfter) }
        }, connectionIsCurrent: { appState.isCurrent(connection) })
    }
}

/// One kept field: its path above its value, the value wrapped in full so
/// every member of a long object or array can be read and selected.
private struct KeptFieldRow: View {
    let path: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(path)
                .font(.caption)
                .foregroundColor(.secondary)
            Text(value)
                .font(.system(.caption, design: .monospaced))
                .fixedSize(horizontal: false, vertical: true)
                .textSelection(.enabled)
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
                KeptFieldRow(path: path, value: removal.keptFields[path]?.displayText ?? "")
            }
            if !removal.keptFields.isEmpty {
                Text("Field names follow the contract's current layout.")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            ModerationFieldRow(label: "Hash", value: removal.documentHash, monospaced: true)
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }
}

/// The removal records filter: the ids looked up (`nil` while every record
/// is read a page at a time) and the token of the read last started, so a
/// read queued before a newer one, or before an edit of the ids it looks up,
/// is not sent.
struct RemovalsLookupFilter: Equatable {
    private(set) var ids: [String]?
    private(set) var token = 0

    /// Starts a read of `ids`: shows them and returns the read's token.
    mutating func begin(ids: [String]?) -> Int {
        self.ids = ids
        token += 1
        return token
    }

    /// The typed ids changed. A look up by ids no longer answers them: it is
    /// dropped (`true`, the rows are to be cleared). Reading every record does
    /// not depend on them and goes on (`false`).
    mutating func draftChanged() -> Bool {
        guard ids != nil else { return false }
        ids = nil
        token += 1
        return true
    }

    /// Whether the read of `token` is still the one to send and show.
    func isCurrent(_ token: Int) -> Bool {
        token == self.token
    }
}
