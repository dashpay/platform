import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins that `AppState.rebuildSdk` runs one SDK rebuild at a time for the
/// whole app. `AppState` is shared by every window, and a rebuild changes
/// shared settings, the published SDK and network, and the active wallet
/// manager. A second rebuild running beside the first would roll its
/// prepared settings back over the first one's, or publish its network in
/// whichever order the two finish, so it is refused before any of its steps
/// runs, and the first holds the reservation through its activation and
/// rollback.
///
/// The steps `AppState` does not own are injected. No run publishes:
/// publishing writes the selected network to the app's UserDefaults.
@MainActor
final class AppStateSdkRebuildTests: XCTestCase {

    /// Point the regtest SDK a rebuild builds at loopback endpoints, in the
    /// app's UserDefaults these tests share, and return what restores them:
    /// a quorum URL or DAPI address left by the app would make the build
    /// reach another host.
    static func pinLoopbackEndpoints() -> () -> Void {
        let defaults = UserDefaults.standard
        let keys = ["platformQuorumURL", "platformDAPIAddresses"]
        let saved = keys.map { defaults.string(forKey: $0) }
        for key in keys {
            defaults.set("http://127.0.0.1:1", forKey: key)
        }
        return {
            for (key, value) in zip(keys, saved) {
                if let value {
                    defaults.set(value, forKey: key)
                } else {
                    defaults.removeObject(forKey: key)
                }
            }
        }
    }

    private enum Step: Equatable {
        case stopSpv
        case prepare
        case activateManager
        case rollBack
        case beforePublish
    }

    /// The steps one rebuild ran.
    @MainActor
    private final class Recorder {
        var steps: [Step] = []
    }

    /// A setting a rebuild prepares and rolls back.
    @MainActor
    private final class Setting {
        var docker = true
    }

    private struct NeverHeld: Error {}

    /// A step held open until the test resumes it.
    @MainActor
    private final class Gate {
        var continuation: CheckedContinuation<Bool, Never>?

        func hold() async -> Bool {
            await withCheckedContinuation { continuation = $0 }
        }

        /// Suspend until a step is held, failing instead of hanging.
        func waitUntilHeld() async throws {
            let deadline = ContinuousClock.now + .seconds(10)
            while continuation == nil {
                guard ContinuousClock.now < deadline else { throw NeverHeld() }
                await Task.yield()
            }
        }

        func resume(returning value: Bool) {
            continuation?.resume(returning: value)
            continuation = nil
        }
    }

    private func rebuild(
        _ appState: AppState,
        recording recorder: Recorder,
        stopSpv: @escaping @MainActor () async -> Bool = { false },
        prepare: @escaping @MainActor () -> Void = {},
        activateManager: @escaping @MainActor () async -> Bool = { false },
        rollBack: @escaping @MainActor () -> Void = {}
    ) async -> SdkRebuildOutcome {
        await appState.rebuildSdk(
            from: appState.currentNetwork,
            to: .regtest,
            stopSpv: {
                recorder.steps.append(.stopSpv)
                return await stopSpv()
            },
            prepare: {
                recorder.steps.append(.prepare)
                prepare()
            },
            activateManager: { (_: Network, _: SDK) async -> Bool in
                recorder.steps.append(.activateManager)
                return await activateManager()
            },
            rollBack: {
                recorder.steps.append(.rollBack)
                rollBack()
            },
            beforePublish: { recorder.steps.append(.beforePublish) }
        )
    }

    /// A rebuild started, from any window, while another one stops SPV is
    /// refused without running a step; once the first ends, the next one
    /// runs.
    func testShouldRefuseASecondRebuildWhileTheFirstStopsSpv() async throws {
        let appState = AppState()
        let gate = Gate()
        let first = Recorder()
        let running = Task {
            await rebuild(appState, recording: first, stopSpv: { await gate.hold() })
        }
        try await gate.waitUntilHeld()
        XCTAssertTrue(appState.isRebuildingSdk)

        let second = Recorder()
        let refused = await rebuild(appState, recording: second, stopSpv: { true })
        XCTAssertEqual(refused, .busy)
        XCTAssertEqual(second.steps, [], "a refused rebuild must not run any step")

        gate.resume(returning: false)
        let outcome = await running.value
        XCTAssertEqual(outcome, .stopFailed)
        XCTAssertEqual(first.steps, [.stopSpv])
        XCTAssertFalse(appState.isRebuildingSdk)

        let third = Recorder()
        let next = await rebuild(appState, recording: third)
        XCTAssertEqual(next, .stopFailed)
        XCTAssertEqual(third.steps, [.stopSpv], "the reservation must be free once a rebuild ends")
    }

    /// The reservation is held through the wallet manager's activation and
    /// the rollback after it fails. A rebuild started meanwhile must not
    /// run: its rollback would restore its own captured setting over the
    /// one the running rebuild built its SDK from.
    func testShouldKeepTheLosersRollBackOffTheWinnersSettings() async throws {
        let restoreEndpoints = Self.pinLoopbackEndpoints()
        defer { restoreEndpoints() }
        let appState = AppState()
        let gate = Gate()
        let setting = Setting()
        let first = Recorder()
        let running = Task {
            await rebuild(
                appState,
                recording: first,
                stopSpv: { true },
                prepare: { setting.docker = false },
                activateManager: { await gate.hold() },
                rollBack: { setting.docker = true }
            )
        }
        do {
            try await gate.waitUntilHeld()
        } catch {
            // The activation is reached only once a real regtest SDK built.
            XCTFail("the rebuild never reached the activation: \(appState.errorMessage)")
            throw error
        }
        XCTAssertFalse(setting.docker)

        let second = Recorder()
        let refused = await rebuild(
            appState,
            recording: second,
            stopSpv: { true },
            prepare: { setting.docker = false },
            activateManager: { true },
            rollBack: { setting.docker = true }
        )
        XCTAssertEqual(refused, .busy)
        XCTAssertEqual(second.steps, [])
        XCTAssertFalse(
            setting.docker, "the refused rebuild must not roll the running one's setting back")
        XCTAssertTrue(appState.isRebuildingSdk)

        gate.resume(returning: false)
        let outcome = await running.value
        XCTAssertEqual(outcome, .failed)
        XCTAssertEqual(first.steps, [.stopSpv, .prepare, .activateManager, .rollBack])
        XCTAssertFalse(appState.isRebuildingSdk)
    }
}
