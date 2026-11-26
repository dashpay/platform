import XCTest
@testable import SwiftExampleApp

/// Pins how an edit of the typed ids orders against reads already queued on
/// the removal records screen.
final class RemovalsLookupFilterTests: XCTestCase {
    /// An edit while an all-records read is queued leaves that read current,
    /// so it still runs and ends the spinner.
    func testShouldKeepAQueuedAllRecordsReadWhenTheDraftChanges() {
        var filter = RemovalsLookupFilter()
        let token = filter.begin(ids: nil)

        XCTAssertFalse(filter.draftChanged())
        XCTAssertTrue(filter.isCurrent(token))
        XCTAssertNil(filter.ids)
    }

    /// An edit while a look up by ids is queued drops it and clears the
    /// filter shown.
    func testShouldDropAQueuedLookUpWhenTheDraftChanges() {
        var filter = RemovalsLookupFilter()
        let token = filter.begin(ids: ["A"])

        XCTAssertTrue(filter.draftChanged())
        XCTAssertFalse(filter.isCurrent(token))
        XCTAssertNil(filter.ids)
    }

    /// A newer read supersedes an older one queued before it.
    func testShouldSupersedeAnOlderQueuedRead() {
        var filter = RemovalsLookupFilter()
        let older = filter.begin(ids: ["A"])
        let newer = filter.begin(ids: nil)

        XCTAssertFalse(filter.isCurrent(older))
        XCTAssertTrue(filter.isCurrent(newer))
        XCTAssertNil(filter.ids)
    }
}
