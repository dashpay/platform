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
    public let isRetryable: Bool

    public init(
        title: String,
        message: String,
        category: ErrorCategory = .unknown,
        recoverySuggestion: String? = nil,
        underlyingError: String? = nil,
        isRetryable: Bool = false
    ) {
        self.title = title
        self.message = message
        self.category = category
        self.recoverySuggestion = recoverySuggestion
        self.underlyingError = underlyingError
        self.isRetryable = isRetryable
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

    /// Format an error with its category prefix
    public static func formatWithCategory(_ error: Error, category: ErrorCategory) -> String {
        let message = formatForDisplay(error)
        return "[\(category.rawValue)] \(message)"
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

    /// Format an error for logging (includes more technical details)
    public static func formatForLogging(_ error: Error, context: String? = nil) -> String {
        var result = ""
        if let ctx = context {
            result += "[\(ctx)] "
        }
        result += String(describing: type(of: error))
        result += ": "
        result += error.localizedDescription

        if let underlying = (error as NSError).userInfo[NSUnderlyingErrorKey] as? Error {
            result += " (underlying: \(underlying.localizedDescription))"
        }

        return result
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

    /// Get recovery suggestion based on error message content
    public static func suggestionFromMessage(_ message: String) -> String? {
        let lowercased = message.lowercased()

        if lowercased.contains("network") || lowercased.contains("connection") {
            return suggestion(for: .network)
        }
        if lowercased.contains("timeout") || lowercased.contains("timed out") {
            return suggestion(for: .timeout)
        }
        if lowercased.contains("invalid") || lowercased.contains("validation") {
            return suggestion(for: .validation)
        }
        if lowercased.contains("not found") || lowercased.contains("notfound") {
            return suggestion(for: .notFound)
        }
        if lowercased.contains("unauthorized") || lowercased.contains("authentication") {
            return suggestion(for: .authentication)
        }
        if lowercased.contains("permission") || lowercased.contains("forbidden") {
            return suggestion(for: .authorization)
        }

        return nil
    }

    /// Determine if an error is retryable based on its category
    public static func isRetryable(category: ErrorCategory) -> Bool {
        switch category {
        case .network, .timeout, .system:
            return true
        case .validation, .userInput, .authentication, .authorization,
             .notFound, .serialization, .cryptography, .storage,
             .configuration, .unknown:
            return false
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
        let isRetryable = ErrorRecovery.isRetryable(category: category)

        return UserFacingError(
            title: category.rawValue,
            message: message,
            category: category,
            recoverySuggestion: suggestion,
            underlyingError: String(describing: error),
            isRetryable: isRetryable
        )
    }
}

// MARK: - Error Builder

/// Fluent builder for creating UserFacingError instances.
public final class ErrorBuilder: @unchecked Sendable {
    private var title: String = "Error"
    private var message: String = ""
    private var category: ErrorCategory = .unknown
    private var recoverySuggestion: String?
    private var underlyingError: String?
    private var isRetryable: Bool = false

    public init() {}

    @discardableResult
    public func withTitle(_ title: String) -> ErrorBuilder {
        self.title = title
        return self
    }

    @discardableResult
    public func withMessage(_ message: String) -> ErrorBuilder {
        self.message = message
        return self
    }

    @discardableResult
    public func withCategory(_ category: ErrorCategory) -> ErrorBuilder {
        self.category = category
        return self
    }

    @discardableResult
    public func withRecoverySuggestion(_ suggestion: String) -> ErrorBuilder {
        self.recoverySuggestion = suggestion
        return self
    }

    @discardableResult
    public func withUnderlyingError(_ error: Error) -> ErrorBuilder {
        self.underlyingError = String(describing: error)
        return self
    }

    @discardableResult
    public func retryable(_ isRetryable: Bool = true) -> ErrorBuilder {
        self.isRetryable = isRetryable
        return self
    }

    public func build() -> UserFacingError {
        UserFacingError(
            title: title,
            message: message,
            category: category,
            recoverySuggestion: recoverySuggestion ?? ErrorRecovery.suggestion(for: category),
            underlyingError: underlyingError,
            isRetryable: isRetryable
        )
    }

    // MARK: - Convenience Factory Methods

    public static func validation(_ message: String) -> UserFacingError {
        ErrorBuilder()
            .withTitle("Validation Error")
            .withMessage(message)
            .withCategory(.validation)
            .build()
    }

    public static func network(_ message: String) -> UserFacingError {
        ErrorBuilder()
            .withTitle("Network Error")
            .withMessage(message)
            .withCategory(.network)
            .retryable()
            .build()
    }

    public static func notFound(_ message: String) -> UserFacingError {
        ErrorBuilder()
            .withTitle("Not Found")
            .withMessage(message)
            .withCategory(.notFound)
            .build()
    }

    public static func timeout(_ message: String = "The request took too long") -> UserFacingError {
        ErrorBuilder()
            .withTitle("Timeout")
            .withMessage(message)
            .withCategory(.timeout)
            .retryable()
            .build()
    }

    public static func authentication(_ message: String) -> UserFacingError {
        ErrorBuilder()
            .withTitle("Authentication Error")
            .withMessage(message)
            .withCategory(.authentication)
            .build()
    }

    public static func userInput(_ message: String) -> UserFacingError {
        ErrorBuilder()
            .withTitle("Input Error")
            .withMessage(message)
            .withCategory(.userInput)
            .build()
    }
}

// MARK: - Result Extensions

public extension Result where Failure == Error {
    /// Convert a Result to a UserFacingError result
    func mapToUserFacingError() -> Result<Success, UserFacingError> {
        mapError { ErrorCategorizer.toUserFacingError($0) }
    }

    /// Get the error message if this is a failure
    var errorMessage: String? {
        if case .failure(let error) = self {
            return ErrorFormatter.formatForDisplay(error)
        }
        return nil
    }
}

// MARK: - Error Aggregator

/// Aggregates multiple errors into a single error representation.
public struct ErrorAggregator: Sendable {
    private var errors: [UserFacingError] = []

    public init() {}

    public mutating func add(_ error: Error) {
        errors.append(ErrorCategorizer.toUserFacingError(error))
    }

    public mutating func add(_ message: String, category: ErrorCategory = .unknown) {
        errors.append(UserFacingError(
            title: category.rawValue,
            message: message,
            category: category
        ))
    }

    public mutating func addValidation(_ message: String) {
        add(message, category: .validation)
    }

    public var hasErrors: Bool {
        !errors.isEmpty
    }

    public var count: Int {
        errors.count
    }

    public var allErrors: [UserFacingError] {
        errors
    }

    public var messages: [String] {
        errors.map { $0.message }
    }

    public var combinedMessage: String {
        ErrorFormatter.formatValidationErrors(messages)
    }

    /// Get the most severe error (by category priority)
    public var primaryError: UserFacingError? {
        // Priority: system > network > authentication > validation > unknown
        let priority: [ErrorCategory] = [
            .system, .cryptography, .storage,
            .network, .timeout,
            .authentication, .authorization,
            .serialization, .configuration,
            .validation, .userInput, .notFound,
            .unknown
        ]

        return errors.min { e1, e2 in
            let p1 = priority.firstIndex(of: e1.category) ?? priority.count
            let p2 = priority.firstIndex(of: e2.category) ?? priority.count
            return p1 < p2
        }
    }

    public mutating func clear() {
        errors.removeAll()
    }
}
