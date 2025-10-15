import Foundation
import SwiftDashSDK

// Example of using the new Data-based fetchBalances API

func exampleFetchBalances(sdk: SDK) async throws {
    // Example 1: Using Data objects directly (recommended for secp256k1 compatibility)
    
    // Create identity IDs as Data objects (32 bytes each)
    let id1 = Data(hexString: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")!
    let id2 = Data(hexString: "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210")!
    
    // Fetch balances using Data objects
    let balances = try sdk.identities.fetchBalances(ids: [id1, id2])
    
    // Process results
    for (idData, balance) in balances {
        let idHex = idData.toHexString()
        if let balance = balance {
            print("Identity \(idHex) has balance: \(balance)")
        } else {
            print("Identity \(idHex) not found")
        }
    }
    
    // Example 2: Using string IDs (convenience method)
    
    let stringIds = [
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
    ]
    
    let dataIds = stringIds.compactMap { Data(hexString: $0) }
    let stringBalances = try sdk.identities.fetchBalances(ids: dataIds)
    
    for (id, balance) in stringBalances {
        if let balance = balance {
            print("Identity \(id) has balance: \(balance)")
        } else {
            print("Identity \(id) not found")
        }
    }
}


// Example with secp256k1 integration
// When using swift-secp256k1, you typically have keys/identifiers as 32-byte arrays
// You can convert them to Data for use with fetchBalances:

func exampleWithSecp256k1() async throws {
    // Assuming you have a secp256k1 public key or identifier
    // let secp256k1Bytes: [UInt8] = [...] // 32 bytes from secp256k1
    
    // Convert to Data
    // let identityData = Data(secp256k1Bytes)
    
    // Use with fetchBalances
    // let balances = try sdk.identities.fetchBalances(ids: [identityData])
}