import XCTest
import SwiftDashSDKMock

class DataContractTests: XCTestCase {
    
    var sdk: UnsafeMutablePointer<SwiftDashSDKHandle>?
    
    // Test configuration data - matching rs-sdk-ffi test vectors
    let existingDataContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
    let nonExistentContractId = "1111111111111111111111111111111111111111111"
    let existingIdentityId = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"
    
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
    
    // MARK: - Data Contract Fetch Tests
    
    func testDataContractFetchNotFound() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_data_contract_fetch(sdk, nonExistentContractId)
        XCTAssertNil(result, "Non-existent data contract should return nil")
    }
    
    func testDataContractFetch() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_data_contract_fetch(sdk, existingDataContractId)
        XCTAssertNotNil(result, "Existing data contract should return data")
        
        if let jsonString = result {
            let jsonStr = String(cString: jsonString)
            XCTAssertFalse(jsonStr.isEmpty, "JSON string should not be empty")
            
            // Verify we can parse the JSON
            guard let jsonData = jsonStr.data(using: .utf8),
                  let json = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
                XCTFail("Should be valid JSON")
                return
            }
            
            // Verify we got a data contract back
            XCTAssertNotNil(json["id"], "Data contract should have an id field")
            XCTAssertNotNil(json["version"], "Data contract should have a version field")
            
            // Verify the contract ID matches
            if let id = json["id"] as? String {
                XCTAssertEqual(id, existingDataContractId, "Contract ID should match requested ID")
            }
            
            // Clean up
            swift_dash_string_free(jsonString)
        }
    }
    
    func testDataContractFetchWithNullSDK() {
        let result = swift_dash_data_contract_fetch(nil, existingDataContractId)
        XCTAssertNil(result, "Should return nil for null SDK handle")
    }
    
    func testDataContractFetchWithNullContractId() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_data_contract_fetch(sdk, nil)
        XCTAssertNil(result, "Should return nil for null contract ID")
    }
    
    // MARK: - Data Contract History Tests
    
    func testDataContractHistory() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_data_contract_get_history(sdk, existingDataContractId, 10, 0)
        
        if let jsonString = result {
            let jsonStr = String(cString: jsonString)
            XCTAssertFalse(jsonStr.isEmpty, "JSON string should not be empty")
            
            // Verify we can parse the JSON
            guard let jsonData = jsonStr.data(using: .utf8),
                  let json = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
                XCTFail("Should be valid JSON")
                return
            }
            
            // Should have contract_id and history fields
            XCTAssertNotNil(json["contract_id"], "Should have contract_id field")
            XCTAssertNotNil(json["history"], "Should have history field")
            
            if let contractId = json["contract_id"] as? String {
                XCTAssertEqual(contractId, existingDataContractId, "Contract ID should match")
            }
            
            // Clean up
            swift_dash_string_free(jsonString)
        } else {
            // No history is also valid for test vectors
            XCTAssertTrue(true, "Contract history may return nil if no history exists")
        }
    }
    
    func testDataContractHistoryNotFound() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_data_contract_get_history(sdk, nonExistentContractId, 10, 0)
        XCTAssertNil(result, "Non-existent contract should have no history")
    }
    
    func testDataContractHistoryWithNullSDK() {
        let result = swift_dash_data_contract_get_history(nil, existingDataContractId, 10, 0)
        XCTAssertNil(result, "Should return nil for null SDK handle")
    }
    
    func testDataContractHistoryWithNullContractId() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let result = swift_dash_data_contract_get_history(sdk, nil, 10, 0)
        XCTAssertNil(result, "Should return nil for null contract ID")
    }
    
    // MARK: - Data Contract Creation Tests
    
    func testDataContractCreate() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let schemaJson = """
        {
            "documents": {
                "message": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "maxLength": 256
                        }
                    },
                    "required": ["content"]
                }
            }
        }
        """
        
        let result = swift_dash_data_contract_create(sdk, schemaJson, existingIdentityId)
        
        // Since this is not implemented in mock, should return not implemented error
        XCTAssertFalse(result.success, "Data contract creation should fail (not implemented)")
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
    
    func testDataContractCreateWithNullParams() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let schemaJson = "{\"documents\":{\"test\":{\"type\":\"object\"}}}"
        
        // Test with null SDK
        var result = swift_dash_data_contract_create(nil, schemaJson, existingIdentityId)
        XCTAssertFalse(result.success, "Should fail with null SDK")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
        
        // Test with null schema JSON
        result = swift_dash_data_contract_create(sdk, nil, existingIdentityId)
        XCTAssertFalse(result.success, "Should fail with null schema JSON")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
        
        // Test with null owner ID
        result = swift_dash_data_contract_create(sdk, schemaJson, nil)
        XCTAssertFalse(result.success, "Should fail with null owner ID")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
    }
    
    // MARK: - Data Contract Update Tests
    
    func testDataContractUpdate() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let schemaJson = """
        {
            "documents": {
                "message": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "maxLength": 512
                        }
                    },
                    "required": ["content"]
                }
            }
        }
        """
        
        let result = swift_dash_data_contract_update(sdk, existingDataContractId, schemaJson, 2)
        
        // Since this is not implemented in mock, should return not implemented error
        XCTAssertFalse(result.success, "Data contract update should fail (not implemented)")
        XCTAssertNotNil(result.error, "Should have error for not implemented")
        
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, NotImplemented, "Should be NotImplemented error")
            swift_dash_error_free(error)
        }
    }
    
    func testDataContractUpdateWithNullParams() {
        guard let sdk = sdk else {
            XCTFail("SDK not initialized")
            return
        }
        
        let schemaJson = "{\"documents\":{\"test\":{\"type\":\"object\"}}}"
        
        // Test with null SDK
        var result = swift_dash_data_contract_update(nil, existingDataContractId, schemaJson, 2)
        XCTAssertFalse(result.success, "Should fail with null SDK")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
        
        // Test with null contract ID
        result = swift_dash_data_contract_update(sdk, nil, schemaJson, 2)
        XCTAssertFalse(result.success, "Should fail with null contract ID")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
        
        // Test with null schema JSON
        result = swift_dash_data_contract_update(sdk, existingDataContractId, nil, 2)
        XCTAssertFalse(result.success, "Should fail with null schema JSON")
        if let error = result.error {
            XCTAssertEqual(error.pointee.code, InvalidParameter, "Should be InvalidParameter error")
            swift_dash_error_free(error)
        }
    }
}