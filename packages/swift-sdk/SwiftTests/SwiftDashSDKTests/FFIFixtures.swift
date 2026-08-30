import Foundation
@testable import SwiftDashSDK

// Shared fixtures for suites that hand-build the C structs the
// persistence handler consumes. Both conversions below were previously
// re-declared privately in every such suite; they are pure value
// transforms with no test-local state, so one copy serves all of them.

/// Copy a 32-byte `Data` into the C fixed-array tuple shape the FFI
/// structs use for txids, wallet ids, and identity ids.
func tuple32(_ data: Data) -> FFIByteTuple32 {
    precondition(data.count == 32)
    var tuple: FFIByteTuple32 = (
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    )
    withUnsafeMutableBytes(of: &tuple) { $0.copyBytes(from: data) }
    return tuple
}

/// Deterministic 32-byte txid for index `i`: the little-endian `UInt64`
/// in the leading bytes keeps ids readable in failure output and lets a
/// test recover `i` back out of a stored key (see the outpoint decode in
/// `WalletChangesetRoundTests`).
func makeTxid(_ i: Int) -> Data {
    var txid = Data(count: 32)
    withUnsafeBytes(of: UInt64(i).littleEndian) { txid.replaceSubrange(0..<8, with: $0) }
    return txid
}
