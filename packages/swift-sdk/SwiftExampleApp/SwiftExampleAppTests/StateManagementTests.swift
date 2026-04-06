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
}
