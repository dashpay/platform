import XCTest
import SwiftDashSDKMock

class IdentityTests: XCTestCase {
    
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
    
    // MARK: - Identity Fetch Tests
    
    func testIdentityFetchSuccess() {
        let identityId = "test_identity_123"
        let identity = swift_dash_identity_fetch(sdk, identityId)
        
        XCTAssertNotNil(identity)
    }
    
    func testIdentityFetchNotFound() {
        let identityId = "non_existent_identity"
        let identity = swift_dash_identity_fetch(sdk, identityId)
        
        XCTAssertNil(identity)
    }
    
    func testIdentityFetchNullParameters() {
        // Test null SDK handle
        var identity = swift_dash_identity_fetch(nil, "test_id")
        XCTAssertNil(identity)
        
        // Test null identity ID
        identity = swift_dash_identity_fetch(sdk, nil)
        XCTAssertNil(identity)
        
        // Test both null
        identity = swift_dash_identity_fetch(nil, nil)
        XCTAssertNil(identity)
    }
    
    // MARK: - Identity Info Tests
    
    func testIdentityGetInfo() {
        let identity = swift_dash_identity_fetch(sdk, "test_identity_123")
        XCTAssertNotNil(identity)
        
        guard let identity = identity else { return }
        
        let info = swift_dash_identity_get_info(identity)
        XCTAssertNotNil(info)
        
        guard let info = info else { return }
        defer { swift_dash_identity_info_free(info) }
        
        // Verify info contents
        XCTAssertNotNil(info.pointee.id)
        let idString = String(cString: info.pointee.id)
        XCTAssertEqual(idString, "test_identity_123")
        XCTAssertEqual(info.pointee.balance, 1000000)
        XCTAssertEqual(info.pointee.revision, 1)
        XCTAssertEqual(info.pointee.public_keys_count, 2)
    }
    
    func testIdentityGetInfoNullHandle() {
        let info = swift_dash_identity_get_info(nil)
        XCTAssertNil(info)
    }
    
    // MARK: - Put to Platform Tests
    
    func testIdentityPutToPlatformWithInstantLock() {
        let identity = swift_dash_identity_fetch(sdk, "test_identity_123")
        XCTAssertNotNil(identity)
        
        guard let identity = identity else { return }
        
        var settings = swift_dash_put_settings_default()
        settings.timeout_ms = 60000
        
        let result = swift_dash_identity_put_to_platform_with_instant_lock(
            sdk, identity, 0, signer, &settings
        )
        
        XCTAssertNotNil(result)
        
        guard let result = result else { return }
        defer { swift_dash_binary_data_free(result) }
        
        // Verify binary data
        XCTAssertGreaterThan(result.pointee.len, 0)
        XCTAssertNotNil(result.pointee.data)
        
        // Convert to Data for verification
        let data = Data(bytes: result.pointee.data, count: result.pointee.len)
        XCTAssertEqual(data.count, 64) // Mock returns 64 bytes
    }
    
    func testIdentityPutToPlatformWithInstantLockAndWait() {
        let identity = swift_dash_identity_fetch(sdk, "test_identity_123")
        XCTAssertNotNil(identity)
        
        guard let identity = identity else { return }
        
        var settings = swift_dash_put_settings_default()
        settings.wait_timeout_ms = 120000
        
        let confirmedIdentity = swift_dash_identity_put_to_platform_with_instant_lock_and_wait(
            sdk, identity, 0, signer, &settings
        )
        
        XCTAssertNotNil(confirmedIdentity)
        XCTAssertEqual(confirmedIdentity, identity) // Mock returns same handle
    }
    
    func testIdentityPutToPlatformNullSafety() {
        var settings = swift_dash_put_settings_default()
        
        // Test with null SDK
        var result = swift_dash_identity_put_to_platform_with_instant_lock(
            nil, nil, 0, signer, &settings
        )
        XCTAssertNil(result)
        
        // Test with null identity
        result = swift_dash_identity_put_to_platform_with_instant_lock(
            sdk, nil, 0, signer, &settings
        )
        XCTAssertNil(result)
        
        // Test with null signer
        let identity = swift_dash_identity_fetch(sdk, "test_identity_123")
        result = swift_dash_identity_put_to_platform_with_instant_lock(
            sdk, identity, 0, nil, &settings
        )
        XCTAssertNil(result)
    }
    
    // MARK: - Transfer Credits Tests
    
    func testIdentityTransferCredits() {
        let identity = swift_dash_identity_fetch(sdk, "test_identity_123")
        XCTAssertNotNil(identity)
        
        guard let identity = identity else { return }
        
        let recipientId = "recipient_identity_456"
        let amount: UInt64 = 50000
        var settings = swift_dash_put_settings_default()
        
        let result = swift_dash_identity_transfer_credits(
            sdk, identity, recipientId, amount, 0, signer, &settings
        )
        
        XCTAssertNotNil(result)
        
        guard let result = result else { return }
        defer { swift_dash_transfer_credits_result_free(result) }
        
        // Verify result
        XCTAssertEqual(result.pointee.amount, amount)
        XCTAssertNotNil(result.pointee.recipient_id)
        
        let recipient = String(cString: result.pointee.recipient_id)
        XCTAssertEqual(recipient, recipientId)
        
        XCTAssertNotNil(result.pointee.transaction_data)
        XCTAssertEqual(result.pointee.transaction_data_len, 32) // Mock returns 32 bytes
    }
    
    func testIdentityTransferCreditsNullSafety() {
        var settings = swift_dash_put_settings_default()
        
        // Test all null parameters
        var result = swift_dash_identity_transfer_credits(
            nil, nil, nil, 0, 0, nil, &settings
        )
        XCTAssertNil(result)
        
        // Test null recipient ID
        let identity = swift_dash_identity_fetch(sdk, "test_identity_123")
        result = swift_dash_identity_transfer_credits(
            sdk, identity, nil, 1000, 0, signer, &settings
        )
        XCTAssertNil(result)
    }
    
    // MARK: - Settings Tests
    
    func testPutOperationsWithCustomSettings() {
        let identity = swift_dash_identity_fetch(sdk, "test_identity_123")
        XCTAssertNotNil(identity)
        
        guard let identity = identity else { return }
        
        var settings = swift_dash_put_settings_default()
        settings.retries = 5
        settings.ban_failed_address = true
        settings.user_fee_increase = 15
        
        let result = swift_dash_identity_put_to_platform_with_instant_lock(
            sdk, identity, 0, signer, &settings
        )
        
        XCTAssertNotNil(result)
        
        if let result = result {
            swift_dash_binary_data_free(result)
        }
    }
    
    func testPutOperationsWithNullSettings() {
        let identity = swift_dash_identity_fetch(sdk, "test_identity_123")
        XCTAssertNotNil(identity)
        
        guard let identity = identity else { return }
        
        // Pass nil for settings (should use defaults)
        let result = swift_dash_identity_put_to_platform_with_instant_lock(
            sdk, identity, 0, signer, nil
        )
        
        XCTAssertNotNil(result)
        
        if let result = result {
            swift_dash_binary_data_free(result)
        }
    }
}