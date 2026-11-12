import XCTest

@testable import SwiftDashSDK

/// `SDK.quorumSource` must report the source the native provider actually
/// uses. A strict SPV SDK can never switch modes, so a value remembered from
/// the last mode change would show `.trusted` for its whole lifetime.
///
/// Regtest needs no network to build: its DAPI and quorum endpoints are
/// loopback defaults, and a strict SPV SDK does not prefetch trusted quorums.
final class SDKQuorumSourceTests: XCTestCase {

    func testShouldReportSpvForAStrictSpvSdk() throws {
        let sdk = try SDK(network: .regtest, contextProvider: .spv)

        XCTAssertEqual(sdk.quorumSource, .spv)
    }

    /// A provider that does not say where its quorum keys come from, here the
    /// mock's, must not be shown as the trusted service.
    func testShouldReportCustomForAProviderThatNamesNoSource() throws {
        let sdk = try SDK(mockVectorsDirectory: nil)

        XCTAssertEqual(sdk.quorumSource, .custom)
    }
}
