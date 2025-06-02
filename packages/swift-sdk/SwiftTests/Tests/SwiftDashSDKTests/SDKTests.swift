import XCTest
import SwiftDashSDKMock

class SDKTests: XCTestCase {
    
    override func setUp() {
        super.setUp()
        // Initialize the SDK before each test
        swift_dash_sdk_init()
    }
    
    // MARK: - Initialization Tests
    
    func testSDKInitialization() {
        // SDK should be initialized in setUp
        // If we get here without crashing, initialization worked
        XCTAssertTrue(true, "SDK initialized successfully")
    }
    
    func testSDKVersion() {
        let version = swift_dash_sdk_get_version()
        XCTAssertNotNil(version)
        
        if let version = version {
            let versionString = String(cString: version)
            XCTAssertFalse(versionString.isEmpty)
            XCTAssertTrue(versionString.contains("2.0.0"))
            
            // Clean up - in real SDK this would be ios_sdk_string_free
            free(version)
        }
    }
    
    // MARK: - Configuration Tests
    
    func testMainnetConfiguration() {
        let config = swift_dash_sdk_config_mainnet()
        
        XCTAssertEqual(config.network, SwiftDashNetwork_Mainnet)
        XCTAssertFalse(config.skip_asset_lock_proof_verification)
        XCTAssertEqual(config.request_retry_count, 3)
        XCTAssertEqual(config.request_timeout_ms, 30000)
    }
    
    func testTestnetConfiguration() {
        let config = swift_dash_sdk_config_testnet()
        
        XCTAssertEqual(config.network, SwiftDashNetwork_Testnet)
        XCTAssertFalse(config.skip_asset_lock_proof_verification)
        XCTAssertEqual(config.request_retry_count, 3)
        XCTAssertEqual(config.request_timeout_ms, 30000)
    }
    
    func testLocalConfiguration() {
        let config = swift_dash_sdk_config_local()
        
        XCTAssertEqual(config.network, SwiftDashNetwork_Local)
        XCTAssertTrue(config.skip_asset_lock_proof_verification)
        XCTAssertEqual(config.request_retry_count, 1)
        XCTAssertEqual(config.request_timeout_ms, 10000)
    }
    
    func testDefaultPutSettings() {
        var settings = swift_dash_put_settings_default()
        
        XCTAssertEqual(settings.connect_timeout_ms, 0)
        XCTAssertEqual(settings.timeout_ms, 0)
        XCTAssertEqual(settings.retries, 0)
        XCTAssertFalse(settings.ban_failed_address)
        XCTAssertEqual(settings.identity_nonce_stale_time_s, 0)
        XCTAssertEqual(settings.user_fee_increase, 0)
        XCTAssertFalse(settings.allow_signing_with_any_security_level)
        XCTAssertFalse(settings.allow_signing_with_any_purpose)
        XCTAssertEqual(settings.wait_timeout_ms, 0)
    }
    
    // MARK: - SDK Lifecycle Tests
    
    func testSDKCreateAndDestroy() {
        let config = swift_dash_sdk_config_testnet()
        let sdk = swift_dash_sdk_create(config)
        
        XCTAssertNotNil(sdk)
        
        if let sdk = sdk {
            // Test we can get network from SDK
            let network = swift_dash_sdk_get_network(sdk)
            XCTAssertEqual(network, SwiftDashNetwork_Testnet)
            
            // Clean up
            swift_dash_sdk_destroy(sdk)
        }
    }
    
    func testSDKCreateWithInvalidConfig() {
        var config = swift_dash_sdk_config_testnet()
        config.request_timeout_ms = 0 // Invalid timeout
        
        let sdk = swift_dash_sdk_create(config)
        XCTAssertNil(sdk, "SDK should not be created with invalid config")
    }
    
    func testSDKDestroyNullHandle() {
        // Should not crash
        swift_dash_sdk_destroy(nil)
        XCTAssertTrue(true, "Destroying null handle should not crash")
    }
    
    func testGetNetworkWithNullHandle() {
        let network = swift_dash_sdk_get_network(nil)
        XCTAssertEqual(network, SwiftDashNetwork_Testnet, "Should return default network for null handle")
    }
    
    // MARK: - Signer Tests
    
    func testSignerCreateAndDestroy() {
        let signer = swift_dash_signer_create_test()
        XCTAssertNotNil(signer)
        
        if let signer = signer {
            swift_dash_signer_destroy(signer)
        }
    }
    
    func testSignerDestroyNullHandle() {
        // Should not crash
        swift_dash_signer_destroy(nil)
        XCTAssertTrue(true, "Destroying null signer should not crash")
    }
    
    // MARK: - Custom Put Settings Tests
    
    func testCustomPutSettings() {
        var settings = swift_dash_put_settings_default()
        
        // Customize settings
        settings.timeout_ms = 60000 // 60 seconds
        settings.wait_timeout_ms = 120000 // 2 minutes
        settings.retries = 5
        settings.ban_failed_address = true
        settings.user_fee_increase = 10 // 10% increase
        
        XCTAssertEqual(settings.timeout_ms, 60000)
        XCTAssertEqual(settings.wait_timeout_ms, 120000)
        XCTAssertEqual(settings.retries, 5)
        XCTAssertTrue(settings.ban_failed_address)
        XCTAssertEqual(settings.user_fee_increase, 10)
    }
}