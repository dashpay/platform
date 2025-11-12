import SwiftUI
import SwiftData

struct DocumentTypeDetailsView: View {
    let documentType: PersistentDocumentType
    @Environment(\.dismiss) var dismiss
    @State private var expandedIndices: Set<String> = []
    
    var body: some View {
        List {
            documentInfoSection
            documentSettingsSection
            documentIndexesSection
            documentPropertiesSection
        }
        .navigationTitle(documentType.name)
        .navigationBarTitleDisplayMode(.inline)
    }
    
    // MARK: - Section Views
    
    @ViewBuilder
    private var documentInfoSection: some View {
        Section("Document Type Information") {
            VStack(alignment: .leading, spacing: 8) {
                InfoRow(label: "Name:", value: documentType.name)
                
                if documentType.documentCount > 0 {
                    InfoRow(label: "Documents:", value: "\(documentType.documentCount)")
                }
                
                if let persistentProperties = documentType.persistentProperties, !persistentProperties.isEmpty {
                    InfoRow(label: "Properties:", value: "\(persistentProperties.count)")
                } else if let properties = documentType.properties, !properties.isEmpty {
                    InfoRow(label: "Properties:", value: "\(properties.count)")
                }
                
                if let indices = documentType.indices {
                    InfoRow(label: "Indices:", value: "\(indices.count)")
                }
                
                if let requiredFields = documentType.requiredFields, !requiredFields.isEmpty {
                    InfoRow(label: "Required Fields:", value: "\(requiredFields.count)")
                }
                
                InfoRow(label: "Security Level:", value: "\(documentType.securityLevel)")
            }
            .padding(.vertical, 4)
        }
    }
    
    @ViewBuilder
    private var documentSettingsSection: some View {
        Section("Document Settings") {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Label("Keep History", systemImage: documentType.documentsKeepHistory ? "clock.fill" : "clock")
                        .foregroundColor(documentType.documentsKeepHistory ? .blue : .secondary)
                    Spacer()
                }
                
                HStack {
                    Label("Mutable", systemImage: documentType.documentsMutable ? "pencil.circle.fill" : "pencil.circle")
                        .foregroundColor(documentType.documentsMutable ? .green : .secondary)
                    Spacer()
                }
                
                HStack {
                    Label("Can Be Deleted", systemImage: documentType.documentsCanBeDeleted ? "trash.circle.fill" : "trash.circle")
                        .foregroundColor(documentType.documentsCanBeDeleted ? .red : .secondary)
                    Spacer()
                }
                
                HStack {
                    Label("Transferable", systemImage: documentType.documentsTransferable ? "arrow.left.arrow.right.circle.fill" : "arrow.left.arrow.right.circle")
                        .foregroundColor(documentType.documentsTransferable ? .purple : .secondary)
                    Spacer()
                }
                
                HStack {
                    Label("Trade Mode", systemImage: documentType.tradeMode > 0 ? "cart.fill" : "cart")
                        .foregroundColor(documentType.tradeMode > 0 ? .orange : .secondary)
                    Spacer()
                }
                
                // Creation restrictions
                if documentType.creationRestrictionMode > 0 {
                    HStack {
                        let restrictionText = documentType.creationRestrictionMode == 1 ? "Owner Only" : "System Only"
                        let restrictionIcon = documentType.creationRestrictionMode == 1 ? "person.fill.checkmark" : "lock.fill"
                        Label("Creation: \(restrictionText)", systemImage: restrictionIcon)
                            .foregroundColor(documentType.creationRestrictionMode == 2 ? .red : .yellow)
                        Spacer()
                    }
                }
                
                if documentType.requiresIdentityEncryptionBoundedKey || documentType.requiresIdentityDecryptionBoundedKey {
                    Divider()
                    
                    if documentType.requiresIdentityEncryptionBoundedKey {
                        HStack {
                            Label("Requires Encryption Key", systemImage: "lock.shield.fill")
                                .foregroundColor(.indigo)
                            Spacer()
                        }
                    }
                    
                    if documentType.requiresIdentityDecryptionBoundedKey {
                        HStack {
                            Label("Requires Decryption Key", systemImage: "lock.open.fill")
                                .foregroundColor(.indigo)
                            Spacer()
                        }
                    }
                }
            }
            .font(.subheadline)
            .padding(.vertical, 4)
        }
    }
    
    @ViewBuilder
    private var documentIndexesSection: some View {
        if let indices = documentType.indices, !indices.isEmpty {
            Section("Indices (\(indices.count))") {
                ForEach(indices.sorted(by: { $0.name < $1.name }), id: \.id) { index in
                    ExpandableIndexRowView(index: index, isExpanded: expandedIndices.contains(index.name)) {
                        if expandedIndices.contains(index.name) {
                            expandedIndices.remove(index.name)
                        } else {
                            expandedIndices.insert(index.name)
                        }
                    }
                }
            }
        }
    }
    
    @ViewBuilder
    private var documentPropertiesSection: some View {
        if let properties = documentType.properties, !properties.isEmpty {
            Section("Properties (\(properties.count))") {
                ForEach(properties.sorted(by: { $0.key < $1.key }), id: \.key) { key, value in
                    PropertyRowView(
                        propertyName: key,
                        propertyData: value,
                        isRequired: documentType.requiredFields?.contains(key) ?? false
                    )
                }
            }
        }
    }
}

// MARK: - Supporting Views

struct ExpandableIndexRowView: View {
    let index: PersistentIndex
    let isExpanded: Bool
    let onTap: () -> Void
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Button(action: onTap) {
                HStack {
                    Text(index.name)
                        .font(.headline)
                        .foregroundColor(.primary)
                    
                    Spacer()
                    
                    if index.unique {
                        Text("UNIQUE")
                            .font(.caption2)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Color.purple.opacity(0.2))
                            .foregroundColor(.purple)
                            .cornerRadius(4)
                    }
                    
                    Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .buttonStyle(PlainButtonStyle())
            
            if isExpanded {
                VStack(alignment: .leading, spacing: 6) {
                    if let properties = index.properties, !properties.isEmpty {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Properties:")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            ForEach(properties, id: \.self) { prop in
                                HStack {
                                    Image(systemName: "arrow.right")
                                        .font(.caption2)
                                        .foregroundColor(.secondary)
                                    Text(prop)
                                        .font(.caption)
                                        .foregroundColor(.primary)
                                }
                                .padding(.leading, 8)
                            }
                        }
                    }
                    
                    HStack(spacing: 12) {
                        if index.nullSearchable {
                            Label("Null Searchable", systemImage: "magnifyingglass")
                                .font(.caption2)
                                .foregroundColor(.blue)
                        }
                        
                        if index.contested {
                            Label("Contested", systemImage: "exclamationmark.triangle.fill")
                                .font(.caption2)
                                .foregroundColor(.orange)
                        }
                    }
                    
                    // Show contested details if available
                    if index.contested, let contestedDetails = index.contestedDetails {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Contest Rules:")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .padding(.top, 4)
                            
                            if let description = contestedDetails["description"] as? String {
                                Text(description)
                                    .font(.caption2)
                                    .foregroundColor(.orange)
                                    .padding(.leading, 8)
                            }
                            
                            if let fieldMatches = contestedDetails["fieldMatches"] as? [[String: Any]] {
                                ForEach(fieldMatches.indices, id: \.self) { idx in
                                    if let field = fieldMatches[idx]["field"] as? String,
                                       let pattern = fieldMatches[idx]["regexPattern"] as? String {
                                        HStack {
                                            Text("Field: \(field)")
                                                .font(.caption2)
                                                .foregroundColor(.secondary)
                                            Text("Pattern: \(pattern)")
                                                .font(.caption2)
                                                .foregroundColor(.purple)
                                        }
                                        .padding(.leading, 8)
                                    }
                                }
                            }
                        }
                    }
                }
                .padding(.top, 4)
            }
        }
        .padding(.vertical, 4)
    }
}

struct PropertyRowView: View {
    let propertyName: String
    let propertyData: Any
    let isRequired: Bool
    
    var propertyDict: [String: Any]? {
        propertyData as? [String: Any]
    }
    
    var propertyType: String {
        if let dict = propertyDict,
           let type = dict["type"] as? String {
            return type
        }
        return "unknown"
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(propertyName)
                    .font(.headline)
                Spacer()
                Text(propertyType)
                    .font(.caption)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 3)
                    .background(propertyTypeColor.opacity(0.2))
                    .foregroundColor(propertyTypeColor)
                    .cornerRadius(6)
            }
            
            // Property attributes
            propertyAttributesView
            
            // Sub-properties for objects
            if propertyType == "object", let dict = propertyDict {
                subPropertiesView(dict: dict)
            }
            
            // Description
            if let dict = propertyDict,
               let description = dict["description"] as? String {
                Text(description)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(3)
                    .padding(.top, 2)
            }
        }
        .padding(.vertical, 4)
    }
    
    @ViewBuilder
    private var propertyAttributesView: some View {
        if let dict = propertyDict {
            HStack(spacing: 8) {
                if isRequired {
                    Label("Required", systemImage: "asterisk.circle.fill")
                        .font(.caption2)
                        .foregroundColor(.red)
                }
                
                if let minLength = dict["minLength"] as? Int {
                    Text("Min: \(minLength)")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                
                if let maxLength = dict["maxLength"] as? Int {
                    Text("Max: \(maxLength)")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                
                if dict["pattern"] != nil {
                    Label("Pattern", systemImage: "textformat")
                        .font(.caption2)
                        .foregroundColor(.purple)
                }
                
                if let byteArray = dict["byteArray"] as? Bool, byteArray {
                    Label("Byte Array", systemImage: "square.grid.3x3")
                        .font(.caption2)
                        .foregroundColor(.orange)
                }
                
                if let contentMediaType = dict["contentMediaType"] as? String {
                    Label(contentMediaType.components(separatedBy: ".").last ?? "Media", 
                          systemImage: "doc.text")
                        .font(.caption2)
                        .foregroundColor(.indigo)
                }
            }
        }
    }
    
    @ViewBuilder
    private func subPropertiesView(dict: [String: Any]) -> some View {
        if let subProperties = dict["properties"] as? [String: Any] {
            VStack(alignment: .leading, spacing: 4) {
                Text("Sub-properties:")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding(.top, 4)
                
                ForEach(subProperties.sorted(by: { $0.key < $1.key }), id: \.key) { key, value in
                    if let subPropDict = value as? [String: Any] {
                        HStack {
                            Image(systemName: "arrow.right")
                                .font(.caption2)
                                .foregroundColor(.secondary)
                            
                            Text(key)
                                .font(.caption)
                                .fontWeight(.medium)
                            
                            if let type = subPropDict["type"] as? String {
                                Text(type)
                                    .font(.caption2)
                                    .padding(.horizontal, 4)
                                    .padding(.vertical, 1)
                                    .background(Color.gray.opacity(0.2))
                                    .cornerRadius(3)
                            }
                            
                            Spacer()
                        }
                        .padding(.leading, 8)
                    }
                }
            }
        }
    }
    
    private var propertyTypeColor: Color {
        switch propertyType.lowercased() {
        case "string":
            return .blue
        case "integer", "number":
            return .green
        case "boolean":
            return .orange
        case "array":
            return .purple
        case "object":
            return .indigo
        default:
            return .gray
        }
    }
}

#Preview {
    NavigationView {
        DocumentTypeDetailsView(
            documentType: PersistentDocumentType(
                contractId: Data(),
                name: "domain",
                schemaJSON: Data(),
                propertiesJSON: Data()
            )
        )
    }
}