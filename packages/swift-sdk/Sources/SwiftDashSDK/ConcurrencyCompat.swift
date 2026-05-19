import Foundation

// Swift 6 sendability adjustments for FFI pointers and wrappers.
// These are safe under our usage patterns where FFI pointers are thread-confined
// or explicitly synchronized at the Rust boundary.
//
// Swift 6.2+'s standard library ships a conformance
// `@available(*, unavailable) extension OpaquePointer : Sendable {}`,
// which:
//   (a) causes app code that previously relied on `OpaquePointer` being
//       Sendable to fail with "conformance is unavailable" if we drop
//       the retroactive conformance here, and
//   (b) trips a "conformance was already stated in the type's module
//       'Swift'" warning — promoted to error by `-warnings-as-errors` —
//       if we declare a plain retroactive `@unchecked Sendable`.
//
// Gate on the compiler version: Swift 6.2+ emits the stdlib conformance
// itself (even as unavailable), so we provide `SendableOpaquePointer`
// below as a Sendable wrapper at call sites there; older toolchains
// still benefit from the retroactive shim so existing call sites
// compile unchanged.
#if compiler(<6.2)
extension OpaquePointer: @retroactive @unchecked Sendable {}
#endif

/// Sendable wrapper around a raw `OpaquePointer`.
///
/// Swift 6.2's stdlib marks `OpaquePointer: Sendable` as
/// `@available(*, unavailable)`, so crossing a `Task` / `MainActor`
/// boundary with a bare `OpaquePointer` fails strict-concurrency
/// checking. Wrap the pointer in this struct inside the producer
/// closure, extract `.pointer` on the consumer side. Safety is the
/// caller's responsibility — the wrapped pointer is not retained or
/// ref-counted.
public struct SendableOpaquePointer: @unchecked Sendable {
    public let pointer: OpaquePointer

    public init(_ pointer: OpaquePointer) {
        self.pointer = pointer
    }
}
