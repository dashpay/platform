import XCTest
@testable import SwiftExampleApp

/// Behavioral tests for `LogExporter.selectSessions` — the pure
/// policy deciding which log-session directories go into a
/// diagnostic export.
///
/// The invariants under test:
/// - the caller-provided current session is authoritative: always
///   first and always included, regardless of how its timestamp
///   sorts against other directories (clock rollbacks, stale
///   future-dated dirs) and regardless of the byte cap;
/// - older sessions fill remaining slots newest-first, subject to
///   the session-count cap and the total-bytes cap;
/// - the walk stops at the first over-budget session instead of
///   skipping past it, so the archive is always a contiguous run
///   of the most recent history;
/// - with no known current session (file logging didn't install),
///   the newest directory on disk still gets exported.
final class LogExporterSelectionTests: XCTestCase {
    private func candidate(_ stamp: String, bytes: UInt64) -> LogExporter.SessionCandidate {
        LogExporter.SessionCandidate(
            url: URL(fileURLWithPath: "/logs/\(stamp)", isDirectory: true),
            bytes: bytes
        )
    }

    func testCurrentSessionIsFirstEvenWhenOlderStampedThanOthers() {
        // A stale future-dated directory sorts lexicographically
        // after the real current session. It must not displace it.
        let current = candidate("2026-07-15T10-00-00Z", bytes: 100)
        let futureStale = candidate("2027-01-01T00-00-00Z", bytes: 100)
        let previous = candidate("2026-07-14T09-00-00Z", bytes: 100)

        let selected = LogExporter.selectSessions(
            current: current,
            others: [previous, futureStale]
        )

        XCTAssertEqual(selected.first, current)
        XCTAssertEqual(selected.count, 3)
        // Remaining slots are newest-first among the others.
        XCTAssertEqual(selected[1], futureStale)
        XCTAssertEqual(selected[2], previous)
    }

    func testSessionCountCap() {
        let current = candidate("2026-07-15T10-00-00Z", bytes: 1)
        let others = (1...5).map {
            candidate("2026-07-10T0\($0)-00-00Z", bytes: 1)
        }

        let selected = LogExporter.selectSessions(current: current, others: others)

        XCTAssertEqual(selected.count, LogExporter.maxSessions)
        XCTAssertEqual(selected.first, current)
        // Newest two of the others.
        XCTAssertEqual(
            selected.dropFirst().map { $0.url.lastPathComponent },
            ["2026-07-10T05-00-00Z", "2026-07-10T04-00-00Z"]
        )
    }

    func testOversizedCurrentSessionIsStillIncluded() {
        let huge = candidate("2026-07-15T10-00-00Z", bytes: 500)
        let previous = candidate("2026-07-14T09-00-00Z", bytes: 10)

        let selected = LogExporter.selectSessions(
            current: huge,
            others: [previous],
            maxTotalBytes: 100
        )

        // The current session is the point of the export; the cap
        // only stops *additional* sessions from being added.
        XCTAssertEqual(selected, [huge])
    }

    func testByteCapStopsAtFirstOversizedOlderSession() {
        let current = candidate("2026-07-15T10-00-00Z", bytes: 40)
        let bigPrevious = candidate("2026-07-14T09-00-00Z", bytes: 90)
        let smallOlder = candidate("2026-07-13T08-00-00Z", bytes: 5)

        let selected = LogExporter.selectSessions(
            current: current,
            others: [smallOlder, bigPrevious],
            maxTotalBytes: 100
        )

        // bigPrevious breaches the cap; smallOlder must NOT be
        // pulled in past it — a gap in the middle of the history
        // would be more confusing than a shorter archive.
        XCTAssertEqual(selected, [current])
    }

    func testUnknownCurrentFallsBackToNewestOnDisk() {
        let newest = candidate("2026-07-15T10-00-00Z", bytes: 500)
        let older = candidate("2026-07-14T09-00-00Z", bytes: 10)

        let selected = LogExporter.selectSessions(
            current: nil,
            others: [older, newest],
            maxTotalBytes: 100
        )

        // Even over-budget, the newest session is always included
        // so an export is never empty when logs exist.
        XCTAssertEqual(selected, [newest])
    }

    func testUnknownCurrentSelectsNewestFirstUnderCaps() {
        let a = candidate("2026-07-15T10-00-00Z", bytes: 10)
        let b = candidate("2026-07-14T09-00-00Z", bytes: 10)
        let c = candidate("2026-07-13T08-00-00Z", bytes: 10)
        let d = candidate("2026-07-12T07-00-00Z", bytes: 10)

        let selected = LogExporter.selectSessions(current: nil, others: [d, b, a, c])

        XCTAssertEqual(selected, [a, b, c])
    }

    func testNoSessionsSelectsNothing() {
        XCTAssertEqual(LogExporter.selectSessions(current: nil, others: []), [])
    }
}
