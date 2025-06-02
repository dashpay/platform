import XCTest
import SwiftDashSDKMock

class DocumentTests: XCTestCase {
    
    var sdk: OpaquePointer!
    var signer: OpaquePointer!
    var contract: OpaquePointer!
    
    override func setUp() {
        super.setUp()
        swift_dash_sdk_init()
        
        let config = swift_dash_sdk_config_testnet()
        sdk = swift_dash_sdk_create(config)
        signer = swift_dash_signer_create_test()
        
        // Create a contract for testing documents
        contract = swift_dash_data_contract_fetch(sdk, "test_contract_456")
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
    
    // MARK: - Document Creation Tests
    
    func testDocumentCreate() {
        let ownerId = "test_identity_123"
        let documentType = "message"
        let dataJson = """
        {
            "content": "Hello, Dash Platform!",
            "timestamp": 1640000000000,
            "author": "test_user"
        }
        """
        
        let document = swift_dash_document_create(
            sdk, contract, ownerId, documentType, dataJson
        )
        
        XCTAssertNotNil(document)
    }
    
    func testDocumentCreateNullSafety() {
        let ownerId = "test_identity_123"
        let documentType = "message"
        let dataJson = "{}"
        
        // Test null SDK
        var document = swift_dash_document_create(
            nil, contract, ownerId, documentType, dataJson
        )
        XCTAssertNil(document)
        
        // Test null contract
        document = swift_dash_document_create(
            sdk, nil, ownerId, documentType, dataJson
        )
        XCTAssertNil(document)
        
        // Test null owner ID
        document = swift_dash_document_create(
            sdk, contract, nil, documentType, dataJson
        )
        XCTAssertNil(document)
        
        // Test null document type
        document = swift_dash_document_create(
            sdk, contract, ownerId, nil, dataJson
        )
        XCTAssertNil(document)
        
        // Test null data JSON
        document = swift_dash_document_create(
            sdk, contract, ownerId, documentType, nil
        )
        XCTAssertNil(document)
    }
    
    // MARK: - Document Fetch Tests
    
    func testDocumentFetchSuccess() {
        let documentType = "message"
        let documentId = "test_doc_789"
        
        let document = swift_dash_document_fetch(
            sdk, contract, documentType, documentId
        )
        
        XCTAssertNotNil(document)
    }
    
    func testDocumentFetchNotFound() {
        let documentType = "message"
        let documentId = "non_existent_doc"
        
        let document = swift_dash_document_fetch(
            sdk, contract, documentType, documentId
        )
        
        XCTAssertNil(document)
    }
    
    func testDocumentFetchNullSafety() {
        let documentType = "message"
        let documentId = "test_doc_789"
        
        // Test null SDK
        var document = swift_dash_document_fetch(
            nil, contract, documentType, documentId
        )
        XCTAssertNil(document)
        
        // Test null contract
        document = swift_dash_document_fetch(
            sdk, nil, documentType, documentId
        )
        XCTAssertNil(document)
        
        // Test null document type
        document = swift_dash_document_fetch(
            sdk, contract, nil, documentId
        )
        XCTAssertNil(document)
        
        // Test null document ID
        document = swift_dash_document_fetch(
            sdk, contract, documentType, nil
        )
        XCTAssertNil(document)
    }
    
    // MARK: - Document Info Tests
    
    func testDocumentGetInfo() {
        let document = swift_dash_document_fetch(
            sdk, contract, "message", "test_doc_789"
        )
        XCTAssertNotNil(document)
        
        guard let document = document else { return }
        
        let info = swift_dash_document_get_info(document)
        XCTAssertNotNil(info)
        
        guard let info = info else { return }
        defer { swift_dash_document_info_free(info) }
        
        // Verify info contents
        XCTAssertNotNil(info.pointee.id)
        let idString = String(cString: info.pointee.id)
        XCTAssertEqual(idString, "test_doc_789")
        
        XCTAssertNotNil(info.pointee.owner_id)
        let ownerString = String(cString: info.pointee.owner_id)
        XCTAssertEqual(ownerString, "test_identity_123")
        
        XCTAssertNotNil(info.pointee.data_contract_id)
        let contractString = String(cString: info.pointee.data_contract_id)
        XCTAssertEqual(contractString, "test_contract_456")
        
        XCTAssertNotNil(info.pointee.document_type)
        let typeString = String(cString: info.pointee.document_type)
        XCTAssertEqual(typeString, "message")
        
        XCTAssertEqual(info.pointee.revision, 1)
        XCTAssertEqual(info.pointee.created_at, 1640000000000)
        XCTAssertEqual(info.pointee.updated_at, 1640000001000)
    }
    
    func testDocumentGetInfoNullHandle() {
        let info = swift_dash_document_get_info(nil)
        XCTAssertNil(info)
    }
    
    // MARK: - Put to Platform Tests
    
    func testDocumentPutToPlatform() {
        let document = swift_dash_document_create(
            sdk, contract, "test_identity_123", "message",
            """
            {
                "content": "Test message",
                "timestamp": 1640000000000
            }
            """
        )
        XCTAssertNotNil(document)
        
        guard let document = document else { return }
        
        var settings = swift_dash_put_settings_default()
        settings.timeout_ms = 60000
        
        let result = swift_dash_document_put_to_platform(
            sdk, document, 0, signer, &settings
        )
        
        XCTAssertNotNil(result)
        
        guard let result = result else { return }
        defer { swift_dash_binary_data_free(result) }
        
        // Verify binary data
        XCTAssertGreaterThan(result.pointee.len, 0)
        XCTAssertNotNil(result.pointee.data)
        XCTAssertEqual(result.pointee.len, 256) // Mock returns 256 bytes
    }
    
    func testDocumentPutToPlatformNullSafety() {
        var settings = swift_dash_put_settings_default()
        
        // Test null SDK
        var result = swift_dash_document_put_to_platform(
            nil, nil, 0, signer, &settings
        )
        XCTAssertNil(result)
        
        // Test null document
        result = swift_dash_document_put_to_platform(
            sdk, nil, 0, signer, &settings
        )
        XCTAssertNil(result)
        
        // Test null signer
        let document = swift_dash_document_fetch(
            sdk, contract, "message", "test_doc_789"
        )
        result = swift_dash_document_put_to_platform(
            sdk, document, 0, nil, &settings
        )
        XCTAssertNil(result)
    }
    
    // MARK: - Complex Document Tests
    
    func testCreateComplexDocument() {
        let ownerId = "test_identity_123"
        
        // Create a more complex document with nested data
        let complexData = """
        {
            "title": "My Blog Post",
            "content": "This is a detailed blog post about Dash Platform",
            "author": {
                "name": "John Doe",
                "id": "author_identity_123"
            },
            "metadata": {
                "tags": ["blockchain", "dash", "decentralized"],
                "category": "Technology",
                "readTime": 5
            },
            "stats": {
                "views": 1000,
                "likes": 50,
                "shares": 10
            },
            "published": true,
            "publishedAt": 1640000000000
        }
        """
        
        let document = swift_dash_document_create(
            sdk, contract, ownerId, "blog_post", complexData
        )
        
        XCTAssertNotNil(document)
    }
    
    func testCreateDocumentWithArrays() {
        let ownerId = "test_identity_123"
        
        // Document with array fields
        let arrayData = """
        {
            "title": "Shopping List",
            "items": [
                {"name": "Apples", "quantity": 5},
                {"name": "Bananas", "quantity": 3},
                {"name": "Oranges", "quantity": 7}
            ],
            "categories": ["fruits", "groceries"],
            "createdBy": "\(ownerId)",
            "timestamp": 1640000000000
        }
        """
        
        let document = swift_dash_document_create(
            sdk, contract, ownerId, "list", arrayData
        )
        
        XCTAssertNotNil(document)
    }
    
    func testDocumentLifecycle() {
        // Test creating, fetching, and putting a document
        let ownerId = "test_identity_123"
        let documentType = "profile"
        let profileData = """
        {
            "username": "dashuser",
            "displayName": "Dash User",
            "bio": "Enthusiast of decentralized platforms",
            "avatar": "https://example.com/avatar.png",
            "verified": false,
            "joinedAt": 1640000000000
        }
        """
        
        // 1. Create document
        let createdDoc = swift_dash_document_create(
            sdk, contract, ownerId, documentType, profileData
        )
        XCTAssertNotNil(createdDoc)
        
        guard let createdDoc = createdDoc else { return }
        
        // 2. Put to platform
        var settings = swift_dash_put_settings_default()
        settings.timeout_ms = 60000
        
        let putResult = swift_dash_document_put_to_platform(
            sdk, createdDoc, 0, signer, &settings
        )
        XCTAssertNotNil(putResult)
        
        if let putResult = putResult {
            swift_dash_binary_data_free(putResult)
        }
        
        // 3. Fetch document (simulating retrieval after put)
        let fetchedDoc = swift_dash_document_fetch(
            sdk, contract, documentType, "test_doc_789"
        )
        XCTAssertNotNil(fetchedDoc)
        
        // 4. Get info from fetched document
        if let fetchedDoc = fetchedDoc {
            let info = swift_dash_document_get_info(fetchedDoc)
            XCTAssertNotNil(info)
            
            if let info = info {
                swift_dash_document_info_free(info)
            }
        }
    }
    
    func testDocumentBatchOperations() {
        let ownerId = "test_identity_123"
        var settings = swift_dash_put_settings_default()
        settings.timeout_ms = 60000
        
        // Create multiple documents
        let documents = [
            ("message", """
            {"content": "First message", "timestamp": 1640000000000}
            """),
            ("message", """
            {"content": "Second message", "timestamp": 1640000001000}
            """),
            ("message", """
            {"content": "Third message", "timestamp": 1640000002000}
            """)
        ]
        
        var createdDocs: [OpaquePointer] = []
        
        // Create all documents
        for (docType, data) in documents {
            if let doc = swift_dash_document_create(
                sdk, contract, ownerId, docType, data
            ) {
                createdDocs.append(doc)
            }
        }
        
        XCTAssertEqual(createdDocs.count, documents.count)
        
        // Put all documents to platform
        for doc in createdDocs {
            let result = swift_dash_document_put_to_platform(
                sdk, doc, 0, signer, &settings
            )
            XCTAssertNotNil(result)
            
            if let result = result {
                swift_dash_binary_data_free(result)
            }
        }
    }
}