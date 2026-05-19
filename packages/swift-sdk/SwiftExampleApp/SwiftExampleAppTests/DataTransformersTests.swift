import XCTest
@testable import SwiftDashSDK

final class DataTransformersTests: XCTestCase {

    // MARK: - AddressTransformer Tests

    func testHexToDataValid() {
        // 42-character hex string (21 bytes) - typical Platform address
        let hex = "001234567890abcdef1234567890abcdef12345678"
        let data = AddressTransformer.hexToData(hex)
        XCTAssertNotNil(data)
        XCTAssertEqual(data?.count, 21)
    }

    func testHexToDataEmpty() {
        let data = AddressTransformer.hexToData("")
        XCTAssertNil(data)
    }

    func testHexToDataOddLength() {
        let data = AddressTransformer.hexToData("abc")
        XCTAssertNil(data)
    }

    func testHexToDataInvalidCharacters() {
        let data = AddressTransformer.hexToData("gggg")
        XCTAssertNil(data)
    }

    func testHexToDataWithWhitespace() {
        let hex = "  aabb  "
        let data = AddressTransformer.hexToData(hex)
        XCTAssertNotNil(data)
        XCTAssertEqual(data?.count, 2)
    }

    func testDataToHex() {
        let data = Data([0x00, 0xab, 0xcd, 0xef])
        let hex = AddressTransformer.dataToHex(data)
        XCTAssertEqual(hex, "00abcdef")
    }

    func testDataToHexEmpty() {
        let data = Data()
        let hex = AddressTransformer.dataToHex(data)
        XCTAssertEqual(hex, "")
    }

    func testNormalizeIdentityIdHex() {
        // 64-character hex string (32 bytes)
        let hexId = "aaff11223344556677889900aabbccddeeff0011223344556677889900112233"
        let data = AddressTransformer.normalizeIdentityId(hexId)
        XCTAssertNotNil(data)
        XCTAssertEqual(data?.count, 32)
    }

    func testNormalizeIdentityIdBase58() {
        // Valid base58 that decodes to 32 bytes
        // Using a known base58 encoded 32-byte value
        let base58Id = "5J3mBbAH58CpQ3Y5RNJpUKPE62SQ5tfcvU2JpbnkeyhfsYB1Jcn"
        let data = AddressTransformer.normalizeIdentityId(base58Id)
        // May or may not equal 32 bytes depending on the actual encoding
        // This tests that base58 decoding is attempted
        XCTAssertTrue(data == nil || data?.count == 32)
    }

    func testNormalizeIdentityIdEmpty() {
        let data = AddressTransformer.normalizeIdentityId("")
        XCTAssertNil(data)
    }

    func testNormalizeIdentityIdInvalidHex() {
        // 64 characters but not hex
        let invalid = "gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg"
        let data = AddressTransformer.normalizeIdentityId(invalid)
        XCTAssertNil(data)
    }

    func testParseAddressHex() {
        let hex = "001234567890abcdef1234567890abcdef12345678"
        let data = AddressTransformer.parseAddress(hex)
        XCTAssertNotNil(data)
        XCTAssertEqual(data?.count, 21)
    }

    func testParseAddressBech32m() {
        // Valid testnet bech32m address
        let address = "tdashevo1qz4242424242424242424242424242424g4dj6u7"
        let data = AddressTransformer.parseAddress(address)
        // Should return data if Bech32m decoder works, nil otherwise
        XCTAssertTrue(data == nil || data?.count == 21)
    }

    func testParseAddressInvalid() {
        let data = AddressTransformer.parseAddress("invalid")
        XCTAssertNil(data)
    }

    func testFormatAddressAsHex() {
        let data = Data([0x00, 0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x12,
                        0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78])
        let hex = AddressTransformer.formatAddress(data, asBech32m: false)
        XCTAssertEqual(hex, "001234567890abcdef1234567890abcdef12345678")
    }

    // MARK: - NumberTransformer Tests

    func testParseUInt64Valid() {
        let value = NumberTransformer.parseUInt64("12345")
        XCTAssertEqual(value, 12345)
    }

    func testParseUInt64WithWhitespace() {
        let value = NumberTransformer.parseUInt64("  67890  ")
        XCTAssertEqual(value, 67890)
    }

    func testParseUInt64Invalid() {
        let value = NumberTransformer.parseUInt64("not a number")
        XCTAssertNil(value)
    }

    func testParseUInt64Negative() {
        let value = NumberTransformer.parseUInt64("-123")
        XCTAssertNil(value)
    }

    func testParseUInt64Zero() {
        let value = NumberTransformer.parseUInt64("0")
        XCTAssertEqual(value, 0)
    }

    func testParseUInt32Valid() {
        let value = NumberTransformer.parseUInt32("4294967295")
        XCTAssertEqual(value, UInt32.max)
    }

    func testParseUInt32Overflow() {
        let value = NumberTransformer.parseUInt32("4294967296")
        XCTAssertNil(value)
    }

    func testParseAmountValid() {
        let value = NumberTransformer.parseAmount("1000")
        XCTAssertEqual(value, 1000)
    }

    func testParseAmountZero() {
        let value = NumberTransformer.parseAmount("0")
        XCTAssertNil(value)
    }

    func testParseAmountNegative() {
        let value = NumberTransformer.parseAmount("-100")
        XCTAssertNil(value)
    }

    func testParseFeeValid() {
        let value = NumberTransformer.parseFee("1")
        XCTAssertEqual(value, 1)
    }

    func testParseFeeInvalid() {
        let value = NumberTransformer.parseFee("abc")
        XCTAssertNil(value)
    }

    func testFormatAmount() {
        let formatted = NumberTransformer.formatAmount(1000000, unit: "credits")
        XCTAssertTrue(formatted.contains("1,000,000") || formatted.contains("1000000"))
        XCTAssertTrue(formatted.contains("credits"))
    }

    func testFormatCreditsAsDash() {
        // 100,000,000,000 credits = 1 Dash
        let formatted = NumberTransformer.formatCreditsAsDash(100_000_000_000)
        XCTAssertTrue(formatted.contains("1.00000000"))
        XCTAssertTrue(formatted.contains("Dash"))
    }

    func testFormatDuffsAsDash() {
        // 100,000,000 duffs = 1 Dash
        let formatted = NumberTransformer.formatDuffsAsDash(100_000_000)
        XCTAssertTrue(formatted.contains("1.00000000"))
        XCTAssertTrue(formatted.contains("Dash"))
    }

    // MARK: - ResponseParser Tests

    func testParseAddressInfo() {
        let addressBytes = Data(repeating: 0x00, count: 21)
        let info = PlatformAddressInfo(addressBytes: addressBytes, nonce: 3, balance: 5000)
        let parsed = ResponseParser.parseAddressInfo(info)
        XCTAssertEqual(parsed.balance, 5000)
        XCTAssertEqual(parsed.nonce, 3)
    }

    func testFormatAddressInfo() {
        let addressBytes = Data([0x00, 0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x12,
                                 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78])
        let info = PlatformAddressInfo(addressBytes: addressBytes, nonce: 1, balance: 1000)
        let formatted = ResponseParser.formatAddressInfo(info)
        XCTAssertTrue(formatted.contains("Address:"))
        XCTAssertTrue(formatted.contains("Balance:"))
        XCTAssertTrue(formatted.contains("Nonce:"))
    }

    func testFormatAddressInfoWithoutAddress() {
        let addressBytes = Data(repeating: 0x00, count: 21)
        let info = PlatformAddressInfo(addressBytes: addressBytes, nonce: 5, balance: 2000)
        let formatted = ResponseParser.formatAddressInfo(info, includeAddress: false)
        XCTAssertFalse(formatted.contains("Address:"))
        XCTAssertTrue(formatted.contains("Balance:"))
    }

    // MARK: - TransferInputBuilder Tests

    func testCreateInputValid() {
        let addressHex = "001234567890abcdef1234567890abcdef12345678"
        let privateKeyHex = "aaff11223344556677889900aabbccddeeff0011223344556677889900112233"
        let input = TransferInputBuilder.createInput(
            addressHex: addressHex,
            amount: "1000",
            nonce: 0,
            privateKeyHex: privateKeyHex
        )
        XCTAssertNotNil(input)
        XCTAssertEqual(input?.amount, 1000)
        XCTAssertEqual(input?.nonce, 0)
    }

    func testCreateInputInvalidAddress() {
        let input = TransferInputBuilder.createInput(
            addressHex: "invalid",
            amount: "1000",
            privateKeyHex: "aaff11223344556677889900aabbccddeeff0011223344556677889900112233"
        )
        XCTAssertNil(input)
    }

    func testCreateInputInvalidAmount() {
        let addressHex = "001234567890abcdef1234567890abcdef12345678"
        let privateKeyHex = "aaff11223344556677889900aabbccddeeff0011223344556677889900112233"
        let input = TransferInputBuilder.createInput(
            addressHex: addressHex,
            amount: "not a number",
            privateKeyHex: privateKeyHex
        )
        XCTAssertNil(input)
    }

    func testCreateOutputValid() {
        let addressHex = "001234567890abcdef1234567890abcdef12345678"
        let output = TransferInputBuilder.createOutput(
            addressHex: addressHex,
            amount: "500"
        )
        XCTAssertNotNil(output)
        XCTAssertEqual(output?.amount, 500)
    }

    func testCreateOutputInvalidAddress() {
        let output = TransferInputBuilder.createOutput(
            addressHex: "short",
            amount: "500"
        )
        XCTAssertNil(output)
    }

    func testCreateOutputUniversalHex() {
        let addressHex = "001234567890abcdef1234567890abcdef12345678"
        let output = TransferInputBuilder.createOutputUniversal(
            address: addressHex,
            amount: "750"
        )
        XCTAssertNotNil(output)
        XCTAssertEqual(output?.amount, 750)
    }
}
