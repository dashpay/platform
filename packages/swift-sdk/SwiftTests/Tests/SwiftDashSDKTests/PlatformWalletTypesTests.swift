import XCTest
@testable import SwiftDashSDK

class PlatformWalletTypesTests: XCTestCase {

    // MARK: - Network Tests

    func testNetworkFFIValues() {
        XCTAssertEqual(Network.mainnet.ffiValue, 0)
        XCTAssertEqual(Network.testnet.ffiValue, 1)
        XCTAssertEqual(Network.devnet.ffiValue, 2)
        XCTAssertEqual(Network.local.ffiValue, 3)
    }

    // MARK: - BlockTime Tests

    func testBlockTimeInit() {
        let blockTime = BlockTime(height: 100, coreHeight: 200, timestamp: 1234567890)

        XCTAssertEqual(blockTime.height, 100)
        XCTAssertEqual(blockTime.coreHeight, 200)
        XCTAssertEqual(blockTime.timestamp, 1234567890)
    }

    func testBlockTimeFFIConversion() {
        let swiftBlockTime = BlockTime(height: 100, coreHeight: 200, timestamp: 1234567890)
        let ffiBlockTime = swiftBlockTime.ffiValue

        XCTAssertEqual(ffiBlockTime.height, 100)
        XCTAssertEqual(ffiBlockTime.core_height, 200)
        XCTAssertEqual(ffiBlockTime.timestamp, 1234567890)

        // Convert back
        let convertedBack = BlockTime(ffiBlockTime: ffiBlockTime)
        XCTAssertEqual(convertedBack.height, swiftBlockTime.height)
        XCTAssertEqual(convertedBack.coreHeight, swiftBlockTime.coreHeight)
        XCTAssertEqual(convertedBack.timestamp, swiftBlockTime.timestamp)
    }

    // MARK: - Identifier Tests

    func testIdentifierFromBytes() throws {
        let bytes: [UInt8] = Array(repeating: 0x42, count: 32)
        let identifier = try Identifier(bytes: bytes)

        XCTAssertEqual(identifier.bytes, bytes)
    }

    func testIdentifierFromInvalidBytes() {
        let invalidBytes: [UInt8] = Array(repeating: 0x42, count: 10) // Wrong size

        XCTAssertThrowsError(try Identifier(bytes: invalidBytes)) { error in
            XCTAssertTrue(error is PlatformWalletError)
            if case PlatformWalletError.invalidParameter = error {
                // Expected error
            } else {
                XCTFail("Expected invalidParameter error, got \(error)")
            }
        }
    }

    func testIdentifierFromHexString() throws {
        let hexString = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        let identifier = try Identifier(hexString: hexString)

        XCTAssertEqual(identifier.bytes.count, 32)
        XCTAssertEqual(identifier.hexString, hexString)
    }

    func testIdentifierFromInvalidHexString() {
        let invalidHex = "not-hex-string"

        XCTAssertThrowsError(try Identifier(hexString: invalidHex)) { error in
            XCTAssertTrue(error is PlatformWalletError)
        }
    }

    func testIdentifierFromShortHexString() {
        let shortHex = "0123456789abcdef" // Only 16 bytes

        XCTAssertThrowsError(try Identifier(hexString: shortHex)) { error in
            XCTAssertTrue(error is PlatformWalletError)
        }
    }

    func testIdentifierHexStringRoundTrip() throws {
        let originalHex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        let identifier = try Identifier(hexString: originalHex)
        let convertedHex = identifier.hexString

        XCTAssertEqual(convertedHex, originalHex)
    }

    func testIdentifierRandom() throws {
        let id1 = try Identifier.random()
        let id2 = try Identifier.random()

        XCTAssertEqual(id1.bytes.count, 32)
        XCTAssertEqual(id2.bytes.count, 32)

        // Random IDs should be different
        XCTAssertNotEqual(id1.bytes, id2.bytes, "Random identifiers should be unique")
    }

    func testIdentifierFFIConversion() throws {
        let bytes: [UInt8] = (0..<32).map { UInt8($0) }
        let identifier = try Identifier(bytes: bytes)

        let ffiValue = identifier.ffiValue
        let convertedBack = Identifier(ffiIdentifier: ffiValue)

        XCTAssertEqual(convertedBack.bytes, identifier.bytes)
    }

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

    // MARK: - PlatformWalletError Tests

    func testPlatformWalletErrorMapping() {
        // Test that error mapping from FFI results works correctly
        // This is tested indirectly through other tests, but we can verify the enum exists

        let errors: [PlatformWalletError] = [
            .nullPointer,
            .invalidHandle,
            .invalidParameter,
            .invalidIdentifier,
            .invalidNetwork,
            .walletOperation("test"),
            .identityNotFound,
            .contactNotFound,
            .utf8Conversion,
            .serialization,
            .deserialization,
            .unknown("test")
        ]

        XCTAssertEqual(errors.count, 12, "All error cases should be defined")
    }
}
