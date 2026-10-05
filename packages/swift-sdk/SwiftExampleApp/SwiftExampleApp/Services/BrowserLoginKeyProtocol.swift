import CoreBluetooth
import CryptoKit
import Foundation
import SwiftDashSDK

/// The wire protocol a browser and this wallet speak over Bluetooth LE
/// to hand the browser a bounded login key.
///
/// The browser is the GATT central, the phone the peripheral. The
/// phone advertises `serviceUUID` with three characteristics:
///
///   - `requestCharacteristicUUID` (write): the browser writes one
///     `BrowserLoginRequest`.
///   - `statusCharacteristicUUID` (read + notify): one `Status` byte
///     the phone updates as it moves through the flow.
///   - `responseCharacteristicUUID` (read): the `BrowserLoginResponse`
///     once `Status.ready` is signalled, empty before that.
///
/// The cryptography is byte-for-byte the Yappr key-exchange envelope
/// that the `dash-key:` QR flow already uses, so the browser decrypts
/// a Bluetooth response with the same code as an on-chain
/// `loginKeyResponse` document:
///
///   sharedX  = ECDH(walletEphemeralPriv, appEphemeralPub).x
///   key      = HKDF-SHA256(ikm: sharedX, salt: "dash:key-exchange:v1", info: "", 32)
///   payload  = AES-256-GCM(key, nonce(12)) over loginKey(32) → nonce || ciphertext || tag (60 bytes)
///   authPriv = HKDF-SHA256(ikm: loginKey, salt: identityId, info: "auth", 32)
///
/// The identity key the phone registers for the browser is
/// `ECDSA_HASH160(authPub)`, AUTHENTICATION / HIGH, with the budget
/// and expiry the user chose. The phone never keeps `loginKey`.
enum BrowserLoginKeyProtocol {
    /// Protocol version carried by both the request and the response.
    static let version: UInt8 = 1

    static let serviceUUIDString = "8f9a3e10-5c2b-4d6e-9f1a-2b3c4d5e6f01"
    static let requestCharacteristicUUIDString = "8f9a3e10-5c2b-4d6e-9f1a-2b3c4d5e6f02"
    static let statusCharacteristicUUIDString = "8f9a3e10-5c2b-4d6e-9f1a-2b3c4d5e6f03"
    static let responseCharacteristicUUIDString = "8f9a3e10-5c2b-4d6e-9f1a-2b3c4d5e6f04"

    // `CBUUID` is not Sendable, so the constants above are the strings
    // and these build a fresh value per use.
    static var serviceUUID: CBUUID { CBUUID(string: serviceUUIDString) }
    static var requestCharacteristicUUID: CBUUID { CBUUID(string: requestCharacteristicUUIDString) }
    static var statusCharacteristicUUID: CBUUID { CBUUID(string: statusCharacteristicUUIDString) }
    static var responseCharacteristicUUID: CBUUID { CBUUID(string: responseCharacteristicUUIDString) }

    /// Longest app label the request may carry, in UTF-8 bytes.
    static let maxLabelLength = 64

    /// Salt of the HKDF step that turns the ECDH x-coordinate into the
    /// AES key. Shared with the QR flow, hence the unversioned name.
    static let sharedSecretSalt = Data("dash:key-exchange:v1".utf8)

    /// Value of the status characteristic.
    enum Status: UInt8 {
        /// Advertising, no request received yet.
        case idle = 0
        /// A request arrived and the user is being asked to confirm it.
        case awaitingConfirmation = 1
        /// The user confirmed; the key is being registered on Platform.
        case registering = 2
        /// The response characteristic holds the encrypted login key.
        case ready = 3
        /// The user declined the request.
        case rejected = 4
        /// The request was malformed, for another network, or the
        /// registration failed.
        case failed = 5
    }

    /// The network a request is for, as the single ASCII byte the
    /// `dash-key:` URI uses in its `n=` parameter.
    enum NetworkTag: UInt8 {
        case mainnet = 0x6d  // "m"
        case testnet = 0x74  // "t"
        case devnet = 0x64  // "d"

        init?(network: Network) {
            switch network {
            case .mainnet: self = .mainnet
            case .testnet: self = .testnet
            case .devnet: self = .devnet
            case .regtest: return nil
            }
        }

        var network: Network {
            switch self {
            case .mainnet: return .mainnet
            case .testnet: return .testnet
            case .devnet: return .devnet
            }
        }
    }

    enum ProtocolError: LocalizedError, Equatable {
        case truncatedRequest
        case unsupportedVersion(UInt8)
        case unknownNetwork(UInt8)
        case invalidEphemeralPublicKey
        case labelNotUTF8
        case labelTooLong(Int)
        case invalidIdentityId
        case randomnessUnavailable
        case ephemeralKeyGenerationFailed

        var errorDescription: String? {
            switch self {
            case .truncatedRequest:
                return "The browser's request was shorter than the protocol requires."
            case .unsupportedVersion(let version):
                return "The browser spoke protocol version \(version); this wallet only speaks version \(BrowserLoginKeyProtocol.version)."
            case .unknownNetwork(let tag):
                return "The browser asked for an unknown network (tag \(tag))."
            case .invalidEphemeralPublicKey:
                return "The browser's ephemeral public key is not a valid secp256k1 point."
            case .labelNotUTF8:
                return "The app label is not valid UTF-8."
            case .labelTooLong(let length):
                return "The app label is \(length) bytes; at most \(BrowserLoginKeyProtocol.maxLabelLength) are allowed."
            case .invalidIdentityId:
                return "The identity id must be 32 bytes."
            case .randomnessUnavailable:
                return "The system random number generator was unavailable."
            case .ephemeralKeyGenerationFailed:
                return "Could not generate a valid ephemeral secp256k1 key."
            }
        }
    }

    // MARK: - Request

    /// What the browser writes to the request characteristic:
    ///
    ///     version(1) || network(1) || appEphemeralPubKey(33) || contractId(32) || labelLen(1) || label(labelLen)
    struct BrowserLoginRequest: Equatable {
        static let minimumLength = 1 + 1 + 33 + 32 + 1

        let network: NetworkTag
        /// Compressed secp256k1 point the wallet does ECDH against.
        let appEphemeralPublicKey: Data
        /// The application contract the login is for.
        let contractId: Data
        /// Human-readable name the browser gave itself, e.g. "Login to Yappr".
        let label: String

        static func parse(_ bytes: Data) throws -> BrowserLoginRequest {
            guard bytes.count >= minimumLength else {
                throw ProtocolError.truncatedRequest
            }
            var cursor = bytes.startIndex
            let version = bytes[cursor]
            cursor += 1
            guard version == BrowserLoginKeyProtocol.version else {
                throw ProtocolError.unsupportedVersion(version)
            }
            guard let network = NetworkTag(rawValue: bytes[cursor]) else {
                throw ProtocolError.unknownNetwork(bytes[cursor])
            }
            cursor += 1
            let appEphemeralPublicKey = Data(bytes[cursor..<cursor + 33])
            cursor += 33
            guard Secp256k1Primitives.isValidCompressedPoint(appEphemeralPublicKey) else {
                throw ProtocolError.invalidEphemeralPublicKey
            }
            let contractId = Data(bytes[cursor..<cursor + 32])
            cursor += 32
            let labelLength = Int(bytes[cursor])
            cursor += 1
            guard labelLength <= maxLabelLength else {
                throw ProtocolError.labelTooLong(labelLength)
            }
            guard bytes.endIndex - cursor >= labelLength else {
                throw ProtocolError.truncatedRequest
            }
            guard let label = String(data: bytes[cursor..<cursor + labelLength], encoding: .utf8) else {
                throw ProtocolError.labelNotUTF8
            }
            return BrowserLoginRequest(
                network: network,
                appEphemeralPublicKey: appEphemeralPublicKey,
                contractId: contractId,
                label: label
            )
        }

        func serialized() throws -> Data {
            let labelBytes = Data(label.utf8)
            guard labelBytes.count <= maxLabelLength else {
                throw ProtocolError.labelTooLong(labelBytes.count)
            }
            var out = Data()
            out.append(BrowserLoginKeyProtocol.version)
            out.append(network.rawValue)
            out.append(appEphemeralPublicKey)
            out.append(contractId)
            out.append(UInt8(labelBytes.count))
            out.append(labelBytes)
            return out
        }

        /// Six decimal digits both sides derive from the browser's
        /// ephemeral public key. The browser shows them, the user checks
        /// the phone shows the same ones before confirming, which rules
        /// out a third radio in range answering the browser's request.
        var pairingCode: String {
            BrowserLoginKeyProtocol.pairingCode(for: appEphemeralPublicKey)
        }
    }

    static func pairingCode(for appEphemeralPublicKey: Data) -> String {
        let digest = hash160(appEphemeralPublicKey)
        let value = digest.prefix(4).reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
        return String(format: "%06d", value % 1_000_000)
    }

    /// RIPEMD160(SHA256(data)), the identity-key hash Platform uses.
    static func hash160(_ data: Data) -> Data {
        Data(hexString: SwiftDashSDK.KeychainManager.computePublicKeyHashHex(data)) ?? Data()
    }

    // MARK: - Response

    /// What the phone exposes on the response characteristic:
    ///
    ///     version(1) || identityId(32) || walletEphemeralPubKey(33) || encryptedPayload(60)
    ///     || keyId(4, big endian) || expiresAt(8, big endian, 0 = none) || totalBudget(8, big endian, 0 = none)
    struct BrowserLoginResponse: Equatable {
        static let length = 1 + 32 + 33 + 60 + 4 + 8 + 8

        let identityId: Data
        let walletEphemeralPublicKey: Data
        /// nonce(12) || ciphertext(32) || tag(16)
        let encryptedPayload: Data
        /// Id of the key the phone registered on the identity.
        let keyId: UInt32
        /// Block time in milliseconds from which the key can no longer sign.
        let expiresAt: UInt64?
        /// Lifetime spend cap of the key in credits.
        let totalBudget: UInt64?

        func serialized() -> Data {
            var out = Data(capacity: Self.length)
            out.append(BrowserLoginKeyProtocol.version)
            out.append(identityId)
            out.append(walletEphemeralPublicKey)
            out.append(encryptedPayload)
            out.append(bigEndian: keyId)
            out.append(bigEndian: expiresAt ?? 0)
            out.append(bigEndian: totalBudget ?? 0)
            return out
        }

        static func parse(_ bytes: Data) throws -> BrowserLoginResponse {
            guard bytes.count == length else { throw ProtocolError.truncatedRequest }
            var cursor = bytes.startIndex
            guard bytes[cursor] == BrowserLoginKeyProtocol.version else {
                throw ProtocolError.unsupportedVersion(bytes[cursor])
            }
            cursor += 1
            let identityId = Data(bytes[cursor..<cursor + 32])
            cursor += 32
            let walletEphemeralPublicKey = Data(bytes[cursor..<cursor + 33])
            cursor += 33
            let encryptedPayload = Data(bytes[cursor..<cursor + 60])
            cursor += 60
            let keyId = Data(bytes[cursor..<cursor + 4]).reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
            cursor += 4
            let expiresAt = Data(bytes[cursor..<cursor + 8]).reduce(UInt64(0)) { ($0 << 8) | UInt64($1) }
            cursor += 8
            let totalBudget = Data(bytes[cursor..<cursor + 8]).reduce(UInt64(0)) { ($0 << 8) | UInt64($1) }
            return BrowserLoginResponse(
                identityId: identityId,
                walletEphemeralPublicKey: walletEphemeralPublicKey,
                encryptedPayload: encryptedPayload,
                keyId: keyId,
                expiresAt: expiresAt == 0 ? nil : expiresAt,
                totalBudget: totalBudget == 0 ? nil : totalBudget
            )
        }
    }

    // MARK: - Key material

    /// A fresh 32-byte login key, the secret the browser receives.
    static func generateLoginKey() throws -> Data {
        try randomBytes(32)
    }

    /// A fresh secp256k1 keypair for one ECDH exchange.
    static func generateEphemeralKeyPair() throws -> (privateKey: Data, publicKey: Data) {
        // A random 32-byte string is outside the curve order with
        // probability ~2^-128, so one retry loop is plenty.
        for _ in 0..<8 {
            let privateKey = try randomBytes(32)
            if let publicKey = try? Secp256k1Primitives.compressedPublicKey(privateKey: privateKey) {
                return (privateKey, publicKey)
            }
        }
        throw ProtocolError.ephemeralKeyGenerationFailed
    }

    /// The private key of the AUTHENTICATION identity key the browser
    /// will derive from `loginKey`: `HKDF-SHA256(ikm: loginKey, salt: identityId, info: "auth")`.
    static func deriveAuthPrivateKey(loginKey: Data, identityId: Data) throws -> Data {
        guard identityId.count == 32 else { throw ProtocolError.invalidIdentityId }
        let key = HKDF<SHA256>.deriveKey(
            inputKeyMaterial: SymmetricKey(data: loginKey),
            salt: identityId,
            info: Data("auth".utf8),
            outputByteCount: 32
        )
        return key.withUnsafeBytes { Data($0) }
    }

    /// The AES-256-GCM key both sides derive from the ECDH x-coordinate.
    static func deriveSharedKey(sharedX: Data) -> SymmetricKey {
        HKDF<SHA256>.deriveKey(
            inputKeyMaterial: SymmetricKey(data: sharedX),
            salt: sharedSecretSalt,
            info: Data(),
            outputByteCount: 32
        )
    }

    /// Encrypt `loginKey` for the browser: nonce(12) || ciphertext(32) || tag(16).
    static func seal(
        loginKey: Data,
        walletEphemeralPrivateKey: Data,
        appEphemeralPublicKey: Data
    ) throws -> Data {
        let sharedX = try Secp256k1Primitives.ecdhSharedX(
            privateKey: walletEphemeralPrivateKey,
            publicKey: appEphemeralPublicKey
        )
        let key = deriveSharedKey(sharedX: sharedX)
        let box = try AES.GCM.seal(loginKey, using: key)
        guard let combined = box.combined else {
            // Only a non-standard nonce size yields nil, and AES.GCM.seal
            // picks the standard 12 bytes when no nonce is passed.
            throw ProtocolError.ephemeralKeyGenerationFailed
        }
        return combined
    }

    /// The browser's side of `seal`, here so tests can round-trip and so
    /// a wallet-to-wallet transfer could reuse it.
    static func open(
        encryptedPayload: Data,
        appEphemeralPrivateKey: Data,
        walletEphemeralPublicKey: Data
    ) throws -> Data {
        let sharedX = try Secp256k1Primitives.ecdhSharedX(
            privateKey: appEphemeralPrivateKey,
            publicKey: walletEphemeralPublicKey
        )
        let key = deriveSharedKey(sharedX: sharedX)
        let box = try AES.GCM.SealedBox(combined: encryptedPayload)
        return try AES.GCM.open(box, using: key)
    }

    static func randomBytes(_ count: Int) throws -> Data {
        var bytes = [UInt8](repeating: 0, count: count)
        let status = bytes.withUnsafeMutableBufferPointer { buffer -> Int32 in
            guard let base = buffer.baseAddress else { return errSecParam }
            return SecRandomCopyBytes(kSecRandomDefault, buffer.count, base)
        }
        guard status == errSecSuccess else { throw ProtocolError.randomnessUnavailable }
        return Data(bytes)
    }
}

private extension Data {
    mutating func append(bigEndian value: UInt32) {
        var be = value.bigEndian
        Swift.withUnsafeBytes(of: &be) { append(contentsOf: $0) }
    }

    mutating func append(bigEndian value: UInt64) {
        var be = value.bigEndian
        Swift.withUnsafeBytes(of: &be) { append(contentsOf: $0) }
    }
}
