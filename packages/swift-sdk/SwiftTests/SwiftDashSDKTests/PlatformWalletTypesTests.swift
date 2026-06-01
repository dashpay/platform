import XCTest
@testable import SwiftDashSDK

class PlatformWalletTypesTests: XCTestCase {
    // MARK: - Data Hex Extension Tests

    func testDataFromHexString() {
        let hexString = "48656c6c6f" // "Hello" in hex
        let data = Data(hexString: hexString)

        XCTAssertNotNil(data)
        XCTAssertEqual(data?.count, 5)

        if let data = data {
            let string = String(data: data, encoding: .utf8)
            XCTAssertEqual(string, "Hello")
        }
    }

    func testDataFromInvalidHexString() {
        let invalidHex = "xyz"
        let data = Data(hexString: invalidHex)

        XCTAssertNil(data)
    }

    func testDataFromOddLengthHexString() {
        let oddHex = "123" // Odd number of characters
        let data = Data(hexString: oddHex)

        // Should handle gracefully (depends on implementation)
        // Current implementation treats this as 1 byte
        XCTAssertNotNil(data)
    }
}
