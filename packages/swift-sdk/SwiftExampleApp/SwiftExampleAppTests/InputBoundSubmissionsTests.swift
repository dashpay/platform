import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins the guard of the catalog's moderation reads (`QueryDetailView`): a
/// result that comes back after an input was edited, or after a newer
/// Execute, is dropped. Each test follows the view's flow (submit, read,
/// show only while the token is current) with a fee pot read the test holds
/// until it releases it, so the edit provably lands while the read runs.
@MainActor
final class InputBoundSubmissionsTests: XCTestCase {
    /// What the screen shows.
    private final class Screen {
        var shown: ContractFeePots?
        var error: String?
    }

    private struct Refused: Error {}

    private func pots(credits: UInt64) throws -> ContractFeePots {
        try JSONDecoder().decode(
            ContractFeePots.self,
            from: Data(#"{"owner":{"credits":\#(credits)},"moderators":{"credits":0}}"#.utf8)
        )
    }

    /// Submits a fee pot read the way the view's Execute does and returns
    /// the task showing its outcome; the read waits on `read`.
    private func execute(
        _ submissions: InputBoundSubmissions,
        _ read: ReleasedRead,
        on screen: Screen
    ) -> Task<Void, Never> {
        let submission = submissions.submit(["contractId": "c"])
        return Task {
            do {
                let pots = try await read.value()
                guard submissions.isCurrent(submission) else { return }
                screen.shown = pots
            } catch {
                guard submissions.isCurrent(submission) else { return }
                screen.error = "\(error)"
            }
        }
    }

    func testShouldDiscardADelayedFeePotReadAfterAnInputEdit() async throws {
        let submissions = InputBoundSubmissions()
        let screen = Screen()
        let read = ReleasedRead()

        let task = execute(submissions, read, on: screen)
        await read.waitUntilStarted()
        submissions.invalidate()
        read.release(.success(try pots(credits: 5)))
        await task.value

        XCTAssertNil(screen.shown)
        XCTAssertNil(screen.error)
    }

    func testShouldDiscardADelayedFailureAfterAnInputEdit() async {
        let submissions = InputBoundSubmissions()
        let screen = Screen()
        let read = ReleasedRead()

        let task = execute(submissions, read, on: screen)
        await read.waitUntilStarted()
        submissions.invalidate()
        read.release(.failure(Refused()))
        await task.value

        XCTAssertNil(screen.error)
    }

    func testShouldShowOnlyTheNewestOfTwoReadsInFlight() async throws {
        let submissions = InputBoundSubmissions()
        let screen = Screen()
        let older = ReleasedRead()
        let newer = ReleasedRead()

        let olderTask = execute(submissions, older, on: screen)
        await older.waitUntilStarted()
        let newerTask = execute(submissions, newer, on: screen)
        await newer.waitUntilStarted()
        newer.release(.success(try pots(credits: 2)))
        await newerTask.value
        older.release(.success(try pots(credits: 1)))
        await olderTask.value

        XCTAssertEqual(screen.shown?.owner.credits, 2)
    }

    /// The inputs travel with the token as they were at submission: what the
    /// form holds afterwards does not change them.
    func testShouldCarryTheInputsAsSubmitted() {
        let submissions = InputBoundSubmissions()
        var form = ["contractId": "A"]
        let submission = submissions.submit(form)
        form["contractId"] = "B"
        submissions.invalidate()

        XCTAssertEqual(submission.inputs, ["contractId": "A"])
        XCTAssertFalse(submissions.isCurrent(submission))
        XCTAssertTrue(submissions.isCurrent(submissions.submit(form)))
    }

    /// A result read through a replaced SDK is not current, whatever its
    /// inputs.
    func testShouldNotShowAResultReadThroughAReplacedConnection() {
        let submissions = InputBoundSubmissions()
        let submission = submissions.submit(["contractId": "A"], connection: 1)

        XCTAssertTrue(submissions.isCurrent(submission, connection: 1))
        XCTAssertFalse(submissions.isCurrent(submission, connection: 2))
    }

    func testShouldShowAReadNoEditInterrupted() async throws {
        let submissions = InputBoundSubmissions()
        let screen = Screen()
        let read = ReleasedRead()

        let task = execute(submissions, read, on: screen)
        await read.waitUntilStarted()
        read.release(.success(try pots(credits: 7)))
        await task.value

        XCTAssertEqual(screen.shown?.owner.credits, 7)
    }
}

/// A read that announces it started and waits for the test to release its
/// outcome.
@MainActor
final class ReleasedRead {
    private var pending: CheckedContinuation<ContractFeePots, Error>?
    private var startWaiters: [CheckedContinuation<Void, Never>] = []

    func value() async throws -> ContractFeePots {
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

    func release(_ outcome: Result<ContractFeePots, Error>) {
        pending?.resume(with: outcome)
        pending = nil
    }
}
