import XCTest
@testable import SwiftDashSDK

final class StateManagementTests: XCTestCase {

    // MARK: - LoadingState Tests

    func testLoadingStateIdle() {
        let state = LoadingState.idle
        XCTAssertTrue(state.isIdle)
        XCTAssertFalse(state.isLoading)
        XCTAssertFalse(state.isLoaded)
        XCTAssertFalse(state.isFailed)
        XCTAssertNil(state.errorMessage)
    }

    func testLoadingStateLoading() {
        let state = LoadingState.loading
        XCTAssertFalse(state.isIdle)
        XCTAssertTrue(state.isLoading)
        XCTAssertFalse(state.isLoaded)
        XCTAssertFalse(state.isFailed)
    }

    func testLoadingStateLoaded() {
        let state = LoadingState.loaded
        XCTAssertFalse(state.isIdle)
        XCTAssertFalse(state.isLoading)
        XCTAssertTrue(state.isLoaded)
        XCTAssertFalse(state.isFailed)
    }

    func testLoadingStateFailed() {
        let state = LoadingState.failed("Test error")
        XCTAssertFalse(state.isIdle)
        XCTAssertFalse(state.isLoading)
        XCTAssertFalse(state.isLoaded)
        XCTAssertTrue(state.isFailed)
        XCTAssertEqual(state.errorMessage, "Test error")
    }

    func testLoadingStateEquatable() {
        XCTAssertEqual(LoadingState.idle, LoadingState.idle)
        XCTAssertEqual(LoadingState.loading, LoadingState.loading)
        XCTAssertEqual(LoadingState.loaded, LoadingState.loaded)
        XCTAssertEqual(LoadingState.failed("error"), LoadingState.failed("error"))
        XCTAssertNotEqual(LoadingState.failed("error1"), LoadingState.failed("error2"))
        XCTAssertNotEqual(LoadingState.idle, LoadingState.loading)
    }

    // MARK: - ResultState Tests

    func testResultStateIdle() {
        let state: ResultState<String> = .idle
        XCTAssertFalse(state.isLoading)
        XCTAssertFalse(state.isSuccess)
        XCTAssertFalse(state.isFailure)
        XCTAssertNil(state.value)
        XCTAssertNil(state.error)
    }

    func testResultStateLoading() {
        let state: ResultState<String> = .loading
        XCTAssertTrue(state.isLoading)
        XCTAssertFalse(state.isSuccess)
        XCTAssertFalse(state.isFailure)
    }

    func testResultStateSuccess() {
        let state: ResultState<String> = .success("test value")
        XCTAssertFalse(state.isLoading)
        XCTAssertTrue(state.isSuccess)
        XCTAssertFalse(state.isFailure)
        XCTAssertEqual(state.value, "test value")
        XCTAssertNil(state.error)
    }

    func testResultStateFailure() {
        let state: ResultState<String> = .failure("test error")
        XCTAssertFalse(state.isLoading)
        XCTAssertFalse(state.isSuccess)
        XCTAssertTrue(state.isFailure)
        XCTAssertNil(state.value)
        XCTAssertEqual(state.error, "test error")
    }

    func testResultStateMap() {
        let intState: ResultState<Int> = .success(42)
        let stringState = intState.map { String($0) }
        XCTAssertEqual(stringState.value, "42")
    }

    func testResultStateMapIdle() {
        let state: ResultState<Int> = .idle
        let mapped = state.map { String($0) }
        XCTAssertFalse(mapped.isSuccess)
    }

    func testResultStateMapFailure() {
        let state: ResultState<Int> = .failure("error")
        let mapped = state.map { String($0) }
        XCTAssertTrue(mapped.isFailure)
        XCTAssertEqual(mapped.error, "error")
    }

    // MARK: - FormState Tests

    func testFormStateEditing() {
        let state = FormState.editing
        XCTAssertFalse(state.isSubmitting)
        XCTAssertFalse(state.isSubmitted)
        XCTAssertFalse(state.hasError)
        XCTAssertTrue(state.canSubmit)
    }

    func testFormStateSubmitting() {
        let state = FormState.submitting
        XCTAssertTrue(state.isSubmitting)
        XCTAssertFalse(state.isSubmitted)
        XCTAssertFalse(state.hasError)
        XCTAssertFalse(state.canSubmit)
    }

    func testFormStateSubmitted() {
        let state = FormState.submitted
        XCTAssertFalse(state.isSubmitting)
        XCTAssertTrue(state.isSubmitted)
        XCTAssertFalse(state.hasError)
        XCTAssertFalse(state.canSubmit)
    }

    func testFormStateError() {
        let state = FormState.error("Validation failed")
        XCTAssertFalse(state.isSubmitting)
        XCTAssertFalse(state.isSubmitted)
        XCTAssertTrue(state.hasError)
        XCTAssertTrue(state.canSubmit)
        XCTAssertEqual(state.errorMessage, "Validation failed")
    }

    // MARK: - PaginationState Tests

    func testPaginationStateDefault() {
        let state = PaginationState()
        XCTAssertEqual(state.currentPage, 0)
        XCTAssertEqual(state.totalPages, 0)
        XCTAssertFalse(state.isLoadingMore)
        XCTAssertTrue(state.hasMore)
        XCTAssertTrue(state.canLoadMore)
    }

    func testPaginationStateStartLoadingMore() {
        var state = PaginationState()
        state.startLoadingMore()
        XCTAssertTrue(state.isLoadingMore)
        XCTAssertFalse(state.canLoadMore)
    }

    func testPaginationStateFinishLoadingMore() {
        var state = PaginationState()
        state.startLoadingMore()
        state.finishLoadingMore(hasMore: true)
        XCTAssertEqual(state.currentPage, 1)
        XCTAssertFalse(state.isLoadingMore)
        XCTAssertTrue(state.hasMore)
    }

    func testPaginationStateFinishLoadingMoreNoMore() {
        var state = PaginationState()
        state.finishLoadingMore(hasMore: false)
        XCTAssertFalse(state.hasMore)
        XCTAssertFalse(state.canLoadMore)
    }

    func testPaginationStateReset() {
        var state = PaginationState(currentPage: 5, totalPages: 10, isLoadingMore: false, hasMore: false)
        state.reset()
        XCTAssertEqual(state.currentPage, 0)
        XCTAssertEqual(state.totalPages, 0)
        XCTAssertTrue(state.hasMore)
    }

    // MARK: - RefreshState Tests

    func testRefreshStateDefault() {
        let state = RefreshState()
        XCTAssertFalse(state.isRefreshing)
        XCTAssertNil(state.lastRefreshed)
        XCTAssertNil(state.timeSinceLastRefresh)
    }

    func testRefreshStateStartRefresh() {
        var state = RefreshState()
        state.startRefresh()
        XCTAssertTrue(state.isRefreshing)
    }

    func testRefreshStateFinishRefresh() {
        var state = RefreshState()
        state.startRefresh()
        state.finishRefresh()
        XCTAssertFalse(state.isRefreshing)
        XCTAssertNotNil(state.lastRefreshed)
        XCTAssertNotNil(state.timeSinceLastRefresh)
    }

    // MARK: - ErrorState Tests

    func testErrorStateDefault() {
        let state = ErrorState()
        XCTAssertNil(state.message)
        XCTAssertFalse(state.showError)
        XCTAssertFalse(state.hasError)
    }

    func testErrorStateSetError() {
        var state = ErrorState()
        state.setError("Something went wrong")
        XCTAssertEqual(state.message, "Something went wrong")
        XCTAssertTrue(state.showError)
        XCTAssertTrue(state.hasError)
    }

    func testErrorStateClearError() {
        var state = ErrorState()
        state.setError("Error")
        state.clearError()
        XCTAssertNil(state.message)
        XCTAssertFalse(state.showError)
        XCTAssertFalse(state.hasError)
    }

    // MARK: - AsyncOperationResult Tests

    func testAsyncOperationResultSuccess() {
        let result = AsyncOperationResult(value: "success", duration: 1.5)
        XCTAssertTrue(result.isSuccess)
        XCTAssertEqual(result.value, "success")
        XCTAssertNil(result.error)
        XCTAssertEqual(result.duration, 1.5)
    }

    func testAsyncOperationResultErrorString() {
        let result = AsyncOperationResult<String>(error: "failed", duration: 0.5)
        XCTAssertFalse(result.isSuccess)
        XCTAssertNil(result.value)
        XCTAssertEqual(result.error, "failed")
        XCTAssertEqual(result.duration, 0.5)
    }

    // MARK: - AsyncOperation Tests

    func testAsyncOperationRunSuccess() async {
        let result = await AsyncOperation.run {
            return 42
        }
        XCTAssertTrue(result.isSuccess)
        XCTAssertEqual(result.value, 42)
        XCTAssertNil(result.error)
        XCTAssertGreaterThanOrEqual(result.duration, 0)
    }

    func testAsyncOperationRunFailure() async {
        struct TestError: Error {}
        let result = await AsyncOperation.run {
            throw TestError()
        }
        XCTAssertFalse(result.isSuccess)
        XCTAssertNil(result.value)
        XCTAssertNotNil(result.error)
    }
}
