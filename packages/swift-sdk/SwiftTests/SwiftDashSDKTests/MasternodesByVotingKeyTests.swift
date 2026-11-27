import XCTest
@testable import SwiftDashSDK

/// Input guards of `PlatformWalletManager.masternodeProTxHashes(votingKeyId:)`.
/// The lookup itself needs a synced masternode list and is covered by the
/// Rust tests behind `platform_wallet_manager_masternodes_by_voting_key` and
/// by TEST_PLAN `CORE-25`.
@MainActor
final class MasternodesByVotingKeyTests: XCTestCase {
    private func assertInvalidParameter(
        _ votingKeyId: Data,
        file: StaticString = #filePath,
        line: UInt = #line
    ) async {
        let manager = PlatformWalletManager()
        do {
            _ = try await manager.masternodeProTxHashes(votingKeyId: votingKeyId)
            XCTFail("expected invalidParameter, call succeeded", file: file, line: line)
        } catch PlatformWalletError.invalidParameter {
            // expected
        } catch {
            XCTFail("expected invalidParameter, got \(error)", file: file, line: line)
        }
    }

    func testShouldRejectAVotingKeyIdThatIsNotTwentyBytes() async {
        await assertInvalidParameter(Data())
        await assertInvalidParameter(Data(repeating: 0xAA, count: 19))
        await assertInvalidParameter(Data(repeating: 0xAA, count: 21))
        // A 32-byte proTxHash passed by mistake.
        await assertInvalidParameter(Data(repeating: 0xAA, count: 32))
    }

    func testShouldRejectAnUnconfiguredManager() async {
        await assertInvalidParameter(Data(repeating: 0xAA, count: 20))
    }
}
