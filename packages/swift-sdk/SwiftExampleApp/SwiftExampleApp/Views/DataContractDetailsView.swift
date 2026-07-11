import SwiftUI
import SwiftData
import SwiftDashSDK
import UIKit
import SwiftDashSDK

struct DataContractDetailsView: View {
    let contract: PersistentDataContract
    /// Optional "Save to Device" hook. Set when the view is rendered
    /// against a preview-only contract held in an in-memory
    /// `ModelContainer`. Tapping the Save button calls this closure;
    /// the parent runs the persist pipeline and dismisses the sheet.
    /// Nil for the normal saved-contract path.
    var onSave: (() -> Void)? = nil
    /// In-flight indicator for the Save button. Owned by the parent
    /// (the persist call lives there); we just toggle the spinner.
    var isSaving: Bool = false
    @Environment(\.dismiss) var dismiss
    @Environment(\.modelContext) private var modelContext
    @State private var showingShareSheet = false
    @State private var showCopiedAlert = false

    var displayName: String {
        // Check if this is a token-only contract
        if let tokens = contract.tokens,
           tokens.count == 1,
           let documentTypes = contract.documentTypes,
           documentTypes.isEmpty,
           let token = tokens.first {
            // Use the token's singular form for display
            if let singularName = token.getSingularForm(languageCode: "en") {
                return "\(singularName) Token Contract"
            } else {
                return "Token Contract"
            }
        }

        // Otherwise use the stored name
        return contract.name
    }

    var body: some View {
        // `NavigationStack` + value-based navigation (rather than the
        // deprecated `NavigationView` + `NavigationLink(destination:)`).
        // Destination-based links inside a `ForEach` get torn down and
        // popped whenever the parent re-renders; a re-render of the
        // contracts list (or this view) would otherwise pop a pushed
        // drill-down — and dismiss any sheet presented from it, e.g.
        // the document-create flow. Value-based routes survive a
        // parent re-render.
        NavigationStack {
            List {
                contractConfigurationSection
                contractInfoSection
                tokensSection
                groupsSection
                documentTypesSection
                documentsSection
                actionsSection
            }
            .navigationTitle("Contract Details")
            .navigationBarTitleDisplayMode(.inline)
            .navigationDestination(for: PersistentToken.self) { token in
                TokenDetailsView(token: token)
            }
            .navigationDestination(for: PersistentDocumentType.self) { docType in
                DocumentTypeDetailsView(documentType: docType)
            }
            .navigationDestination(for: PersistentDocument.self) { document in
                DocumentStorageDetailView(record: document)
            }
            .navigationDestination(for: ParsedGroup.self) { group in
                GroupDetailView(
                    contractId: contract.id,
                    position: group.position,
                    members: group.members,
                    requiredPower: group.requiredPower
                )
            }
            .navigationDestination(for: OwnerRoute.self) { route in
                IdentityDetailView(identityId: route.identityId)
            }
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
            .sheet(isPresented: $showingShareSheet) {
                if let url = exportContract() {
                    ShareSheet(items: [url])
                }
            }
            .alert("Copied to Clipboard", isPresented: $showCopiedAlert) {
                Button("OK", role: .cancel) { }
            } message: {
                Text("Contract hex has been copied to your clipboard")
            }
        }
        .onAppear {
            updateLastAccessedDate()
        }
    }

    // MARK: - Section Views

    @ViewBuilder
    private var contractConfigurationSection: some View {
        Section("Contract Configuration") {
            VStack(alignment: .leading, spacing: 8) {
                // Contract-level settings
                HStack {
                    Label("Can Be Deleted", systemImage: contract.canBeDeleted ? "checkmark.circle.fill" : "xmark.circle")
                        .foregroundColor(contract.canBeDeleted ? .green : .red)
                    Spacer()
                }

                HStack {
                    Label("Read Only", systemImage: contract.readonly ? "lock.fill" : "lock.open")
                        .foregroundColor(contract.readonly ? .orange : .green)
                    Spacer()
                }

                HStack {
                    Label("Keeps History", systemImage: contract.keepsHistory ? "clock.fill" : "clock")
                        .foregroundColor(contract.keepsHistory ? .blue : .secondary)
                    Spacer()
                }

                // Document defaults
                if contract.documentsKeepHistoryContractDefault {
                    HStack {
                        Label("Documents Keep History (Default)", systemImage: "doc.text.fill")
                            .foregroundColor(.blue)
                        Spacer()
                    }
                }

                if contract.documentsMutableContractDefault {
                    HStack {
                        Label("Documents Mutable (Default)", systemImage: "pencil.circle.fill")
                            .foregroundColor(.green)
                        Spacer()
                    }
                }

                if contract.documentsCanBeDeletedContractDefault {
                    HStack {
                        Label("Documents Can Be Deleted (Default)", systemImage: "trash.circle.fill")
                            .foregroundColor(.red)
                        Spacer()
                    }
                }

                // Schema information
                if let schemaDefs = contract.schemaDefs {
                    InfoRow(label: "Schema Definitions:", value: "\(schemaDefs)")
                }
            }
            .font(.subheadline)
            .padding(.vertical, 4)
        }
    }

    @ViewBuilder
    private var contractInfoSection: some View {
        Section("Contract Information") {
            VStack(alignment: .leading, spacing: 8) {
                InfoRow(label: "Name:", value: displayName)
                InfoRow(label: "ID:", value: contract.idBase58, font: .caption, truncate: true)

                if let version = contract.version {
                    InfoRow(label: "Version:", value: "\(version)")
                }

                ownerRow

                InfoRow(label: "JSON Size:", value: ByteCountFormatter.string(fromByteCount: Int64(contract.serializedContract.count), countStyle: .binary))

                if let binaryData = contract.binarySerialization {
                    InfoRow(label: "Binary Size:", value: ByteCountFormatter.string(fromByteCount: Int64(binaryData.count), countStyle: .binary))
                }

                InfoRow(label: "Created:", value: contract.createdAt, style: .date)
                InfoRow(label: "Last Used:", value: contract.lastAccessedAt, style: .relative)
            }
            .padding(.vertical, 4)
        }
    }

    @ViewBuilder
    private var tokensSection: some View {
        if let tokens = contract.tokens, !tokens.isEmpty {
            Section("Tokens (\(tokens.count))") {
                ForEach(tokens.sorted(by: { $0.position < $1.position }), id: \.id) { token in
                    NavigationLink(value: token) {
                        TokenRowView(token: token)
                    }
                }
            }
        }
    }

    @ViewBuilder
    private var documentTypesSection: some View {
        if let documentTypes = contract.documentTypes, !documentTypes.isEmpty {
            Section("Document Types (\(documentTypes.count))") {
                ForEach(documentTypes.sorted(by: { $0.name < $1.name }), id: \.id) { docType in
                    NavigationLink(value: docType) {
                        DocumentTypeRowView(docType: docType)
                    }
                }
            }
        }
    }

    /// Owner row for `contractInfoSection`. Three modes:
    ///  - `ownerIdentity` is set → tappable navigation link to the
    ///    identity detail view, showing the identity's
    ///    `displayName` plus an "Owner" subtitle.
    ///  - `ownerId` is set but the identity isn't held locally →
    ///    show the truncated base58 owner id (non-tappable, in the
    ///    same style the previous implementation used).
    ///  - Both are nil (system contracts) → omit the row entirely.
    @ViewBuilder
    private var ownerRow: some View {
        if let owner = contract.ownerIdentity {
            NavigationLink(value: OwnerRoute(identityId: owner.identityId)) {
                HStack {
                    Text("Owner:")
                        .foregroundColor(.secondary)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(owner.displayName)
                            .font(.body)
                            .foregroundColor(.blue)
                        Text("Owner")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
            }
        } else if let ownerId = contract.ownerIdBase58 {
            InfoRow(label: "Owner:", value: ownerId, font: .caption, truncate: true)
        }
    }

    /// Documents *instances* persisted under this contract. Distinct
    /// from `documentTypesSection`, which lists the contract's
    /// declared schemas. Hidden when the contract row hasn't
    /// accumulated any local documents yet.
    @ViewBuilder
    private var documentsSection: some View {
        if !contract.documents.isEmpty {
            let sortedDocs = contract.documents.sorted(by: { $0.documentId < $1.documentId })
            Section("Documents (\(contract.documents.count))") {
                ForEach(sortedDocs, id: \.documentId) { document in
                    NavigationLink(value: document) {
                        DocumentInstanceRowView(document: document)
                    }
                }
            }
        }
    }

    /// Groups defined on this contract. Populated lazily by
    /// `DataContractParser.parseDataContract` from the `groups`
    /// block of the contract JSON. Hidden entirely when the
    /// contract has no groups (the common case for the cache).
    @ViewBuilder
    private var groupsSection: some View {
        if let parsedGroups = parseGroups(), !parsedGroups.isEmpty {
            Section("Groups (\(parsedGroups.count))") {
                ForEach(parsedGroups, id: \.position) { group in
                    NavigationLink(value: group) {
                        GroupRowView(
                            position: group.position,
                            memberCount: group.members.count,
                            requiredPower: group.requiredPower
                        )
                    }
                }
            }
        }
    }

    @ViewBuilder
    private var actionsSection: some View {
        Section {
            // Preview-mode entry point: caller (the contracts search
            // sheet) supplies an `onSave` closure that runs the
            // persist pipeline against the *real* model context.
            // Hidden in saved-contract mode (`onSave == nil`) since
            // saving is meaningless for an already-persisted row.
            if let onSave = onSave {
                Button(action: onSave) {
                    HStack {
                        if isSaving {
                            ProgressView().controlSize(.small)
                        }
                        Label("Save to Device", systemImage: "square.and.arrow.down")
                            .foregroundColor(.blue)
                    }
                }
                .disabled(isSaving)
            }

            Button(action: { showingShareSheet = true }) {
                Label("Export Contract", systemImage: "square.and.arrow.up")
                    .foregroundColor(.blue)
            }

            if contract.binarySerializationHex != nil {
                Button(action: copyContractHex) {
                    Label("Copy Contract Hex", systemImage: "doc.on.doc")
                        .foregroundColor(.blue)
                }
            }
        }
    }

    // MARK: - Helper Methods

    /// Flat group entry used by `groupsSection` / `GroupDetailView`.
    /// Decoupled from the on-row `[String: Any]` shape so the
    /// section view doesn't have to re-parse on every row render.
    /// `Hashable` so it can drive value-based `.navigationDestination`
    /// (synthesized — `GroupMember` is already `Hashable`).
    private struct ParsedGroup: Hashable {
        let position: Int
        let requiredPower: Int
        let members: [GroupMember]
    }

    /// Value-based navigation route for the owner-identity drill-down.
    /// A thin wrapper around the owner's `identityId` so the
    /// `.navigationDestination(for:)` route type is unambiguous (a bare
    /// `Data` value could collide with any other `Data`-keyed route).
    private struct OwnerRoute: Hashable {
        let identityId: Data
    }

    /// Decode `contract.groups` (`[String: Any]?`) into a sorted
    /// list of `ParsedGroup` rows. The on-chain shape is:
    ///   `{ "<position>": { "members": { "<idBase58>": <power> },
    ///       "requiredPower": <u32> } }`
    /// Returns nil when the contract has no groups, an empty array
    /// when the JSON existed but contained no decodable entries.
    private func parseGroups() -> [ParsedGroup]? {
        guard let raw = contract.groups, !raw.isEmpty else { return nil }
        var out: [ParsedGroup] = []
        for (positionKey, value) in raw {
            guard let position = Int(positionKey),
                  let groupDict = value as? [String: Any] else { continue }
            let requiredPowerRaw = groupDict["requiredPower"]
            let requiredPower: Int = {
                if let i = requiredPowerRaw as? Int { return i }
                if let n = requiredPowerRaw as? NSNumber { return n.intValue }
                if let s = requiredPowerRaw as? String, let i = Int(s) { return i }
                return 0
            }()

            var members: [GroupMember] = []
            if let memberDict = groupDict["members"] as? [String: Any] {
                for (memberId, powerValue) in memberDict {
                    let power: Int = {
                        if let i = powerValue as? Int { return i }
                        if let n = powerValue as? NSNumber { return n.intValue }
                        if let s = powerValue as? String, let i = Int(s) { return i }
                        return 0
                    }()
                    members.append(GroupMember(memberIdBase58: memberId, power: power))
                }
            }
            out.append(
                ParsedGroup(
                    position: position,
                    requiredPower: requiredPower,
                    members: members
                )
            )
        }
        // Position 0 first, then ascending. Bad position keys land
        // at the bottom by construction (Int.max via `parseGroups`
        // skip-on-fail above filters those out, but the sort key
        // here keeps the contract well-defined for any future
        // change).
        out.sort(by: { $0.position < $1.position })
        return out
    }

    private func copyContractHex() {
        guard let hexString = contract.binarySerializationHex else { return }

        UIPasteboard.general.string = hexString
        showCopiedAlert = true

        print("📋 Copied contract hex to clipboard: \(hexString.prefix(20))...")
    }

    private func exportContract() -> URL? {
        do {
            let fileName = "\(contract.name.replacingOccurrences(of: " ", with: "_"))_\(contract.idBase58.prefix(8)).json"
            let tempURL = FileManager.default.temporaryDirectory.appendingPathComponent(fileName)

            try contract.serializedContract.write(to: tempURL)
            return tempURL
        } catch {
            print("Failed to export contract: \(error)")
            return nil
        }
    }

    private func updateLastAccessedDate() {
        contract.lastAccessedAt = Date()
        do {
            try modelContext.save()
        } catch {
            print("Failed to update last accessed date: \(error)")
        }
    }
}

// MARK: - Supporting Views

struct InfoRow: View {
    let label: String
    let value: String
    var font: Font = .body
    var truncate: Bool = false

    init(label: String, value: String, font: Font = .body, truncate: Bool = false) {
        self.label = label
        self.value = value
        self.font = font
        self.truncate = truncate
    }

    init(label: String, value: Date, style: Text.DateStyle) {
        self.label = label
        if style == .date {
            self.value = AppDate.formatted(value, dateStyle: .abbreviated, timeStyle: .omitted)
        } else {
            self.value = AppDate.relative(value)
        }
        self.font = .body
        self.truncate = false
    }

    var body: some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
            if truncate {
                Text(value)
                    .font(font)
                    .lineLimit(1)
                    .truncationMode(.middle)
            } else {
                Text(value)
                    .font(font)
            }
        }
    }
}

struct TokenRowView: View {
    let token: PersistentToken

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(token.getPluralForm() ?? token.displayName)
                    .font(.headline)
                Spacer()
                Text("Position \(token.position)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            tokenSupplyInfo
            tokenFeatures
        }
        .padding(.vertical, 4)
    }

    @ViewBuilder
    private var tokenSupplyInfo: some View {
        HStack {
            Text("Base Supply:")
                .font(.caption)
                .foregroundColor(.secondary)
            Text(token.formattedBaseSupply)
                .font(.caption)

            if let maxSupply = token.maxSupply {
                Spacer()
                Text("Max Supply:")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Text(maxSupply)
                    .font(.caption)
            }
        }
    }

    @ViewBuilder
    private var tokenFeatures: some View {
        HStack(spacing: 12) {
            if token.keepsAnyHistory {
                Label("History", systemImage: "clock")
                    .font(.caption2)
                    .foregroundColor(.blue)
            }
            if token.isPaused {
                Label("Paused", systemImage: "pause.circle")
                    .font(.caption2)
                    .foregroundColor(.orange)
            }
            if token.allowTransferToFrozenBalance {
                Label("Frozen Transfer", systemImage: "snowflake")
                    .font(.caption2)
                    .foregroundColor(.cyan)
            }
        }
    }
}

struct DocumentTypeRowView: View {
    let docType: PersistentDocumentType

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(docType.name)
                    .font(.headline)
                Spacer()
                if docType.documentCount > 0 {
                    Text("\(docType.documentCount) docs")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }

            if let properties = docType.properties {
                Text("\(properties.count) properties")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            documentFeatures
        }
        .padding(.vertical, 4)
    }

    @ViewBuilder
    private var documentFeatures: some View {
        HStack(spacing: 12) {
            if docType.documentsKeepHistory {
                Label("History", systemImage: "clock")
                    .font(.caption2)
                    .foregroundColor(.blue)
            }
            if docType.documentsMutable {
                Label("Mutable", systemImage: "pencil")
                    .font(.caption2)
                    .foregroundColor(.green)
            }
            if docType.documentsCanBeDeleted {
                Label("Deletable", systemImage: "trash")
                    .font(.caption2)
                    .foregroundColor(.red)
            }
            if docType.documentsTransferable {
                Label("Transferable", systemImage: "arrow.left.arrow.right")
                    .font(.caption2)
                    .foregroundColor(.purple)
            }
        }
    }
}

/// Row for one instance of `PersistentDocument` within a contract's
/// detail Documents section. Mirrors the look of `DocumentTypeRowView`
/// but works on a *document instance* rather than a document type
/// definition. Title falls back through `displayTitle` (the
/// document's own preferred-field heuristic) so application data
/// surfaces sensibly even when there's no `title`/`name` field.
struct DocumentInstanceRowView: View {
    let document: PersistentDocument

    private var truncatedId: String {
        let s = document.idBase58
        guard s.count > 14 else { return s }
        return "\(s.prefix(8))..\(s.suffix(4))"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(document.displayTitle)
                .font(.headline)
                .lineLimit(1)
                .truncationMode(.tail)

            Text(document.documentType)
                .font(.caption)
                .foregroundColor(.secondary)

            Text(truncatedId)
                .font(.caption2)
                .foregroundColor(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .padding(.vertical, 4)
    }
}

/// Row for a single group entry in the contract's Groups section.
/// Compact: just the position and a one-line summary. The detailed
/// member list lives in `GroupDetailView`.
struct GroupRowView: View {
    let position: Int
    let memberCount: Int
    let requiredPower: Int

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Group \(position)")
                .font(.headline)
            Text("\(memberCount) members · required power \(requiredPower)")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(.vertical, 4)
    }
}

struct ShareSheet: UIViewControllerRepresentable {
    let items: [Any]

    func makeUIViewController(context: Context) -> UIActivityViewController {
        UIActivityViewController(activityItems: items, applicationActivities: nil)
    }

    func updateUIViewController(_ uiViewController: UIActivityViewController, context: Context) {}
}

#Preview {
    DataContractDetailsView(
        contract: PersistentDataContract(
            id: Data(),
            name: "Sample Contract",
            serializedContract: Data(),
            network: .testnet
        )
    )
}
