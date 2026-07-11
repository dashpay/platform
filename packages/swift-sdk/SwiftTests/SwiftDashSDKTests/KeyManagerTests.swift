import XCTest
@testable import SwiftDashSDK

final class KeyManagerTests: XCTestCase {

    // MARK: - KeyFormatDetector Tests

    func testDetectFormatHex() {
        // Valid 64-character hex string
        let hex = "aaff11223344556677889900aabbccddeeff0011223344556677889900112233"
        XCTAssertEqual(KeyFormatDetector.detectFormat(hex), .hex)
    }

    func testDetectFormatHexUppercase() {
        let hex = "AAFF11223344556677889900AABBCCDDEEFF0011223344556677889900112233"
        XCTAssertEqual(KeyFormatDetector.detectFormat(hex), .hex)
    }

    func testDetectFormatHexMixedCase() {
        let hex = "AaFf11223344556677889900aAbBcCdDeEfF0011223344556677889900112233"
        XCTAssertEqual(KeyFormatDetector.detectFormat(hex), .hex)
    }

    func testDetectFormatHexTooShort() {
        let hex = "aaff1122334455"
        XCTAssertEqual(KeyFormatDetector.detectFormat(hex), .unknown)
    }

    func testDetectFormatHexTooLong() {
        let hex = "aaff11223344556677889900aabbccddeeff00112233445566778899001122330011"
        XCTAssertEqual(KeyFormatDetector.detectFormat(hex), .unknown)
    }

    func testDetectFormatWIF() {
        // Typical testnet WIF (starts with 'c')
        let wif = "cNJFgo1DriFnPcBVKuXxyFbE46W4fPmqbMbjt7sWjrCmb3F1F8Vy"
        XCTAssertEqual(KeyFormatDetector.detectFormat(wif), .wif)
    }

    func testDetectFormatWIFMainnet() {
        // Mainnet WIF (starts with '5')
        let wif = "5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ"
        XCTAssertEqual(KeyFormatDetector.detectFormat(wif), .wif)
    }

    func testDetectFormatEmpty() {
        XCTAssertEqual(KeyFormatDetector.detectFormat(""), .unknown)
    }

    func testDetectFormatWhitespace() {
        XCTAssertEqual(KeyFormatDetector.detectFormat("   "), .unknown)
    }

    func testDetectFormatUnknown() {
        XCTAssertEqual(KeyFormatDetector.detectFormat("not-a-key"), .unknown)
    }

    func testIsHexValid() {
        XCTAssertTrue(KeyFormatDetector.isHex("0123456789abcdef"))
        XCTAssertTrue(KeyFormatDetector.isHex("ABCDEF"))
        XCTAssertTrue(KeyFormatDetector.isHex(""))
    }

    func testIsHexInvalid() {
        XCTAssertFalse(KeyFormatDetector.isHex("ghijkl"))
        XCTAssertFalse(KeyFormatDetector.isHex("0x1234"))
        XCTAssertFalse(KeyFormatDetector.isHex("12 34"))
    }

    func testIsLikelyWIFValid() {
        XCTAssertTrue(KeyFormatDetector.isLikelyWIF("5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ"))
        XCTAssertTrue(KeyFormatDetector.isLikelyWIF("cNJFgo1DriFnPcBVKuXxyFbE46W4fPmqbMbjt7sWjrCmb3F1F8Vy"))
        XCTAssertTrue(KeyFormatDetector.isLikelyWIF("KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn"))
    }

    func testIsLikelyWIFInvalid() {
        XCTAssertFalse(KeyFormatDetector.isLikelyWIF("short"))
        XCTAssertFalse(KeyFormatDetector.isLikelyWIF("AHueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ"))
    }

    // MARK: - PrivateKeyParser Tests

    func testParseHexValid() {
        let hex = "aaff11223344556677889900aabbccddeeff0011223344556677889900112233"
        let result = PrivateKeyParser.parse(hex)
        XCTAssertTrue(result.isValid)
        XCTAssertEqual(result.format, .hex)
        XCTAssertEqual(result.data?.count, 32)
        XCTAssertNil(result.error)
    }

    func testParseHexWithWhitespace() {
        let hex = "  aaff11223344556677889900aabbccddeeff0011223344556677889900112233  "
        let result = PrivateKeyParser.parse(hex)
        XCTAssertTrue(result.isValid)
        XCTAssertEqual(result.data?.count, 32)
    }

    func testParseHexInvalidLength() {
        let hex = "aaff1122"
        let result = PrivateKeyParser.parseHex(hex)
        XCTAssertFalse(result.isValid)
        XCTAssertNotNil(result.error)
        XCTAssertTrue(result.error?.contains("64 characters") ?? false)
    }

    func testParseHexInvalidCharacters() {
        let hex = "ggff11223344556677889900aabbccddeeff0011223344556677889900112233"
        let result = PrivateKeyParser.parseHex(hex)
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.error?.contains("Invalid hex") ?? false)
    }

    func testParseEmpty() {
        let result = PrivateKeyParser.parse("")
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.error?.contains("empty") ?? false)
    }

    func testParseUnknownFormat() {
        let result = PrivateKeyParser.parse("not-a-valid-key")
        XCTAssertFalse(result.isValid)
        XCTAssertEqual(result.format, .unknown)
    }

    // MARK: - KeySizeValidator Tests

    func testExpectedPrivateKeySizeECDSA() {
        XCTAssertEqual(KeySizeValidator.expectedPrivateKeySize(for: .ecdsaSecp256k1), 32)
    }

    func testExpectedPrivateKeySizeBLS() {
        XCTAssertEqual(KeySizeValidator.expectedPrivateKeySize(for: .bls12_381), 32)
    }

    func testExpectedPrivateKeySizeEdDSA() {
        XCTAssertEqual(KeySizeValidator.expectedPrivateKeySize(for: .eddsa25519Hash160), 32)
    }

    func testIsValidSizeCorrect() {
        let key = Data(repeating: 0x00, count: 32)
        XCTAssertTrue(KeySizeValidator.isValidSize(key, for: .ecdsaSecp256k1))
    }

    func testIsValidSizeTooShort() {
        let key = Data(repeating: 0x00, count: 16)
        XCTAssertFalse(KeySizeValidator.isValidSize(key, for: .ecdsaSecp256k1))
    }

    func testIsValidSizeTooLong() {
        let key = Data(repeating: 0x00, count: 64)
        XCTAssertFalse(KeySizeValidator.isValidSize(key, for: .ecdsaSecp256k1))
    }

    // MARK: - KeyFormatter Tests

    func testToHex() {
        let data = Data([0xaa, 0xbb, 0xcc, 0xdd])
        let hex = KeyFormatter.toHex(data)
        XCTAssertEqual(hex, "aabbccdd")
    }

    func testToHexEmpty() {
        let data = Data()
        let hex = KeyFormatter.toHex(data)
        XCTAssertEqual(hex, "")
    }

    func testToWIFAndBack() {
        let originalKey = Data([
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
            0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
            0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff
        ])

        // Encode to WIF
        guard let wif = KeyFormatter.toWIF(originalKey, network: .testnet) else {
            XCTFail("Failed to encode to WIF")
            return
        }

        // Decode back
        guard let decodedKey = WIFParser.parseWIF(wif) else {
            XCTFail("Failed to decode WIF")
            return
        }

        XCTAssertEqual(originalKey, decodedKey)
    }

    func testToWIFInvalidSize() {
        let shortKey = Data(repeating: 0x00, count: 16)
        XCTAssertNil(KeyFormatter.toWIF(shortKey))
    }

    // MARK: - KeyValidator Tests

    func testValidatePrivateKeyInvalidSize() {
        let shortKey = Data(repeating: 0x00, count: 16)
        let result = KeyValidator.validatePrivateKey(shortKey, against: [])
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.error?.contains("32 bytes") ?? false)
    }

    func testValidatePrivateKeyNoPublicKeys() {
        let key = Data(repeating: 0x00, count: 32)
        let result = KeyValidator.validatePrivateKey(key, against: [])
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.error?.contains("No public keys") ?? false)
    }

    func testValidatePrivateKeyInputEmpty() {
        let result = KeyValidator.validatePrivateKeyInput("", against: [])
        XCTAssertFalse(result.isValid)
    }

    func testValidatePrivateKeyInputInvalidFormat() {
        let result = KeyValidator.validatePrivateKeyInput("not-a-key", against: [])
        XCTAssertFalse(result.isValid)
    }
}
