import Foundation
import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// Hermetic tests for decoding the rows `platform_wallet_identity_withdrawals`
/// writes. The C values are built here in Swift, so nothing calls into Rust:
/// no wallet handle, no network.
final class IdentityWithdrawalDecodingTests: XCTestCase {
    private let withdrawalId = Data(repeating: 0x11, count: 32)
    private let ownerId = Data(repeating: 0x22, count: 32)

    /// Run `body` with a row whose strings and script point at buffers that
    /// stay alive for the call, as Rust's allocations do until the free.
    private func withRow<R>(
        status: UInt8,
        statusLabel: String? = "Broadcasted",
        isFinal: Bool = false,
        destination: String? = nil,
        script: [UInt8] = [],
        transactionIndex: UInt64? = nil,
        transactionSignHeight: UInt32? = nil,
        _ body: (IdentityWithdrawalFFI) throws -> R
    ) rethrows -> R {
        var row = IdentityWithdrawalFFI()
        _ = withUnsafeMutableBytes(of: &row.id) { withdrawalId.copyBytes(to: $0) }
        _ = withUnsafeMutableBytes(of: &row.owner_id) { ownerId.copyBytes(to: $0) }
        row.amount = 150_000
        row.core_fee_per_byte = 1
        row.status = status
        row.is_final = isFinal
        row.has_transaction_index = transactionIndex != nil
        // A value behind a false flag must not leak through.
        row.transaction_index = transactionIndex ?? 99
        row.has_transaction_sign_height = transactionSignHeight != nil
        row.transaction_sign_height = transactionSignHeight ?? 77
        row.created_at_ms = 1_700_000_000_000
        row.updated_at_ms = 1_700_000_600_000

        let label = statusLabel.map { strdup($0)! }
        let address = destination.map { strdup($0)! }
        defer {
            free(label)
            free(address)
        }
        row.status_label = label
        row.destination = address
        var scriptBytes = script
        return try scriptBytes.withUnsafeMutableBufferPointer { scriptBp in
            row.output_script = script.isEmpty ? nil : scriptBp.baseAddress
            row.output_script_len = UInt(script.count)
            return try body(row)
        }
    }

    func testShouldDecodeABroadcastWithdrawalWithItsCoreTransaction() throws {
        let decoded = try withRow(
            status: 2,
            destination: "yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf",
            script: [0x76, 0xa9, 0x14],
            transactionIndex: 42,
            transactionSignHeight: 1_234_567
        ) { try IdentityWithdrawal(ffi: $0) }

        XCTAssertEqual(
            decoded,
            IdentityWithdrawal(
                id: withdrawalId,
                ownerId: ownerId,
                amountCredits: 150_000,
                coreFeePerByte: 1,
                status: .broadcasted,
                statusLabel: "Broadcasted",
                isFinal: false,
                destination: "yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf",
                outputScript: Data([0x76, 0xa9, 0x14]),
                transactionIndex: 42,
                transactionSignHeight: 1_234_567,
                createdAtMs: 1_700_000_000_000,
                updatedAtMs: 1_700_000_600_000
            )
        )
    }

    func testShouldLeaveOptionalFieldsNilWhenTheirFlagIsFalse() throws {
        let decoded = try withRow(status: 0, statusLabel: "Queued") {
            try IdentityWithdrawal(ffi: $0)
        }

        XCTAssertEqual(decoded.status, .queued)
        XCTAssertNil(decoded.transactionIndex)
        XCTAssertNil(decoded.transactionSignHeight)
        XCTAssertNil(decoded.destination)
        XCTAssertEqual(decoded.outputScript, Data())
    }

    func testShouldMirrorEveryKnownStatusCodeAndTakeFinalityAsWritten() throws {
        let cases: [(UInt8, WithdrawalStatus)] = [
            (0, .queued), (1, .pooled), (2, .broadcasted),
            (3, .complete), (4, .expired), (5, .failed),
        ]
        for (code, expected) in cases {
            let decoded = try withRow(status: code, statusLabel: "label \(code)") {
                try IdentityWithdrawal(ffi: $0)
            }
            XCTAssertEqual(decoded.status, expected)
            XCTAssertEqual(decoded.statusLabel, "label \(code)")
        }
        // Finality is Rust's answer, carried as written, not derived from the
        // status in Swift.
        let finalQueued = try withRow(status: 0, isFinal: true) { try IdentityWithdrawal(ffi: $0) }
        XCTAssertTrue(finalQueued.isFinal)
        let openComplete = try withRow(status: 3, isFinal: false) {
            try IdentityWithdrawal(ffi: $0)
        }
        XCTAssertFalse(openComplete.isFinal)
    }

    func testShouldThrowForAnUnknownStatusCode() throws {
        try withRow(status: 6) { row in
            XCTAssertThrowsError(try IdentityWithdrawal(ffi: row)) { error in
                guard case PlatformWalletError.deserialization(let message) = error else {
                    return XCTFail("unexpected error \(error)")
                }
                XCTAssertTrue(message.contains("6"), message)
            }
        }
        try withRow(status: 255) { row in
            XCTAssertThrowsError(try IdentityWithdrawal(ffi: row))
        }
    }

    func testShouldThrowForARowWithoutAStatusLabel() throws {
        try withRow(status: 0, statusLabel: nil) { row in
            XCTAssertThrowsError(try IdentityWithdrawal(ffi: row))
        }
    }

    func testShouldDecodeAPageAndItsNextCursorOnlyWhenThereIsANextPage() throws {
        let cursor = [UInt8](repeating: 0x33, count: 32)
        let withNext = try withRow(status: 1, statusLabel: "Pooled") { row -> IdentityWithdrawalsPage in
            var rows = [row, row]
            return try rows.withUnsafeMutableBufferPointer { bp in
                try IdentityWithdrawalsPage.decode(
                    rows: UnsafePointer(bp.baseAddress),
                    count: bp.count,
                    hasNext: true,
                    nextStartAfter: cursor
                )
            }
        }
        XCTAssertEqual(withNext.withdrawals.count, 2)
        XCTAssertEqual(withNext.withdrawals.first?.status, .pooled)
        XCTAssertEqual(withNext.nextStartAfter, Data(cursor))

        let last = try IdentityWithdrawalsPage.decode(
            rows: nil,
            count: 0,
            hasNext: false,
            nextStartAfter: cursor
        )
        XCTAssertEqual(last, IdentityWithdrawalsPage(withdrawals: [], nextStartAfter: nil))
    }

    func testShouldFailThePageWhenOneRowHasAnUnknownStatus() throws {
        try withRow(status: 9) { row in
            var rows = [row]
            try rows.withUnsafeMutableBufferPointer { bp in
                XCTAssertThrowsError(
                    try IdentityWithdrawalsPage.decode(
                        rows: UnsafePointer(bp.baseAddress),
                        count: bp.count,
                        hasNext: false,
                        nextStartAfter: [UInt8](repeating: 0, count: 32)
                    )
                )
            }
        }
    }
}
