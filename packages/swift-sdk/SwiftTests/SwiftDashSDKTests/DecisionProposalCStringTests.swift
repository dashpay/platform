import Foundation
import XCTest
@testable import SwiftDashSDK

/// Hermetic tests for `withDecisionProposalCString`, the Swift-to-C
/// conversion every decision proposal string (title, description, url,
/// base58 ids) goes through. The Rust side reads each one with
/// `CStr::from_ptr`, which stops at the first NUL, so a string holding U+0000
/// must be refused rather than handed over cut short. No FFI call is made.
final class DecisionProposalCStringTests: XCTestCase {
    private struct BodyError: Error, Equatable {}

    /// The bytes `pointer` holds up to its NUL terminator.
    private func cStringBytes(_ pointer: UnsafePointer<CChar>?) -> [UInt8]? {
        guard let pointer else { return nil }
        return Array(UnsafeBufferPointer(start: pointer, count: strlen(pointer)))
            .map { UInt8(bitPattern: $0) }
    }

    func testShouldHandOverAStringWithoutANULWhole() throws {
        let title = "Raise the block size? Ja, 是, oui ✓"
        let bytes = try withDecisionProposalCString(
            title, field: "title", invalid: SDKError.invalidParameter
        ) { pointer in
            cStringBytes(pointer)
        }
        XCTAssertEqual(bytes, Array(title.utf8))
    }

    func testShouldHandOverAnEmptyString() throws {
        // Empty text is representable; whether a proposal may have it is
        // Platform's call.
        let bytes = try withDecisionProposalCString(
            "", field: "description", invalid: SDKError.invalidParameter
        ) { pointer in
            cStringBytes(pointer)
        }
        XCTAssertEqual(bytes, [])
    }

    func testShouldHandOverNilAsANullPointer() throws {
        let isNull = try withDecisionProposalCString(
            nil, field: "url", invalid: SDKError.invalidParameter
        ) { pointer in
            pointer == nil
        }
        XCTAssertTrue(isNull)
    }

    func testShouldRefuseAnEmbeddedNULBeforeMakingAPointer() {
        for value in ["Raise\0 the block size?", "\0Raise", "Raise\0", "\0"] {
            var bodyRan = false
            XCTAssertThrowsError(
                try withDecisionProposalCString(
                    value, field: "title", invalid: SDKError.invalidParameter
                ) { _ in
                    bodyRan = true
                },
                "\(value.debugDescription) must be refused"
            ) { error in
                guard case SDKError.invalidParameter(let message) = error else {
                    return XCTFail("expected SDKError.invalidParameter, got \(error)")
                }
                XCTAssertTrue(message.hasPrefix("title "), message)
                XCTAssertTrue(message.contains("NUL"), message)
            }
            XCTAssertFalse(bodyRan, "\(value.debugDescription) must not reach C")
        }
    }

    func testShouldRefuseWithTheCallingAPIsInvalidParameterError() {
        // The platform wallet's create and edit refuse with their own error
        // type, naming the field.
        XCTAssertThrowsError(
            try withDecisionProposalCString(
                "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF\0junk",
                field: "startAfter",
                invalid: PlatformWalletError.invalidParameter
            ) { _ in }
        ) { error in
            guard case PlatformWalletError.invalidParameter(let message) = error else {
                return XCTFail("expected PlatformWalletError.invalidParameter, got \(error)")
            }
            XCTAssertTrue(message.hasPrefix("startAfter "), message)
        }
    }

    func testShouldPassTheBodysErrorThroughUnchanged() {
        XCTAssertThrowsError(
            try withDecisionProposalCString(
                "Raise the block size?", field: "title", invalid: SDKError.invalidParameter
            ) { _ in
                throw BodyError()
            }
        ) { error in
            XCTAssertEqual(error as? BodyError, BodyError())
        }
    }
}
