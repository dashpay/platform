import Foundation
import DashSDKFFI

/// Outpoint of an asset lock a fresh-build funding call has just built.
public struct AssetLockBuiltOutPoint: Hashable, Sendable {
    /// 32-byte txid in wire (little-endian) order — the order the resume
    /// calls take.
    public let txid: Data
    /// Output index of the credit output.
    public let vout: UInt32
    /// The same outpoint as `PersistentAssetLock.outPointHex`
    /// (`<txid display hex>:<vout>`), the key of the lock's persisted row.
    public let outPointHex: String

    /// Decodes a 36-byte raw outpoint (`txid_le || vout_le`), the form the
    /// FFI reports, so a consumer can build one to test its handler. `nil`
    /// for any other length.
    public init?(rawBytes: Data) {
        guard rawBytes.count == 36 else { return nil }
        self.init(reportedBytes: rawBytes)
    }

    /// Decodes the 36 bytes the FFI reports.
    init(reportedBytes rawBytes: Data) {
        precondition(rawBytes.count == 36, "outpoint must be 36 bytes")
        txid = Data(rawBytes.prefix(32))
        // Byte-copy into an aligned local: `Data` does not guarantee
        // `UInt32` alignment (see `PersistentAssetLock.encodeOutPoint`).
        var value: UInt32 = 0
        withUnsafeMutableBytes(of: &value) { dst in
            dst.copyBytes(from: rawBytes.suffix(4))
        }
        vout = UInt32(littleEndian: value)
        outPointHex = PersistentAssetLock.encodeOutPoint(rawBytes: rawBytes)
    }
}

/// Handler a fresh-build funding call invokes with the new asset lock's
/// outpoint.
///
/// It runs at most once per call, on the FFI worker thread, after the lock
/// transaction is signed and before the lock's row is persisted or the lock
/// is broadcast — so no persisted row for the lock exists before the handler
/// has returned. It does not run when the build itself fails. It does run
/// for a lock whose broadcast then fails: a thrown
/// `PlatformWalletError.transactionBroadcastRejected` means the lock was
/// removed again; any other error leaves it tracked and resumable by its
/// outpoint (a resume may find it already consumed). Both hold for this
/// session: the row store and removal behind them are best-effort, so after
/// a restart a failed store can leave no row for a broadcast lock, or the
/// row of a removed one.
///
/// Record the outpoint in thread-safe state before returning — a hop to
/// another actor would let the row arrive first — and durably if it has to
/// survive a restart. Keep it short and do not call any wallet or SDK
/// function from it: it runs on the SDK's runtime thread, where a blocking
/// FFI call aborts the process.
public typealias AssetLockBuiltHandler = @Sendable (AssetLockBuiltOutPoint) -> Void

/// Box that carries an `AssetLockBuiltHandler` across the C ABI as an
/// opaque context pointer.
///
/// `Sendable`: the only stored value is a `let` holding an `@Sendable` closure.
private final class AssetLockBuiltBox: Sendable {
    let handler: AssetLockBuiltHandler
    init(_ handler: @escaping AssetLockBuiltHandler) {
        self.handler = handler
    }
}

/// C trampoline matching the FFI's `AssetLockBuiltFn`. Re-materializes the
/// box from the opaque context and forwards the decoded outpoint.
private func assetLockBuiltTrampoline(
    context: UnsafeMutableRawPointer?,
    outPoint36: UnsafePointer<UInt8>?
) {
    guard let context, let outPoint36 else { return }
    let box = Unmanaged<AssetLockBuiltBox>.fromOpaque(context).takeUnretainedValue()
    box.handler(AssetLockBuiltOutPoint(reportedBytes: Data(bytes: outPoint36, count: 36)))
}

/// Runs `body` with the C callback and context for `handler` (both `nil`
/// when `handler` is `nil`). The box stays alive until `body` returns, which
/// covers the synchronous FFI call that may invoke it.
func withAssetLockBuiltCallback<R>(
    _ handler: AssetLockBuiltHandler?,
    _ body: (AssetLockBuiltFn?, UnsafeMutableRawPointer?) throws -> R
) rethrows -> R {
    guard let handler else { return try body(nil, nil) }
    let box = AssetLockBuiltBox(handler)
    return try withExtendedLifetime(box) {
        try body(assetLockBuiltTrampoline, Unmanaged.passUnretained(box).toOpaque())
    }
}
