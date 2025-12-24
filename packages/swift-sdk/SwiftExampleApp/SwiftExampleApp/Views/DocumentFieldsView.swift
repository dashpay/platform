import SwiftUI
import SwiftData

struct DocumentFieldsView: View {
    let documentType: PersistentDocumentType
    @Binding var fieldValues: [String: Any]
    
    @State private var textFields: [String: String] = [:]
    @State private var numberFields: [String: String] = [:]
    @State private var boolFields: [String: Bool] = [:]
    @State private var arrayFields: [String: String] = [:]
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {            
            if let properties = documentType.propertiesList, !properties.isEmpty {
                ForEach(properties.sorted(by: { $0.name < $1.name }), id: \.id) { property in
                    fieldView(for: property)
                }
            } else {
                Text("No properties defined for this document type")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.orange.opacity(0.1))
                    .cornerRadius(8)
            }
        }
        .padding()
        .cornerRadius(12)
        .onAppear {
            initializeFields()
        }
    }
    
    @ViewBuilder
    private func fieldView(for property: PersistentProperty) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(property.name)
                    .font(.subheadline)
                    .fontWeight(.medium)
                if property.isRequired {
                    Text("*")
                        .foregroundColor(.red)
                }
            }
            
            // Check if this is an identifier field (contentMediaType contains identifier)
            let isIdentifier = property.contentMediaType?.contains("identifier") ?? false
            
            if isIdentifier {
                // Handle identifier fields - ask for base58 input
                VStack(alignment: .leading, spacing: 4) {
                    TextField("Base58 identifier", text: binding(for: property.name, in: $textFields))
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                        .font(.system(.body, design: .monospaced))
                    Text("Enter a valid base58 identifier (e.g., 4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF)")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            } else {
                switch property.type {
            case "string":
                TextField(placeholderText(for: property), text: binding(for: property.name, in: $textFields))
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                
            case "number", "integer":
                TextField(placeholderText(for: property), text: binding(for: property.name, in: $numberFields))
                    .keyboardType(.numberPad)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                
            case "boolean":
                Toggle(isOn: binding(for: property.name, in: $boolFields)) {
                    Text("")
                }
                .labelsHidden()
                
            case "array":
                if property.byteArray {
                    // Byte arrays should be entered as hex strings
                    byteArrayField(for: property)
                } else {
                    // Regular arrays with comma-separated values
                    VStack(alignment: .leading, spacing: 4) {
                        TextField("Enter comma-separated values", text: binding(for: property.name, in: $arrayFields))
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                        Text("Separate multiple values with commas")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
                
            case "object":
                TextEditor(text: binding(for: property.name, in: $textFields))
                    .font(.system(.caption, design: .monospaced))
                    .frame(minHeight: 100)
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .stroke(Color.gray.opacity(0.3), lineWidth: 1)
                    )
                
                default:
                    TextField("Enter \(property.name)", text: binding(for: property.name, in: $textFields))
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                }
            }
            
            if let description = property.fieldDescription {
                Text(description)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
    }
    
    private func placeholderText(for property: PersistentProperty) -> String {
        var placeholder = "Enter \(property.name)"
        
        if let min = property.minLength, let max = property.maxLength {
            placeholder += " (\(min)-\(max) chars)"
        } else if let min = property.minLength {
            placeholder += " (min \(min) chars)"
        } else if let max = property.maxLength {
            placeholder += " (max \(max) chars)"
        }
        
        if let min = property.minValue, let max = property.maxValue {
            placeholder = "Enter value between \(min) and \(max)"
        } else if let min = property.minValue {
            placeholder = "Enter value ≥ \(min)"
        } else if let max = property.maxValue {
            placeholder = "Enter value ≤ \(max)"
        }
        
        return placeholder
    }
    
    private func binding<T>(for key: String, in dictionary: Binding<[String: T]>) -> Binding<T> where T: DefaultInitializable {
        Binding(
            get: { dictionary.wrappedValue[key] ?? T() },
            set: { 
                dictionary.wrappedValue[key] = $0
                updateFieldValues()
            }
        )
    }
    
    private func initializeFields() {
        // Initialize with default values
        if let properties = documentType.propertiesList {
            for property in properties {
                switch property.type {
                case "string", "object":
                    textFields[property.name] = ""
                case "number", "integer":
                    numberFields[property.name] = ""
                case "boolean":
                    boolFields[property.name] = false
                case "array":
                    if property.byteArray {
                        textFields[property.name] = ""  // Use text field for hex input
                    } else {
                        arrayFields[property.name] = ""  // Use array field for comma-separated
                    }
                default:
                    textFields[property.name] = ""
                }
            }
        }
        
        updateFieldValues()
    }
    
    private func updateFieldValues() {
        var values: [String: Any] = [:]
        
        // Check for identifier fields and convert base58 to Data
        if let propertiesList = documentType.propertiesList {
            // Using PersistentProperty objects
            for (key, value) in textFields {
                if !value.isEmpty {
                    if let property = propertiesList.first(where: { $0.name == key }) {
                        let isIdentifier = (property.type == "array" && property.byteArray && 
                                         property.minItems == 32 && property.maxItems == 32) ||
                                         property.contentMediaType?.contains("identifier") ?? false
                        
                        if isIdentifier {
                            // Convert base58 string to Data for identifier fields
                            if let identifierData = Data.identifier(fromBase58: value) {
                                values[key] = identifierData
                            } else {
                                // Invalid base58, keep as string for now (will fail validation)
                                values[key] = value
                            }
                        } else if property.type == "array" && property.byteArray {
                            // Non-identifier byte arrays - convert hex string to Data
                            let hexString = value.hasPrefix("0x") ? String(value.dropFirst(2)) : value
                            if let data = Data(hexString: hexString) {
                                values[key] = data
                            } else {
                                // Invalid hex, keep as string for now (will fail validation)
                                values[key] = value
                            }
                        } else {
                            values[key] = value
                        }
                    } else {
                        values[key] = value
                    }
                }
            }
        }
        
        // Add number fields
        for (key, value) in numberFields {
            if !value.isEmpty {
                if let intValue = Int(value) {
                    values[key] = intValue
                } else if let doubleValue = Double(value) {
                    values[key] = doubleValue
                }
            }
        }
        
        // Add boolean fields
        for (key, value) in boolFields {
            values[key] = value
        }
        
        // Add array fields
        for (key, value) in arrayFields {
            if !value.isEmpty {
                let items = value.split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
                values[key] = items
            }
        }
        
        fieldValues = values
    }
}


// Protocol for default initialization
protocol DefaultInitializable {
    init()
}

extension String: DefaultInitializable {}
extension Bool: DefaultInitializable {
    init() { self = false }
}

// MARK: - Byte Array Field Helper

extension DocumentFieldsView {
    @ViewBuilder
    private func byteArrayField(for property: PersistentProperty) -> some View {
        let expectedBytes = property.minItems ?? property.maxItems ?? 32 // Default to 32 if not specified
        let expectedHexLength = expectedBytes * 2
        let currentValue = textFields[property.name] ?? ""
        
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                TextField("Hex Data", text: binding(for: property.name, in: $textFields))
                    .font(.system(.body, design: .monospaced))
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .autocapitalization(.none)
                    .disableAutocorrection(true)
                    .onChange(of: currentValue) { _, newValue in
                        // Remove any non-hex characters and convert to lowercase
                        let cleaned = newValue.lowercased().filter { "0123456789abcdef".contains($0) }
                        if cleaned != newValue {
                            textFields[property.name] = cleaned
                        }
                    }
                
                // Validation indicator
                if !currentValue.isEmpty {
                    Image(systemName: isValidHex(currentValue, expectedLength: expectedHexLength) ? "checkmark.circle.fill" : "xmark.circle.fill")
                        .foregroundColor(isValidHex(currentValue, expectedLength: expectedHexLength) ? .green : .red)
                }
            }
            
            // Help text
            Text("Enter a valid \(expectedBytes) byte array in hex format (\(expectedHexLength) characters)")
                .font(.caption2)
                .foregroundColor(.secondary)
            
            // Current status
            if !currentValue.isEmpty {
                HStack {
                    Text("\(currentValue.count)/\(expectedHexLength) characters")
                        .font(.caption2)
                        .foregroundColor(currentValue.count == expectedHexLength ? .green : .orange)
                    
                    Spacer()
                    
                    if currentValue.count == expectedHexLength {
                        Text("✓ Valid hex data")
                            .font(.caption2)
                            .foregroundColor(.green)
                    }
                }
            }
        }
    }
    
    private func isValidHex(_ string: String, expectedLength: Int) -> Bool {
        // Check if string contains only hex characters
        let hexCharacterSet = CharacterSet(charactersIn: "0123456789abcdefABCDEF")
        let stringCharacterSet = CharacterSet(charactersIn: string)
        
        return stringCharacterSet.isSubset(of: hexCharacterSet) && string.count == expectedLength
    }
}
