import Foundation
import DashSDKFFI

/// Handler a fresh-build funding call invokes with the new asset lock's
/// outpoint, in `PersistentAssetLock.outPointHex` form
/// (`<txid display hex>:<vout>`).
///
/// It runs at most once per call, on the FFI worker thread, after the lock
/// transaction is signed and before the lock's row is persisted or the lock
/// is broadcast — so no persisted row for the lock exists before the handler
/// has returned. It does not run when the build itself fails. It does run
/// for a lock whose broadcast then fails: after a definite rejection the
/// lock is removed again, otherwise it stays tracked and can be resumed by
/// its outpoint. Record the outpoint in thread-safe state before returning —
/// a hop to another actor would let the row arrive first. Keep it short and
/// do not call back into the wallet from it.
public typealias AssetLockBuiltHandler = @Sendable (String) -> Void

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
    box.handler(PersistentAssetLock.encodeOutPoint(rawBytes: Data(bytes: outPoint36, count: 36)))
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
