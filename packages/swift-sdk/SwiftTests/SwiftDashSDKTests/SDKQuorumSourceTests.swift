import XCTest

@testable import SwiftDashSDK

/// `SDK.quorumSource` must report the source the native provider actually
/// uses. A strict SPV SDK can never switch modes, so a value remembered from
/// the last mode change would show `.trusted` for its whole lifetime.
///
/// Regtest needs no network to build: its DAPI and quorum endpoints are
/// loopback defaults. A strict SPV SDK does not prefetch trusted quorums, and
/// the prefetch of one that does fails in the background without failing the
/// build.
final class SDKQuorumSourceTests: XCTestCase {

    /// An SDK built without naming a provider is adaptive, in Auto: with no
    /// wallet manager serving it SPV it asks the trusted service, and it
    /// takes the mode changes a trusted SDK refuses. Consumers that never
    /// chose a provider get SPV-backed proofs once a wallet manager they
    /// configure with the SDK has synced.
    func testShouldBuildAnAdaptiveSdkInAutoByDefault() throws {
        let sdk = try SDK(network: .regtest)

        XCTAssertEqual(sdk.quorumSource, .trusted, "Auto without SPV asks the trusted service")
        try sdk.setQuorumMode(.spv)
        XCTAssertEqual(sdk.quorumSource, .spv)
        try sdk.setQuorumMode(.trusted)
        XCTAssertEqual(sdk.quorumSource, .trusted)
    }

    /// Opting out of the default restores the trusted provider, which has
    /// one source and no mode to change.
    func testShouldRefuseAModeChangeOnAnExplicitTrustedSdk() throws {
        let sdk = try SDK(network: .regtest, contextProvider: .trusted)

        XCTAssertEqual(sdk.quorumSource, .trusted)
        XCTAssertThrowsError(try sdk.setQuorumMode(.spv))
    }

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
