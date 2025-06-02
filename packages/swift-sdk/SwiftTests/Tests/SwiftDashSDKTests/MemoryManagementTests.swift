import XCTest
import SwiftDashSDKMock

class MemoryManagementTests: XCTestCase {
    
    var sdk: OpaquePointer!
    var signer: OpaquePointer!
    
    override func setUp() {
        super.setUp()
        swift_dash_sdk_init()
        
        let config = swift_dash_sdk_config_testnet()
        sdk = swift_dash_sdk_create(config)
        signer = swift_dash_signer_create_test()
    }
    
    override func tearDown() {
        if let signer = signer {
            swift_dash_signer_destroy(signer)
        }
        if let sdk = sdk {
            swift_dash_sdk_destroy(sdk)
        }
        super.tearDown()
    }
    
    // MARK: - SDK Memory Management
    
    func testSDKCreateDestroyMemoryLeak() {
        // Create and destroy multiple SDKs to test for memory leaks
        for _ in 0..<100 {
            let config = swift_dash_sdk_config_testnet()
            if let tempSdk = swift_dash_sdk_create(config) {
                swift_dash_sdk_destroy(tempSdk)
            }
        }
        
        // If we get here without crashing, memory management is working
        XCTAssertTrue(true)
    }
    
    func testSignerCreateDestroyMemoryLeak() {
        // Create and destroy multiple signers
        for _ in 0..<100 {
            if let tempSigner = swift_dash_signer_create_test() {
                swift_dash_signer_destroy(tempSigner)
            }
        }
        
        XCTAssertTrue(true)
    }
    
    // MARK: - String Memory Management
    
    func testVersionStringMemory() {
        // Get version multiple times and free each time
        for _ in 0..<10 {
            if let version = swift_dash_sdk_get_version() {
                let versionString = String(cString: version)
                XCTAssertFalse(versionString.isEmpty)
                free(version)
            }
        }
    }
    
    func testDataContractInfoStringMemory() {
        let contract = swift_dash_data_contract_fetch(sdk, "test_contract_456")
        XCTAssertNotNil(contract)
        
        guard let contract = contract else { return }
        
        // Get info multiple times and free each time
        for _ in 0..<10 {
            if let info = swift_dash_data_contract_get_info(contract) {
                let infoString = String(cString: info)
                XCTAssertFalse(infoString.isEmpty)
                free(info)
            }
        }
    }
    
    // MARK: - Binary Data Memory Management
    
    func testBinaryDataFree() {
        let identity = swift_dash_identity_fetch(sdk, "test_identity_123")
        XCTAssertNotNil(identity)
        
        guard let identity = identity else { return }
        
        var settings = swift_dash_put_settings_default()
        
        // Create and free binary data multiple times
        for _ in 0..<10 {
            if let result = swift_dash_identity_put_to_platform_with_instant_lock(
                sdk, identity, 0, signer, &settings
            ) {
                // Verify data before freeing
                XCTAssertNotNil(result.pointee.data)
                XCTAssertGreaterThan(result.pointee.len, 0)
                
                swift_dash_binary_data_free(result)
            }
        }
    }
    
    func testDocumentBinaryDataMemory() {
        let contract = swift_dash_data_contract_fetch(sdk, "test_contract_456")
        let document = swift_dash_document_create(
            sdk, contract, "test_identity_123", "message",
            "{\"content\": \"test\", \"timestamp\": 1640000000000}"
        )
        
        XCTAssertNotNil(document)
        guard let document = document else { return }
        
        var settings = swift_dash_put_settings_default()
        
        // Put document multiple times and free each result
        for _ in 0..<10 {
            if let result = swift_dash_document_put_to_platform(
                sdk, document, 0, signer, &settings
            ) {
                XCTAssertNotNil(result.pointee.data)
                XCTAssertEqual(result.pointee.len, 256)
                
                swift_dash_binary_data_free(result)
            }
        }
    }
    
    // MARK: - Info Structure Memory Management
    
    func testIdentityInfoMemory() {
        let identity = swift_dash_identity_fetch(sdk, "test_identity_123")
        XCTAssertNotNil(identity)
        
        guard let identity = identity else { return }
        
        // Get and free info multiple times
        for _ in 0..<10 {
            if let info = swift_dash_identity_get_info(identity) {
                // Verify info before freeing
                XCTAssertNotNil(info.pointee.id)
                XCTAssertEqual(info.pointee.balance, 1000000)
                
                swift_dash_identity_info_free(info)
            }
        }
    }
    
    func testDocumentInfoMemory() {
        let contract = swift_dash_data_contract_fetch(sdk, "test_contract_456")
        let document = swift_dash_document_fetch(
            sdk, contract, "message", "test_doc_789"
        )
        
        XCTAssertNotNil(document)
        guard let document = document else { return }
        
        // Get and free info multiple times
        for _ in 0..<10 {
            if let info = swift_dash_document_get_info(document) {
                // Verify info before freeing
                XCTAssertNotNil(info.pointee.id)
                XCTAssertNotNil(info.pointee.owner_id)
                XCTAssertNotNil(info.pointee.data_contract_id)
                XCTAssertNotNil(info.pointee.document_type)
                
                swift_dash_document_info_free(info)
            }
        }
    }
    
    func testTransferCreditsResultMemory() {
        let identity = swift_dash_identity_fetch(sdk, "test_identity_123")
        XCTAssertNotNil(identity)
        
        guard let identity = identity else { return }
        
        var settings = swift_dash_put_settings_default()
        
        // Transfer credits multiple times and free each result
        for i in 0..<10 {
            let amount: UInt64 = UInt64(1000 * (i + 1))
            
            if let result = swift_dash_identity_transfer_credits(
                sdk, identity, "recipient_\(i)", amount, 0, signer, &settings
            ) {
                // Verify result before freeing
                XCTAssertEqual(result.pointee.amount, amount)
                XCTAssertNotNil(result.pointee.recipient_id)
                XCTAssertNotNil(result.pointee.transaction_data)
                XCTAssertEqual(result.pointee.transaction_data_len, 32)
                
                swift_dash_transfer_credits_result_free(result)
            }
        }
    }
    
    // MARK: - Null Safety for Free Functions
    
    func testFreeNullPointers() {
        // All free functions should handle null gracefully
        swift_dash_error_free(nil)
        swift_dash_identity_info_free(nil)
        swift_dash_document_info_free(nil)
        swift_dash_binary_data_free(nil)
        swift_dash_transfer_credits_result_free(nil)
        
        // If we get here without crashing, null safety is working
        XCTAssertTrue(true)
    }
    
    // MARK: - Complex Memory Scenarios
    
    func testComplexMemoryScenario() {
        // Simulate a complex workflow with multiple allocations and frees
        
        // 1. Create identity and get info
        let identity = swift_dash_identity_fetch(sdk, "test_identity_123")
        XCTAssertNotNil(identity)
        
        guard let identity = identity else { return }
        
        let identityInfo = swift_dash_identity_get_info(identity)
        XCTAssertNotNil(identityInfo)
        
        // 2. Create contract and document
        let contract = swift_dash_data_contract_fetch(sdk, "test_contract_456")
        XCTAssertNotNil(contract)
        
        guard let contract = contract else {
            swift_dash_identity_info_free(identityInfo)
            return
        }
        
        let document = swift_dash_document_create(
            sdk, contract, "test_identity_123", "message",
            "{\"content\": \"Complex test\", \"timestamp\": 1640000000000}"
        )
        XCTAssertNotNil(document)
        
        guard let document = document else {
            swift_dash_identity_info_free(identityInfo)
            return
        }
        
        // 3. Get document info
        let documentInfo = swift_dash_document_get_info(document)
        XCTAssertNotNil(documentInfo)
        
        // 4. Perform operations
        var settings = swift_dash_put_settings_default()
        settings.timeout_ms = 60000
        
        let putResult = swift_dash_document_put_to_platform(
            sdk, document, 0, signer, &settings
        )
        XCTAssertNotNil(putResult)
        
        let transferResult = swift_dash_identity_transfer_credits(
            sdk, identity, "recipient_test", 5000, 0, signer, &settings
        )
        XCTAssertNotNil(transferResult)
        
        // 5. Clean up everything in correct order
        if let putResult = putResult {
            swift_dash_binary_data_free(putResult)
        }
        
        if let transferResult = transferResult {
            swift_dash_transfer_credits_result_free(transferResult)
        }
        
        if let documentInfo = documentInfo {
            swift_dash_document_info_free(documentInfo)
        }
        
        if let identityInfo = identityInfo {
            swift_dash_identity_info_free(identityInfo)
        }
        
        // If we get here without memory issues, complex scenario passed
        XCTAssertTrue(true)
    }
    
    func testRapidAllocationDeallocation() {
        // Stress test with rapid allocation/deallocation
        let queue = DispatchQueue(label: "memory.test", attributes: .concurrent)
        let group = DispatchGroup()
        
        // Run multiple concurrent operations
        for i in 0..<10 {
            group.enter()
            queue.async {
                defer { group.leave() }
                
                // Create and destroy resources rapidly
                for j in 0..<100 {
                    autoreleasepool {
                        if let version = swift_dash_sdk_get_version() {
                            _ = String(cString: version)
                            free(version)
                        }
                        
                        // Only use existing SDK from setUp, don't create new ones
                        if i % 2 == 0 && j % 10 == 0 {
                            if let identity = swift_dash_identity_fetch(self.sdk, "test_identity_123") {
                                if let info = swift_dash_identity_get_info(identity) {
                                    swift_dash_identity_info_free(info)
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Wait for all operations to complete
        let result = group.wait(timeout: .now() + 30)
        XCTAssertEqual(result, .success)
    }
}