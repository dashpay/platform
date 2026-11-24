import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// The resume form of an external asset lock enables "Complete External
/// Payment" only once the payees the lock recorded are loaded, and tells a
/// failed read apart from a lock that records none: a failed read offers a
/// retry instead of reading as "no recipients" or as a load that never ends.
final class RecordedPayeesLoadTests: XCTestCase {

    private struct ReadFailure: LocalizedError {
        var errorDescription: String? { "store unavailable" }
    }

    private let payee = ManagedPlatformAddressWallet.FundFromAssetLockRecipient(
        addressType: 0,
        hash: Data(repeating: 0x11, count: 20),
        credits: 5_000
    )

    func testRecordedPayeesLoadOnlyFromASuccessfulReadThatFindsThem() {
        let load = RecordedPayeesLoad(readResult: .success([payee]))
        XCTAssertEqual(load.loadedPayees?.map(\.hash), [payee.hash])
    }

    func testALockWithoutRecordedPayeesIsNotFoundAndKeepsSubmitDisabled() {
        for result: Result<[ManagedPlatformAddressWallet.FundFromAssetLockRecipient]?, Error>
            in [.success(nil), .success([])] {
            let load = RecordedPayeesLoad(readResult: result)
            guard case .notFound = load else {
                return XCTFail("expected notFound, got \(load)")
            }
            XCTAssertNil(load.loadedPayees)
        }
    }

    func testAFailedReadIsReportedAsAFailureNotAsNotFound() {
        let load = RecordedPayeesLoad(readResult: .failure(ReadFailure()))
        guard case .failed(let message) = load else {
            return XCTFail("expected failed, got \(load)")
        }
        XCTAssertEqual(message, "store unavailable")
        XCTAssertNil(load.loadedPayees)
    }

    func testWaitingForTheWalletAndLoadingKeepSubmitDisabled() {
        XCTAssertNil(RecordedPayeesLoad.waitingForWallet.loadedPayees)
        XCTAssertNil(RecordedPayeesLoad.loading.loadedPayees)
    }
}
