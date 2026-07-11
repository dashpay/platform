import Foundation
import DashSDKFFI

let NULL_HANDLE: Handle = 0

extension Identifier {
    /// Run `body` with a `*const u8` to this identifier's 32-byte
    /// payload.
    ///
    /// Preconditions: `count == 32`. Identifiers on Platform are
    /// always exactly 32 bytes; a precondition here surfaces the
    /// drift loudly rather than letting Rust read garbage on
    /// dereference.
    @inline(__always)
    func withFFIBytes<R>(_ body: (UnsafePointer<UInt8>) throws -> R) rethrows -> R {
        precondition(count == 32, "identifier must be 32 bytes, got \(count)")
        return try withUnsafeBytes { raw in
            try body(raw.bindMemory(to: UInt8.self).baseAddress!)
        }
    }
}

/// Identity lifecycle status as carried on the Rust-side
/// `ManagedIdentity.status`. Mirrors `IdentityStatusFFI` /
/// `platform_wallet::wallet::identity::state::managed_identity::IdentityStatus`.
public enum IdentityStatus: UInt8, Sendable {
    case unknown = 0
    case pendingCreation = 1
    case active = 2
    case failedCreation = 3
    case notFound = 4

    /// Short human-readable label for the explorer UI.
    public var displayName: String {
        switch self {
        case .unknown: return "Unknown"
        case .pendingCreation: return "Pending Creation"
        case .active: return "Active"
        case .failedCreation: return "Failed Creation"
        case .notFound: return "Not Found"
        }
    }
}

// MARK: - Identifier FFI Conversion Helpers

/// Read 32 bytes from `ptr` into an `Identifier` (a `Data` of length 32).
///
/// Replaces the old `identifierFromFFI(_: IdentifierBytes)` helper.
/// `ptr` must point at exactly 32 readable bytes.
@inline(__always)
func identifierFromFFI(_ ptr: UnsafePointer<UInt8>) -> Identifier {
    Data(bytes: ptr, count: 32)
}

/// Read a row from an `IdentifierArray` returned by Rust.
///
/// `array.items` points at a contiguous `[[u8; 32]; count]` buffer;
/// take a `Data` snapshot of the i-th 32-byte row. Swift imports
/// `uint8_t (*)[32]` as a pointer to a 32-tuple, so we rebind to a
/// flat `UInt8` pointer before indexing.
@inline(__always)
func identifierFromFFIArray(_ array: IdentifierArray, at index: Int) -> Identifier {
    precondition(index >= 0 && index < Int(array.count), "index out of range")
    let raw = UnsafeRawPointer(array.items!)
    let base = raw.assumingMemoryBound(to: UInt8.self)
    let row = base.advanced(by: index * 32)
    return Data(bytes: row, count: 32)
}

/// Generate a random identifier via the FFI.
public func generateRandomIdentifier() throws -> Identifier {
    var buf = [UInt8](repeating: 0, count: 32)
    try buf.withUnsafeMutableBufferPointer { bp in
        try platform_wallet_generate_random_identifier(bp.baseAddress!).check()
    }
    return Data(buf)
}

extension Data {
    public init?(hexString: String) {
        let len = hexString.count / 2
        var data = Data(capacity: len)
        var index = hexString.startIndex
        for _ in 0..<len {
            let nextIndex = hexString.index(index, offsetBy: 2)
            if let b = UInt8(hexString[index..<nextIndex], radix: 16) {
                data.append(b)
            } else {
                return nil
            }
            index = nextIndex
        }
        self = data
    }
}
