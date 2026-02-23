import Foundation

// MARK: - Data Models

public struct TransitionDefinition: Sendable {
    public let key: String
    public let label: String
    public let description: String
    public let inputs: [TransitionInput]

    public init(key: String, label: String, description: String, inputs: [TransitionInput]) {
        self.key = key
        self.label = label
        self.description = description
        self.inputs = inputs
    }
}

public struct TransitionInput: Sendable {
    public let name: String
    public let type: String
    public let label: String
    public let required: Bool
    public let placeholder: String?
    public let help: String?
    public let defaultValue: String?
    public let options: [SelectOption]?
    public let action: String?
    public let min: Int?
    public let max: Int?

    public init(
        name: String,
        type: String,
        label: String,
        required: Bool,
        placeholder: String? = nil,
        help: String? = nil,
        defaultValue: String? = nil,
        options: [SelectOption]? = nil,
        action: String? = nil,
        min: Int? = nil,
        max: Int? = nil
    ) {
        self.name = name
        self.type = type
        self.label = label
        self.required = required
        self.placeholder = placeholder
        self.help = help
        self.defaultValue = defaultValue
        self.options = options
        self.action = action
        self.min = min
        self.max = max
    }
}

public struct SelectOption: Sendable {
    public let value: String
    public let label: String

    public init(value: String, label: String) {
        self.value = value
        self.label = label
    }
}
