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

/// 36-byte fixed tuple — used by the asset-lock persister to carry
/// outpoints (32-byte raw txid + 4-byte little-endian vout). Matches
/// the Rust-side `AssetLockEntryFFI.out_point` and the parallel
/// removed-outpoint array on `on_persist_asset_locks_fn`.
typealias FFIByteTuple36 = (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8
)

/// 43-byte fixed tuple — the raw Orchard recipient address carried by
/// the shielded outgoing-note (OVK send-history) FFI structs
/// (`ShieldedOutgoingNoteFFI.recipient` /
/// `ShieldedOutgoingNoteRestoreFFI.recipient`, both `uint8_t[43]`).
typealias FFIByteTuple43 = (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8
)

/// Zeroed `FFIByteTuple43` literal — spares the 43-element
/// initializer at every `loadShieldedOutgoingNotes` call site.
let ffiByteTuple43Zero: FFIByteTuple43 = (
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
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
    /// The key derived at the path did not reproduce the caller-supplied
    /// expected key — the signature was withheld (mirrors the Rust
    /// `SIGN_WITH_RESOLVER_ERR_PUBKEY_MISMATCH`).
    case pubkeyMismatch = 11
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
