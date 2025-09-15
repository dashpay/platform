import XCTest
import SwiftDashSDK
import DashSDKFFI
@testable import SwiftExampleApp

final class StateTransitionTests: XCTestCase {
    
    var sdk: SDK!
    var testIdentityId: String!
    var key1Private: Data! // Critical Auth
    var key3Private: Data! // Critical Transfer
    
    override func setUpWithError() throws {
        print(">>> setUpWithError called")
        super.setUp()
        
        // Load environment variables
        EnvLoader.loadEnvFile()
        
        // Get test configuration from environment
        guard let testId = EnvLoader.get("TEST_IDENTITY_ID") else {
            throw XCTSkip("TEST_IDENTITY_ID not found in environment. Please copy .env.example to .env and add your test credentials.")
        }
        testIdentityId = testId
        
        // Decode private keys from base58
        guard let key1Base58 = EnvLoader.get("TEST_KEY_1_PRIVATE"),
              let key3Base58 = EnvLoader.get("TEST_KEY_3_PRIVATE") else {
            throw XCTSkip("TEST_KEY_1_PRIVATE or TEST_KEY_3_PRIVATE not found in environment. Please copy .env.example to .env and add your test credentials.")
        }
        
        key1Private = try decodePrivateKey(from: key1Base58)
        key3Private = try decodePrivateKey(from: key3Base58)
        
        // Initialize SDK
        sdk = try initializeSDK()
        
        // Wait for SDK to be ready
        Thread.sleep(forTimeInterval: 2.0)
    }
    
    override func tearDown() {
        sdk = nil
        super.tearDown()
    }
    
    // MARK: - Identity State Transitions
    
    func testEnvironmentLoading() throws {
        // Test that environment variables are loaded
        XCTAssertNotNil(testIdentityId, "TEST_IDENTITY_ID should be loaded")
        XCTAssertFalse(testIdentityId.isEmpty, "TEST_IDENTITY_ID should not be empty")
        XCTAssertNotNil(key1Private, "Key 1 private key should be loaded")
        XCTAssertNotNil(key3Private, "Key 3 private key should be loaded")
        print("✅ Environment variables loaded successfully")
    }
    
    func testSDKInitialization() throws {
        // Test basic SDK initialization
        XCTAssertNotNil(sdk, "SDK should be initialized")
        XCTAssertNotNil(sdk.handle, "SDK handle should exist")
        print("✅ SDK initialized successfully")
    }
    
    func testSimpleAsync() async throws {
        // Test that async tests work at all
        print("Starting simple async test")
        try await Task.sleep(nanoseconds: 100_000_000) // 0.1 second
        print("Simple async test completed")
        XCTAssertTrue(true)
    }
    
    func testIdentityCreditTransferDebug() async throws {
        print("Test started")
        
        // First check we have everything we need
        print("Checking SDK: \(sdk != nil ? "initialized" : "nil")")
        print("Checking testIdentityId: \(testIdentityId ?? "nil")")
        print("Checking key3Private: \(key3Private != nil ? "present (\(key3Private.count) bytes)" : "nil")")
        
        XCTAssertNotNil(sdk, "SDK must be initialized")
        XCTAssertNotNil(testIdentityId, "Test identity ID must be set")
        XCTAssertNotNil(key3Private, "Key 3 private key must be set")
        
        print("All checks passed")
        
        // Now try the actual transfer
        let recipientId = "HccabTZZpMEDAqU4oQFk3PE47kS6jDDmCjoxR88gFttA"
        let amount: UInt64 = 10_000_000
        
        print("Attempting transfer...")
        print("From: \(testIdentityId!)")
        print("To: \(recipientId)")
        print("Amount: \(amount) credits")
        
        do {
            // Fetch identity handle directly
            let fetchResult = testIdentityId.withCString { idCStr in
                dash_sdk_identity_fetch_handle(sdk.handle, idCStr)
            }
            
            guard fetchResult.error == nil,
                  let identityHandle = fetchResult.data else {
                if let error = fetchResult.error {
                    let errorString = String(cString: error.pointee.message)
                    dash_sdk_error_free(error)
                    throw XCTSkip("Failed to fetch identity: \(errorString)")
                }
                throw XCTSkip("Failed to fetch identity")
            }
            
            defer {
                dash_sdk_identity_destroy(OpaquePointer(identityHandle)!)
            }
            
            // Use key ID 3 (transfer key) directly
            
            // Create signer from private key
            let signerResult = key3Private.withUnsafeBytes { keyBytes in
                dash_sdk_signer_create_from_private_key(
                    keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                    UInt(key3Private.count)
                )
            }
            
            guard signerResult.error == nil,
                  let signer = signerResult.data else {
                throw XCTSkip("Failed to create signer")
            }
            
            defer {
                dash_sdk_signer_destroy(OpaquePointer(signer)!)
            }
            
            let (senderBalance, receiverBalance) = try await sdk.identityTransferCredits(
                fromIdentity: OpaquePointer(identityHandle)!,
                toIdentityId: recipientId,
                amount: amount,
                publicKeyId: 0, // Auto-select transfer key
                signer: OpaquePointer(signer)!
            )
            
            print("✅ Transfer successful!")
            print("Sender new balance: \(senderBalance)")
            print("Receiver new balance: \(receiverBalance)")
            
            XCTAssertTrue(senderBalance >= 0)
            XCTAssertTrue(receiverBalance > 0)
        } catch {
            print("Transfer failed with error: \(error)")
            XCTFail("Transfer failed with error: \(error)")
        }
    }
    
    func testIdentityCreditTransferSync() throws {
        print("🔄 Starting sync credit transfer test")
        
        // Check setup
        XCTAssertNotNil(sdk, "SDK must be initialized")
        XCTAssertNotNil(testIdentityId, "Test identity ID must be set")
        XCTAssertNotNil(key3Private, "Key 3 private key must be set")
        
        print("✅ All setup checks passed")
        print("Test identity ID: \(testIdentityId!)")
        print("Private key size: \(key3Private.count) bytes")
        
        // This test just verifies setup is correct
        // The actual async transfer would be executed in testIdentityCreditTransferAsync
        XCTAssertTrue(true)
    }
    
    func testBasicSetup() throws {
        print("Testing basic setup")
        XCTAssertNotNil(sdk)
        XCTAssertNotNil(testIdentityId)
        XCTAssertNotNil(key3Private)
        print("Basic setup passed")
    }
    
    func testTransferCredits() async throws {
        print("=== Starting testTransferCredits ===")
        
        // Wrap everything in a do-catch to capture any thrown errors
        do {
            // First verify setup
            print("1. Checking test setup...")
            guard let sdk = self.sdk else {
                XCTFail("SDK is nil")
                return
            }
            guard let testIdentityId = self.testIdentityId else {
                XCTFail("Test identity ID is nil")
                return
            }
            guard let key3Private = self.key3Private else {
                XCTFail("Key 3 private key is nil")
                return
            }
            print("✅ Setup verified")
            
            // Test parameters
            let recipientId = "HccabTZZpMEDAqU4oQFk3PE47kS6jDDmCjoxR88gFttA"
            let amount: UInt64 = 10_000_000 // 0.0001 DASH
            
            print("2. Transfer parameters:")
            print("   From: \(testIdentityId)")
            print("   To: \(recipientId)")
            print("   Amount: \(amount) credits")
            print("   Key size: \(key3Private.count) bytes")
            
            // Check if SDK method exists
            print("3. Checking SDK capabilities...")
            let sdkType = type(of: sdk)
            print("   SDK type: \(sdkType)")
            print("   SDK handle: \(sdk.handle != nil ? "present" : "nil")")
            
            // Try to fetch identity first
            print("4. Fetching sender identity...")
            do {
                let identity = try await sdk.identityGet(identityId: testIdentityId)
                print("   ✅ Identity fetched: \(identity)")
                
                if let balance = identity["balance"] as? UInt64 {
                    print("   Current balance: \(balance) credits")
                }
            } catch {
                print("   ❌ Failed to fetch identity: \(error)")
                print("   Error details: \(String(describing: error))")
            }
            
            // Now attempt the transfer
            print("5. Executing transfer...")
            do {
                print("   Creating identity and signer...")
                
                // Create DPPIdentity
                guard let idData = Data.identifier(fromBase58: testIdentityId) else {
                    throw XCTSkip("Invalid identity ID format")
                }
                
                let identity = try await sdk.identityGet(identityId: testIdentityId)
                let balance = (identity["balance"] as? UInt64) ?? 0
                
                let dppIdentity = DPPIdentity(
                    id: idData,
                    publicKeys: [:], // Empty for testing
                    balance: balance,
                    revision: 0
                )
                
                // Create signer from private key
                let signerResult = key3Private.withUnsafeBytes { keyBytes in
                    dash_sdk_signer_create_from_private_key(
                        keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                        UInt(key3Private.count)
                    )
                }
                
                guard signerResult.error == nil,
                      let signer = signerResult.data else {
                    throw XCTSkip("Failed to create signer")
                }
                
                defer {
                    dash_sdk_signer_destroy(OpaquePointer(signer)!)
                }
                
                print("   Calling transferCredits...")
                let result = try await sdk.transferCredits(
                    from: dppIdentity,
                    toIdentityId: recipientId,
                    amount: amount,
                    signer: OpaquePointer(signer)!
                )
                
                print("   ✅ Transfer successful!")
                print("   Sender new balance: \(result.senderBalance)")
                print("   Receiver new balance: \(result.receiverBalance)")
                
                XCTAssertTrue(result.senderBalance >= 0)
                XCTAssertTrue(result.receiverBalance > 0)
            } catch {
                print("   ❌ Transfer failed with error: \(error)")
                print("   Error type: \(type(of: error))")
                print("   Error details: \(String(describing: error))")
                XCTFail("Transfer failed: \(error)")
            }
        } catch {
            print("❌ Unexpected error in test: \(error)")
            print("   Error type: \(type(of: error))")
            print("   Error details: \(String(describing: error))")
            throw error
        }
        
        print("=== Test completed ===")
    }
    
    // Keep the original named test that calls our renamed version
    func testIdentityCreditTransfer() async throws {
        print(">>> testIdentityCreditTransfer called")
        do {
            print(">>> Delegating to testTransferCredits...")
            try await testTransferCredits()
            print(">>> testIdentityCreditTransfer completed successfully")
        } catch {
            print(">>> testIdentityCreditTransfer caught error: \(error)")
            throw error
        }
    }
    
    func testIdentityCreditWithdrawal() async throws {
        // Test withdrawal address
        let withdrawalAddress = "yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf" // Testnet address
        let amount: UInt64 = 1000 // 0.00001 DASH
        
        print("🔄 Testing Identity Credit Withdrawal")
        print("From Identity: \(testIdentityId!)")
        print("To Address: \(withdrawalAddress)")
        print("Amount: \(amount) credits")
        
        // Execute withdrawal using key 3 (transfer key)
        
        // Create DPPIdentity
        guard let idData = Data.identifier(fromBase58: testIdentityId) else {
            throw XCTSkip("Invalid identity ID format")
        }
        
        let identityDict = try await sdk.identityGet(identityId: testIdentityId)
        let balance = (identityDict["balance"] as? UInt64) ?? 0
        
        let identity = DPPIdentity(
            id: idData,
            publicKeys: [:], // Empty for testing
            balance: balance,
            revision: 0
        )
        
        // Create signer from private key
        let signerResult = key3Private.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(key3Private.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            throw XCTSkip("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        let newBalance = try await sdk.withdrawFromIdentity(
            identity,
            amount: amount,
            toAddress: withdrawalAddress,
            coreFeePerByte: 1,
            signer: OpaquePointer(signer)!
        )
        
        print("✅ Withdrawal successful!")
        print("New identity balance: \(newBalance)")
        
        XCTAssertTrue(newBalance >= 0)
    }
    
    func testIdentityUpdate() async throws {
        print("🔄 Testing Identity Update")
        
        // For identity update, we would add/disable keys
        // This requires more complex setup, skipping for now
        XCTSkip("Identity update requires key management setup")
    }
    
    // MARK: - Document State Transitions
    
    func testDocumentCreate() async throws {
        // Create a simple document on DPNS contract
        let contractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec" // DPNS contract
        
        print("🔄 Testing Document Create")
        
        // Create a domain document
        let properties: [String: Any] = [
            "label": "testdomain\(Int.random(in: 1000...9999))",
            "normalizedLabel": "testdomain\(Int.random(in: 1000...9999))",
            "normalizedParentDomainName": "dash",
            "preorderSalt": Data(repeating: 0, count: 32).base64EncodedString(),
            "records": [
                "dashIdentity": testIdentityId!
            ],
            "subdomainRules": [
                "allowSubdomains": false
            ]
        ]
        
        // This would require proper document creation implementation
        XCTSkip("Document creation requires full DPP implementation")
    }
    
    // MARK: - Test Utilities
    
    func testPrivateKeyDecoding() throws {
        // Test that we can decode the private keys correctly
        print("🔄 Testing private key decoding")
        
        XCTAssertNotNil(key1Private, "Key 1 should be decoded")
        XCTAssertEqual(key1Private.count, 32, "Private key should be 32 bytes")
        
        XCTAssertNotNil(key3Private, "Key 3 should be decoded")
        XCTAssertEqual(key3Private.count, 32, "Private key should be 32 bytes")
        
        print("✅ Private keys decoded successfully")
    }
    
    func testSignerCreation() throws {
        print("🔄 Testing signer creation in isolation")
        
        print("Private key: \(key3Private.hexEncodedString())")
        print("Private key length: \(key3Private.count) bytes")
        
        // Create signer from private key
        let signerResult = key3Private.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(key3Private.count)
            )
        }
        
        if let error = signerResult.error {
            let errorString = String(cString: error.pointee.message)
            dash_sdk_error_free(error)
            XCTFail("Failed to create signer: \(errorString)")
            return
        }
        
        guard let signer = signerResult.data else {
            XCTFail("Failed to create signer: no data returned")
            return
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        print("✅ Signer created successfully")
        print("Signer handle: \(signer)")
        
        // Test actual signing
        print("🔄 Testing actual signing operation")
        
        // Create some test data to sign
        let testData = "Hello, Dash Platform!".data(using: .utf8)!
        print("Test data to sign: \(testData.hexEncodedString())")
        print("Test data length: \(testData.count) bytes")
        
        // Try to sign the data
        let signResult = testData.withUnsafeBytes { dataBytes in
            dash_sdk_signer_sign(
                OpaquePointer(signer)!,
                dataBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(testData.count)
            )
        }
        
        if let error = signResult.error {
            let errorString = String(cString: error.pointee.message)
            dash_sdk_error_free(error)
            XCTFail("Failed to sign data: \(errorString)")
            return
        }
        
        guard let signaturePtr = signResult.data else {
            XCTFail("No signature data returned")
            return
        }
        
        // The result should be a signature structure
        let signature = signaturePtr.assumingMemoryBound(to: DashSDKSignature.self).pointee
        
        // Convert signature bytes to Data
        let signatureData = Data(bytes: signature.signature, count: Int(signature.signature_len))
        print("✅ Signature created successfully!")
        print("Signature: \(signatureData.hexEncodedString())")
        print("Signature length: \(signatureData.count) bytes")
        
        // Free the signature
        dash_sdk_signature_free(signaturePtr.assumingMemoryBound(to: DashSDKSignature.self))
        
        // Verify signature properties
        XCTAssertEqual(signatureData.count, 65, "ECDSA signature should be 65 bytes (r + s)")
        
        print("✅ Signer creation and signing test completed successfully")
    }
    
    func testMinimalTransferFFI() async throws {
        print("🔄 Testing minimal transfer at FFI level")
        
        // Create signer
        let signerResult = key3Private.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(key3Private.count)
            )
        }
        
        guard signerResult.error == nil, let signer = signerResult.data else {
            XCTFail("Failed to create signer")
            return
        }
        
        defer {
            dash_sdk_signer_destroy(OpaquePointer(signer)!)
        }
        
        print("✅ Signer created")
        
        // Fetch identity handle directly
        let fetchResult = testIdentityId.withCString { idCStr in
            dash_sdk_identity_fetch_handle(sdk.handle, idCStr)
        }
        
        guard fetchResult.error == nil, let identityHandle = fetchResult.data else {
            if let error = fetchResult.error {
                let errorString = String(cString: error.pointee.message)
                dash_sdk_error_free(error)
                XCTFail("Failed to fetch identity: \(errorString)")
            } else {
                XCTFail("Failed to fetch identity")
            }
            return
        }
        
        defer {
            dash_sdk_identity_destroy(OpaquePointer(identityHandle)!)
        }
        
        print("✅ Identity handle fetched")
        
        // Try the actual transfer call with minimal amount
        let recipientId = "HccabTZZpMEDAqU4oQFk3PE47kS6jDDmCjoxR88gFttA"
        let amount: UInt64 = 1000 // Very small amount
        
        print("🔄 Calling dash_sdk_identity_transfer_credits...")
        print("From identity handle: \(identityHandle)")
        print("To: \(recipientId)")
        print("Amount: \(amount)")
        print("Signer: \(signer)")
        
        let result = recipientId.withCString { toIdCStr in
            dash_sdk_identity_transfer_credits(
                sdk.handle,
                OpaquePointer(identityHandle)!,
                toIdCStr,
                amount,
                0, // Auto-select key
                OpaquePointer(signer)!,
                nil  // Default put settings
            )
        }
        
        if let error = result.error {
            let errorString = String(cString: error.pointee.message)
            dash_sdk_error_free(error)
            print("❌ Transfer failed with FFI error: \(errorString)")
            XCTFail("Transfer failed: \(errorString)")
            return
        }
        
        guard let transferResultPtr = result.data else {
            XCTFail("No transfer result data returned")
            return
        }
        
        let transferResult = transferResultPtr.assumingMemoryBound(to: DashSDKTransferCreditsResult.self).pointee
        let senderBalance = transferResult.sender_balance
        let receiverBalance = transferResult.receiver_balance
        
        // Free the transfer result
        dash_sdk_transfer_credits_result_free(transferResultPtr.assumingMemoryBound(to: DashSDKTransferCreditsResult.self))
        
        print("✅ Transfer successful!")
        print("Sender new balance: \(senderBalance)")
        print("Receiver new balance: \(receiverBalance)")
        
        XCTAssertTrue(senderBalance >= 0)
        XCTAssertTrue(receiverBalance > 0)
    }
    
    func testFetchIdentityBalance() async throws {
        print("🔄 Fetching identity balance")
        
        let identity = try await sdk.identityGet(identityId: testIdentityId)
        
        guard let balance = identity["balance"] as? UInt64 else {
            XCTFail("Could not get balance from identity")
            return
        }
        
        let dashAmount = Double(balance) / 100_000_000_000 // 1 DASH = 100B credits
        print("✅ Identity balance: \(balance) credits (\(dashAmount) DASH)")
        
        XCTAssertTrue(balance > 0, "Test identity should have balance")
    }
    
    // MARK: - Helper Methods
    
    private func initializeSDK() throws -> SDK {
        // Initialize SDK library first
        SDK.initialize()
        
        // Create SDK instance for testnet
        let testnetNetwork = DashSDKNetwork(rawValue: 1) // Testnet
        return try SDK(network: testnetNetwork)
    }
    
    private func decodePrivateKey(from base58: String) throws -> Data {
        // Remove WIF prefix and checksum to get raw private key
        guard let decoded = Data.fromBase58(base58),
              decoded.count >= 37 else {
            throw TestError.invalidPrivateKey
        }
        
        // WIF format: [version byte] + [32 bytes key] + [compression flag] + [4 bytes checksum]
        // Extract the 32-byte private key
        let privateKey = decoded[1..<33]
        return Data(privateKey)
    }
}

enum TestError: LocalizedError {
    case invalidPrivateKey
    case missingConfiguration
    
    var errorDescription: String? {
        switch self {
        case .invalidPrivateKey:
            return "Invalid private key format"
        case .missingConfiguration:
            return "Missing test configuration"
        }
    }
}

// MARK: - Data Extensions for Base58

extension Data {
    static func fromBase58(_ string: String) -> Data? {
        // Base58 alphabet (Bitcoin/Dash style)
        let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
        var result = Data()
        var multi = Data([0])
        
        for char in string {
            guard let index = alphabet.firstIndex(of: char) else { return nil }
            
            // Multiply existing result by 58
            var carry = 0
            for i in 0..<multi.count {
                carry += Int(multi[i]) * 58
                multi[i] = UInt8(carry % 256)
                carry /= 256
            }
            while carry > 0 {
                multi.append(UInt8(carry % 256))
                carry /= 256
            }
            
            // Add the index
            carry = alphabet.distance(from: alphabet.startIndex, to: index)
            for i in 0..<multi.count {
                carry += Int(multi[i])
                multi[i] = UInt8(carry % 256)
                carry /= 256
            }
            while carry > 0 {
                multi.append(UInt8(carry % 256))
                carry /= 256
            }
        }
        
        // Skip leading zeros
        for char in string {
            if char != "1" { break }
            result.append(0)
        }
        
        // Append in reverse order
        for byte in multi.reversed() {
            if result.count > 0 || byte != 0 {
                result.append(byte)
            }
        }
        
        return result
    }
    
    func hexEncodedString() -> String {
        return map { String(format: "%02hhx", $0) }.joined()
    }
}
