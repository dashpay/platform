// ErrorHandling.swift
// SwiftDashSDK
//
// Centralized error handling utilities for consistent error management across the SDK.

import Foundation

// MARK: - Error Categories

/// Categories for classifying errors by their nature and handling requirements.
public enum ErrorCategory: String, Sendable, CaseIterable {
    case validation = "Validation"
    case network = "Network"
    case authentication = "Authentication"
    case authorization = "Authorization"
    case notFound = "Not Found"
    case timeout = "Timeout"
    case serialization = "Serialization"
    case cryptography = "Cryptography"
    case storage = "Storage"
    case configuration = "Configuration"
    case userInput = "User Input"
    case system = "System"
    case unknown = "Unknown"

    /// Whether this category of error is typically recoverable by the user
    public var isUserRecoverable: Bool {
        switch self {
        case .validation, .userInput, .authentication, .timeout:
            return true
        case .network, .notFound, .configuration:
            return true
        case .authorization, .serialization, .cryptography, .storage, .system, .unknown:
            return false
        }
    }

    /// Whether this error should be logged for debugging
    public var shouldLog: Bool {
        switch self {
        case .validation, .userInput:
            return false
        default:
            return true
        }
    }
}

// MARK: - User Facing Error

/// A user-friendly error representation with recovery suggestions.
public struct UserFacingError: Error, LocalizedError, Sendable, Equatable {
    public let title: String
    public let message: String
    public let category: ErrorCategory
    public let recoverySuggestion: String?
    public let underlyingError: String?

    public init(
        title: String,
        message: String,
        category: ErrorCategory = .unknown,
        recoverySuggestion: String? = nil,
        underlyingError: String? = nil,
    ) {
        self.title = title
        self.message = message
        self.category = category
        self.recoverySuggestion = recoverySuggestion
        self.underlyingError = underlyingError
    }

    public var errorDescription: String? {
        message
    }

    public var failureReason: String? {
        title
    }

    public var localizedRecoverySuggestion: String? {
        recoverySuggestion
    }

    /// Formatted display string combining title and message
    public var displayText: String {
        "\(title): \(message)"
    }

    /// Full description including recovery suggestion if available
    public var fullDescription: String {
        var result = displayText
        if let suggestion = recoverySuggestion {
            result += "\n\(suggestion)"
        }
        return result
    }
}

// MARK: - Error Formatter

/// Utilities for formatting error messages consistently.
public enum ErrorFormatter {

    /// Format an error for user display
    public static func formatForDisplay(_ error: Error) -> String {
        if let userFacing = error as? UserFacingError {
            return userFacing.displayText
        }
        if let localized = error as? LocalizedError, let description = localized.errorDescription {
            return description
        }
        return error.localizedDescription
    }

    /// Extract a user-friendly message from any error
    public static func userFriendlyMessage(from error: Error) -> String {
        // Handle specific SDK error types
        let errorString = String(describing: error)

        // Clean up common error patterns
        if errorString.contains("invalidParameter") {
            return extractMessage(from: errorString, prefix: "Invalid parameter")
        }
        if errorString.contains("networkError") {
            return extractMessage(from: errorString, prefix: "Network error")
        }
        if errorString.contains("notFound") {
            return extractMessage(from: errorString, prefix: "Not found")
        }
        if errorString.contains("timeout") {
            return extractMessage(from: errorString, prefix: "Request timed out")
        }

        return formatForDisplay(error)
    }

    /// Extract message from error string with associated value
    private static func extractMessage(from errorString: String, prefix: String) -> String {
        // Try to extract the message from pattern like "errorType(\"message\")"
        if let start = errorString.firstIndex(of: "("),
           let end = errorString.lastIndex(of: ")") {
            let messageStart = errorString.index(after: start)
            var message = String(errorString[messageStart..<end])
            // Remove quotes if present
            message = message.trimmingCharacters(in: CharacterSet(charactersIn: "\""))
            return "\(prefix): \(message)"
        }
        return prefix
    }

    /// Format validation errors into a single message
    public static func formatValidationErrors(_ errors: [String]) -> String {
        guard !errors.isEmpty else { return "" }
        if errors.count == 1 {
            return errors[0]
        }
        return errors.enumerated()
            .map { "\($0.offset + 1). \($0.element)" }
            .joined(separator: "\n")
    }
}

// MARK: - Error Recovery

/// Provides recovery suggestions for common error types.
public enum ErrorRecovery {
    /// Get recovery suggestion for an error category
    public static func suggestion(for category: ErrorCategory) -> String {
        switch category {
        case .validation:
            return "Please check your input and try again."
        case .network:
            return "Please check your internet connection and try again."
        case .authentication:
            return "Please verify your credentials and try again."
        case .authorization:
            return "You don't have permission to perform this action."
        case .notFound:
            return "The requested item could not be found."
        case .timeout:
            return "The request took too long. Please try again."
        case .serialization:
            return "There was a problem processing the data."
        case .cryptography:
            return "There was a security-related error."
        case .storage:
            return "There was a problem accessing storage."
        case .configuration:
            return "Please check your configuration settings."
        case .userInput:
            return "Please correct the highlighted fields."
        case .system:
            return "A system error occurred. Please try again later."
        case .unknown:
            return "An unexpected error occurred. Please try again."
        }
    }
}

// MARK: - Error Categorizer

/// Utilities for categorizing errors.
public enum ErrorCategorizer {

    /// Categorize an error based on its type and message
    public static func categorize(_ error: Error) -> ErrorCategory {
        // Check if it's already a UserFacingError
        if let userFacing = error as? UserFacingError {
            return userFacing.category
        }

        let errorString = String(describing: error).lowercased()
        let description = error.localizedDescription.lowercased()

        // Check error string patterns
        if errorString.contains("validation") || description.contains("invalid") {
            return .validation
        }
        if errorString.contains("network") || description.contains("network") ||
           description.contains("connection") {
            return .network
        }
        if errorString.contains("timeout") || description.contains("timeout") {
            return .timeout
        }
        if errorString.contains("notfound") || description.contains("not found") {
            return .notFound
        }
        if errorString.contains("authentication") || errorString.contains("credential") {
            return .authentication
        }
        if errorString.contains("authorization") || errorString.contains("permission") {
            return .authorization
        }
        if errorString.contains("serialization") || errorString.contains("encoding") ||
           errorString.contains("decoding") {
            return .serialization
        }
        if errorString.contains("crypto") || errorString.contains("signing") ||
           errorString.contains("encryption") {
            return .cryptography
        }
        if errorString.contains("storage") || errorString.contains("keychain") ||
           errorString.contains("database") {
            return .storage
        }
        if errorString.contains("configuration") || errorString.contains("config") {
            return .configuration
        }

        return .unknown
    }

    /// Create a UserFacingError from any error
    public static func toUserFacingError(_ error: Error) -> UserFacingError {
        if let userFacing = error as? UserFacingError {
            return userFacing
        }

        let category = categorize(error)
        let message = ErrorFormatter.userFriendlyMessage(from: error)
        let suggestion = ErrorRecovery.suggestion(for: category)

        return UserFacingError(
            title: category.rawValue,
            message: message,
            category: category,
            recoverySuggestion: suggestion,
            underlyingError: String(describing: error),
        )
    }
}
