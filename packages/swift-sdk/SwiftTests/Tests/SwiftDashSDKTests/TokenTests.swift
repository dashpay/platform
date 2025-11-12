import XCTest
import SwiftDashSDKMock

class TokenTests: XCTestCase {
    
    var sdk: UnsafeMutablePointer<SwiftDashSDKHandle>?
    
    // Test configuration data
    let tokenContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
    let existingIdentityId = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"
    let recipientIdentityId = "7777777777777777777777777777777777777777777"
    
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
    
    // MARK: - Token Total Supply Tests
    
    func testTokenGetTotalSupply() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_token_get_total_supply(sdk, tokenContractId)
        XCTAssertNotNil(result, "Should return total supply")
        
        if let supplyString = result {
            let supplyStr = String(cString: supplyString)
            XCTAssertFalse(supplyStr.isEmpty, "Supply string should not be empty")
            
            // Mock returns "1000000000"
            XCTAssertEqual(supplyStr, "1000000000", "Mock should return 1000000000")
            
            // Clean up
            swift_dash_string_free(supplyString)
        }
    }
    
    func testTokenGetTotalSupplyWithNullSDK() {
        let result = swift_dash_token_get_total_supply(nil, tokenContractId)
        XCTAssertNil(result, "Should return nil for null SDK handle")
    }
    
    func testTokenGetTotalSupplyWithNullContractId() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_token_get_total_supply(sdk, nil)
        XCTAssertNil(result, "Should return nil for null contract ID")
    }
    
    // MARK: - Token Transfer Tests
    
    func testTokenTransfer() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let amount: UInt64 = 1000
        
        let result = swift_dash_token_transfer(
            sdk,
            tokenContractId,
            existingIdentityId,
            recipientIdentityId,
            amount
        )
        
        // Since this is not implemented in mock, should return not implemented error
        XCTAssertFalse(result.success, "Token transfer should fail (not implemented)")
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
    
    func testTokenTransferWithNullParams() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        // Test with null SDK
        var result = swift_dash_token_transfer(nil, tokenContractId, existingIdentityId, recipientIdentityId, 1000)
        XCTAssertFalse(result.success, "Should fail with null SDK")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
        
        // Test with null token contract ID
        result = swift_dash_token_transfer(sdk, nil, existingIdentityId, recipientIdentityId, 1000)
        XCTAssertFalse(result.success, "Should fail with null token contract ID")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
        
        // Test with null from identity ID
        result = swift_dash_token_transfer(sdk, tokenContractId, nil, recipientIdentityId, 1000)
        XCTAssertFalse(result.success, "Should fail with null from identity ID")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
        
        // Test with null to identity ID
        result = swift_dash_token_transfer(sdk, tokenContractId, existingIdentityId, nil, 1000)
        XCTAssertFalse(result.success, "Should fail with null to identity ID")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
    }
    
    // MARK: - Token Mint Tests
    
    func testTokenMint() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let amount: UInt64 = 5000
        
        let result = swift_dash_token_mint(sdk, tokenContractId, existingIdentityId, amount)
        
        // Since this is not implemented in mock, should return not implemented error
        XCTAssertFalse(result.success, "Token minting should fail (not implemented)")
        XCTAssertNotNil(result.error, "Should have error for not implemented")
        
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, NotImplemented, "Should be NotImplemented error")
            swift_dash_error_free(error)
        }
    }
    
    func testTokenMintWithNullParams() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        // Test with null SDK
        var result = swift_dash_token_mint(nil, tokenContractId, existingIdentityId, 1000)
        XCTAssertFalse(result.success, "Should fail with null SDK")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
        
        // Test with null token contract ID
        result = swift_dash_token_mint(sdk, nil, existingIdentityId, 1000)
        XCTAssertFalse(result.success, "Should fail with null token contract ID")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
        
        // Test with null to identity ID
        result = swift_dash_token_mint(sdk, tokenContractId, nil, 1000)
        XCTAssertFalse(result.success, "Should fail with null to identity ID")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
    }
    
    // MARK: - Token Burn Tests
    
    func testTokenBurn() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let amount: UInt64 = 2000
        
        let result = swift_dash_token_burn(sdk, tokenContractId, existingIdentityId, amount)
        
        // Since this is not implemented in mock, should return not implemented error
        XCTAssertFalse(result.success, "Token burning should fail (not implemented)")
        XCTAssertNotNil(result.error, "Should have error for not implemented")
        
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, NotImplemented, "Should be NotImplemented error")
            swift_dash_error_free(error)
        }
    }
    
    func testTokenBurnWithNullParams() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        // Test with null SDK
        var result = swift_dash_token_burn(nil, tokenContractId, existingIdentityId, 1000)
        XCTAssertFalse(result.success, "Should fail with null SDK")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
        
        // Test with null token contract ID
        result = swift_dash_token_burn(sdk, nil, existingIdentityId, 1000)
        XCTAssertFalse(result.success, "Should fail with null token contract ID")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
        
        // Test with null from identity ID
        result = swift_dash_token_burn(sdk, tokenContractId, nil, 1000)
        XCTAssertFalse(result.success, "Should fail with null from identity ID")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
    }
}