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

    /// The app's connection, as the views read it from `AppState`.
    private final class Connection {
        var generation = 1
    }

    /// A replacement of the SDK while a response is pending drops it.
    func testShouldDropAPendingAnswerWhenTheConnectionIsReplaced() async {
        let fetches = ControlledFetch()
        let connection = Connection()
        let loader = Loader()
        let read = Task {
            await loader.reload(fetches.source("A"), connectionIsCurrent: { connection.generation == 1 })
        }
        await fetches.waitUntilCalled("A-first")
        connection.generation = 2
        fetches.answer("A-first", with: Page(items: ["old"], nextCursor: "old"))
        await read.value

        XCTAssertEqual(loader.items, [])
        XCTAssertNil(loader.nextCursor)
    }

    /// A replacement after a page is shown clears the page and its cursor (the
    /// view calls `clear()` when the generation changes), and Load More then
    /// queries nothing, the old SDK included.
    func testShouldClearAShownPageAndNotLoadMoreAfterAReplacement() async {
        let fetches = ControlledFetch()
        let connection = Connection()
        let loader = Loader()
        let read = Task {
            await loader.reload(fetches.source("A"), connectionIsCurrent: { connection.generation == 1 })
        }
        await fetches.waitUntilCalled("A-first")
        fetches.answer("A-first", with: Page(items: ["a1"], nextCursor: "a1"))
        await read.value
        XCTAssertEqual(loader.nextCursor, "a1")

        connection.generation = 2
        // Before the view's clear() runs, Load More already refuses the old SDK
        await loader.loadMore()
        XCTAssertFalse(loader.isLoading)
        XCTAssertFalse(fetches.wasCalled("A-a1"))

        loader.clear()
        XCTAssertEqual(loader.items, [])
        XCTAssertNil(loader.nextCursor)
        await loader.loadMore()
        XCTAssertFalse(fetches.wasCalled("A-a1"))
    }

    /// A single read (fee pots, action counts, signers) drops an answer that
    /// arrives after the connection was replaced.
    func testShouldDropASingleReadAnsweredAfterAReplacement() async {
        let connection = Connection()
        let loader = ModerationSingleLoader<String>()
        let gate = ReleasedValue()
        let read = Task {
            await loader.reload({ try await gate.value() }, connectionIsCurrent: { connection.generation == 1 })
        }
        await gate.waitUntilStarted()
        connection.generation = 2
        gate.release("old balance")
        await read.value

        if case .loaded = loader.load {
            XCTFail("an answer from the replaced connection was shown")
        }
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
    private var called: Set<String> = []

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

    /// Whether the call `key` was ever made.
    func wasCalled(_ key: String) -> Bool {
        called.contains(key)
    }

    func answer(_ key: String, with page: ModerationPage<String, String>?) {
        pending.removeValue(forKey: key)?.resume(returning: page)
    }

    func fail(_ key: String, with error: Error) {
        pending.removeValue(forKey: key)?.resume(throwing: error)
    }

    private func call(_ key: String) async throws -> ModerationPage<String, String>? {
        called.insert(key)
        return try await withCheckedThrowingContinuation { continuation in
            pending[key] = continuation
            callWaiters.removeValue(forKey: key)?.forEach { $0.resume() }
        }
    }
}

/// A single value the test releases once the read waits for it.
@MainActor
final class ReleasedValue {
    private var pending: CheckedContinuation<String, Error>?
    private var startWaiters: [CheckedContinuation<Void, Never>] = []

    nonisolated func value() async throws -> String {
        try await wait()
    }

    private func wait() async throws -> String {
        try await withCheckedThrowingContinuation { continuation in
            pending = continuation
            startWaiters.forEach { $0.resume() }
            startWaiters = []
        }
    }

    func waitUntilStarted() async {
        if pending != nil { return }
        await withCheckedContinuation { startWaiters.append($0) }
    }

    func release(_ value: String) {
        pending?.resume(returning: value)
        pending = nil
    }
}
