import Foundation

// This example demonstrates how to use the Swift Dash SDK
// The actual implementation would import the compiled library

class SwiftDashSDKExample {
    
    func runExample() {
        // Initialize the SDK
        swift_dash_sdk_init()
        
        // Create SDK configuration for testnet
        let config = swift_dash_sdk_config_testnet()
        
        // Create SDK instance
        guard let sdk = swift_dash_sdk_create(config) else {
            print("Failed to create SDK instance")
            return
        }
        
        defer {
            // Always clean up SDK when done
            swift_dash_sdk_destroy(sdk)
        }
        
        // Create a test signer for development
        guard let signer = swift_dash_signer_create_test() else {
            print("Failed to create test signer")
            return
        }
        
        defer {
            swift_dash_signer_destroy(signer)
        }
        
        // Example: Working with identities
        identityExample(sdk: sdk, signer: signer)
        
        // Example: Working with data contracts
        dataContractExample(sdk: sdk, signer: signer)
        
        // Example: Working with documents
        documentExample(sdk: sdk, signer: signer)
    }
    
    func identityExample(sdk: OpaquePointer, signer: OpaquePointer) {
        print("\n--- Identity Example ---")
        
        // Fetch an identity by ID
        let identityId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
        
        guard let identity = swift_dash_identity_fetch(sdk, identityId) else {
            print("Failed to fetch identity")
            return
        }
        
        // Get identity information
        if let info = swift_dash_identity_get_info(identity) {
            defer {
                swift_dash_identity_info_free(info)
            }
            
            let idString = String(cString: info.pointee.id)
            print("Identity ID: \(idString)")
            print("Balance: \(info.pointee.balance) credits")
            print("Revision: \(info.pointee.revision)")
            print("Public Keys: \(info.pointee.public_keys_count)")
        }
        
        // Example: Put identity to platform with instant lock
        var settings = swift_dash_put_settings_default()
        settings.timeout_ms = 60000 // 60 seconds
        settings.wait_timeout_ms = 120000 // 2 minutes
        
        if let result = swift_dash_identity_put_to_platform_with_instant_lock(
            sdk, identity, 0, signer, &settings
        ) {
            defer {
                swift_dash_binary_data_free(result)
            }
            
            print("State transition size: \(result.pointee.len) bytes")
            
            // Convert to Data for further processing
            let data = Data(bytes: result.pointee.data, count: result.pointee.len)
            print("State transition created successfully")
        }
        
        // Example: Transfer credits
        let recipientId = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ8ihhL"
        let amount: UInt64 = 1000000 // 1 million credits
        
        if let transferResult = swift_dash_identity_transfer_credits(
            sdk, identity, recipientId, amount, 0, signer, &settings
        ) {
            defer {
                swift_dash_transfer_credits_result_free(transferResult)
            }
            
            print("Transferred \(transferResult.pointee.amount) credits")
            let recipient = String(cString: transferResult.pointee.recipient_id)
            print("To recipient: \(recipient)")
        }
    }
    
    func dataContractExample(sdk: OpaquePointer, signer: OpaquePointer) {
        print("\n--- Data Contract Example ---")
        
        // Create a simple data contract
        let ownerId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
        let contractSchema = """
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
                        "author": {
                            "type": "string"
                        },
                        "timestamp": {
                            "type": "integer"
                        }
                    },
                    "required": ["content", "author", "timestamp"],
                    "additionalProperties": false
                }
            }
        }
        """
        
        guard let contract = swift_dash_data_contract_create(sdk, ownerId, contractSchema) else {
            print("Failed to create data contract")
            return
        }
        
        // Get contract info
        if let infoJson = swift_dash_data_contract_get_info(contract) {
            defer {
                swift_dash_string_free(infoJson)
            }
            
            let info = String(cString: infoJson)
            print("Contract info: \(info)")
        }
        
        // Put contract to platform
        var settings = swift_dash_put_settings_default()
        settings.user_fee_increase = 10 // 10% fee increase for priority
        
        if let result = swift_dash_data_contract_put_to_platform(
            sdk, contract, 0, signer, &settings
        ) {
            defer {
                swift_dash_binary_data_free(result)
            }
            
            print("Data contract state transition created")
            print("Size: \(result.pointee.len) bytes")
        }
    }
    
    func documentExample(sdk: OpaquePointer, signer: OpaquePointer) {
        print("\n--- Document Example ---")
        
        // First, fetch the data contract
        let contractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
        guard let contract = swift_dash_data_contract_fetch(sdk, contractId) else {
            print("Failed to fetch data contract")
            return
        }
        
        // Create a new document
        let ownerId = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ8ihhL"
        let documentType = "message"
        let documentData = """
        {
            "content": "Hello from Swift Dash SDK!",
            "author": "Swift Developer",
            "timestamp": \(Int(Date().timeIntervalSince1970 * 1000))
        }
        """
        
        guard let document = swift_dash_document_create(
            sdk, contract, ownerId, documentType, documentData
        ) else {
            print("Failed to create document")
            return
        }
        
        // Get document info
        if let info = swift_dash_document_get_info(document) {
            defer {
                swift_dash_document_info_free(info)
            }
            
            let docId = String(cString: info.pointee.id)
            let docType = String(cString: info.pointee.document_type)
            print("Document ID: \(docId)")
            print("Document Type: \(docType)")
            print("Revision: \(info.pointee.revision)")
        }
        
        // Put document to platform and wait for confirmation
        var settings = swift_dash_put_settings_default()
        settings.retries = 5
        settings.ban_failed_address = true
        
        if let confirmedDoc = swift_dash_document_put_to_platform_and_wait(
            sdk, document, 0, signer, &settings
        ) {
            print("Document successfully published to platform!")
            
            // Get info of confirmed document
            if let confirmedInfo = swift_dash_document_get_info(confirmedDoc) {
                defer {
                    swift_dash_document_info_free(confirmedInfo)
                }
                
                let docId = String(cString: confirmedInfo.pointee.id)
                print("Confirmed document ID: \(docId)")
            }
        }
        
        // Example: Purchase a document
        let docToPurchase = "someDocumentId123"
        if let docToBuy = swift_dash_document_fetch(
            sdk, contract, documentType, docToPurchase
        ) {
            if let purchaseResult = swift_dash_document_purchase_to_platform(
                sdk, docToBuy, 0, signer, &settings
            ) {
                defer {
                    swift_dash_binary_data_free(purchaseResult)
                }
                
                print("Document purchase state transition created")
            }
        }
    }
}

// Helper function to safely free C strings
func swift_dash_string_free(_ string: UnsafeMutablePointer<CChar>?) {
    guard let string = string else { return }
    // This would call the actual C function
    // ios_sdk_string_free(string)
}