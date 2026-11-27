import XCTest
@testable import SwiftDashSDK

/// Input guards of `PlatformWalletManager.masternodeProTxHashes(votingKeyId:)`.
/// The lookup itself needs a synced masternode list and is covered by the
/// Rust tests behind `platform_wallet_manager_masternodes_by_voting_key` and
/// by TEST_PLAN `CORE-25`.
@MainActor
final class MasternodesByVotingKeyTests: XCTestCase {
    /// `reason` is the guard's own message: both guards throw
    /// `.invalidParameter`, and every test here uses an unconfigured manager,
    /// so the case alone would not show which guard refused the call.
    private func assertInvalidParameter(
        _ votingKeyId: Data,
        reason: String,
        file: StaticString = #filePath,
        line: UInt = #line
    ) async {
        let manager = PlatformWalletManager()
        do {
            _ = try await manager.masternodeProTxHashes(votingKeyId: votingKeyId)
            XCTFail("expected invalidParameter, call succeeded", file: file, line: line)
        } catch PlatformWalletError.invalidParameter(let message) {
            XCTAssertEqual(message, reason, file: file, line: line)
        } catch {
            XCTFail("expected invalidParameter, got \(error)", file: file, line: line)
        }
    }

    func testShouldRejectAVotingKeyIdThatIsNotTwentyBytes() async {
        let reason = "votingKeyId must be 20 bytes"
        await assertInvalidParameter(Data(), reason: reason)
        await assertInvalidParameter(Data(repeating: 0xAA, count: 19), reason: reason)
        await assertInvalidParameter(Data(repeating: 0xAA, count: 21), reason: reason)
        // A 32-byte proTxHash passed by mistake.
        await assertInvalidParameter(Data(repeating: 0xAA, count: 32), reason: reason)
    }

    func testShouldRejectAnUnconfiguredManager() async {
        await assertInvalidParameter(
            Data(repeating: 0xAA, count: 20), reason: "Manager not configured")
    }
}
