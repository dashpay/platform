import SwiftUI
import SwiftData
import SwiftDashSDK

struct DocumentFieldsView: View {
    let documentType: PersistentDocumentType
    @Binding var fieldValues: [String: Any]

    /// The per-property freeze a REPLACE must respect (protocol version 14).
    /// `.none` for a create, which writes every property for the first time
    /// and is never refused on these grounds.
    var immutability: DocumentTypeImmutability = .none

    /// Top-level property names the document being replaced already has a
    /// value for. Empty when the form cannot see the stored document, which
    /// leaves a settable-once property editable: setting one that is in fact
    /// already present is the only case consensus would then refuse, and the
    /// caption says so.
    var storedPropertyNames: Set<String> = []

    /// Top-level typed array properties (protocol version 14) by name, read
    /// off the persisted schema once per view value. A typed array gets the
    /// list editor; any other `"array"` row keeps its old editor.
    private let typedArrays: [String: DocumentTypedArray]

    @State private var textFields: [String: String] = [:]
    @State private var numberFields: [String: String] = [:]
    @State private var boolFields: [String: Bool] = [:]
    @State private var arrayFields: [String: String] = [:]
    /// One entry per element of each typed array, in list order, holding the
    /// text entered for it (`"true"` / `"false"` for a boolean toggle, the
    /// chosen member's input text for an `enum` picker).
    @State private var typedArrayRows: [String: [TypedArrayRow]] = [:]
    /// Boolean fields the user has actually toggled. Untouched optional
    /// booleans are omitted from the payload (absence ≠ `false` for some
    /// schemas) rather than broadcast as the seeded `false` default.
    @State private var touchedBoolFields: Set<String> = []

    init(
        documentType: PersistentDocumentType,
        fieldValues: Binding<[String: Any]>,
        immutability: DocumentTypeImmutability = .none,
        storedPropertyNames: Set<String> = []
    ) {
        self.documentType = documentType
        self._fieldValues = fieldValues
        self.immutability = immutability
        self.storedPropertyNames = storedPropertyNames

        var typedArrays: [String: DocumentTypedArray] = [:]
        for property in documentType.propertiesList ?? []
        where property.type == "array" && !property.byteArray {
            typedArrays[property.name] = documentType.typedArray(named: property.name)
        }
        self.typedArrays = typedArrays
    }

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
        // Frozen properties are locked rather than validated on submit: a
        // replace that touches one is refused by consensus with
        // `DocumentImmutablePropertyChangedError` (code 40128), and the
        // rejected transition is still paid for.
        let lock = immutability.lockState(
            for: property.name,
            hasStoredValue: storedPropertyNames.contains(property.name)
        )

        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(property.name)
                    .font(.subheadline)
                    .fontWeight(.medium)
                if property.isRequired {
                    Text("*")
                        .foregroundColor(.red)
                }
                if lock != .editable {
                    Text(lock == .frozen ? "Immutable" : "Immutable once set")
                        .font(.caption2)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.orange.opacity(0.2))
                        .foregroundColor(.orange)
                        .cornerRadius(4)
                        .accessibilityIdentifier("createDocument.lock.\(property.name)")
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
                        .accessibilityIdentifier("createDocument.field.\(property.name)")
                    Text("Enter a valid base58 identifier (e.g., 4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF)")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            } else {
                switch property.type {
            case "string":
                TextField(placeholderText(for: property), text: binding(for: property.name, in: $textFields))
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .accessibilityIdentifier("createDocument.field.\(property.name)")

            case "number", "integer":
                TextField(placeholderText(for: property), text: binding(for: property.name, in: $numberFields))
                    .keyboardType(.numberPad)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .accessibilityIdentifier("createDocument.field.\(property.name)")

            case "boolean":
                Toggle(isOn: boolBinding(for: property.name)) {
                    Text("")
                }
                .labelsHidden()
                .accessibilityLabel(property.name)
                .accessibilityIdentifier("createDocument.field.\(property.name)")

            case "array":
                if property.byteArray {
                    // Byte arrays should be entered as hex strings
                    byteArrayField(for: property)
                } else if let typedArray = typedArrays[property.name] {
                    // Typed arrays: one typed input per element
                    typedArrayField(for: property, typedArray: typedArray)
                } else {
                    // Regular arrays with comma-separated values
                    VStack(alignment: .leading, spacing: 4) {
                        TextField("Enter comma-separated values", text: binding(for: property.name, in: $arrayFields))
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                            .accessibilityIdentifier("createDocument.field.\(property.name)")
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
                    .accessibilityIdentifier("createDocument.field.\(property.name)")

                default:
                    TextField("Enter \(property.name)", text: binding(for: property.name, in: $textFields))
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                        .accessibilityIdentifier("createDocument.field.\(property.name)")
                }
            }

            if let description = property.fieldDescription {
                Text(description)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }

            if lock == .frozen {
                Text("Frozen at creation: a replace cannot change, add or remove it.")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            } else if lock == .settableOnce {
                Text("May be set once while the stored document has no value for it.")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .disabled(lock == .frozen)
        .opacity(lock == .frozen ? 0.6 : 1)
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

    /// Boolean binding that records a user toggle, so untouched optional
    /// booleans can be omitted from the payload (see `touchedBoolFields`).
    private func boolBinding(for key: String) -> Binding<Bool> {
        Binding(
            get: { boolFields[key] ?? false },
            set: {
                boolFields[key] = $0
                touchedBoolFields.insert(key)
                updateFieldValues()
            }
        )
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
                    } else if typedArrays[property.name] != nil {
                        typedArrayRows[property.name] = []  // One row per element
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

        // Add boolean fields. Unlike the other types (which skip empty
        // input), a seeded boolean has no "empty" state — so only include
        // it if it's required or the user actually toggled it. This keeps
        // untouched optional booleans absent from the payload instead of
        // broadcasting the seeded `false` (absence ≠ `false` for some
        // schemas).
        for (key, value) in boolFields {
            let isRequired = documentType.propertiesList?
                .first(where: { $0.name == key })?.isRequired ?? false
            if isRequired || touchedBoolFields.contains(key) {
                values[key] = value
            }
        }

        // Add array fields
        for (key, value) in arrayFields {
            if !value.isEmpty {
                let items = value.split(separator: ",").map { String($0.trimmingCharacters(in: .whitespaces)) }
                values[key] = items
            }
        }

        // Add typed arrays: one JSON value per row, in row order. An empty
        // optional list is left out; every other list goes through the
        // whole-list check, `minItems` included. A list that fails it is
        // represented by the `DocumentTypedArray.InputError` itself, which
        // `CreateDocumentView.propertiesJSON` throws: a refused document
        // transition is still paid for, so an invalid list must never be
        // broadcast.
        for (key, rows) in typedArrayRows {
            guard let typedArray = typedArrays[key] else { continue }
            if rows.isEmpty && !isRequired(key) {
                continue
            }
            switch typedArray.jsonArray(fromInputs: rows.map(\.text)) {
            case .success(let array):
                values[key] = array
            case .failure(let error):
                values[key] = error
            }
        }

        fieldValues = values
    }

    private func isRequired(_ propertyName: String) -> Bool {
        documentType.propertiesList?.first(where: { $0.name == propertyName })?.isRequired ?? false
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
                    .accessibilityIdentifier("createDocument.field.\(property.name)")
                    .onChange(of: currentValue) { _, newValue in
                        // Remove any non-hex characters and convert to lowercase
                        let cleaned = newValue.lowercased().filter { "0123456789abcdef".contains($0) }
                        if cleaned != newValue {
                            textFields[property.name] = cleaned
                            // Direct @State mutation bypasses the binding's
                            // setter, so re-sync the cleaned value into the
                            // submitted `fieldValues`.
                            updateFieldValues()
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

// MARK: - Typed Array Field Helper

/// One element row of a typed array editor. Rows are identified by `id`, not
/// by position, so a binding captured before a removal cannot write into the
/// wrong row.
struct TypedArrayRow: Identifiable, Equatable {
    let id = UUID()
    var text: String
}

extension DocumentFieldsView {
    /// The list editor for a typed array (protocol version 14): one row per
    /// element with an input suited to the element, an add button that stops
    /// at `maxItems`, a remove button per row, and a caption stating what an
    /// element is and how many the list takes. Each row is marked with the
    /// verdict of `DocumentTypedArray.Element.value(fromInput:)`, and a list
    /// failing `DocumentTypedArray.values(fromInputs:)` is never sent: the
    /// submit refuses it (see `updateFieldValues`).
    @ViewBuilder
    private func typedArrayField(for property: PersistentProperty, typedArray: DocumentTypedArray) -> some View {
        let name = property.name
        let element = typedArray.element
        let rows = typedArrayRows[name] ?? []
        let results = rows.map { element.value(fromInput: $0.text) }
        // The whole-list refusal that `updateFieldValues` sends in place of
        // the list. A bad row is already marked on its row, so only the count
        // and repeat refusals are shown here. An empty optional list is left
        // out of the document and is never checked.
        let listError: DocumentTypedArray.InputError? = {
            guard !rows.isEmpty || property.isRequired,
                  case let .failure(error) = typedArray.values(fromInputs: rows.map(\.text))
            else {
                return nil
            }
            if case .invalidElement = error { return nil }
            return error
        }()

        VStack(alignment: .leading, spacing: 6) {
            ForEach(Array(rows.enumerated()), id: \.element.id) { index, row in
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 8) {
                        // The index a refusal names (`tags[1]`)
                        Text("[\(index)]")
                            .font(.system(.caption, design: .monospaced))
                            .foregroundColor(.secondary)

                        typedArrayElementInput(name: name, index: index, rowId: row.id, element: element)

                        if case .failure = results[index] {
                            Image(systemName: "exclamationmark.circle.fill")
                                .foregroundColor(.red)
                                .accessibilityLabel("Invalid \(name)[\(index)]")
                                .accessibilityIdentifier("createDocument.field.\(name).\(index).invalid")
                        }

                        Button {
                            removeTypedArrayRow(name, id: row.id)
                        } label: {
                            Image(systemName: "minus.circle.fill")
                                .foregroundColor(.red)
                        }
                        // Borderless: the whole editor sits in one Form row,
                        // where a default button would claim every tap in it
                        .buttonStyle(.borderless)
                        .accessibilityLabel("Remove \(name)[\(index)]")
                        .accessibilityIdentifier("createDocument.field.\(name).\(index).remove")
                    }

                    if case let .failure(error) = results[index] {
                        Text(error.localizedDescription)
                            .font(.caption2)
                            .foregroundColor(.red)
                    }
                }
            }

            Button {
                addTypedArrayRow(name, typedArray: typedArray)
            } label: {
                Label("Add element", systemImage: "plus.circle")
                    .font(.subheadline)
            }
            .buttonStyle(.borderless)
            .disabled(rows.count >= typedArray.maxItems)
            .accessibilityIdentifier("createDocument.field.\(name).add")

            Text("Each element: \(element.summary)")
                .font(.caption2)
                .foregroundColor(.secondary)

            Text(typedArrayCountCaption(typedArray, count: rows.count))
                .font(.caption2)
                .foregroundColor(.secondary)
                .accessibilityIdentifier("createDocument.field.\(name).count")

            if let listError {
                Text(listError.localizedDescription)
                    .font(.caption2)
                    .foregroundColor(.orange)
                    .accessibilityIdentifier("createDocument.field.\(name).error")
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("createDocument.field.\(name)")
    }

    /// The input for one element, chosen by the element kind: a picker when
    /// an `enum` is declared, a toggle for a boolean, and otherwise a text
    /// field with the keyboard the kind needs.
    @ViewBuilder
    private func typedArrayElementInput(
        name: String,
        index: Int,
        rowId: UUID,
        element: DocumentTypedArray.Element
    ) -> some View {
        let identifier = "createDocument.field.\(name).\(index)"
        let text = typedArrayTextBinding(name: name, rowId: rowId)

        if let options = element.allowedInputs {
            Picker("\(name)[\(index)]", selection: text) {
                ForEach(Array(options.enumerated()), id: \.offset) { _, option in
                    Text(option).tag(option)
                }
            }
            .pickerStyle(.menu)
            .labelsHidden()
            .accessibilityIdentifier(identifier)
            Spacer()
        } else {
            switch element {
            case .boolean:
                Toggle(isOn: Binding(
                    get: { text.wrappedValue == "true" },
                    set: { text.wrappedValue = $0 ? "true" : "false" }
                )) {
                    Text(text.wrappedValue)
                        .font(.subheadline)
                }
                .accessibilityLabel("\(name)[\(index)]")
                .accessibilityIdentifier(identifier)

            case let .integer(minimum, _, _):
                TextField("Whole number", text: text)
                    // The number pad has no minus key: offer it only when
                    // the element cannot be negative
                    .keyboardType((minimum ?? -1) >= 0 ? .numberPad : .numbersAndPunctuation)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .accessibilityIdentifier(identifier)

            case let .number(minimum, _, _):
                TextField("Number", text: text)
                    .keyboardType((minimum ?? -1) >= 0 ? .decimalPad : .numbersAndPunctuation)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .accessibilityIdentifier(identifier)

            case .string:
                TextField("Text", text: text)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .accessibilityIdentifier(identifier)

            case .byteArray:
                TextField("Hex bytes", text: text)
                    .font(.system(.body, design: .monospaced))
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .keyboardType(.asciiCapable)
                    .accessibilityIdentifier(identifier)

            case .identifier:
                TextField("Base58 identifier", text: text)
                    .font(.system(.body, design: .monospaced))
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .keyboardType(.asciiCapable)
                    .accessibilityIdentifier(identifier)
            }
        }
    }

    /// "2 of 1 to 5 elements", plus the uniqueness rule when declared.
    private func typedArrayCountCaption(_ typedArray: DocumentTypedArray, count: Int) -> String {
        var caption: String
        if let minItems = typedArray.minItems, minItems > 0 {
            caption = minItems == typedArray.maxItems
                ? "\(count) of exactly \(minItems) elements"
                : "\(count) of \(minItems) to \(typedArray.maxItems) elements"
        } else {
            caption = "\(count) of at most \(typedArray.maxItems) elements"
        }
        if typedArray.uniqueItems {
            caption += " · elements must be unique"
        }
        return caption
    }

    /// Binds one row's text by row id, so a stale binding after a removal
    /// writes nothing rather than into another row.
    private func typedArrayTextBinding(name: String, rowId: UUID) -> Binding<String> {
        Binding(
            get: { typedArrayRows[name]?.first(where: { $0.id == rowId })?.text ?? "" },
            set: { newValue in
                guard let index = typedArrayRows[name]?.firstIndex(where: { $0.id == rowId }) else {
                    return
                }
                typedArrayRows[name]?[index].text = newValue
                updateFieldValues()
            }
        )
    }

    /// A new row starts on the first allowed value when an `enum` is declared
    /// (a picker must select one of its tags), `false` for a boolean, and
    /// empty otherwise.
    private func addTypedArrayRow(_ name: String, typedArray: DocumentTypedArray) {
        var rows = typedArrayRows[name] ?? []
        guard rows.count < typedArray.maxItems else { return }
        let initialText: String
        if let first = typedArray.element.allowedInputs?.first {
            initialText = first
        } else if case .boolean = typedArray.element {
            initialText = "false"
        } else {
            initialText = ""
        }
        rows.append(TypedArrayRow(text: initialText))
        typedArrayRows[name] = rows
        updateFieldValues()
    }

    private func removeTypedArrayRow(_ name: String, id: UUID) {
        typedArrayRows[name]?.removeAll { $0.id == id }
        updateFieldValues()
    }
}

// MARK: - Typed Array Descriptions

extension DocumentTypedArray.Element {
    /// The element kind with its declared bounds and allowed values, for
    /// captions and schema detail rows: "integer (1 to 10), one of 1, 5, 10".
    var summary: String {
        typealias Value = DocumentTypedArray.ElementValue
        var text: String
        switch self {
        case let .integer(minimum, maximum, _):
            text = "integer" + Self.bounds(
                minimum.map { Value.integer($0).inputText },
                maximum.map { Value.integer($0).inputText },
                unit: nil)
        case let .number(minimum, maximum, _):
            text = "number" + Self.bounds(
                minimum.map { Value.number($0).inputText },
                maximum.map { Value.number($0).inputText },
                unit: nil)
        case .boolean:
            text = "boolean"
        case let .string(minLength, maxLength, _):
            text = "string" + Self.bounds(
                minLength.map(String.init), maxLength.map(String.init), unit: "characters")
        case let .byteArray(minSize, maxSize):
            text = "byte array, hex" + Self.bounds(
                minSize.map(String.init), maxSize.map(String.init), unit: "bytes")
        case .identifier:
            text = "identifier, base58"
        }
        if let allowed = allowedInputs {
            text += ", one of " + allowed.map { $0.isEmpty ? "\"\"" : $0 }.joined(separator: ", ")
        }
        return text
    }

    private static func bounds(_ minimum: String?, _ maximum: String?, unit: String?) -> String {
        let suffix = unit.map { " \($0)" } ?? ""
        switch (minimum, maximum) {
        case let (minimum?, maximum?) where minimum == maximum:
            return " (exactly \(minimum)\(suffix))"
        case let (minimum?, maximum?):
            return " (\(minimum) to \(maximum)\(suffix))"
        case let (minimum?, nil):
            return " (at least \(minimum)\(suffix))"
        case let (nil, maximum?):
            return " (at most \(maximum)\(suffix))"
        case (nil, nil):
            return ""
        }
    }
}

extension DocumentTypedArray {
    /// "list of integer (1 to 10), 1 to 5 elements, unique": the whole
    /// declaration in one line, for schema detail views.
    var summary: String {
        var text = "list of \(element.summary)"
        if let minItems, minItems > 0 {
            text += minItems == maxItems
                ? "; exactly \(maxItems) elements"
                : "; \(minItems) to \(maxItems) elements"
        } else {
            text += "; at most \(maxItems) elements"
        }
        if uniqueItems {
            text += ", unique"
        }
        return text
    }
}
