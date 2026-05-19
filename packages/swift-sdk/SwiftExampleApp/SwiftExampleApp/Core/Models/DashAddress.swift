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
    static func parse(_ input: String, network: Network) -> DashAddress {
        // 1. Try bech32m first
        if let decoded = Bech32m.decode(input) {
            let hrp = decoded.hrp
            let data = decoded.data

            // Check HRP validity
            // Platform and Orchard share the same HRP: "dash" (mainnet) / "tdash" (testnet/regtest)
            // Distinguished by type byte: 0xb0/0x80 = platform, 0x10 = orchard
            let validHrp = (network == .mainnet) ? "dash" : "tdash"

            if hrp == validHrp && data.count == 21 {
                // Platform address bech32m wire bytes per
                // rs-dpp/src/address_funds/platform_address.rs:41-47:
                //   0xb0 = P2PKH, 0x80 = P2SH.
                // 0x00/0x01 are the *storage* bytes (GroveDB keys) and must
                // never appear in a `tdash1…`/`dash1…` string.
                let typeByte = data[0]
                if typeByte == 0xb0 || typeByte == 0x80 {
                    return DashAddress(type: .platform(data), displayString: input)
                }
            }

            if hrp == validHrp && data.count >= 2 {
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

        // 2. Try Core address — validate via Rust FFI (Base58Check + network)
        let keyWalletNetwork: Network = (network == .mainnet) ? .mainnet : .testnet
        if Address.validate(input, network: keyWalletNetwork),
           let script = coreAddressToOutputScript(input) {
            return DashAddress(type: .core(script), displayString: input)
        }

        // 3. Unknown
        return DashAddress(type: .unknown, displayString: input)
    }

    /// Encode raw 43-byte Orchard address to bech32m display string.
    /// Prepends 0x10 type byte then bech32m encodes with dash/tdash HRP.
    static func encodeOrchard(rawBytes: Data, network: Network) -> String? {
        guard rawBytes.count == 43 else { return nil }
        let hrp = (network == .mainnet) ? "dash" : "tdash"
        var payload = Data([0x10])
        payload.append(rawBytes)
        return Bech32m.encode(hrp: hrp, data: payload)
    }

    /// Encode 21-byte platform address to bech32m display string.
    ///
    /// HRP matches what `parse(...)` accepts (`dash` / `tdash`) so a
    /// round-trip through `encodePlatform` → `parse` resolves as
    /// `.platform(...)`. The type byte at `rawBytes[0]` (0xb0 / 0x80)
    /// is what distinguishes platform from Orchard, not the HRP.
    static func encodePlatform(rawBytes: Data, network: Network) -> String? {
        guard rawBytes.count == 21 else { return nil }
        let hrp = (network == .mainnet) ? "dash" : "tdash"
        return Bech32m.encode(hrp: hrp, data: rawBytes)
    }

    /// Convert a base58check Core address to P2PKH output script.
    ///
    /// Caller must validate the address first via `Address.validate()` (Rust FFI)
    /// which handles Base58Check decoding, checksum verification, and network matching.
    /// This method only extracts the pubkey hash and builds the script.
    static func coreAddressToOutputScript(_ address: String) -> Data? {
        guard let decoded = Base58Decode.decode(address) else { return nil }
        // decoded = version(1 byte) + payload(20 bytes for P2PKH) + checksum(4 bytes)
        guard decoded.count == 25 else { return nil }
        let pubkeyHash = decoded[1..<21]

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

// MARK: - Base58 Decode (raw)

/// Minimal Base58 decoder — only decodes the raw bytes.
/// Checksum and network validation is done by Rust via Address.validate().
private enum Base58Decode {
    private static let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"

    static func decode(_ string: String) -> Data? {
        var bigNum: [UInt8] = [0]

        for char in string {
            guard let index = alphabet.firstIndex(of: char) else { return nil }
            let digit = UInt8(alphabet.distance(from: alphabet.startIndex, to: index))

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

        let leadingZeros = string.prefix(while: { $0 == "1" }).count
        return Data(repeating: 0, count: leadingZeros) + Data(bigNum)
    }
}
