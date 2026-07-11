import Foundation

// MARK: - Data Extensions for Base58 and Hex

extension Data {
    /// Create an Identifier from a base58 string
    public static func identifier(fromBase58 base58String: String) -> Data? {
        let alphabet = Array("123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz")
        let base = alphabet.count

        var bytes = [UInt8]()
        var num = [UInt8](repeating: 0, count: 1)

        for char in base58String {
            guard let index = alphabet.firstIndex(of: char) else {
                return nil
            }

            var carry = 0
            for i in 0..<num.count {
                carry = Int(num[i]) * base + carry
                num[i] = UInt8(carry % 256)
                carry /= 256
            }
            while carry > 0 {
                num.append(UInt8(carry % 256))
                carry /= 256
            }

            carry = index
            for i in 0..<num.count {
                carry = Int(num[i]) + carry
                num[i] = UInt8(carry % 256)
                carry /= 256
            }
            while carry > 0 {
                num.append(UInt8(carry % 256))
                carry /= 256
            }
        }

        for char in base58String {
            if char == "1" {
                bytes.append(0)
            } else {
                break
            }
        }

        bytes.append(contentsOf: num.reversed())

        return Data(bytes)
    }

    /// Convert to base58 string
    public func toBase58String() -> String {
        let alphabet = Array("123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz")

        if self.isEmpty {
            return ""
        }

        var bytes = Array(self)

        let zeroCount = bytes.prefix(while: { $0 == 0 }).count
        bytes = Array(bytes.dropFirst(zeroCount))

        if bytes.isEmpty {
            return String(repeating: "1", count: zeroCount)
        }

        var encoded = ""

        while !bytes.isEmpty && !bytes.allSatisfy({ $0 == 0 }) {
            var remainder = 0
            var newBytes = [UInt8]()

            for byte in bytes {
                let temp = remainder * 256 + Int(byte)
                remainder = temp % 58
                let quotient = temp / 58
                if !newBytes.isEmpty || quotient > 0 {
                    newBytes.append(UInt8(quotient))
                }
            }

            bytes = newBytes
            encoded = String(alphabet[remainder]) + encoded
        }

        encoded = String(repeating: "1", count: zeroCount) + encoded

        return encoded
    }

    /// Convert to hex string
    public func toHexString() -> String {
        return self.map { String(format: "%02x", $0) }.joined()
    }

    /// Computed-property alias for [`toHexString()`].
    public var hexString: String {
        toHexString()
    }
}
