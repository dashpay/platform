import SwiftUI
import SwiftData
import UIKit

struct DataContractDetailsView: View {
    let contract: PersistentDataContract
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
        NavigationView {
            List {
                contractConfigurationSection
                contractInfoSection
                tokensSection
                documentTypesSection
                actionsSection
            }
            .navigationTitle("Contract Details")
            .navigationBarTitleDisplayMode(.inline)
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
                
                if let ownerId = contract.ownerIdBase58 {
                    InfoRow(label: "Owner:", value: ownerId, font: .caption, truncate: true)
                }
                
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
                    NavigationLink(destination: TokenDetailsView(token: token)) {
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
                    NavigationLink(destination: DocumentTypeDetailsView(documentType: docType)) {
                        DocumentTypeRowView(docType: docType)
                    }
                }
            }
        }
    }
    
    @ViewBuilder
    private var actionsSection: some View {
        Section {
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
            self.value = value.formatted(date: .abbreviated, time: .omitted)
        } else {
            self.value = value.formatted(.relative(presentation: .named))
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
            serializedContract: Data()
        )
    )
}