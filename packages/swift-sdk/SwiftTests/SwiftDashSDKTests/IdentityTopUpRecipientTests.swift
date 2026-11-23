import DashSDKFFI
import Foundation
import XCTest
@testable import SwiftDashSDK

/// The Swift side of topping up an identity this wallet does not manage:
/// code 81, the built-outpoint report of the external top-up, and the
/// argument checks of the three wrappers. The wrappers run against an
/// unknown wallet handle, so the native side answers before building
/// anything.
final class IdentityTopUpRecipientTests: XCTestCase {
    private let identityId = Data(repeating: 7, count: 32)
    private let txid = Data((0..<32).map(UInt8.init))
    private let wallet = ManagedPlatformWallet(handle: NULL_HANDLE, walletId: Data(repeating: 1, count: 32))

    /// Thread-safe sink for the outpoints a top-up reports.
    private final class Reported: @unchecked Sendable {
        private let lock = NSLock()
        private var values: [AssetLockBuiltOutPoint] = []
        func append(_ value: AssetLockBuiltOutPoint) { lock.withLock { values.append(value) } }
        var all: [AssetLockBuiltOutPoint] { lock.withLock { values } }
    }

    // MARK: - Code 82

    func testShouldMapCode82ToAssetLockRecipientUnknown() {
        XCTAssertEqual(PlatformWalletResultCode.errorAssetLockRecipientUnknown.rawValue, 82)
        XCTAssertEqual(
            PlatformWalletResultCode(ffi: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_RECIPIENT_UNKNOWN),
            .errorAssetLockRecipientUnknown)
        let ffi = PlatformWalletFFIResult(
            code: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_ASSET_LOCK_RECIPIENT_UNKNOWN,
            message: strdup("no recorded recipient"))
        XCTAssertThrowsError(try ffi.check()) { error in
            guard case PlatformWalletError.assetLockRecipientUnknown(let detail) = error else {
                return XCTFail("expected assetLockRecipientUnknown, got \(error)")
            }
            XCTAssertEqual(detail, "no recorded recipient")
            XCTAssertEqual(error.localizedDescription, "no recorded recipient")
        }
    }

    // MARK: - Built outpoint

    func testShouldReportNoOutPointWhenTheFFILeftItZero() {
        let zero = OutPointFFI(txid: (
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0), vout: 0)
        XCTAssertNil(AssetLockBuiltOutPoint(reported: zero))
    }

    func testShouldDecodeTheBuiltOutPointInWireOrder() throws {
        var out = OutPointFFI(txid: (
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0), vout: 3)
        // A zero txid with a non-zero vout is still a built lock.
        XCTAssertEqual(AssetLockBuiltOutPoint(reported: out)?.vout, 3)
        txid.withUnsafeBytes { src in
            Swift.withUnsafeMutableBytes(of: &out.txid) { $0.copyMemory(from: src) }
        }
        let built = try XCTUnwrap(AssetLockBuiltOutPoint(reported: out))
        XCTAssertEqual(built.txid, txid)
        XCTAssertEqual(built.vout, 3)
        XCTAssertEqual(built, AssetLockBuiltOutPoint(rawBytes: txid + Data([3, 0, 0, 0])))
    }

    // MARK: - Wrappers

    func testShouldRefuseAMalformedIdentityBeforeTheNativeCall() async {
        let reported = Reported()
        do {
            _ = try await wallet.topUpExternalIdentityWithFunding(
                identityId: Data(repeating: 7, count: 31), amountDuffs: 50_500, accountIndex: 0,
                onAssetLockBuilt: { reported.append($0) })
            XCTFail("expected invalidParameter")
        } catch PlatformWalletError.invalidParameter {
        } catch {
            XCTFail("expected invalidParameter, got \(error)")
        }
        XCTAssertTrue(reported.all.isEmpty)

        do {
            try await wallet.assignTopUpRecipient(
                outPointTxid: txid, outPointVout: 0, identityId: Data(repeating: 7, count: 33))
            XCTFail("expected invalidParameter")
        } catch PlatformWalletError.invalidParameter {
        } catch {
            XCTFail("expected invalidParameter, got \(error)")
        }
    }

    func testShouldRefuseAMalformedTxidBeforeTheNativeCall() async {
        for call in [
            { [wallet, identityId] in
                try await wallet.assignTopUpRecipient(
                    outPointTxid: Data(repeating: 1, count: 31), outPointVout: 0, identityId: identityId)
            },
            { [wallet] in
                _ = try await wallet.resumeTopUpToRecordedRecipient(
                    outPointTxid: Data(repeating: 1, count: 33), outPointVout: 0)
            },
        ] as [@Sendable () async throws -> Void] {
            do {
                try await call()
                XCTFail("expected invalidParameter")
            } catch PlatformWalletError.invalidParameter {
            } catch {
                XCTFail("expected invalidParameter, got \(error)")
            }
        }
    }

    /// An unknown wallet handle fails before any lock is built: the error
    /// reaches the caller and no outpoint is reported, so a new call is safe.
    func testShouldReportNoOutPointWhenNothingWasBuilt() async {
        let reported = Reported()
        do {
            _ = try await wallet.topUpExternalIdentityWithFunding(
                identityId: identityId, amountDuffs: 50_500, accountIndex: 0,
                onAssetLockBuilt: { reported.append($0) })
            XCTFail("expected notFound")
        } catch PlatformWalletError.notFound {
        } catch {
            XCTFail("expected notFound, got \(error)")
        }
        XCTAssertTrue(reported.all.isEmpty)
    }

    func testShouldPassTheResumeAndTheAssignmentToTheNativeWallet() async {
        do {
            _ = try await wallet.resumeTopUpToRecordedRecipient(outPointTxid: txid, outPointVout: 1)
            XCTFail("expected notFound")
        } catch PlatformWalletError.notFound {
        } catch {
            XCTFail("expected notFound, got \(error)")
        }
        do {
            try await wallet.assignTopUpRecipient(outPointTxid: txid, outPointVout: 1, identityId: identityId)
            XCTFail("expected notFound")
        } catch PlatformWalletError.notFound {
        } catch {
            XCTFail("expected notFound, got \(error)")
        }
    }
}
