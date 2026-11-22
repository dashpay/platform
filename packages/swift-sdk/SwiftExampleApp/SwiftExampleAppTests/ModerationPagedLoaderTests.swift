import XCTest
@testable import SwiftExampleApp

/// Pins that only the current request of a paged moderation read writes its
/// rows, cursor and loading state: a "Load More" still in flight when the
/// user refreshes must not mix a stale page into the fresh one, nor end the
/// fresh one's loading.
@MainActor
final class ModerationPagedLoaderTests: XCTestCase {
    private typealias Loader = ModerationPagedLoader<String, String>

    /// A fetch that answers `cursor` (`nil` for the first page) from `pages`
    /// after `delays[cursor]` milliseconds.
    private func fetch(
        _ pages: [String: ModerationPage<String, String>],
        delays: [String: UInt64] = [:]
    ) -> Loader.Fetch {
        { cursor in
            let key = cursor ?? "first"
            try await Task.sleep(nanoseconds: (delays[key] ?? 0) * 1_000_000)
            return pages[key]
        }
    }

    func testShouldIgnoreAStaleLoadMoreThatCompletesAfterARefresh() async {
        let loader = Loader()
        let first = fetch(
            [
                "first": ModerationPage(items: ["a1"], nextCursor: "a1"),
                "a1": ModerationPage(items: ["stale"], nextCursor: "stale"),
            ],
            delays: ["a1": 300]
        )
        await loader.reload(first)
        XCTAssertEqual(loader.items, ["a1"])

        let loadMore = Task { await loader.loadMore() }
        try? await Task.sleep(nanoseconds: 20_000_000)
        await loader.reload(fetch(["first": ModerationPage(items: ["b1"], nextCursor: nil)]))
        await loadMore.value

        XCTAssertEqual(loader.items, ["b1"])
        XCTAssertNil(loader.nextCursor)
        XCTAssertFalse(loader.isLoading)
        XCTAssertNil(loader.errorMessage)
    }

    func testShouldKeepLoadingForTheCurrentRequestWhenAStaleOneCompletes() async {
        let loader = Loader()
        await loader.reload(fetch(
            [
                "first": ModerationPage(items: ["a1"], nextCursor: "a1"),
                "a1": ModerationPage(items: ["stale"], nextCursor: nil),
            ],
            delays: ["a1": 50]
        ))

        let loadMore = Task { await loader.loadMore() }
        try? await Task.sleep(nanoseconds: 10_000_000)
        let refresh = Task {
            await loader.reload(self.fetch(
                ["first": ModerationPage(items: ["b1"], nextCursor: "b1")],
                delays: ["first": 300]
            ))
        }
        await loadMore.value

        // The stale page came back while the refresh is still reading
        XCTAssertTrue(loader.isLoading)
        XCTAssertEqual(loader.items, [])
        XCTAssertNil(loader.nextCursor)

        await refresh.value
        XCTAssertFalse(loader.isLoading)
        XCTAssertEqual(loader.items, ["b1"])
        XCTAssertEqual(loader.nextCursor, "b1")
    }

    func testShouldShowAFailureOfTheCurrentRequestOnly() async {
        struct Refused: LocalizedError {
            var errorDescription: String? { "refused by the node" }
        }
        let loader = Loader()
        await loader.reload { _ in throw Refused() }
        XCTAssertEqual(loader.errorMessage, "refused by the node")
        XCTAssertTrue(loader.loadedOnce)
        XCTAssertFalse(loader.isLoading)

        await loader.reload(fetch(["first": ModerationPage(items: ["b1"], nextCursor: nil)]))
        XCTAssertNil(loader.errorMessage)
        XCTAssertEqual(loader.items, ["b1"])
    }
}
