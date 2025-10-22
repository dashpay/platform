import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

final class DebugTests: XCTestCase {
    
    // Test 1: Simple synchronous test
    func testSimpleSync() throws {
        print("=== testSimpleSync ===")
        XCTAssertTrue(true)
        print("✅ Simple sync test passed")
    }
    
    // Test 2: Simple async test
    func testSimpleAsync() async throws {
        print("=== testSimpleAsync ===")
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertTrue(true)
        print("✅ Simple async test passed")
    }
    
    // Test 3: Test with "Transfer" in name (non-async)
    func testTransferSync() throws {
        print("=== testTransferSync ===")
        XCTAssertTrue(true)
        print("✅ Transfer sync test passed")
    }
    
    // Test 4: Test with "Transfer" in name (async)
    func testTransferAsync() async throws {
        print("=== testTransferAsync ===")
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertTrue(true)
        print("✅ Transfer async test passed")
    }
    
    // Test 5: Test with "CreditTransfer" in name
    func testCreditTransferDebug() async throws {
        print("=== testCreditTransferDebug ===")
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertTrue(true)
        print("✅ Credit transfer debug test passed")
    }
    
    // Test 6: Test SDK loading
    func testSDKLoading() throws {
        print("=== testSDKLoading ===")
        
        // Initialize SDK library
        SDK.initialize()
        print("✅ SDK library initialized")
        
        // Try to create SDK instance
        do {
            let sdk = try SDK(network: DashSDKNetwork_SDKTestnet)
            print("✅ SDK instance created")
            XCTAssertNotNil(sdk.handle)
        } catch {
            XCTFail("Failed to create SDK: \(error)")
        }
    }
    
    // Test 7: Test environment loading
    func testEnvLoading() throws {
        print("=== testEnvLoading ===")
        
        // Load env file
        EnvLoader.loadEnvFile()
        
        // Try to get test identity ID
        if let identityId = EnvLoader.get("TEST_IDENTITY_ID") {
            print("✅ Found TEST_IDENTITY_ID: \(identityId)")
            XCTAssertFalse(identityId.isEmpty)
        } else {
            print("⚠️ TEST_IDENTITY_ID not found in environment")
        }
    }
    
    // Test 8: Test with exact method signature
    func testIdentityCreditTransferDebug() async throws {
        print("=== testIdentityCreditTransferDebug ===")
        
        do {
            print("Starting test...")
            try await Task.sleep(nanoseconds: 100_000_000)
            print("✅ Test completed successfully")
            XCTAssertTrue(true)
        } catch {
            print("❌ Test failed: \(error)")
            throw error
        }
    }
    
    // Test 9: Test method discovery
    func testMethodDiscovery() throws {
        print("=== testMethodDiscovery ===")
        
        let testClass = StateTransitionTests.self
        print("Test class: \(testClass)")
        
        // List all test methods
        var methodCount: UInt32 = 0
        let methods = class_copyMethodList(testClass as? AnyClass, &methodCount)
        
        if let methods = methods {
            print("Found \(methodCount) methods:")
            for i in 0..<Int(methodCount) {
                let method = methods[i]
                let selector = method_getName(method)
                let name = NSStringFromSelector(selector)
                if name.hasPrefix("test") {
                    print("  - \(name)")
                }
            }
            free(methods)
        }
        
        XCTAssertTrue(true)
    }
}
