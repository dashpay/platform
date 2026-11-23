import XCTest
@testable import SwiftDashSDK

final class Base58IdentifierDecodingTests: XCTestCase {
    func testAllZeroIdentifierDecodesToOneByteForEachLeadingOne() {
        let thirtyTwo = Data.identifier(fromBase58: String(repeating: "1", count: 32))
        XCTAssertEqual(thirtyTwo, Data(count: 32))

        let thirtyOne = Data.identifier(fromBase58: String(repeating: "1", count: 31))
        XCTAssertEqual(thirtyOne?.count, 31)

        XCTAssertEqual(Data.identifier(fromBase58: "1"), Data([0]))
    }

    func testIdentifierRoundTripsThroughBase58() {
        var bytes = Data(count: 32)
        bytes[31] = 1
        XCTAssertEqual(Data.identifier(fromBase58: bytes.toBase58String()), bytes)

        let leadingZero = Data([0, 0] + (0..<30).map { UInt8($0 + 1) })
        XCTAssertEqual(Data.identifier(fromBase58: leadingZero.toBase58String()), leadingZero)

        let full = Data((0..<32).map { UInt8(255 - $0) })
        XCTAssertEqual(Data.identifier(fromBase58: full.toBase58String()), full)
    }

    func testRejectsCharactersOutsideTheAlphabet() {
        XCTAssertNil(Data.identifier(fromBase58: "0OIl"))
    }
}
