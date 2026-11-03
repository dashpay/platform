import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

// MARK: - Built-lock callback bridge
//
// `withAssetLockBuiltCallback` pairs the C callback with its context for the
// funding FFI calls, and the trampoline decodes the reported 36-byte outpoint
// into `AssetLockBuiltOutPoint`. The decoded txid and vout are the resume
// arguments and `outPointHex` is the persisted row's key, so byte order
// matters on both.

/// Records what a handler received, from whichever thread calls it.
private final class Received: @unchecked Sendable {
    private let lock = NSLock()
    private var values: [AssetLockBuiltOutPoint] = []

    func append(_ value: AssetLockBuiltOutPoint) {
        lock.withLock { values.append(value) }
    }

    var all: [AssetLockBuiltOutPoint] {
        lock.withLock { values }
    }
}

/// Object whose lifetime shows whether the handler (and so the box holding
/// it) was released.
private final class Sentinel: Sendable {
    func touch() {}
}

private struct BodyFailure: Error {}

final class AssetLockBuiltCallbackTests: XCTestCase {
    /// 32 distinct txid bytes in wire order followed by vout 0x01020304,
    /// little-endian.
    private static let txidWire = Data((0..<32).map { UInt8($0 * 7 + 1) })
    private static let vout: UInt32 = 0x0102_0304
    private static var rawOutPoint: Data {
        txidWire + Data([0x04, 0x03, 0x02, 0x01])
    }

    /// A consumer can build the value its handler receives, to test the
    /// handler without a funding call.
    func testShouldBuildTheSameOutPointAConsumerIsHanded() throws {
        let received = Received()
        let buffer = [UInt8](Self.rawOutPoint)
        withAssetLockBuiltCallback({ received.append($0) }) { callback, context in
            buffer.withUnsafeBufferPointer { callback?(context, $0.baseAddress) }
        }

        let built = try XCTUnwrap(AssetLockBuiltOutPoint(rawBytes: Self.rawOutPoint))
        XCTAssertEqual(received.all, [built])
        XCTAssertEqual(built.txid, Self.txidWire)
        XCTAssertEqual(built.vout, Self.vout)
    }

    func testShouldRefuseARawOutPointOfAnyOtherLength() {
        XCTAssertNil(AssetLockBuiltOutPoint(rawBytes: Self.txidWire))
        XCTAssertNil(AssetLockBuiltOutPoint(rawBytes: Self.rawOutPoint + Data([0])))
    }

    func testShouldPassNoCallbackAndNoContextForANilHandler() {
        let (callback, context) = withAssetLockBuiltCallback(nil) { ($0, $1) }
        XCTAssertNil(callback)
        XCTAssertNil(context)
    }

    func testShouldDecodeTheReportedOutPointInWireOrder() throws {
        let received = Received()
        var buffer = [UInt8](Self.rawOutPoint)

        withAssetLockBuiltCallback({ received.append($0) }) { callback, context in
            XCTAssertNotNil(context)
            guard let callback else { return XCTFail("callback missing") }
            buffer.withUnsafeBufferPointer { callback(context, $0.baseAddress) }
        }
        // The decoded outpoint must own its bytes: overwrite the borrowed
        // buffer in place, then read the outpoint.
        buffer.withUnsafeMutableBufferPointer { $0.update(repeating: 0xFF) }

        let outPoint = try XCTUnwrap(received.all.first)
        XCTAssertEqual(received.all.count, 1)
        XCTAssertEqual(outPoint.txid, Self.txidWire)
        XCTAssertEqual(outPoint.vout, Self.vout)
        let displayHex = Self.txidWire.reversed().map { String(format: "%02x", $0) }.joined()
        XCTAssertEqual(outPoint.outPointHex, "\(displayHex):\(Self.vout)")
    }

    func testShouldIgnoreACallWithoutContextOrOutPoint() {
        let received = Received()
        let buffer = [UInt8](Self.rawOutPoint)

        withAssetLockBuiltCallback({ received.append($0) }) { callback, context in
            guard let callback else { return XCTFail("callback missing") }
            callback(context, nil)
            buffer.withUnsafeBufferPointer { callback(nil, $0.baseAddress) }
        }

        XCTAssertTrue(received.all.isEmpty)
    }

    /// The context stays valid for the whole body: a callback made at the end
    /// of it still reaches the handler. A box released early turns this into
    /// a use-after-free, which `swift test --sanitize=address` reports.
    func testShouldDeliverThroughTheContextAtTheEndOfTheBody() {
        let received = Received()
        let buffer = [UInt8](Self.rawOutPoint)

        withAssetLockBuiltCallback({ received.append($0) }) { callback, context in
            guard let callback else { return XCTFail("callback missing") }
            for _ in 0..<1_000 { _ = Data(count: 64) }
            buffer.withUnsafeBufferPointer { callback(context, $0.baseAddress) }
        }

        XCTAssertEqual(received.all.count, 1)
    }

    func testShouldReleaseTheHandlerWhenTheBodyReturns() {
        weak var released: Sentinel?
        do {
            let sentinel = Sentinel()
            released = sentinel
            withAssetLockBuiltCallback({ _ in sentinel.touch() }) { callback, _ in
                XCTAssertNotNil(callback)
            }
        }
        XCTAssertNil(released)
    }

    func testShouldReleaseTheHandlerWhenTheBodyThrows() {
        weak var released: Sentinel?
        do {
            let sentinel = Sentinel()
            released = sentinel
            XCTAssertThrowsError(
                try withAssetLockBuiltCallback({ _ in sentinel.touch() }) { _, _ in
                    throw BodyFailure()
                }
            ) { error in
                XCTAssertTrue(error is BodyFailure, "the body's own error must be rethrown")
            }
        }
        XCTAssertNil(released)
    }
}
