import Foundation

// MARK: - Data Models

struct TransitionDefinition {
    let key: String
    let label: String
    let description: String
    let inputs: [TransitionInput]
}

struct TransitionInput {
    let name: String
    let type: String
    let label: String
    let required: Bool
    let placeholder: String?
    let help: String?
    let defaultValue: String?
    let options: [SelectOption]?
    let action: String?
    let min: Int?
    let max: Int?
    
    init(
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

struct SelectOption {
    let value: String
    let label: String
}