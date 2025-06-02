import XCTest
import SwiftDashSDKMock

class DataContractTests: XCTestCase {
    
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
    
    // MARK: - Contract Fetch Tests
    
    func testDataContractFetchSuccess() {
        let contractId = "test_contract_456"
        let contract = swift_dash_data_contract_fetch(sdk, contractId)
        
        XCTAssertNotNil(contract)
    }
    
    func testDataContractFetchNotFound() {
        let contractId = "non_existent_contract"
        let contract = swift_dash_data_contract_fetch(sdk, contractId)
        
        XCTAssertNil(contract)
    }
    
    func testDataContractFetchNullSafety() {
        // Test null SDK
        var contract = swift_dash_data_contract_fetch(nil, "test_id")
        XCTAssertNil(contract)
        
        // Test null contract ID
        contract = swift_dash_data_contract_fetch(sdk, nil)
        XCTAssertNil(contract)
        
        // Test both null
        contract = swift_dash_data_contract_fetch(nil, nil)
        XCTAssertNil(contract)
    }
    
    // MARK: - Contract Creation Tests
    
    func testDataContractCreate() {
        let ownerId = "test_identity_123"
        let schema = """
        {
            "$format_version": "0",
            "ownerId": "\(ownerId)",
            "documents": {
                "message": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "maxLength": 280
                        },
                        "timestamp": {
                            "type": "integer"
                        }
                    },
                    "required": ["content", "timestamp"],
                    "additionalProperties": false
                }
            }
        }
        """
        
        let contract = swift_dash_data_contract_create(sdk, ownerId, schema)
        XCTAssertNotNil(contract)
    }
    
    func testDataContractCreateNullSafety() {
        let ownerId = "test_identity_123"
        let schema = "{}"
        
        // Test null SDK
        var contract = swift_dash_data_contract_create(nil, ownerId, schema)
        XCTAssertNil(contract)
        
        // Test null owner ID
        contract = swift_dash_data_contract_create(sdk, nil, schema)
        XCTAssertNil(contract)
        
        // Test null schema
        contract = swift_dash_data_contract_create(sdk, ownerId, nil)
        XCTAssertNil(contract)
    }
    
    // MARK: - Contract Info Tests
    
    func testDataContractGetInfo() {
        let contract = swift_dash_data_contract_fetch(sdk, "test_contract_456")
        XCTAssertNotNil(contract)
        
        guard let contract = contract else { return }
        
        let infoJson = swift_dash_data_contract_get_info(contract)
        XCTAssertNotNil(infoJson)
        
        guard let infoJson = infoJson else { return }
        defer { free(infoJson) }
        
        let jsonString = String(cString: infoJson)
        XCTAssertFalse(jsonString.isEmpty)
        XCTAssertTrue(jsonString.contains("test_contract_456"))
        XCTAssertTrue(jsonString.contains("version"))
    }
    
    func testDataContractGetInfoNullHandle() {
        let info = swift_dash_data_contract_get_info(nil)
        XCTAssertNil(info)
    }
    
    // MARK: - Put to Platform Tests
    
    func testDataContractPutToPlatform() {
        let ownerId = "test_identity_123"
        let schema = """
        {
            "documents": {
                "profile": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"type": "integer"}
                    }
                }
            }
        }
        """
        
        let contract = swift_dash_data_contract_create(sdk, ownerId, schema)
        XCTAssertNotNil(contract)
        
        guard let contract = contract else { return }
        
        var settings = swift_dash_put_settings_default()
        settings.timeout_ms = 60000
        
        let result = swift_dash_data_contract_put_to_platform(
            sdk, contract, 0, signer, &settings
        )
        
        XCTAssertNotNil(result)
        
        guard let result = result else { return }
        defer { swift_dash_binary_data_free(result) }
        
        // Verify binary data
        XCTAssertGreaterThan(result.pointee.len, 0)
        XCTAssertNotNil(result.pointee.data)
        XCTAssertEqual(result.pointee.len, 128) // Mock returns 128 bytes
    }
    
    func testDataContractPutToPlatformNullSafety() {
        var settings = swift_dash_put_settings_default()
        
        // Test null SDK
        var result = swift_dash_data_contract_put_to_platform(
            nil, nil, 0, signer, &settings
        )
        XCTAssertNil(result)
        
        // Test null contract
        result = swift_dash_data_contract_put_to_platform(
            sdk, nil, 0, signer, &settings
        )
        XCTAssertNil(result)
        
        // Test null signer
        let contract = swift_dash_data_contract_fetch(sdk, "test_contract_456")
        result = swift_dash_data_contract_put_to_platform(
            sdk, contract, 0, nil, &settings
        )
        XCTAssertNil(result)
    }
    
    // MARK: - Schema Examples
    
    func testComplexDataContractSchema() {
        let ownerId = "test_identity_123"
        
        // DPNS-like contract schema
        let dpnsSchema = """
        {
            "$format_version": "0",
            "id": "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            "ownerId": "\(ownerId)",
            "version": 1,
            "documentSchemas": {
                "domain": {
                    "type": "object",
                    "properties": {
                        "label": {
                            "type": "string",
                            "pattern": "^[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9]$",
                            "minLength": 3,
                            "maxLength": 63,
                            "description": "Domain label"
                        },
                        "normalizedLabel": {
                            "type": "string",
                            "pattern": "^[a-z0-9][a-z0-9-]{0,61}[a-z0-9]$",
                            "maxLength": 63,
                            "description": "Normalized domain label"
                        },
                        "normalizedParentDomainName": {
                            "type": "string",
                            "pattern": "^$|^[a-z0-9][a-z0-9-\\\\.]{0,189}[a-z0-9]$",
                            "maxLength": 190,
                            "description": "Parent domain"
                        },
                        "records": {
                            "type": "object",
                            "properties": {
                                "dashUniqueIdentityId": {
                                    "type": "array",
                                    "byteArray": true,
                                    "minItems": 32,
                                    "maxItems": 32,
                                    "description": "Identity ID"
                                }
                            },
                            "additionalProperties": false
                        }
                    },
                    "required": ["label", "normalizedLabel", "normalizedParentDomainName", "records"],
                    "additionalProperties": false
                }
            }
        }
        """
        
        let contract = swift_dash_data_contract_create(sdk, ownerId, dpnsSchema)
        XCTAssertNotNil(contract)
    }
    
    func testSocialMediaContractSchema() {
        let ownerId = "test_identity_123"
        
        // Social media-like contract
        let socialSchema = """
        {
            "$format_version": "0",
            "ownerId": "\(ownerId)",
            "documents": {
                "post": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "maxLength": 280
                        },
                        "author": {
                            "type": "string"
                        },
                        "timestamp": {
                            "type": "integer"
                        },
                        "likes": {
                            "type": "integer",
                            "minimum": 0
                        },
                        "tags": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "maxLength": 50
                            },
                            "maxItems": 10
                        }
                    },
                    "required": ["content", "author", "timestamp"],
                    "additionalProperties": false
                },
                "comment": {
                    "type": "object",
                    "properties": {
                        "postId": {
                            "type": "string"
                        },
                        "content": {
                            "type": "string",
                            "maxLength": 280
                        },
                        "author": {
                            "type": "string"
                        },
                        "timestamp": {
                            "type": "integer"
                        }
                    },
                    "required": ["postId", "content", "author", "timestamp"],
                    "additionalProperties": false
                }
            }
        }
        """
        
        let contract = swift_dash_data_contract_create(sdk, ownerId, socialSchema)
        XCTAssertNotNil(contract)
    }
}