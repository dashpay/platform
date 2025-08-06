import SwiftUI

struct DynamicDocumentFormView: View {
    let contractId: String
    let documentType: String
    let schema: [String: Any]?
    @Binding var documentData: [String: Any]
    
    @State private var formFields: [DocumentField] = []
    @State private var stringValues: [String: String] = [:]
    @State private var numberValues: [String: Double] = [:]
    @State private var boolValues: [String: Bool] = [:]
    @State private var arrayValues: [String: [String]] = [:]
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            if let properties = getProperties() {
                ForEach(Array(properties.keys.sorted()), id: \.self) { fieldName in
                    if let fieldSchema = properties[fieldName] as? [String: Any] {
                        fieldView(for: fieldName, schema: fieldSchema)
                    }
                }
            } else {
                Text("No schema available for this document type")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding()
                    .frame(maxWidth: .infinity)
                    .background(Color.orange.opacity(0.1))
                    .cornerRadius(8)
            }
        }
        .onAppear {
            parseSchema()
        }
        .onChange(of: stringValues) { _ in updateDocumentData() }
        .onChange(of: numberValues) { _ in updateDocumentData() }
        .onChange(of: boolValues) { _ in updateDocumentData() }
        .onChange(of: arrayValues) { _ in updateDocumentData() }
    }
    
    @ViewBuilder
    private func fieldView(for fieldName: String, schema: [String: Any]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            // Field label
            HStack {
                Text(fieldName.camelCaseToWords())
                    .font(.subheadline)
                    .fontWeight(.medium)
                
                if isRequired(fieldName) {
                    Text("*")
                        .foregroundColor(.red)
                }
            }
            
            // Field input based on type
            if let fieldType = schema["type"] as? String {
                switch fieldType {
                case "string":
                    stringField(for: fieldName, schema: schema)
                case "number", "integer":
                    numberField(for: fieldName, schema: schema)
                case "boolean":
                    booleanField(for: fieldName, schema: schema)
                case "array":
                    arrayField(for: fieldName, schema: schema)
                case "object":
                    objectField(for: fieldName, schema: schema)
                default:
                    TextField("Enter \(fieldName)", text: binding(for: fieldName))
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                }
            }
            
            // Field description/help
            if let description = schema["description"] as? String {
                Text(description)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
    }
    
    @ViewBuilder
    private func stringField(for fieldName: String, schema: [String: Any]) -> some View {
        let maxLength = schema["maxLength"] as? Int
        let minLength = schema["minLength"] as? Int
        let pattern = schema["pattern"] as? String
        let format = schema["format"] as? String
        let enumValues = schema["enum"] as? [String]
        
        if let enumValues = enumValues {
            // Dropdown for enum values
            Picker(fieldName, selection: binding(for: fieldName)) {
                Text("Select...").tag("")
                ForEach(enumValues, id: \.self) { value in
                    Text(value).tag(value)
                }
            }
            .pickerStyle(MenuPickerStyle())
            .padding()
            .background(Color.gray.opacity(0.1))
            .cornerRadius(8)
        } else if maxLength ?? 0 > 100 {
            // Text area for long strings
            TextEditor(text: binding(for: fieldName))
                .frame(minHeight: 100)
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(Color.gray.opacity(0.3), lineWidth: 1)
                )
        } else {
            // Regular text field
            VStack(alignment: .leading) {
                TextField(placeholder(for: fieldName, schema: schema), text: binding(for: fieldName))
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .keyboardType(keyboardType(for: format))
                
                if let maxLength = maxLength {
                    Text("\(stringValues[fieldName]?.count ?? 0)/\(maxLength) characters")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
        }
    }
    
    @ViewBuilder
    private func numberField(for fieldName: String, schema: [String: Any]) -> some View {
        let minimum = schema["minimum"] as? Double
        let maximum = schema["maximum"] as? Double
        
        HStack {
            TextField(placeholder(for: fieldName, schema: schema), text: numberBinding(for: fieldName))
                .keyboardType(.decimalPad)
                .textFieldStyle(RoundedBorderTextFieldStyle())
            
            if let min = minimum, let max = maximum {
                Text("(\(Int(min))-\(Int(max)))")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }
    
    @ViewBuilder
    private func booleanField(for fieldName: String, schema: [String: Any]) -> some View {
        Toggle(isOn: boolBinding(for: fieldName)) {
            Text("")
        }
        .labelsHidden()
    }
    
    @ViewBuilder
    private func arrayField(for fieldName: String, schema: [String: Any]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            // Simple comma-separated input for now
            TextField("Enter comma-separated values", text: arrayBinding(for: fieldName))
                .textFieldStyle(RoundedBorderTextFieldStyle())
            
            if let items = schema["items"] as? [String: Any],
               let itemType = items["type"] as? String {
                Text("Item type: \(itemType)")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
    }
    
    @ViewBuilder
    private func objectField(for fieldName: String, schema: [String: Any]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Object fields:")
                .font(.caption)
                .foregroundColor(.secondary)
            
            if let properties = schema["properties"] as? [String: Any] {
                ForEach(Array(properties.keys.sorted()), id: \.self) { subFieldName in
                    if let subFieldSchema = properties[subFieldName] as? [String: Any] {
                        HStack {
                            Text("• \(subFieldName)")
                                .font(.caption)
                            Spacer()
                        }
                    }
                }
            }
            
            // For now, use JSON input for complex objects
            TextEditor(text: binding(for: fieldName))
                .font(.system(.caption, design: .monospaced))
                .frame(minHeight: 100)
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(Color.gray.opacity(0.3), lineWidth: 1)
                )
        }
    }
    
    // MARK: - Helper Methods
    
    private func getProperties() -> [String: Any]? {
        if let props = schema?["properties"] as? [String: Any] {
            return props
        }
        return nil
    }
    
    private func isRequired(_ fieldName: String) -> Bool {
        if let required = schema?["required"] as? [String] {
            return required.contains(fieldName)
        }
        return false
    }
    
    private func parseSchema() {
        guard let properties = getProperties() else { return }
        
        // Initialize form values from existing document data
        for (fieldName, fieldSchema) in properties {
            if let schema = fieldSchema as? [String: Any],
               let fieldType = schema["type"] as? String {
                
                // Initialize with existing data or defaults
                if let existingValue = documentData[fieldName] {
                    switch fieldType {
                    case "string":
                        stringValues[fieldName] = existingValue as? String ?? ""
                    case "number", "integer":
                        if let num = existingValue as? Double {
                            numberValues[fieldName] = num
                        } else if let num = existingValue as? Int {
                            numberValues[fieldName] = Double(num)
                        }
                    case "boolean":
                        boolValues[fieldName] = existingValue as? Bool ?? false
                    case "array":
                        if let array = existingValue as? [String] {
                            arrayValues[fieldName] = array
                        }
                    default:
                        stringValues[fieldName] = ""
                    }
                } else {
                    // Set defaults
                    switch fieldType {
                    case "string":
                        stringValues[fieldName] = ""
                    case "number", "integer":
                        numberValues[fieldName] = 0
                    case "boolean":
                        boolValues[fieldName] = false
                    case "array":
                        arrayValues[fieldName] = []
                    default:
                        stringValues[fieldName] = ""
                    }
                }
            }
        }
    }
    
    private func updateDocumentData() {
        var newData: [String: Any] = [:]
        
        // Collect all field values
        for (key, value) in stringValues {
            if !value.isEmpty {
                newData[key] = value
            }
        }
        
        for (key, value) in numberValues {
            newData[key] = value
        }
        
        for (key, value) in boolValues {
            newData[key] = value
        }
        
        for (key, value) in arrayValues {
            if !value.isEmpty {
                newData[key] = value
            }
        }
        
        documentData = newData
    }
    
    private func binding(for fieldName: String) -> Binding<String> {
        Binding(
            get: { stringValues[fieldName] ?? "" },
            set: { stringValues[fieldName] = $0 }
        )
    }
    
    private func numberBinding(for fieldName: String) -> Binding<String> {
        Binding(
            get: {
                if let value = numberValues[fieldName] {
                    return value.truncatingRemainder(dividingBy: 1) == 0 ? String(Int(value)) : String(value)
                }
                return ""
            },
            set: {
                if let value = Double($0) {
                    numberValues[fieldName] = value
                }
            }
        )
    }
    
    private func boolBinding(for fieldName: String) -> Binding<Bool> {
        Binding(
            get: { boolValues[fieldName] ?? false },
            set: { boolValues[fieldName] = $0 }
        )
    }
    
    private func arrayBinding(for fieldName: String) -> Binding<String> {
        Binding(
            get: {
                arrayValues[fieldName]?.joined(separator: ", ") ?? ""
            },
            set: {
                arrayValues[fieldName] = $0.split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
            }
        )
    }
    
    private func placeholder(for fieldName: String, schema: [String: Any]) -> String {
        if let placeholder = schema["placeholder"] as? String {
            return placeholder
        }
        
        if let format = schema["format"] as? String {
            switch format {
            case "email":
                return "example@email.com"
            case "uri", "url":
                return "https://example.com"
            case "date":
                return "YYYY-MM-DD"
            case "date-time":
                return "YYYY-MM-DD HH:MM:SS"
            default:
                break
            }
        }
        
        return "Enter \(fieldName.camelCaseToWords().lowercased())"
    }
    
    private func keyboardType(for format: String?) -> UIKeyboardType {
        switch format {
        case "email":
            return .emailAddress
        case "uri", "url":
            return .URL
        case "phone":
            return .phonePad
        default:
            return .default
        }
    }
}

// MARK: - String Extension

extension String {
    func camelCaseToWords() -> String {
        return self.unicodeScalars.reduce("") { (result, scalar) in
            if CharacterSet.uppercaseLetters.contains(scalar) {
                return result + " " + String(scalar)
            } else {
                return result + String(scalar)
            }
        }.capitalized
    }
}

// MARK: - Document Field Model

struct DocumentField: Identifiable {
    let id = UUID()
    let name: String
    let type: String
    let required: Bool
    let schema: [String: Any]
}

// MARK: - Preview

struct DynamicDocumentFormView_Previews: PreviewProvider {
    static var previews: some View {
        DynamicDocumentFormView(
            contractId: "test",
            documentType: "note",
            schema: [
                "type": "object",
                "properties": [
                    "message": [
                        "type": "string",
                        "maxLength": 100
                    ]
                ],
                "required": ["message"]
            ],
            documentData: .constant([:])
        )
        .padding()
    }
}