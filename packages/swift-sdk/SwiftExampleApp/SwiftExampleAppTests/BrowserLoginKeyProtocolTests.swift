//
//  BrowserLoginKeyProtocolTests.swift
//  SwiftExampleAppTests
//
//  Wire format and cryptography of the Bluetooth login-key handoff.
//  The fixed vectors were computed independently in Python so the
//  browser (which implements the same derivations in JavaScript) and
//  the phone cannot drift apart unnoticed.
//

import CryptoKit
import SwiftDashSDK
import XCTest
@testable import SwiftExampleApp

final class BrowserLoginKeyProtocolTests: XCTestCase {
    private let appPrivateKey = Data(repeating: 0x11, count: 32)
    private let walletPrivateKey = Data(repeating: 0x22, count: 32)
    private let identityId = Data(repeating: 0x33, count: 32)
    private let contractId = Data(repeating: 0x44, count: 32)

    private var appPublicKey: Data {
        try! Secp256k1Primitives.compressedPublicKey(privateKey: appPrivateKey)
    }

    // MARK: - Request

    func testRequestRoundTrips() throws {
        let request = BrowserLoginKeyProtocol.BrowserLoginRequest(
            network: .devnet,
            appEphemeralPublicKey: appPublicKey,
            contractId: contractId,
            label: "Login to Yappr"
        )
        let bytes = try request.serialized()
        XCTAssertEqual(bytes.count, 1 + 1 + 33 + 32 + 1 + 14)
        XCTAssertEqual(bytes[0], 1)
        XCTAssertEqual(bytes[1], UInt8(ascii: "d"))
        XCTAssertEqual(try BrowserLoginKeyProtocol.BrowserLoginRequest.parse(bytes), request)
    }

    func testRequestRejectsWrongVersionUnknownNetworkAndBadPoint() throws {
        let request = BrowserLoginKeyProtocol.BrowserLoginRequest(
            network: .testnet,
            appEphemeralPublicKey: appPublicKey,
            contractId: contractId,
            label: "x"
        )
        var bytes = try request.serialized()

        bytes[0] = 2
        XCTAssertThrowsError(try BrowserLoginKeyProtocol.BrowserLoginRequest.parse(bytes)) {
            XCTAssertEqual($0 as? BrowserLoginKeyProtocol.ProtocolError, .unsupportedVersion(2))
        }
        bytes[0] = 1

        bytes[1] = UInt8(ascii: "z")
        XCTAssertThrowsError(try BrowserLoginKeyProtocol.BrowserLoginRequest.parse(bytes)) {
            XCTAssertEqual($0 as? BrowserLoginKeyProtocol.ProtocolError, .unknownNetwork(UInt8(ascii: "z")))
        }
        bytes[1] = UInt8(ascii: "t")

        bytes[2] = 0x05  // not a compressed-point prefix
        XCTAssertThrowsError(try BrowserLoginKeyProtocol.BrowserLoginRequest.parse(bytes)) {
            XCTAssertEqual($0 as? BrowserLoginKeyProtocol.ProtocolError, .invalidEphemeralPublicKey)
        }

        XCTAssertThrowsError(try BrowserLoginKeyProtocol.BrowserLoginRequest.parse(Data(repeating: 1, count: 10))) {
            XCTAssertEqual($0 as? BrowserLoginKeyProtocol.ProtocolError, .truncatedRequest)
        }
    }

    func testRequestLabelLengthIsBounded() throws {
        let long = String(repeating: "a", count: 65)
        let request = BrowserLoginKeyProtocol.BrowserLoginRequest(
            network: .mainnet,
            appEphemeralPublicKey: appPublicKey,
            contractId: contractId,
            label: long
        )
        XCTAssertThrowsError(try request.serialized())
    }

    // MARK: - Response

    func testResponseRoundTripsAndEncodesAbsentLimitsAsZero() throws {
        let payload = Data((0..<60).map { UInt8($0) })
        let response = BrowserLoginKeyProtocol.BrowserLoginResponse(
            identityId: identityId,
            walletEphemeralPublicKey: appPublicKey,
            encryptedPayload: payload,
            keyId: 0x0102_0304,
            expiresAt: 1_800_000_000_000,
            totalBudget: nil
        )
        let bytes = response.serialized()
        XCTAssertEqual(bytes.count, BrowserLoginKeyProtocol.BrowserLoginResponse.length)
        XCTAssertEqual(Array(bytes[126..<130]), [1, 2, 3, 4])
        XCTAssertEqual(Array(bytes[138..<146]), [UInt8](repeating: 0, count: 8))
        XCTAssertEqual(try BrowserLoginKeyProtocol.BrowserLoginResponse.parse(bytes), response)
    }

    // MARK: - Cryptography

    /// HKDF-SHA256(ikm = 0x55 * 32, salt = 0x33 * 32, info = "auth", 32),
    /// computed with Python's hmac/hashlib.
    func testAuthKeyDerivationMatchesReferenceVector() throws {
        let loginKey = Data(repeating: 0x55, count: 32)
        let derived = try BrowserLoginKeyProtocol.deriveAuthPrivateKey(loginKey: loginKey, identityId: identityId)
        XCTAssertEqual(
            derived.toHexString(),
            "6f0aa5dd9454cb33b8a96b93a4d3b8936ce81bd8d2b9e054dee938f59c022e35"
        )
    }

    /// HKDF-SHA256(ikm = 0x66 * 32, salt = "dash:key-exchange:v1", info = "", 32).
    func testSharedKeyDerivationMatchesReferenceVector() {
        let key = BrowserLoginKeyProtocol.deriveSharedKey(sharedX: Data(repeating: 0x66, count: 32))
        XCTAssertEqual(
            key.withUnsafeBytes { Data($0) }.toHexString(),
            "47e69df69b24bbc45453bf907d78c6c6264c1eb15be5ae7563e2b8ea25a45539"
        )
    }

    func testSealedLoginKeyOpensWithTheOtherSidesKeys() throws {
        let loginKey = try BrowserLoginKeyProtocol.generateLoginKey()
        let walletPublicKey = try Secp256k1Primitives.compressedPublicKey(privateKey: walletPrivateKey)

        let sealed = try BrowserLoginKeyProtocol.seal(
            loginKey: loginKey,
            walletEphemeralPrivateKey: walletPrivateKey,
            appEphemeralPublicKey: appPublicKey
        )
        XCTAssertEqual(sealed.count, 60)

        let opened = try BrowserLoginKeyProtocol.open(
            encryptedPayload: sealed,
            appEphemeralPrivateKey: appPrivateKey,
            walletEphemeralPublicKey: walletPublicKey
        )
        XCTAssertEqual(opened, loginKey)

        var tampered = sealed
        tampered[20] ^= 0x01
        XCTAssertThrowsError(try BrowserLoginKeyProtocol.open(
            encryptedPayload: tampered,
            appEphemeralPrivateKey: appPrivateKey,
            walletEphemeralPublicKey: walletPublicKey
        ))
    }

    func testPairingCodeIsSixDigitsAndDeterministic() {
        let code = BrowserLoginKeyProtocol.pairingCode(for: appPublicKey)
        XCTAssertEqual(code.count, 6)
        XCTAssertTrue(code.allSatisfy(\.isNumber))
        XCTAssertEqual(code, BrowserLoginKeyProtocol.pairingCode(for: appPublicKey))
        XCTAssertEqual(code, "350178")
    }

    func testEphemeralKeyPairIsValid() throws {
        let pair = try BrowserLoginKeyProtocol.generateEphemeralKeyPair()
        XCTAssertEqual(pair.privateKey.count, 32)
        XCTAssertTrue(Secp256k1Primitives.isValidCompressedPoint(pair.publicKey))
        XCTAssertEqual(try Secp256k1Primitives.compressedPublicKey(privateKey: pair.privateKey), pair.publicKey)
    }
}
