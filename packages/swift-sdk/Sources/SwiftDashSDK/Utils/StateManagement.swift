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

// MARK: - Result State

/// A generic result state that can hold either a success value or an error.
public enum ResultState<T: Sendable>: Sendable {
    case idle
    case loading
    case success(T)
    case failure(String)

    public var isLoading: Bool {
        if case .loading = self { return true }
        return false
    }

    public var isSuccess: Bool {
        if case .success = self { return true }
        return false
    }

    public var isFailure: Bool {
        if case .failure = self { return true }
        return false
    }

    public var value: T? {
        if case .success(let value) = self { return value }
        return nil
    }

    public var error: String? {
        if case .failure(let error) = self { return error }
        return nil
    }

    /// Map the success value to a new type.
    public func map<U: Sendable>(_ transform: (T) -> U) -> ResultState<U> {
        switch self {
        case .idle:
            return .idle
        case .loading:
            return .loading
        case .success(let value):
            return .success(transform(value))
        case .failure(let error):
            return .failure(error)
        }
    }
}

// MARK: - Async Operation Result

/// Result of an async operation with timing information.
public struct AsyncOperationResult<T: Sendable>: Sendable {
    public let value: T?
    public let error: String?
    public let duration: TimeInterval
    public let isSuccess: Bool

    public init(value: T, duration: TimeInterval) {
        self.value = value
        self.error = nil
        self.duration = duration
        self.isSuccess = true
    }

    public init(error: String, duration: TimeInterval) {
        self.value = nil
        self.error = error
        self.duration = duration
        self.isSuccess = false
    }

    public init(error: Error, duration: TimeInterval) {
        self.value = nil
        self.error = error.localizedDescription
        self.duration = duration
        self.isSuccess = false
    }
}

// MARK: - Async Operation Helper

/// Helper for running async operations with consistent state management.
public enum AsyncOperation {

    /// Run an async operation and return the result with timing.
    /// - Parameters:
    ///   - operation: The async operation to run.
    /// - Returns: AsyncOperationResult with value or error and timing.
    public static func run<T: Sendable>(_ operation: () async throws -> T) async -> AsyncOperationResult<T> {
        let startTime = Date()
        do {
            let result = try await operation()
            let duration = Date().timeIntervalSince(startTime)
            return AsyncOperationResult(value: result, duration: duration)
        } catch {
            let duration = Date().timeIntervalSince(startTime)
            return AsyncOperationResult(error: error, duration: duration)
        }
    }

    /// Run an async operation with callbacks for state updates.
    /// - Parameters:
    ///   - onStart: Called when operation starts.
    ///   - onSuccess: Called with value on success.
    ///   - onError: Called with error message on failure.
    ///   - onComplete: Called when operation completes (success or failure).
    ///   - operation: The async operation to run.
    @MainActor
    public static func execute<T: Sendable>(
        onStart: (() -> Void)? = nil,
        onSuccess: ((T) -> Void)? = nil,
        onError: ((String) -> Void)? = nil,
        onComplete: (() -> Void)? = nil,
        operation: () async throws -> T
    ) async {
        onStart?()
        do {
            let result = try await operation()
            onSuccess?(result)
        } catch {
            onError?(error.localizedDescription)
        }
        onComplete?()
    }
}

// MARK: - Form State

/// Represents the state of a form submission.
public enum FormState: Equatable, Sendable {
    case editing
    case submitting
    case submitted
    case error(String)

    public var isSubmitting: Bool {
        if case .submitting = self { return true }
        return false
    }

    public var isSubmitted: Bool {
        if case .submitted = self { return true }
        return false
    }

    public var hasError: Bool {
        if case .error = self { return true }
        return false
    }

    public var errorMessage: String? {
        if case .error(let message) = self { return message }
        return nil
    }

    public var canSubmit: Bool {
        switch self {
        case .editing, .error:
            return true
        case .submitting, .submitted:
            return false
        }
    }
}

// MARK: - Pagination State

/// Represents pagination state for list views.
public struct PaginationState: Equatable, Sendable {
    public var currentPage: Int
    public var totalPages: Int
    public var isLoadingMore: Bool
    public var hasMore: Bool

    public init(currentPage: Int = 0, totalPages: Int = 0, isLoadingMore: Bool = false, hasMore: Bool = true) {
        self.currentPage = currentPage
        self.totalPages = totalPages
        self.isLoadingMore = isLoadingMore
        self.hasMore = hasMore
    }

    public var canLoadMore: Bool {
        !isLoadingMore && hasMore
    }

    public mutating func startLoadingMore() {
        isLoadingMore = true
    }

    public mutating func finishLoadingMore(hasMore: Bool) {
        currentPage += 1
        isLoadingMore = false
        self.hasMore = hasMore
    }

    public mutating func reset() {
        currentPage = 0
        totalPages = 0
        isLoadingMore = false
        hasMore = true
    }
}

// MARK: - Refresh State

/// Represents pull-to-refresh state.
public struct RefreshState: Equatable, Sendable {
    public var isRefreshing: Bool
    public var lastRefreshed: Date?

    public init(isRefreshing: Bool = false, lastRefreshed: Date? = nil) {
        self.isRefreshing = isRefreshing
        self.lastRefreshed = lastRefreshed
    }

    public var timeSinceLastRefresh: TimeInterval? {
        guard let lastRefreshed = lastRefreshed else { return nil }
        return Date().timeIntervalSince(lastRefreshed)
    }

    public mutating func startRefresh() {
        isRefreshing = true
    }

    public mutating func finishRefresh() {
        isRefreshing = false
        lastRefreshed = Date()
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
