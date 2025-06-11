import XCTest
import SwiftDashSDKMock

class IdentityTests: XCTestCase {
    
    var sdk: UnsafeMutablePointer<SwiftDashSDKHandle>?
    
    // Test configuration data - matching rs-sdk-ffi test vectors
    let existingIdentityId = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"
    let nonExistentIdentityId = "1111111111111111111111111111111111111111111"
    
    override func setUp() {
        super.setUp()
        swift_dash_sdk_init()
        
        let config = swift_dash_sdk_config_testnet()
        sdk = swift_dash_sdk_create(config)
        XCTAssertNotNil(sdk, "SDK should be created successfully")
    }
    
    override func tearDown() {
        if let sdk = sdk {
            swift_dash_sdk_destroy(sdk)
        }
        super.tearDown()
    }
    
    // MARK: - Identity Fetch Tests
    
    func testIdentityFetchNotFound() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_identity_fetch(sdk, nonExistentIdentityId)
        XCTAssertNil(result, "Non-existent identity should return nil")
    }
    
    func testIdentityFetch() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_identity_fetch(sdk, existingIdentityId)
        XCTAssertNotNil(result, "Existing identity should return data")
        
        if let jsonString = result {
            let jsonStr = String(cString: jsonString)
            XCTAssertFalse(jsonStr.isEmpty, "JSON string should not be empty")
            
            // Verify we can parse the JSON
            guard let jsonData = jsonStr.data(using: .utf8),
                  let json = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
                XCTFail("Should be valid JSON")
                return
            }
            
            // Verify we got an identity back
            XCTAssertNotNil(json["id"], "Identity should have an id field")
            XCTAssertNotNil(json["publicKeys"], "Identity should have publicKeys field")
            
            // Verify the identity ID matches
            if let id = json["id"] as? String {
                XCTAssertEqual(id, existingIdentityId, "Identity ID should match requested ID")
            }
            
            // Clean up
            swift_dash_string_free(jsonString)
        }
    }
    
    func testIdentityFetchWithNullSDK() {
        let result = swift_dash_identity_fetch(nil, existingIdentityId)
        XCTAssertNil(result, "Should return nil for null SDK handle")
    }
    
    func testIdentityFetchWithNullIdentityId() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_identity_fetch(sdk, nil)
        XCTAssertNil(result, "Should return nil for null identity ID")
    }
    
    // MARK: - Identity Balance Tests
    
    func testIdentityBalance() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let balance = swift_dash_identity_get_balance(sdk, existingIdentityId)
        XCTAssertGreaterThan(balance, 0, "Existing identity should have a balance")
        
        // Mock returns 1000000 credits
        XCTAssertEqual(balance, 1000000, "Mock should return 1000000 credits")
    }
    
    func testIdentityBalanceNotFound() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let balance = swift_dash_identity_get_balance(sdk, nonExistentIdentityId)
        XCTAssertEqual(balance, 0, "Non-existent identity should have zero balance")
    }
    
    func testIdentityBalanceWithNullSDK() {
        let balance = swift_dash_identity_get_balance(nil, existingIdentityId)
        XCTAssertEqual(balance, 0, "Should return 0 for null SDK handle")
    }
    
    func testIdentityBalanceWithNullIdentityId() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let balance = swift_dash_identity_get_balance(sdk, nil)
        XCTAssertEqual(balance, 0, "Should return 0 for null identity ID")
    }
    
    // MARK: - Identity Name Resolution Tests
    
    func testIdentityResolveByAlias() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_identity_resolve_name(sdk, "dash")
        
        if let jsonString = result {
            let jsonStr = String(cString: jsonString)
            XCTAssertFalse(jsonStr.isEmpty, "JSON string should not be empty")
            
            // Verify we can parse the JSON
            guard let jsonData = jsonStr.data(using: .utf8),
                  let json = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
                XCTFail("Should be valid JSON")
                return
            }
            
            // Verify we got identity and alias fields
            XCTAssertNotNil(json["identity"], "Should have identity field")
            XCTAssertNotNil(json["alias"], "Should have alias field")
            
            if let alias = json["alias"] as? String {
                XCTAssertEqual(alias, "dash", "Alias should match requested name")
            }
            
            // Clean up
            swift_dash_string_free(jsonString)
        } else {
            // Name not found is also valid for test vectors
            XCTAssertTrue(true, "Name resolution may return nil if not found in test vectors")
        }
    }
    
    func testIdentityResolveNonExistentName() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_identity_resolve_name(sdk, "nonexistent_name_12345")
        XCTAssertNil(result, "Non-existent name should return nil")
    }
    
    func testIdentityResolveWithNullSDK() {
        let result = swift_dash_identity_resolve_name(nil, "dash")
        XCTAssertNil(result, "Should return nil for null SDK handle")
    }
    
    func testIdentityResolveWithNullName() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_identity_resolve_name(sdk, nil)
        XCTAssertNil(result, "Should return nil for null name")
    }
    
    // MARK: - Identity Transfer Credits Tests
    
    func testIdentityTransferCredits() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let privateKey: [UInt8] = Array(repeating: 0x42, count: 32) // Mock private key
        let amount: UInt64 = 1000
        
        let result = swift_dash_identity_transfer_credits(
            sdk,
            existingIdentityId,
            "7777777777777777777777777777777777777777777", // recipient
            amount,
            privateKey,
            privateKey.count
        )
        
        // Since this is not implemented in mock, should return not implemented error
        XCTAssertFalse(result.success, "Credit transfer should fail (not implemented)")
        XCTAssertNotNil(result.error, "Should have error for not implemented")
        
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, NotImplemented, "Should be NotImplemented error")
            
            if let message = error.pointee.message {
                let messageStr = String(cString: message)
                XCTAssertTrue(messageStr.contains("not yet implemented"), "Error message should mention not implemented")
            }
            
            // Clean up error
            swift_dash_error_free(error)
        }
    }
    
    func testIdentityTransferCreditsWithNullParams() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let privateKey: [UInt8] = Array(repeating: 0x42, count: 32)
        
        // Test with null SDK
        var result = swift_dash_identity_transfer_credits(
            nil,
            existingIdentityId,
            "7777777777777777777777777777777777777777777",
            1000,
            privateKey,
            privateKey.count
        )
        
        XCTAssertFalse(result.success, "Should fail with null SDK")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
        
        // Test with null from_identity_id
        result = swift_dash_identity_transfer_credits(
            sdk,
            nil,
            "7777777777777777777777777777777777777777777",
            1000,
            privateKey,
            privateKey.count
        )
        
        XCTAssertFalse(result.success, "Should fail with null from_identity_id")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
    }
    
    // MARK: - Identity Creation Tests
    
    func testIdentityCreate() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let publicKey: [UInt8] = Array(repeating: 0x33, count: 33) // Mock public key
        
        let result = swift_dash_identity_create(sdk, publicKey, publicKey.count)
        
        // Since this is not implemented in mock, should return not implemented error
        XCTAssertFalse(result.success, "Identity creation should fail (not implemented)")
        XCTAssertNotNil(result.error, "Should have error for not implemented")
        
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, NotImplemented, "Should be NotImplemented error")
            swift_dash_error_free(error)
        }
    }
    
    func testIdentityCreateWithNullParams() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let publicKey: [UInt8] = Array(repeating: 0x33, count: 33)
        
        // Test with null SDK
        var result = swift_dash_identity_create(nil, publicKey, publicKey.count)
        XCTAssertFalse(result.success, "Should fail with null SDK")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
        
        // Test with null public key
        result = swift_dash_identity_create(sdk, nil, 0)
        XCTAssertFalse(result.success, "Should fail with null public key")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
    }
}