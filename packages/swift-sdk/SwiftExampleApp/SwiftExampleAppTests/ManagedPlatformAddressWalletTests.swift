import XCTest
@testable import SwiftDashSDK

final class ManagedPlatformAddressWalletTests: XCTestCase {

    /// Convert the FFI's 20-byte tuple back to Data for assertion.
    private func hashData(_ entry: AddressBalanceEntryFFI) -> Data {
        withUnsafeBytes(of: entry.address.hash) { Data($0) }
    }

    // Pre-fix this returned changeIndex == 1 (insertion order, change was
    // last). Rust then indexed sorted row 1 (the recipient) and carved the
    // fee out of them. Regression scenario from issue #3738.
    func test_buildSortedFFIOutputs_changeSortsBeforeRecipient_indexIsZero() {
        let recipientHash = Data(repeating: 0xFF, count: 20)
        let changeHash = Data(repeating: 0x00, count: 20)
        let recipient = ManagedPlatformAddressWallet.TransferOutput(
            addressType: 0,
            hash: recipientHash,
            credits: 100
        )
        let change = (
            addressType: UInt8(0),
            hash: changeHash,
            balance: UInt64(50)
        )

        let (rows, changeIndex) = ManagedPlatformAddressWallet.buildSortedFFIOutputs(
            recipients: [recipient],
            change: change
        )

        XCTAssertEqual(changeIndex, 0)
        XCTAssertEqual(rows.count, 2)
        XCTAssertEqual(hashData(rows[0]), changeHash, "row 0 = change address (0x00…)")
        XCTAssertEqual(rows[0].balance, 50)
        XCTAssertEqual(hashData(rows[1]), recipientHash, "row 1 = recipient address (0xFF…)")
        XCTAssertEqual(rows[1].balance, 100)
    }

    // Multi-recipient: change address sorts into the MIDDLE of the
    // output list. Defends against an off-by-one or
    // last-position-assumption regression in the helper, and crosses
    // the 0x7F/0x80 byte boundary so that any accidental signed-byte
    // comparison would flip the order and fail the test.
    func test_buildSortedFFIOutputs_multipleRecipients_changeInMiddle() {
        let lowRecipientHash = Data(repeating: 0x10, count: 20)
        let changeHash = Data(repeating: 0x80, count: 20)
        let highRecipientHash = Data(repeating: 0xF0, count: 20)
        let recipients = [
            ManagedPlatformAddressWallet.TransferOutput(
                addressType: 0,
                hash: lowRecipientHash,
                credits: 100
            ),
            ManagedPlatformAddressWallet.TransferOutput(
                addressType: 0,
                hash: highRecipientHash,
                credits: 200
            ),
        ]
        let change = (
            addressType: UInt8(0),
            hash: changeHash,
            balance: UInt64(75)
        )

        let (rows, changeIndex) = ManagedPlatformAddressWallet.buildSortedFFIOutputs(
            recipients: recipients,
            change: change
        )

        XCTAssertEqual(rows.count, 3)
        XCTAssertEqual(changeIndex, 1, "change at 0x80… sorts between 0x10… and 0xF0…")
        XCTAssertEqual(hashData(rows[0]), lowRecipientHash)
        XCTAssertEqual(rows[0].balance, 100)
        XCTAssertEqual(hashData(rows[1]), changeHash)
        XCTAssertEqual(rows[1].balance, 75)
        XCTAssertEqual(hashData(rows[2]), highRecipientHash)
        XCTAssertEqual(rows[2].balance, 200)
    }
}
