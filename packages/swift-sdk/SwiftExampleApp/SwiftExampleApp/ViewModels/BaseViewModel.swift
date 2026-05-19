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

    /// Unified loading state for more granular state tracking.
    @Published var loadingState: LoadingState = .idle

    /// Error state with show/hide management.
    @Published var errorState = ErrorState()

    /// Set error message and show result section.
    func handleError(_ error: Error) {
        let userFacing = ErrorCategorizer.toUserFacingError(error)
        currentError = userFacing
        errorMessage = userFacing.message
        showResult = true
        loadingState = .failed(error.localizedDescription)
        errorState.setError(error.localizedDescription)
    }

    /// Set error from a string message.
    func handleError(message: String) {
        errorMessage = message
        showResult = true
        loadingState = .failed(message)
        errorState.setError(message)
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
        loadingState = .idle
        errorState.clearError()
    }

    /// Start loading state.
    func startLoading() {
        isLoading = true
        currentError = nil
        showResult = false
        loadingState = .loading
        errorMessage = nil
    }

    /// Finish loading with success.
    func finishLoading() {
        isLoading = false
        loadingState = .loaded
    }

    /// Finish loading with error.
    func finishLoading(error: Error) {
        isLoading = false
        showResult = true
        handleError(error)
    }

    /// Finish loading with error message.
    func finishLoading(errorMessage: String) {
        isLoading = false
        handleError(message: errorMessage)
    }

    /// Execute an async operation with automatic state management.
    /// - Parameters:
    ///   - showResultOnSuccess: Whether to set showResult = true on success.
    ///   - operation: The async operation to execute.
    /// - Returns: The result of the operation, or nil if it failed.
    @discardableResult
    func executeAsync<T: Sendable>(
        showResultOnSuccess: Bool = true,
        operation: () async throws -> T
    ) async -> T? {
        startLoading()
        do {
            let result = try await operation()
            finishLoading()
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
