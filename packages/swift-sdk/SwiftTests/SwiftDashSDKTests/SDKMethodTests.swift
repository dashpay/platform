import DashSDKFFI
import SwiftDashSDK
import XCTest

final class SDKMethodTests: XCTestCase {

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

  func testDirectMethodCall() async throws {
    print("=== Testing Direct Method Call ===")

    // Initialize SDK
    SDK.initialize()
    let sdk = try SDK(network: .testnet)

    print("SDK created: \(sdk)")
    print("SDK handle: \(String(describing: sdk.handle))")
    print("SDK type: \(type(of: sdk))")

    // Test if we can call identityTransferCredits without crashing
    do {
      print("Attempting to call identityTransferCredits...")
      let toId = "test2"
      let amount: UInt64 = 1
      let key = Data(repeating: 0, count: 32)

      // Create a dummy identity
      let identity = DPPIdentity(
        id: Data(repeating: 0, count: 32),
        publicKeys: [:],
        balance: 0,
        revision: 0
      )

      // Create signer from private key
      let signerResult = key.withUnsafeBytes { keyBytes in
        dash_sdk_signer_create_from_private_key(
          keyBytes.bindMemory(to: UInt8.self).baseAddress!,
          UInt(key.count),
          Network.testnet.ffiValue
        )
      }

      guard signerResult.error == nil,
        let signer = signerResult.data
      else {
        print("Failed to create signer")
        return
      }

      defer {
        dash_sdk_signer_destroy(OpaquePointer(signer))
      }

      let signerPtr = OpaquePointer(signer)
      _ = try await sdk.transferCredits(
        from: identity,
        toIdentityId: toId,
        amount: amount,
        signer: signerPtr
      )
      XCTFail("transferCredits should fail with dummy data")
    } catch {
      // Expected: dummy data causes an error
    }
  }

  func testSimpleIdentityFetch() async throws {
    print("=== Testing Simple Identity Fetch ===")

    SDK.initialize()
    let sdk = try SDK(network: .testnet)

    do {
      // Use a known testnet identity
      let testIdentityId = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk"
      print("Fetching identity: \(testIdentityId)")

      try await Task { @MainActor in
        let identity = try await sdk.identityGet(identityId: testIdentityId)
        print("✅ Identity fetched successfully")
        print("Identity keys: \(identity.keys.sorted())")
      }.value
    } catch {
      print("❌ Failed to fetch identity: \(error)")
      throw error
    }
  }
}
