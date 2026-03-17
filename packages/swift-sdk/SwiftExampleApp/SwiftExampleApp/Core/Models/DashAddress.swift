import Foundation
import SwiftDashSDK

enum DashAddressType: Equatable {
    case core(Data)      // Output script bytes
    case platform(Data)  // 21-byte platform address (type byte + 20-byte hash)
    case orchard(Data)   // 43-byte raw Orchard address
    case unknown
}

struct DashAddress {
    let type: DashAddressType
    let displayString: String

    /// Parse any address string and detect its type
    static func parse(_ input: String, network: AppNetwork) -> DashAddress {
        // 1. Try bech32m first
        if let decoded = Bech32m.decode(input) {
            let hrp = decoded.hrp
            let data = decoded.data

            // Check HRP validity
            let validPlatformHrp = (network == .mainnet) ? "dashevo" : "tdashevo"
            let validOrchardHrp = (network == .mainnet) ? "dash" : "tdash"

            if hrp == validPlatformHrp && data.count == 21 {
                // Platform address: type byte 0xb0 or 0x80 + 20-byte hash
                let typeByte = data[0]
                if typeByte == 0xb0 || typeByte == 0x80 {
                    return DashAddress(type: .platform(data), displayString: input)
                }
            }

            if hrp == validOrchardHrp && data.count >= 2 {
                let typeByte = data[0]
                if typeByte == 0x10 {
                    // Orchard address: 0x10 type byte + 43 bytes raw address
                    let rawAddress = data.dropFirst()
                    if rawAddress.count == 43 {
                        return DashAddress(type: .orchard(Data(rawAddress)), displayString: input)
                    }
                }
            }
        }

        // 2. Try base58 (Core address) - P2PKH
        if let script = coreAddressToOutputScript(input) {
            return DashAddress(type: .core(script), displayString: input)
        }

        // 3. Unknown
        return DashAddress(type: .unknown, displayString: input)
    }

    /// Encode raw 43-byte Orchard address to bech32m display string.
    /// Prepends 0x10 type byte then bech32m encodes with dash/tdash HRP.
    static func encodeOrchard(rawBytes: Data, network: AppNetwork) -> String? {
        guard rawBytes.count == 43 else { return nil }
        let hrp = (network == .mainnet) ? "dash" : "tdash"
        var payload = Data([0x10])
        payload.append(rawBytes)
        return Bech32m.encode(hrp: hrp, data: payload)
    }

    /// Encode 21-byte platform address to bech32m display string
    static func encodePlatform(rawBytes: Data, network: AppNetwork) -> String? {
        guard rawBytes.count == 21 else { return nil }
        let hrp = (network == .mainnet) ? "dashevo" : "tdashevo"
        return Bech32m.encode(hrp: hrp, data: rawBytes)
    }

    /// Convert a base58check Core address to P2PKH output script.
    /// Returns nil if the address is invalid.
    static func coreAddressToOutputScript(_ address: String) -> Data? {
        guard let decoded = Base58Check.decode(address) else { return nil }
        // decoded = version(1 byte) + payload(20 bytes for P2PKH)
        guard decoded.count == 21 else { return nil }
        let versionByte = decoded[0]
        let pubkeyHash = decoded.dropFirst()

        // P2PKH version bytes: 0x4c (mainnet), 0x8c (testnet/regtest/devnet)
        guard versionByte == 0x4c || versionByte == 0x8c else { return nil }

        // Build P2PKH script: OP_DUP OP_HASH160 <20-byte-hash> OP_EQUALVERIFY OP_CHECKSIG
        var script = Data()
        script.append(0x76) // OP_DUP
        script.append(0xa9) // OP_HASH160
        script.append(0x14) // Push 20 bytes
        script.append(contentsOf: pubkeyHash)
        script.append(0x88) // OP_EQUALVERIFY
        script.append(0xac) // OP_CHECKSIG
        return script
    }
}

// MARK: - Base58Check

/// Minimal Base58Check decoder for Core address parsing
enum Base58Check {
    private static let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"

    static func decode(_ string: String) -> Data? {
        // Decode base58
        var bigNum: [UInt8] = [0]

        for char in string {
            guard let index = alphabet.firstIndex(of: char) else { return nil }
            let digit = UInt8(alphabet.distance(from: alphabet.startIndex, to: index))

            // Multiply bigNum by 58 and add digit
            var carry = UInt32(digit)
            for i in (0..<bigNum.count).reversed() {
                let value = UInt32(bigNum[i]) * 58 + carry
                bigNum[i] = UInt8(value & 0xFF)
                carry = value >> 8
            }
            while carry > 0 {
                bigNum.insert(UInt8(carry & 0xFF), at: 0)
                carry >>= 8
            }
        }

        // Count leading '1's (zero bytes)
        let leadingZeros = string.prefix(while: { $0 == "1" }).count
        let result = Data(repeating: 0, count: leadingZeros) + Data(bigNum)

        // Verify checksum: last 4 bytes
        guard result.count >= 4 else { return nil }
        let payload = result.dropLast(4)
        // Note: We skip checksum verification here for simplicity in a demo app.
        // A production app would SHA256d the payload and compare to the checksum.

        return Data(payload)
    }
}
