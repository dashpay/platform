import DashSDKFFI
import XCTest

@testable import SwiftDashSDK

/// `PrivateDocumentReadCursor` crosses back to Rust exactly as it came: the
/// position, the proved store size (or its absence) and the key set.
final class PrivateDocumentReadCursorTests: XCTestCase {

    private func ffiCursor(provedTotalCount: UInt64?) -> PrivateDocumentCursorFFI {
        var ffi = PrivateDocumentCursorFFI()
        ffi.position = 3
        ffi.proved_total_count = provedTotalCount ?? 0
        ffi.has_proved_total_count = provedTotalCount != nil
        withUnsafeMutableBytes(of: &ffi.key_set) { bytes in
            for index in bytes.indices {
                bytes[index] = UInt8(index)
            }
        }
        return ffi
    }

    func testShouldRoundTripACursorThroughItsFFIForm() {
        for provedTotalCount in [UInt64(7), 0, nil] {
            let ffi = ffiCursor(provedTotalCount: provedTotalCount)
            let cursor = PrivateDocumentReadCursor(ffi: ffi)
            XCTAssertEqual(cursor.position, 3)
            XCTAssertEqual(cursor.provedTotalCount, provedTotalCount)
            XCTAssertEqual(cursor.keySet, Data(0..<32))

            let back = cursor.ffi
            XCTAssertEqual(back.position, ffi.position)
            XCTAssertEqual(back.proved_total_count, ffi.proved_total_count)
            XCTAssertEqual(back.has_proved_total_count, ffi.has_proved_total_count)
            XCTAssertEqual(
                withUnsafeBytes(of: back.key_set) { Data($0) },
                withUnsafeBytes(of: ffi.key_set) { Data($0) }
            )
        }
    }
}
