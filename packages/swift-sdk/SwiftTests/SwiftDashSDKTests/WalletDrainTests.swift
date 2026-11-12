import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

final class WalletDrainTests: XCTestCase {
    func testShouldKeepUnavailablePreviewAmountsAbsent() throws {
        var ffi = WalletDrainPreviewFFI()
        ffi.status = 2
        ffi.candidate_input_count = 501
        ffi.candidate_balance_duffs = .max
        ffi.estimated_l1_fee_duffs = .max
        ffi.estimated_recipient_credits = .max
        let preview = try WalletDrainPreview(ffi: ffi)
        XCTAssertEqual(preview.status, .inputLimitMayBeExceeded)
        XCTAssertEqual(preview.candidateInputCount, 501)
        XCTAssertEqual(preview.candidateBalanceDuffs, .max)
        XCTAssertNil(preview.estimatedL1FeeDuffs)
        XCTAssertNil(preview.estimatedRecipientCredits)
    }

    func testShouldPreserveExactOutpointAndErrorForPendingOperation() {
        var ffi = WalletDrainOutcomeFFI()
        ffi.state = 1
        ffi.has_out_point = true
        _ = withUnsafeMutableBytes(of: &ffi.out_point.txid) {
            $0.initializeMemory(as: UInt8.self, repeating: 7)
        }
        ffi.out_point.vout = 3
        let outcome = WalletDrainOutcome(
            ffi: ffi, error: .transactionBroadcastUnconfirmed("transport closed"))
        XCTAssertEqual(outcome.state, .pending)
        XCTAssertEqual(outcome.outpoint?.txidWire, Data(repeating: 7, count: 32))
        XCTAssertEqual(outcome.outpoint?.vout, 3)
        guard case .transactionBroadcastUnconfirmed = outcome.error else {
            return XCTFail("the typed ambiguity must survive alongside its outpoint")
        }
    }

    func testShouldTreatAnUnknownOutcomeStateAsPending() {
        var ffi = WalletDrainOutcomeFFI()
        ffi.state = 255
        let outcome = WalletDrainOutcome(ffi: ffi)
        XCTAssertEqual(outcome.state, .pending)
        XCTAssertNil(outcome.outpoint)
        guard case .deserialization = outcome.error else {
            return XCTFail("an unknown state must carry a decoding error")
        }
    }

    func testShouldPreserveTypedInputCapRefusal() {
        let error = PlatformWalletError(code: .errorAssetLockTooManyInputs, message: "101 exceeds 100")
        guard case .assetLockTooManyInputs(let detail) = error else {
            return XCTFail("input-cap refusal lost its typed case")
        }
        XCTAssertEqual(detail, "101 exceeds 100")
        XCTAssertEqual(PlatformWalletResultCode.errorAssetLockTooManyInputs.rawValue, 60)
    }

    @MainActor
    func testShouldRejectAnUnconfiguredManagerBeforeExecuting() async {
        let manager = PlatformWalletManager()
        let outcome = await manager.shieldedFundFromWalletDrain(
            walletId: Data(repeating: 1, count: 32),
            operationId: UUID(),
            recipientRaw43: Data(repeating: 2, count: 43)
        )
        XCTAssertEqual(outcome.state, .pending, "local validation cannot disprove an earlier dispatch of this UUID")
        XCTAssertNil(outcome.outpoint)
        guard case .invalidHandle = outcome.error else {
            return XCTFail("an unconfigured manager cannot start a drain")
        }
        do {
            _ = try await manager.shieldedWalletDrainStatus(
                walletId: Data(repeating: 1, count: 32), operationId: UUID())
            XCTFail("an unavailable status read must throw, not return notSent")
        } catch {
            guard case PlatformWalletError.invalidHandle = error else {
                return XCTFail("unexpected error: \(error)")
            }
        }
    }

    func testShouldKeepNativeStatusAuthoritativeWhileACallIsActive() {
        // Rust already registered or finished the operation: pass through.
        let outpoint = WalletDrainOutpoint(txidWire: Data(repeating: 7, count: 32), vout: 3)
        let pending = WalletDrainOutcome(state: .pending, outpoint: outpoint)
        XCTAssertEqual(
            PlatformWalletManager.walletDrainStatusDuringActiveCall(native: pending).outpoint,
            outpoint
        )
        let completed = WalletDrainOutcome(state: .completed, outpoint: outpoint)
        XCTAssertEqual(
            PlatformWalletManager.walletDrainStatusDuringActiveCall(native: completed).state,
            .completed
        )
        // Not yet in the Rust registry: an in-flight dispatch is pending, never "never sent".
        let notSent = WalletDrainOutcome(state: .notSent, outpoint: nil)
        let masked = PlatformWalletManager.walletDrainStatusDuringActiveCall(native: notSent)
        XCTAssertEqual(masked.state, .pending)
        XCTAssertNil(masked.outpoint)
    }
}
