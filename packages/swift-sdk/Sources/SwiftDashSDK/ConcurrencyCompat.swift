import Foundation

// Swift 6 sendability adjustments for FFI pointers and wrappers.
// These are safe under our usage patterns where FFI pointers are thread-confined
// or explicitly synchronized at the Rust boundary.

extension OpaquePointer: @unchecked Sendable {}

