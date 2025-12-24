import Foundation

// Swift 6 sendability adjustments for FFI pointers and wrappers.
// These are safe under our usage patterns where FFI pointers are thread-confined
// or explicitly synchronized at the Rust boundary.

extension OpaquePointer: @retroactive @unchecked Sendable {}

// FFI value types from DashSDKFFI headers used across actor boundaries
// These are plain C structs and treated as inert data blobs.
extension FFIDetailedSyncProgress: @retroactive @unchecked Sendable {}
