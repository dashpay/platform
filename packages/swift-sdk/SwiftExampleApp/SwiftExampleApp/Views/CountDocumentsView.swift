import SwiftUI
import SwiftData
import SwiftDashSDK

/// READ-only view that drives the document COUNT aggregation FFI
/// (`dash_sdk_document_count` via `SDK.documentCount`). Covers QA tests
/// DOC-10 (count total), DOC-11 (count filtered by `where`), and DOC-12
/// (count grouped by `group_by`).
///
/// This is a query view, not a state-transition builder — nothing is
/// signed or broadcast. It picks a loaded contract + document type using
/// the same accessible navigationLink pickers the Document builders use,
/// optionally takes `where` / `group_by` JSON, calls the wrapper, and
/// renders the total (and per-group counts when grouped) or the platform
/// error (e.g. "requires a countable index").
struct CountDocumentsView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext

    @Query private var contracts: [PersistentDataContract]

    @State private var selectedContract: PersistentDataContract?
    @State private var selectedDocumentTypeName = ""
    @State private var whereJSON = ""
    @State private var groupByJSON = ""

    @State private var isRunning = false
    /// Set once a run completes (success or failure) so the result
    /// section appears.
    @State private var didRun = false
    @State private var result: DocumentCountResult?
    @State private var errorMessage: String?

    var body: some View {
        Form {
            selectionSection
            filterSection
            runSection
            if didRun {
                resultSection
            }
        }
        .navigationTitle("Count Documents")
        .navigationBarTitleDisplayMode(.inline)
        .onChange(of: selectedContract) { _, _ in
            // A new contract may not have the previously-selected type —
            // clear so the picker isn't stale, and drop any prior result.
            selectedDocumentTypeName = ""
            resetResult()
        }
        .onChange(of: selectedDocumentTypeName) { _, _ in
            resetResult()
        }
    }

    // MARK: - Sections

    private var selectionSection: some View {
        Section("Document") {
            Picker("Contract", selection: $selectedContract) {
                Text("Select a contract").tag(nil as PersistentDataContract?)
                ForEach(activeContracts) { contract in
                    Text(contract.name)
                        .tag(contract as PersistentDataContract?)
                        .accessibilityIdentifier("countDocuments.contract.\(contract.idBase58)")
                }
            }
            .accessibleFormPicker("countDocuments.contractPicker")
            .disabled(isRunning)

            if let contract = selectedContract {
                Picker("Document Type", selection: $selectedDocumentTypeName) {
                    Text("Select type").tag("")
                    ForEach(documentTypeNames(for: contract), id: \.self) { type in
                        Text(type)
                            .tag(type)
                            .accessibilityIdentifier("countDocuments.docType.\(type)")
                    }
                }
                .accessibleFormPicker("countDocuments.docTypePicker")
                .disabled(isRunning)
            }
        }
    }

    private var filterSection: some View {
        Section {
            TextField("[{\"field\":\"...\",\"operator\":\"==\",\"value\":...}]", text: $whereJSON)
                .textInputAutocapitalization(.never)
                .disableAutocorrection(true)
                .font(.system(.footnote, design: .monospaced))
                .accessibilityIdentifier("countDocuments.whereField")
                .disabled(isRunning)

            TextField("[\"field1\",\"field2\"]", text: $groupByJSON)
                .textInputAutocapitalization(.never)
                .disableAutocorrection(true)
                .font(.system(.footnote, design: .monospaced))
                .accessibilityIdentifier("countDocuments.groupByField")
                .disabled(isRunning)
        } header: {
            Text("Filters (optional)")
        } footer: {
            Text("`where` is a JSON array of [{field, operator, value}]. `group_by` is a JSON array of field names. Leave blank for an unfiltered total count. Counting requires a countable index on the document type.")
        }
    }

    private var runSection: some View {
        Section {
            Button(action: runCount) {
                HStack {
                    if isRunning {
                        ProgressView()
                            .progressViewStyle(.circular)
                    } else {
                        Image(systemName: "number")
                    }
                    Text(isRunning ? "Counting…" : "Run Count")
                        .fontWeight(.semibold)
                }
                .frame(maxWidth: .infinity)
            }
            .disabled(!canRun)
            .accessibilityIdentifier("countDocuments.runButton")
        }
    }

    @ViewBuilder
    private var resultSection: some View {
        if let errorMessage = errorMessage {
            Section("Error") {
                Text(errorMessage)
                    .foregroundColor(.red)
                    .font(.callout)
                    .textSelection(.enabled)
                    .accessibilityIdentifier("countDocuments.errorText")
            }
        } else if let result = result {
            Section("Total") {
                HStack {
                    Text("Count")
                    Spacer()
                    Text(result.total.map(String.init) ?? "—")
                        .fontWeight(.bold)
                        .foregroundColor(.primary)
                        .accessibilityIdentifier("countDocuments.totalCount")
                }
            }

            if result.isGrouped {
                Section("Per-group counts") {
                    ForEach(groupedRows(result), id: \.key) { row in
                        HStack {
                            Text(row.key)
                                .font(.system(.footnote, design: .monospaced))
                                .lineLimit(1)
                                .truncationMode(.middle)
                            Spacer()
                            Text(String(row.value))
                                .fontWeight(.semibold)
                        }
                        .accessibilityIdentifier("countDocuments.groupRow.\(row.key)")
                    }
                }
            }
        }
    }

    // MARK: - Derived state

    /// Contracts limited to the active network — counting against another
    /// network's contract would hit the wrong SDK network.
    private var activeContracts: [PersistentDataContract] {
        contracts.filter { $0.network == appState.currentNetwork }
    }

    private var canRun: Bool {
        selectedContract != nil
            && !selectedDocumentTypeName.isEmpty
            && !isRunning
    }

    private func documentTypeNames(for contract: PersistentDataContract) -> [String] {
        if let types = contract.documentTypes, !types.isEmpty {
            return types.map { $0.name }.sorted()
        }
        return contract.documentTypesList.sorted()
    }

    /// Per-group rows, sorted by hex key for a stable render. Excludes the
    /// empty-string aggregate entry (shown in the Total section).
    private func groupedRows(_ result: DocumentCountResult) -> [(key: String, value: UInt64)] {
        result.counts
            .filter { !$0.key.isEmpty }
            .map { (key: $0.key, value: $0.value) }
            .sorted { $0.key < $1.key }
    }

    // MARK: - Actions

    private func resetResult() {
        didRun = false
        result = nil
        errorMessage = nil
    }

    private func runCount() {
        guard let contract = selectedContract,
              !selectedDocumentTypeName.isEmpty,
              let sdk = appState.sdk else {
            errorMessage = "SDK not initialized or no contract selected"
            didRun = true
            return
        }

        let contractId = contract.idBase58
        let documentType = selectedDocumentTypeName
        // Trim blanks → nil so empty fields mean "none" (null at the FFI).
        let whereArg = trimmedOrNil(whereJSON)
        let groupByArg = trimmedOrNil(groupByJSON)

        isRunning = true
        errorMessage = nil
        result = nil

        Task {
            do {
                let counted = try await sdk.documentCount(
                    dataContractId: contractId,
                    documentType: documentType,
                    whereJSON: whereArg,
                    orderByJSON: nil,
                    groupByJSON: groupByArg,
                    limit: -1
                )
                await MainActor.run {
                    self.result = counted
                    self.didRun = true
                    self.isRunning = false
                }
            } catch {
                await MainActor.run {
                    self.errorMessage = error.localizedDescription
                    self.didRun = true
                    self.isRunning = false
                }
            }
        }
    }

    private func trimmedOrNil(_ s: String) -> String? {
        let trimmed = s.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}
