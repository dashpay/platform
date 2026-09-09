import XCTest
@testable import SwiftExampleApp

@MainActor
final class WalletSyncCoordinatorLifecycleTests: XCTestCase {
    private struct LifecycleError: Error {}

    func testStopFailureDoesNotSkipDpnsStop() {
        var attempted: [String] = []
        var failures: [String] = []

        stopWalletSyncCoordinatorsBestEffort(
            stopPlatformAddress: {
                attempted.append("platform address")
                throw LifecycleError()
            },
            stopShielded: {
                attempted.append("shielded")
            },
            stopDashPay: {
                attempted.append("DashPay")
                throw LifecycleError()
            },
            stopDpns: {
                attempted.append("DPNS")
            }
        ) { coordinator, _ in
            failures.append(coordinator)
        }

        XCTAssertEqual(
            attempted,
            ["platform address", "shielded", "DashPay", "DPNS"]
        )
        XCTAssertEqual(failures, ["platform address", "DashPay"])
    }

    func testStartFailureDoesNotSkipDpnsCheckAndStart() {
        var attempted: [String] = []
        var failures: [String] = []

        ensureWalletSyncCoordinatorsRunningBestEffort(
            ensurePlatformAddress: {
                attempted.append("platform address check")
                throw LifecycleError()
            },
            ensureShielded: {
                attempted.append("shielded check")
            },
            ensureDashPay: {
                attempted.append("DashPay check")
                throw LifecycleError()
            },
            ensureDpns: {
                attempted.append("DPNS check")
                attempted.append("DPNS start")
            }
        ) { coordinator, _ in
            failures.append(coordinator)
        }

        XCTAssertEqual(
            attempted,
            [
                "platform address check",
                "shielded check",
                "DashPay check",
                "DPNS check",
                "DPNS start",
            ]
        )
        XCTAssertEqual(failures, ["platform address", "DashPay"])
    }
}
