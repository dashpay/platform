import SwiftUI
import SwiftData
import SwiftDashSDK

/// READ-only view that drives the document SUM and AVERAGE aggregation FFI
/// (`dash_sdk_document_sum` / `dash_sdk_document_average` via `SDK.documentSum`
/// / `SDK.documentAverage`). Covers QA tests DOC-13 (sum of a numeric
/// property) and DOC-14 (average of a numeric property).
///
/// This is a query view, not a state-transition builder — nothing is signed or
/// broadcast. It picks a loaded contract + document type using the same
/// accessible navigationLink pickers the Document builders use, takes a
/// required numeric `sum property`, optional `where` / `group_by` JSON, calls
/// the wrapper for the chosen operation, and renders the total (and per-group
/// rows when grouped) or the platform error (e.g. "requires a summable index").
///
/// The average FFI returns the raw `(count, sum)` pair un-divided; computing
/// the displayed average (`sum / count`) is presentation and happens here, not
/// in the wrapper.
struct SumAverageDocumentsView: View {
    /// Which aggregation the view runs.
    private enum Operation: String, CaseIterable, Identifiable {
        case sum = "Sum"
        case average = "Average"

        var id: String { rawValue }
    }

    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext

    @Query private var contracts: [PersistentDataContract]

    @State private var selectedContract: PersistentDataContract?
    @State private var selectedDocumentTypeName = ""
    @State private var operation: Operation = .sum
    @State private var sumProperty = ""
    @State private var whereJSON = ""
    @State private var groupByJSON = ""

    @State private var isRunning = false
    /// Set once a run completes (success or failure) so the result section
    /// appears.
    @State private var didRun = false
    @State private var sumResult: DocumentSumResult?
    @State private var averageResult: DocumentAverageResult?
    @State private var errorMessage: String?

    var body: some View {
        Form {
            selectionSection
            operationSection
            filterSection
            runSection
            if didRun {
                resultSection
            }
        }
        .navigationTitle("Sum / Average Documents")
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
        .onChange(of: operation) { _, _ in
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
                        .accessibilityIdentifier("sumAverageDocuments.contract.\(contract.idBase58)")
                }
            }
            .accessibleFormPicker("sumAverageDocuments.contractPicker")
            .disabled(isRunning)

            if let contract = selectedContract {
                Picker("Document Type", selection: $selectedDocumentTypeName) {
                    Text("Select type").tag("")
                    ForEach(documentTypeNames(for: contract), id: \.self) { type in
                        Text(type)
                            .tag(type)
                            .accessibilityIdentifier("sumAverageDocuments.docType.\(type)")
                    }
                }
                .accessibleFormPicker("sumAverageDocuments.docTypePicker")
                .disabled(isRunning)
            }
        }
    }

    private var operationSection: some View {
        Section {
            Picker("Operation", selection: $operation) {
                ForEach(Operation.allCases) { op in
                    Text(op.rawValue).tag(op)
                }
            }
            .pickerStyle(.segmented)
            .accessibilityIdentifier("sumAverageDocuments.opPicker")
            .disabled(isRunning)

            TextField("Numeric property to aggregate (e.g. amount)", text: $sumProperty)
                .textInputAutocapitalization(.never)
                .disableAutocorrection(true)
                .font(.system(.footnote, design: .monospaced))
                .accessibilityIdentifier("sumAverageDocuments.sumPropertyField")
                .disabled(isRunning)
        } header: {
            Text("Aggregation")
        } footer: {
            Text("Pick \(Operation.sum.rawValue) or \(Operation.average.rawValue), then name the numeric property to aggregate. This property is required.")
        }
    }

    private var filterSection: some View {
        Section {
            TextField("[{\"field\":\"...\",\"operator\":\"==\",\"value\":...}]", text: $whereJSON)
                .textInputAutocapitalization(.never)
                .disableAutocorrection(true)
                .font(.system(.footnote, design: .monospaced))
                .accessibilityIdentifier("sumAverageDocuments.whereField")
                .disabled(isRunning)

            TextField("[\"field1\",\"field2\"]", text: $groupByJSON)
                .textInputAutocapitalization(.never)
                .disableAutocorrection(true)
                .font(.system(.footnote, design: .monospaced))
                .accessibilityIdentifier("sumAverageDocuments.groupByField")
                .disabled(isRunning)
        } header: {
            Text("Filters (optional)")
        } footer: {
            Text("`where` is a JSON array of [{field, operator, value}]. `group_by` is a JSON array of field names. Leave blank for an aggregate total. Aggregation requires a `summable` index on the numeric property of the document type.")
        }
    }

    private var runSection: some View {
        Section {
            Button(action: runAggregation) {
                HStack {
                    if isRunning {
                        ProgressView()
                            .progressViewStyle(.circular)
                    } else {
                        Image(systemName: "sum")
                    }
                    Text(isRunning ? "Running…" : "Run \(operation.rawValue)")
                        .fontWeight(.semibold)
                }
                .frame(maxWidth: .infinity)
            }
            .disabled(!canRun)
            .accessibilityIdentifier("sumAverageDocuments.runButton")
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
                    .accessibilityIdentifier("sumAverageDocuments.errorText")
            }
        } else if operation == .sum, let result = sumResult {
            sumResultSections(result)
        } else if operation == .average, let result = averageResult {
            averageResultSections(result)
        }
    }

    @ViewBuilder
    private func sumResultSections(_ result: DocumentSumResult) -> some View {
        Section("Total") {
            HStack {
                Text("Sum")
                Spacer()
                Text(result.total.map(String.init) ?? "—")
                    .fontWeight(.bold)
                    .foregroundColor(.primary)
                    .accessibilityIdentifier("sumAverageDocuments.total")
            }
        }

        if result.isGrouped {
            Section("Per-group sums") {
                ForEach(groupedSumRows(result), id: \.key) { row in
                    HStack {
                        Text(row.key)
                            .font(.system(.footnote, design: .monospaced))
                            .lineLimit(1)
                            .truncationMode(.middle)
                        Spacer()
                        Text(String(row.value))
                            .fontWeight(.semibold)
                    }
                    .accessibilityIdentifier("sumAverageDocuments.groupRow.\(row.key)")
                }
            }
        }
    }

    @ViewBuilder
    private func averageResultSections(_ result: DocumentAverageResult) -> some View {
        Section("Total") {
            HStack {
                Text("Average")
                Spacer()
                Text(formatAverage(result.total))
                    .fontWeight(.bold)
                    .foregroundColor(.primary)
                    .accessibilityIdentifier("sumAverageDocuments.average")
            }
            if let entry = result.total {
                HStack {
                    Text("Count")
                    Spacer()
                    Text(String(entry.count))
                        .foregroundColor(.secondary)
                }
                HStack {
                    Text("Sum")
                    Spacer()
                    Text(String(entry.sum))
                        .foregroundColor(.secondary)
                }
            }
        }

        if result.isGrouped {
            Section("Per-group averages") {
                ForEach(groupedAverageRows(result), id: \.key) { row in
                    HStack {
                        Text(row.key)
                            .font(.system(.footnote, design: .monospaced))
                            .lineLimit(1)
                            .truncationMode(.middle)
                        Spacer()
                        VStack(alignment: .trailing, spacing: 2) {
                            Text(formatAverage(row.entry))
                                .fontWeight(.semibold)
                            Text("n=\(row.entry.count), Σ=\(row.entry.sum)")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                        }
                    }
                    .accessibilityIdentifier("sumAverageDocuments.groupRow.\(row.key)")
                }
            }
        }
    }

    // MARK: - Derived state

    /// Contracts limited to the active network — aggregating against another
    /// network's contract would hit the wrong SDK network.
    private var activeContracts: [PersistentDataContract] {
        contracts.filter { $0.network == appState.currentNetwork }
    }

    private var canRun: Bool {
        selectedContract != nil
            && !selectedDocumentTypeName.isEmpty
            && !sumProperty.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !isRunning
    }

    private func documentTypeNames(for contract: PersistentDataContract) -> [String] {
        if let types = contract.documentTypes, !types.isEmpty {
            return types.map { $0.name }.sorted()
        }
        return contract.documentTypesList.sorted()
    }

    /// Per-group sum rows, sorted by hex key for a stable render. Excludes the
    /// empty-string aggregate entry (shown in the Total section).
    private func groupedSumRows(_ result: DocumentSumResult) -> [(key: String, value: Int64)] {
        result.sums
            .filter { !$0.key.isEmpty }
            .map { (key: $0.key, value: $0.value) }
            .sorted { $0.key < $1.key }
    }

    /// Per-group average rows, sorted by hex key for a stable render. Excludes
    /// the empty-string aggregate entry (shown in the Total section).
    private func groupedAverageRows(_ result: DocumentAverageResult) -> [(key: String, entry: DocumentAverageEntry)] {
        result.averages
            .filter { !$0.key.isEmpty }
            .map { (key: $0.key, entry: $0.value) }
            .sorted { $0.key < $1.key }
    }

    /// Render an average entry's computed `sum / count`. The FFI returns the
    /// raw `(count, sum)` pair; the division (presentation) happens here. A
    /// `nil` entry or a zero `count` (no matched documents) renders as `—`.
    private func formatAverage(_ entry: DocumentAverageEntry?) -> String {
        guard let avg = entry?.average else { return "—" }
        return String(format: "%.4f", avg)
    }

    // MARK: - Actions

    private func resetResult() {
        didRun = false
        sumResult = nil
        averageResult = nil
        errorMessage = nil
    }

    private func runAggregation() {
        let trimmedProperty = sumProperty.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let contract = selectedContract,
              !selectedDocumentTypeName.isEmpty,
              !trimmedProperty.isEmpty,
              let sdk = appState.sdk else {
            errorMessage = "SDK not initialized, no contract selected, or no property given"
            didRun = true
            return
        }

        let contractId = contract.idBase58
        let documentType = selectedDocumentTypeName
        let op = operation
        // Trim blanks → nil so empty fields mean "none" (null at the FFI).
        let whereArg = trimmedOrNil(whereJSON)
        let groupByArg = trimmedOrNil(groupByJSON)

        isRunning = true
        errorMessage = nil
        sumResult = nil
        averageResult = nil

        Task {
            do {
                switch op {
                case .sum:
                    let summed = try await sdk.documentSum(
                        dataContractId: contractId,
                        documentType: documentType,
                        sumProperty: trimmedProperty,
                        whereJSON: whereArg,
                        orderByJSON: nil,
                        groupByJSON: groupByArg,
                        limit: -1
                    )
                    await MainActor.run {
                        self.sumResult = summed
                        self.didRun = true
                        self.isRunning = false
                    }
                case .average:
                    let averaged = try await sdk.documentAverage(
                        dataContractId: contractId,
                        documentType: documentType,
                        sumProperty: trimmedProperty,
                        whereJSON: whereArg,
                        orderByJSON: nil,
                        groupByJSON: groupByArg,
                        limit: -1
                    )
                    await MainActor.run {
                        self.averageResult = averaged
                        self.didRun = true
                        self.isRunning = false
                    }
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
