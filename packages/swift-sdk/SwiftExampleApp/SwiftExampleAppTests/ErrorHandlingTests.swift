// ErrorHandlingTests.swift
// SwiftExampleAppTests
//
// Unit tests for centralized error handling utilities.

import XCTest
@testable import SwiftDashSDK

final class ErrorHandlingTests: XCTestCase {

    // MARK: - ErrorCategory Tests

    func testErrorCategoryIsUserRecoverable() {
        // Recoverable
        XCTAssertTrue(ErrorCategory.validation.isUserRecoverable)
        XCTAssertTrue(ErrorCategory.userInput.isUserRecoverable)
        XCTAssertTrue(ErrorCategory.authentication.isUserRecoverable)
        XCTAssertTrue(ErrorCategory.timeout.isUserRecoverable)
        XCTAssertTrue(ErrorCategory.network.isUserRecoverable)

        // Not recoverable
        XCTAssertFalse(ErrorCategory.authorization.isUserRecoverable)
        XCTAssertFalse(ErrorCategory.serialization.isUserRecoverable)
        XCTAssertFalse(ErrorCategory.cryptography.isUserRecoverable)
        XCTAssertFalse(ErrorCategory.system.isUserRecoverable)
    }

    func testErrorCategoryShouldLog() {
        // Should not log user input errors
        XCTAssertFalse(ErrorCategory.validation.shouldLog)
        XCTAssertFalse(ErrorCategory.userInput.shouldLog)

        // Should log system errors
        XCTAssertTrue(ErrorCategory.network.shouldLog)
        XCTAssertTrue(ErrorCategory.system.shouldLog)
        XCTAssertTrue(ErrorCategory.cryptography.shouldLog)
    }

    // MARK: - UserFacingError Tests

    func testUserFacingErrorCreation() {
        let error = UserFacingError(
            title: "Test Error",
            message: "Something went wrong",
            category: .network,
            recoverySuggestion: "Try again",
            underlyingError: "Original error",
        )

        XCTAssertEqual(error.title, "Test Error")
        XCTAssertEqual(error.message, "Something went wrong")
        XCTAssertEqual(error.category, .network)
        XCTAssertEqual(error.recoverySuggestion, "Try again")
        XCTAssertEqual(error.underlyingError, "Original error")
    }

    func testUserFacingErrorLocalizedError() {
        let error = UserFacingError(
            title: "Title",
            message: "Message",
            category: .validation,
            recoverySuggestion: "Suggestion"
        )

        XCTAssertEqual(error.errorDescription, "Message")
        XCTAssertEqual(error.failureReason, "Title")
        XCTAssertEqual(error.localizedRecoverySuggestion, "Suggestion")
    }

    func testUserFacingErrorDisplayText() {
        let error = UserFacingError(
            title: "Network Error",
            message: "Connection failed"
        )

        XCTAssertEqual(error.displayText, "Network Error: Connection failed")
    }

    func testUserFacingErrorFullDescription() {
        let error = UserFacingError(
            title: "Error",
            message: "Failed",
            recoverySuggestion: "Try again"
        )

        XCTAssertEqual(error.fullDescription, "Error: Failed\nTry again")
    }

    func testUserFacingErrorFullDescriptionWithoutSuggestion() {
        let error = UserFacingError(
            title: "Error",
            message: "Failed"
        )

        XCTAssertEqual(error.fullDescription, "Error: Failed")
    }

    func testUserFacingErrorEquatable() {
        let error1 = UserFacingError(title: "A", message: "B", category: .network)
        let error2 = UserFacingError(title: "A", message: "B", category: .network)
        let error3 = UserFacingError(title: "C", message: "D", category: .network)

        XCTAssertEqual(error1, error2)
        XCTAssertNotEqual(error1, error3)
    }

    // MARK: - ErrorFormatter Tests

    func testErrorFormatterFormatForDisplay() {
        let userFacing = UserFacingError(title: "Title", message: "Message")
        XCTAssertEqual(ErrorFormatter.formatForDisplay(userFacing), "Title: Message")
    }

    func testErrorFormatterFormatValidationErrorsSingle() {
        let errors = ["Invalid email address"]
        let formatted = ErrorFormatter.formatValidationErrors(errors)
        XCTAssertEqual(formatted, "Invalid email address")
    }

    func testErrorFormatterFormatValidationErrorsMultiple() {
        let errors = ["Error 1", "Error 2", "Error 3"]
        let formatted = ErrorFormatter.formatValidationErrors(errors)
        XCTAssertEqual(formatted, "1. Error 1\n2. Error 2\n3. Error 3")
    }

    func testErrorFormatterFormatValidationErrorsEmpty() {
        let formatted = ErrorFormatter.formatValidationErrors([])
        XCTAssertEqual(formatted, "")
    }

    // MARK: - ErrorRecovery Tests

    func testErrorRecoverySuggestionForCategory() {
        let networkSuggestion = ErrorRecovery.suggestion(for: .network)
        XCTAssertTrue(networkSuggestion.contains("internet") || networkSuggestion.contains("connection"))

        let validationSuggestion = ErrorRecovery.suggestion(for: .validation)
        XCTAssertTrue(validationSuggestion.contains("check") || validationSuggestion.contains("input"))

        let timeoutSuggestion = ErrorRecovery.suggestion(for: .timeout)
        XCTAssertTrue(timeoutSuggestion.contains("long") || timeoutSuggestion.contains("again"))
    }

    // MARK: - ErrorCategorizer Tests

    func testErrorCategorizerCategorize() {
        struct ValidationError: Error {}
        struct NetworkError: Error {}

        // UserFacingError returns its own category
        let userFacing = UserFacingError(title: "T", message: "M", category: .storage)
        XCTAssertEqual(ErrorCategorizer.categorize(userFacing), .storage)
    }

    func testErrorCategorizerToUserFacingError() {
        struct TestError: Error, LocalizedError {
            var errorDescription: String? { "Test error message" }
        }

        let userFacing = ErrorCategorizer.toUserFacingError(TestError())

        XCTAssertEqual(userFacing.message, "Test error message")
        XCTAssertNotNil(userFacing.recoverySuggestion)
    }

    func testErrorCategorizerPreservesUserFacingError() {
        let original = UserFacingError(
            title: "Original",
            message: "Keep this",
            category: .cryptography
        )

        let result = ErrorCategorizer.toUserFacingError(original)
        XCTAssertEqual(result.title, "Original")
        XCTAssertEqual(result.message, "Keep this")
        XCTAssertEqual(result.category, .cryptography)
    }
}
