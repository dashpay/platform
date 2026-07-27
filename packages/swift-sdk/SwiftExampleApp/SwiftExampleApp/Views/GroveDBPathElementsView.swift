import SwiftUI
import SwiftDashSDK

/// READ-only diagnostic view that drives the raw GroveDB path-elements FFI
/// (`dash_sdk_system_get_path_elements` via `SDK.systemPathElements`). Covers
/// QA test SYS-06.
///
/// This is a low-level query view, not a state-transition builder — nothing
/// is signed or broadcast. It takes a `path` (JSON array of strings) and a
/// `keys` (JSON array of strings), passes them to the wrapper, and renders
/// the returned elements (`type` / `key` / `element`) or the decode / FFI
/// error.
///
/// The "DPNS contract example" preset fills the fields with a bounded query
/// (the DPNS system contract under the `DataContractDocuments` root) so the
/// query can be run without typing JSON (the simulator's smart-punctuation
/// mangles typed quotes).
struct GroveDBPathElementsView: View {
    @EnvironmentObject var appState: AppState

    /// JSON array string of GroveDB path segments, e.g. `["40"]` or `["60"]`.
    @State private var pathText = ""
    /// JSON array string of keys to fetch within the path.
    @State private var keysText = ""

    @State private var isRunning = false
    /// Set once a run completes (success or failure) so the result section
    /// appears.
    @State private var didRun = false
    @State private var elements: [PathElement] = []
    @State private var errorMessage: String?

    var body: some View {
        Form {
            inputSection
            runSection
            if didRun {
                resultSection
            }
        }
        .navigationTitle("GroveDB Path Elements")
        .navigationBarTitleDisplayMode(.inline)
    }

    // MARK: - Sections

    private var inputSection: some View {
        Section {
            TextField("[] or [\"60\"]", text: $pathText)
                .textInputAutocapitalization(.never)
                .disableAutocorrection(true)
                .font(.system(.footnote, design: .monospaced))
                .accessibilityLabel("Path JSON array")
                .accessibilityIdentifier("sysPathElements.pathField")
                .disabled(isRunning)

            TextField("[\"20\",\"40\",\"10\",\"60\"]", text: $keysText)
                .textInputAutocapitalization(.never)
                .disableAutocorrection(true)
                .font(.system(.footnote, design: .monospaced))
                .accessibilityLabel("Keys JSON array")
                .accessibilityIdentifier("sysPathElements.keysField")
                .disabled(isRunning)

            Button(action: applyExamplePreset) {
                Label("DPNS contract example", systemImage: "tray.2")
            }
            .accessibilityLabel("Fill DPNS contract example preset")
            .accessibilityIdentifier("sysPathElements.presetButton")
            .disabled(isRunning)
        } header: {
            Text("Query")
        } footer: {
            Text("`path` and `keys` are JSON arrays of strings. Each string is hex bytes (preferred) or plain text. \"DPNS contract example\" fills path = [\"40\"] (the DataContractDocuments root, byte 0x40) and keys = [the DPNS system contract id], returning that contract's GroveDB subtree. Use a bounded (non-empty) path: root-level queries (path = []) currently fail GroveDB proof verification.")
        }
    }

    private var runSection: some View {
        Section {
            Button(action: runQuery) {
                HStack {
                    if isRunning {
                        ProgressView()
                            .progressViewStyle(.circular)
                    } else {
                        Image(systemName: "magnifyingglass")
                    }
                    Text(isRunning ? "Running…" : "Run")
                        .fontWeight(.semibold)
                }
                .frame(maxWidth: .infinity)
            }
            .disabled(isRunning || appState.sdk == nil)
            .accessibilityIdentifier("sysPathElements.runButton")
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
                    .accessibilityIdentifier("sysPathElements.errorText")
            }
        } else {
            Section("Elements (\(elements.count))") {
                if elements.isEmpty {
                    Text("No elements found")
                        .foregroundColor(.secondary)
                        .accessibilityIdentifier("sysPathElements.emptyText")
                } else {
                    ForEach(elements, id: \.key) { element in
                        VStack(alignment: .leading, spacing: 4) {
                            Text(element.type)
                                .font(.subheadline)
                                .fontWeight(.semibold)
                                .foregroundColor(.blue)
                            Text(element.key)
                                .font(.system(.footnote, design: .monospaced))
                                .lineLimit(1)
                                .truncationMode(.middle)
                                .textSelection(.enabled)
                            Text(element.element)
                                .font(.system(.caption, design: .monospaced))
                                .foregroundColor(.secondary)
                                .textSelection(.enabled)
                        }
                        .padding(.vertical, 2)
                        .accessibilityElement(children: .combine)
                        .accessibilityIdentifier("sysPathElements.resultRow.\(element.key)")
                    }
                }
            }
        }
    }

    // MARK: - Actions

    /// Fill the fields with a bounded example query: the DPNS system data
    /// contract under the `DataContractDocuments` root (RootTree byte `0x40`).
    /// Sets the @State strings directly so no JSON has to be typed (the
    /// simulator's smart-punctuation mangles typed quotes). A *bounded* path
    /// is used on purpose — empty-path (root-level) queries currently fail
    /// GroveDB proof verification ("Cannot verify lower bound").
    private func applyExamplePreset() {
        pathText = "[\"40\"]"
        keysText = "[\"e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c53155\"]"
        resetResult()
    }

    private func resetResult() {
        didRun = false
        elements = []
        errorMessage = nil
    }

    private func runQuery() {
        guard let sdk = appState.sdk else {
            errorMessage = "SDK not initialized"
            didRun = true
            return
        }

        // Decode the two text fields from JSON arrays of strings.
        let path: [String]
        let keys: [String]
        do {
            path = try decodeStringArray(pathText, field: "path")
            keys = try decodeStringArray(keysText, field: "keys")
        } catch {
            errorMessage = error.localizedDescription
            didRun = true
            return
        }

        isRunning = true
        errorMessage = nil
        elements = []

        Task {
            do {
                let fetched = try await sdk.systemPathElements(path: path, keys: keys)
                await MainActor.run {
                    self.elements = fetched
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

    /// Parse a JSON array-of-strings text field, surfacing a clear error when
    /// it isn't valid JSON or isn't an array of strings.
    private func decodeStringArray(_ text: String, field: String) throws -> [String] {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let data = trimmed.data(using: .utf8) else {
            throw PathElementsInputError.invalid("Could not encode \(field) field")
        }
        guard let array = try? JSONSerialization.jsonObject(with: data) as? [String] else {
            throw PathElementsInputError.invalid("\(field) must be a JSON array of strings, e.g. [\"60\"]")
        }
        return array
    }

    private enum PathElementsInputError: LocalizedError {
        case invalid(String)

        var errorDescription: String? {
            switch self {
            case .invalid(let message):
                return message
            }
        }
    }
}

struct GroveDBPathElementsView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationView {
            GroveDBPathElementsView()
                .environmentObject(AppState())
        }
    }
}
