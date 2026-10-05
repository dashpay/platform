import DashSDKFFI
import XCTest

@testable import SwiftDashSDK

/// These tests run against the FFI mock SDK, never a live network: unit
/// tests must not depend on the network (a halted testnet failed every CI
/// run while they did). The live testnet read is opt-in, in
/// SwiftDashSDKIntegrationTests/Platform/TestnetIdentityFetchTests.swift.
final class SDKMethodTests: XCTestCase {

  /// A directory of rs-sdk's recorded offline vectors
  /// (packages/rs-sdk/tests/vectors/<name>).
  private static func rsSdkVectors(_ name: String) -> String {
    URL(fileURLWithPath: #filePath)
      .deletingLastPathComponent()  // SwiftDashSDKTests
      .deletingLastPathComponent()  // SwiftTests
      .deletingLastPathComponent()  // swift-sdk
      .deletingLastPathComponent()  // packages
      .appendingPathComponent("rs-sdk/tests/vectors/\(name)", isDirectory: true)
      .path
  }

  func testSDKMethodsAvailability() {
    print("=== Testing SDK Methods Availability ===")

    // Test if SDK responds to selectors
    let sdk = SDK.self

    // Check for identityTransferCredits method
    _ = NSSelectorFromString("identityTransferCredits:toIdentityId:amount:signerPrivateKey:")

    // Try using Mirror to inspect SDK methods
    let mirror = Mirror(reflecting: sdk)
    print("SDK type: \(mirror.subjectType)")

    // List all children
    for child in mirror.children {
      if let label = child.label {
        print("  Property: \(label)")
      }
    }

    print("✅ SDK methods inspection complete")
  }

  @MainActor
  func testDirectMethodCall() async throws {
    // Mock SDK with no vectors: a request that got as far as DAPI would fail
    // with a missing-expectation error instead of reaching the network.
    SDK.initialize()
    let sdk = try SDK(mockVectorsDirectory: nil)

    // Create a dummy identity
    let identity = DPPIdentity(
      id: Data(repeating: 0, count: 32),
      publicKeys: [:],
      balance: 0,
      revision: 0
    )

    // Any non-zero scalar below the curve order is a valid key. Zero is not:
    // signer creation would fail and skip the call under test.
    let key = Data(repeating: 1, count: 32)
    let signerResult = key.withUnsafeBytes { keyBytes in
      dash_sdk_signer_create_from_private_key(
        keyBytes.bindMemory(to: UInt8.self).baseAddress!,
        UInt(key.count),
        Network.testnet.ffiValue
      )
    }
    if let error = signerResult.error {
      let sdkError = SDKError.fromDashSDKError(error.pointee)
      dash_sdk_error_free(error)
      XCTFail("Failed to create signer: \(sdkError)")
      return
    }
    let signer = try XCTUnwrap(signerResult.data)
    defer {
      dash_sdk_signer_destroy(OpaquePointer(signer))
    }

    // "test2" is not a 32-byte Base58 identifier, so the FFI must refuse it
    // before building a transition, let alone broadcasting one.
    do {
      _ = try await sdk.transferCredits(
        from: identity,
        toIdentityId: "test2",
        amount: 1,
        signer: OpaquePointer(signer)
      )
      XCTFail("transferCredits should reject the malformed recipient id")
    } catch SDKError.internalError(let message) {
      XCTAssertTrue(
        message.contains("Invalid to_identity_id"),
        "expected the recipient id to be refused, got: \(message)"
      )
    }
  }

  /// Fetches an identity's public keys through `identityGetKeys` from
  /// rs-sdk's recorded `test_identity_public_keys_all_read` vectors: the mock
  /// replays the recorded DAPI response, the SDK verifies its proof against
  /// the recorded quorum key, and the keys are decoded into the Swift result.
  /// It does not read `test_identity_read`: those vectors carry GroveDB V0
  /// proof envelopes, which every client refuses since dashpay/platform#5294.
  @MainActor
  func testIdentityKeysFetch() async throws {
    let vectors = Self.rsSdkVectors("test_identity_public_keys_all_read")
    guard FileManager.default.fileExists(atPath: vectors) else {
      XCTFail("missing rs-sdk offline vectors at \(vectors)")
      return
    }

    SDK.initialize()
    let sdk = try SDK(mockVectorsDirectory: vectors)

    // The vectors record identity [1; 32], rs-sdk's IDENTITY_ID_1.
    let identityId = Data(repeating: 1, count: 32).toBase58()
    let keys = try await sdk.identityGetKeys(identityId: identityId)

    // The keys come back keyed by key id; every recorded key is present and
    // carries its own id.
    XCTAssertFalse(keys.isEmpty, "the recorded identity has public keys")
    for (keyId, value) in keys {
      let key = try XCTUnwrap(value as? [String: Any], "key \(keyId) is absent")
      let id = try XCTUnwrap(key["id"] as? Int, "key \(keyId) has no numeric id")
      XCTAssertEqual(String(id), keyId)
    }
  }
}
