import Foundation
import DashSDKFFI

// MARK: - Fixed-size byte-tuple aliases
//
// Swift imports `uint8_t x[N]` from C as a flat tuple of `N` `UInt8`
// values. The two sizes Platform actually traffics in are 32 bytes
// (identity ids, wallet ids, etc.) and 20 bytes
// (RIPEMD160(SHA256) public-key hashes). Naming them once at module
// scope keeps `withUnsafeBytes(of:)` / `assumingMemoryBound(to:)`
// callsites and ABI-typed function pointers from spelling out a 32-
// or 20-tuple inline every time.

typealias FFIByteTuple32 = (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
)

typealias FFIByteTuple20 = (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8
)

// MARK: - Mnemonic-resolver callback result codes

/// Mirrors the Rust `mnemonic_resolver_result` constants in
/// `derive_and_persist_callbacks.rs`. The C header surfaces them as
/// bare `#define`s with awkwardly generic names (`SUCCESS`,
/// `NOT_FOUND`, ...); we re-expose them as a typed enum so callers
/// don't reach for those globals by accident and the resolver
/// callback's return value reads as a domain-specific code instead
/// of a magic `Int32`.
enum MnemonicResolverResult: Int32 {
    case success = 0
    case notFound = 1
    case bufferTooSmall = 2
    case other = 3
}

// MARK: - Resolver-driven sign error codes

/// Mirrors the Rust `SIGN_WITH_RESOLVER_*` byte tags. Returned via
/// the `out_error` byte parameter on a non-zero rc.
enum SignWithMnemonicResolverError: UInt8 {
    case ok = 0
    case nullPointer = 1
    case invalidUtf8 = 2
    case invalidMnemonic = 3
    case invalidPath = 4
    case derivationFailed = 5
    case signFailed = 6
    case bufferTooSmall = 7
    case unsupportedKeyType = 8
    case resolverNotFound = 9
    case resolverFailed = 10
}

// MARK: - 32-byte tuple helpers

/// Convert a 32-byte FFI tuple into `Data` for SwiftData persistence.
@inline(__always)
func hashData(_ hash: FFIByteTuple32) -> Data {
    Swift.withUnsafeBytes(of: hash) { Data($0) }
}

/// Hex-encode a 32-byte FFI tuple.
@inline(__always)
func hashHex(_ hash: FFIByteTuple32) -> String {
    Swift.withUnsafeBytes(of: hash) { buf in
        buf.map { String(format: "%02x", $0) }.joined()
    }
}
