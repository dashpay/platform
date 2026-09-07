import Foundation
import XCTest

@testable import SwiftDashSDK

/// The file sink is installed by `LoggingPreferences.configure()`, well after
/// the host has already opened its store in `init()`. Everything emitted in
/// between must reach `swift/run.log` in order once the sink exists — and a
/// host that never installs one must not grow the backlog without bound.
///
/// Tested on a fresh `SDKLoggerState` rather than through `SDKLogger`: the
/// process-wide singleton has no way back to "no sink installed" once any
/// test has installed one.
final class SDKLoggerPreInstallBufferTests: XCTestCase {
    private var session: URL!

    override func setUpWithError() throws {
        session = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: session, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: session)
    }

    private func writtenLines(_ state: SDKLoggerState) throws -> [String] {
        state.flush()
        let log = try String(
            contentsOf: session.appendingPathComponent("swift/run.log"),
            encoding: .utf8
        )
        return log.split(separator: "\n").map(String.init)
    }

    func testLinesRecordedBeforeInstallReplayInOrderAheadOfLaterLines() throws {
        let state = SDKLoggerState()
        state.record(severity: .info, line: "first")
        state.record(severity: .warning, line: "second")

        let outcome = state.installSink(at: session, includeDebug: false)
        XCTAssertTrue(outcome.installed)
        XCTAssertEqual(outcome.droppedPendingLineCount, 0)

        state.record(severity: .error, line: "third")
        XCTAssertEqual(try writtenLines(state), ["first", "second", "third"])
    }

    func testReplayHonoursTheSinkDebugSettingItDidNotKnowYet() throws {
        // Before install nothing knows whether debug lines are wanted, so
        // they are buffered regardless and filtered at replay.
        let hidden = SDKLoggerState()
        hidden.record(severity: .debug, line: "debug")
        hidden.record(severity: .info, line: "info")
        _ = hidden.installSink(at: session, includeDebug: false)
        XCTAssertEqual(try writtenLines(hidden), ["info"])

        try FileManager.default.removeItem(at: session.appendingPathComponent("swift"))
        let shown = SDKLoggerState()
        shown.record(severity: .debug, line: "debug")
        shown.record(severity: .info, line: "info")
        _ = shown.installSink(at: session, includeDebug: true)
        XCTAssertEqual(try writtenLines(shown), ["debug", "info"])
    }

    /// Head-not-tail: the store-open line is the first in, and it is the one
    /// the buffer exists to carry, so overflow discards the newest arrival.
    func testBufferKeepsTheHeadAndDropsTheNewestAboveTheLimit() throws {
        let state = SDKLoggerState()
        let limit = SDKLoggerState.pendingLineLimit
        // limit + 1 lines: exactly one over.
        for index in 0...limit {
            state.record(severity: .info, line: "line-\(index)")
        }

        let outcome = state.installSink(at: session, includeDebug: false)
        XCTAssertTrue(outcome.installed)
        // This count is what `SDKLogger.installFileSink` turns into the
        // `log_pre_install_buffer_overflow` event after the replay.
        XCTAssertEqual(outcome.droppedPendingLineCount, 1)

        let lines = try writtenLines(state)
        XCTAssertEqual(lines.count, limit)
        XCTAssertEqual(lines.first, "line-0", "the first line in survives")
        XCTAssertEqual(lines.last, "line-\(limit - 1)", "the newest arrival is the one dropped")
    }

    func testBufferIsClearedByInstallSoASecondInstallReplaysNothing() throws {
        let state = SDKLoggerState()
        state.record(severity: .info, line: "once")
        _ = state.installSink(at: session, includeDebug: false)
        XCTAssertEqual(try writtenLines(state), ["once"])

        try FileManager.default.removeItem(at: session.appendingPathComponent("swift"))
        let outcome = state.installSink(at: session, includeDebug: false)
        XCTAssertEqual(outcome.droppedPendingLineCount, 0)
        XCTAssertEqual(try writtenLines(state), [])
    }
}
