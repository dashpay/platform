import XCTest
@testable import SwiftExampleApp

/// Pins that only the current request of a paged moderation read writes its
/// rows, cursor, error and loading state: a "Load More" still in flight when
/// the user refreshes must not mix a stale page or a stale error into the
/// fresh read, nor end its loading.
///
/// Every fetch goes through ``ControlledFetch``, which tells the test when a
/// request has started and holds its answer until the test releases it, so
/// each test has both requests in flight and picks which completes first.
@MainActor
final class ModerationPagedLoaderTests: XCTestCase {
    private typealias Loader = ModerationPagedLoader<String, String>
    private typealias Page = ModerationPage<String, String>

    private struct Refused: LocalizedError {
        var errorDescription: String? { "refused by the node" }
    }

    /// A loader showing the first page of source `A` (`a1`, cursor `a1`),
    /// with its "Load More" started and waiting for its answer.
    private func loaderWithALoadMoreInFlight(
        _ fetches: ControlledFetch
    ) async -> (Loader, Task<Void, Never>) {
        let loader = Loader()
        let first = Task { await loader.reload(fetches.source("A")) }
        await fetches.waitUntilCalled("A-first")
        fetches.answer("A-first", with: Page(items: ["a1"], nextCursor: "a1"))
        await first.value
        XCTAssertEqual(loader.items, ["a1"])

        let loadMore = Task { await loader.loadMore() }
        await fetches.waitUntilCalled("A-a1")
        XCTAssertTrue(loader.isLoading)
        return (loader, loadMore)
    }

    func testShouldIgnoreAStaleLoadMoreReleasedAfterARefresh() async {
        let fetches = ControlledFetch()
        let (loader, loadMore) = await loaderWithALoadMoreInFlight(fetches)

        let refresh = Task { await loader.reload(fetches.source("B")) }
        await fetches.waitUntilCalled("B-first")
        fetches.answer("B-first", with: Page(items: ["b1"], nextCursor: nil))
        await refresh.value
        fetches.answer("A-a1", with: Page(items: ["stale"], nextCursor: "stale"))
        await loadMore.value

        XCTAssertEqual(loader.items, ["b1"])
        XCTAssertNil(loader.nextCursor)
        XCTAssertFalse(loader.isLoading)
        XCTAssertNil(loader.errorMessage)
    }

    func testShouldKeepLoadingUntilTheCurrentRequestCompletes() async {
        let fetches = ControlledFetch()
        let (loader, loadMore) = await loaderWithALoadMoreInFlight(fetches)

        let refresh = Task { await loader.reload(fetches.source("B")) }
        await fetches.waitUntilCalled("B-first")
        XCTAssertTrue(loader.isLoading)

        // The stale page comes back while the refresh is still reading
        fetches.answer("A-a1", with: Page(items: ["stale"], nextCursor: nil))
        await loadMore.value
        XCTAssertTrue(loader.isLoading)
        XCTAssertEqual(loader.items, [])
        XCTAssertNil(loader.nextCursor)

        fetches.answer("B-first", with: Page(items: ["b1"], nextCursor: "b1"))
        await refresh.value
        XCTAssertFalse(loader.isLoading)
        XCTAssertEqual(loader.items, ["b1"])
        XCTAssertEqual(loader.nextCursor, "b1")
    }

    func testShouldIgnoreAStaleErrorReleasedAfterTheCurrentSuccess() async {
        let fetches = ControlledFetch()
        let (loader, loadMore) = await loaderWithALoadMoreInFlight(fetches)

        let refresh = Task { await loader.reload(fetches.source("B")) }
        await fetches.waitUntilCalled("B-first")
        fetches.answer("B-first", with: Page(items: ["b1"], nextCursor: nil))
        await refresh.value
        fetches.fail("A-a1", with: Refused())
        await loadMore.value

        XCTAssertNil(loader.errorMessage)
        XCTAssertEqual(loader.items, ["b1"])
        XCTAssertFalse(loader.isLoading)
    }

    /// Clearing (the removal lookup's ids were edited) drops what is shown
    /// and what a running request brings back.
    func testShouldDropARunningRequestOnClear() async {
        let fetches = ControlledFetch()
        let (loader, loadMore) = await loaderWithALoadMoreInFlight(fetches)

        loader.clear()
        fetches.answer("A-a1", with: Page(items: ["stale"], nextCursor: "stale"))
        await loadMore.value

        XCTAssertEqual(loader.items, [])
        XCTAssertNil(loader.nextCursor)
        XCTAssertFalse(loader.isLoading)
        XCTAssertFalse(loader.loadedOnce)
        XCTAssertNil(loader.errorMessage)
    }

    /// `begin()` (a lookup tapped, its read still queued) drops the old page
    /// and a running request's answer at once, and shows loading until the
    /// new read runs.
    func testShouldDropTheOldPageAndARunningRequestOnBegin() async {
        let fetches = ControlledFetch()
        let (loader, loadMore) = await loaderWithALoadMoreInFlight(fetches)

        loader.begin()
        XCTAssertEqual(loader.items, [])
        XCTAssertTrue(loader.isLoading)

        fetches.answer("A-a1", with: Page(items: ["stale"], nextCursor: "stale"))
        await loadMore.value
        XCTAssertEqual(loader.items, [])
        XCTAssertNil(loader.nextCursor)
        XCTAssertTrue(loader.isLoading)

        let read = Task { await loader.reload(fetches.source("B")) }
        await fetches.waitUntilCalled("B-first")
        fetches.answer("B-first", with: Page(items: ["b1"], nextCursor: nil))
        await read.value
        XCTAssertEqual(loader.items, ["b1"])
        XCTAssertFalse(loader.isLoading)
    }

    func testShouldShowTheCurrentRequestsFailure() async {
        let fetches = ControlledFetch()
        let loader = Loader()
        let read = Task { await loader.reload(fetches.source("A")) }
        await fetches.waitUntilCalled("A-first")
        fetches.fail("A-first", with: Refused())
        await read.value

        XCTAssertEqual(loader.errorMessage, "refused by the node")
        XCTAssertTrue(loader.loadedOnce)
        XCTAssertFalse(loader.isLoading)
    }
}

/// Fetches that announce each call and wait for the test to answer it. A
/// call is named `<source>-<cursor>`, `first` standing for no cursor.
@MainActor
final class ControlledFetch {
    private var pending: [String: CheckedContinuation<ModerationPage<String, String>?, Error>] = [:]
    private var callWaiters: [String: [CheckedContinuation<Void, Never>]] = [:]

    /// The fetch of source `name`.
    func source(_ name: String) -> ModerationPagedLoader<String, String>.Fetch {
        { [self] cursor in
            try await self.call("\(name)-\(cursor ?? "first")")
        }
    }

    /// Returns once the call `key` has started and waits for its answer.
    func waitUntilCalled(_ key: String) async {
        if pending[key] != nil { return }
        await withCheckedContinuation { continuation in
            callWaiters[key, default: []].append(continuation)
        }
    }

    func answer(_ key: String, with page: ModerationPage<String, String>?) {
        pending.removeValue(forKey: key)?.resume(returning: page)
    }

    func fail(_ key: String, with error: Error) {
        pending.removeValue(forKey: key)?.resume(throwing: error)
    }

    private func call(_ key: String) async throws -> ModerationPage<String, String>? {
        try await withCheckedThrowingContinuation { continuation in
            pending[key] = continuation
            callWaiters.removeValue(forKey: key)?.forEach { $0.resume() }
        }
    }
}
