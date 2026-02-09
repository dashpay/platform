import Foundation
import SwiftUI
import SwiftDashSDK

/// Base ViewModel with common loading, error, and result state.
/// Subclass for form/action ViewModels that need consistent UX.
@MainActor
class BaseViewModel: ObservableObject {

    @Published var isLoading = false
    @Published var errorMessage: String?
    @Published var showResult = false
    @Published var currentError: UserFacingError?

    /// Whether the current error is retryable
    var isErrorRetryable: Bool {
        currentError?.isRetryable ?? false
    }

    /// Recovery suggestion for the current error
    var errorRecoverySuggestion: String? {
        currentError?.recoverySuggestion
    }

    /// The error category for the current error
    var errorCategory: ErrorCategory? {
        currentError?.category
    }

    /// Set error message and show result section.
    func handleError(_ error: Error) {
        let userFacing = ErrorCategorizer.toUserFacingError(error)
        currentError = userFacing
        errorMessage = userFacing.message
        showResult = true
    }

    /// Handle error with a custom message
    func handleError(message: String, category: ErrorCategory = .unknown) {
        let userFacing = UserFacingError(
            title: category.rawValue,
            message: message,
            category: category,
            recoverySuggestion: ErrorRecovery.suggestion(for: category)
        )
        currentError = userFacing
        errorMessage = message
        showResult = true
    }

    /// Handle validation errors from an array of error messages
    func handleValidationErrors(_ errors: [String]) {
        guard !errors.isEmpty else { return }
        let message = ErrorFormatter.formatValidationErrors(errors)
        handleError(message: message, category: .validation)
    }

    /// Clear error and result visibility. Override to also clear form fields and result data.
    func reset() {
        errorMessage = nil
        currentError = nil
        showResult = false
    }

    /// Start an async operation with loading state management
    func startLoading() {
        isLoading = true
        errorMessage = nil
        currentError = nil
        showResult = false
    }

    /// Finish loading with success
    func finishLoading() {
        isLoading = false
        showResult = true
    }

    /// Finish loading with an error
    func finishLoading(error: Error) {
        isLoading = false
        handleError(error)
    }

    /// Execute an async operation with automatic loading state management
    func executeAsync<T: Sendable>(
        showResultOnSuccess: Bool = true,
        operation: @Sendable () async throws -> T
    ) async -> T? {
        startLoading()
        do {
            let result = try await operation()
            isLoading = false
            if showResultOnSuccess {
                showResult = true
            }
            return result
        } catch {
            finishLoading(error: error)
            return nil
        }
    }
}
