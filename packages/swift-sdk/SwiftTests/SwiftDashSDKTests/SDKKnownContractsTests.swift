import XCTest

@testable import SwiftDashSDK

/// `SDK.loadKnownContracts` hands Rust one pointer per serialized contract,
/// and Rust deserializes through them during the call. Every pointer must
/// still address its contract's bytes at that point, not a copy that died
/// when the closure that produced it returned.
final class SDKKnownContractsTests: XCTestCase {

    /// `Data` of up to 14 bytes is stored inline, so the pointer
    /// `withUnsafeBytes` hands out is a temporary stack copy. Returning it out
    /// of the closure leaves the buffer reading whatever later reused that
    /// stack slot. The last buffer is stored on the heap, like a real
    /// contract.
    func testShouldKeepEveryBufferReadableUntilTheBodyReturns() {
        let buffers = [
            Data([0x01, 0x02, 0x03]),
            Data([0x04, 0x05, 0x06, 0x07]),
            Data([0x08]),
            Data(repeating: 0x09, count: 256),
        ]

        let read = SDK.withUnsafeBytePointers(to: buffers) { pointers in
            zip(pointers, buffers).map { pointer, buffer in
                Data(bytes: pointer!, count: buffer.count)
            }
        }

        XCTAssertEqual(read, buffers)
    }

    /// Bytes Rust cannot deserialize as a contract must surface as an SDK
    /// error carrying Rust's reason, not load silently. Reaching Rust's
    /// deserializer also shows the ids, pointers and lengths arrived intact;
    /// the pointer lifetime itself is pinned by the test above, since no
    /// valid contract is small enough to be stored inline.
    func testShouldReportAContractRustCannotDeserialize() throws {
        let sdk = try SDK(network: .regtest, contextProvider: .spv)

        XCTAssertThrowsError(
            try sdk.loadKnownContracts([(id: "unreadable", data: Data(repeating: 0xFF, count: 64))])
        ) { error in
            guard case SDKError.internalError(let message) = error else {
                return XCTFail("expected internalError, got \(error)")
            }
            XCTAssertTrue(
                message.hasPrefix("Failed to add known contracts: Failed to deserialize contract unreadable"),
                message
            )
        }
    }
}
