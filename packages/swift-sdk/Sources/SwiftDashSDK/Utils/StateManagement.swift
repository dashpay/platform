import Foundation

// MARK: - Loading State

/// Represents the loading state of an async operation.
public enum LoadingState: Equatable, Sendable {
    case idle
    case loading
    case loaded
    case failed(String)

    public var isLoading: Bool {
        if case .loading = self { return true }
        return false
    }

    public var isIdle: Bool {
        if case .idle = self { return true }
        return false
    }

    public var isLoaded: Bool {
        if case .loaded = self { return true }
        return false
    }

    public var isFailed: Bool {
        if case .failed = self { return true }
        return false
    }

    public var errorMessage: String? {
        if case .failed(let message) = self { return message }
        return nil
    }
}

// MARK: - Error State Helper

/// Helper for managing error state with auto-dismiss.
public struct ErrorState: Equatable, Sendable {
    public var message: String?
    public var showError: Bool

    public init(message: String? = nil, showError: Bool = false) {
        self.message = message
        self.showError = showError
    }

    public var hasError: Bool {
        showError && message != nil
    }

    public mutating func setError(_ message: String) {
        self.message = message
        self.showError = true
    }

    public mutating func clearError() {
        self.message = nil
        self.showError = false
    }
}
