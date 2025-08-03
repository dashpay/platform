import XCTest
import SwiftDashSDK
import DashSDKFFI
@testable import SwiftExampleApp

final class SimpleTransitionTests: XCTestCase {
    
    // Minimal setup - no instance variables
    
    func testIdentityCreditTransfer() async throws {
        print(">>> SimpleTransitionTests.testIdentityCreditTransfer starting")
        
        // Initialize SDK inline
        SDK.initialize()
        print("SDK initialized")
        
        // Create SDK instance
        let sdk = try SDK(network: DashSDKNetwork(rawValue: 1))
        print("SDK instance created")
        
        // Load env variables
        EnvLoader.loadEnvFile()
        print("Env file loaded")
        
        // Get test data
        let testIdentityId = try EnvLoader.getRequired("TEST_IDENTITY_ID")
        let key3Base58 = try EnvLoader.getRequired("TEST_KEY_3_PRIVATE")
        print("Test identity: \(testIdentityId)")
        
        // Decode private key
        guard let decoded = Data.fromBase58(key3Base58),
              decoded.count >= 37 else {
            throw TestError.invalidPrivateKey
        }
        let key3Private = Data(decoded[1..<33])
        print("Private key decoded: \(key3Private.count) bytes")
        
        // Test parameters
        let recipientId = "HccabTZZpMEDAqU4oQFk3PE47kS6jDDmCjoxR88gFttA"
        let amount: UInt64 = 10_000_000
        
        print("Attempting transfer...")
        print("From: \(testIdentityId)")
        print("To: \(recipientId)")
        print("Amount: \(amount) credits")
        
        // Execute transfer
        do {
            // First, fetch the identity JSON
            let identityJson = try await sdk.identityGet(identityId: testIdentityId)
            
            // Convert the dictionary to JSON string
            let jsonData = try JSONSerialization.data(withJSONObject: identityJson, options: [])
            let jsonString = String(data: jsonData, encoding: .utf8)!
            
            // Parse the JSON to an identity handle
            let parseResult = jsonString.withCString { cString in
                dash_sdk_identity_parse_json(cString)
            }
            
            guard parseResult.error == nil,
                  let identityHandle = parseResult.data else {
                if let error = parseResult.error {
                    let errorString = String(cString: error.pointee.message)
                    dash_sdk_error_free(error)
                    XCTFail("Failed to parse identity JSON: \(errorString)")
                    return
                }
                XCTFail("Failed to parse identity JSON")
                return
            }
            
            defer {
                dash_sdk_identity_destroy(OpaquePointer(identityHandle)!)
            }
            
            // Create signer from private key
            let signerResult = key3Private.withUnsafeBytes { keyBytes in
                dash_sdk_signer_create_from_private_key(
                    keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                    UInt(key3Private.count)
                )
            }
            
            guard signerResult.error == nil,
                  let signer = signerResult.data else {
                XCTFail("Failed to create signer")
                return
            }
            
            defer {
                dash_sdk_signer_destroy(OpaquePointer(signer)!)
            }
            
            let result = try await sdk.identityTransferCredits(
                fromIdentity: OpaquePointer(identityHandle)!,
                toIdentityId: recipientId,
                amount: amount,
                signer: OpaquePointer(signer)!
            )
            
            print("✅ Transfer successful!")
            print("Sender new balance: \(result.senderBalance)")
            print("Receiver new balance: \(result.receiverBalance)")
            
            XCTAssertTrue(result.senderBalance >= 0)
            XCTAssertTrue(result.receiverBalance > 0)
        } catch {
            print("❌ Transfer failed: \(error)")
            throw error
        }
        
        print(">>> SimpleTransitionTests.testIdentityCreditTransfer completed")
    }
}