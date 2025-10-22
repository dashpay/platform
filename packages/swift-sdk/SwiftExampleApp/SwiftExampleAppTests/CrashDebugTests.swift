import XCTest
import SwiftDashSDK
import DashSDKFFI
@testable import SwiftExampleApp

final class CrashDebugTests: XCTestCase {
    
    func testCatchCrash() async throws {
        print("=== Starting crash debug test ===")
        
        // Install exception handler (without capturing context)
        let handler = NSGetUncaughtExceptionHandler()
        NSSetUncaughtExceptionHandler { exception in
            print("!!! Caught exception: \(exception)")
            print("!!! Reason: \(exception.reason ?? "unknown")")
            print("!!! User info: \(exception.userInfo ?? [:])")
            print("!!! Call stack: \(exception.callStackSymbols)")
        }
        
        defer {
            NSSetUncaughtExceptionHandler(handler)
        }
        
        // Try the problematic code
        do {
            print("Initializing SDK...")
            SDK.initialize()
            
            print("Creating SDK instance...")
            let sdk = try SDK(network: DashSDKNetwork_SDKTestnet)
            
            print("SDK created, checking methods...")
            
            // Try to call the method with minimal setup
            _ = "test" // fromId
            let toId = "test2"
            let amount: UInt64 = 1
            let key = Data(repeating: 0, count: 32)
            
            print("Creating identity and signer...")
            
            // Create a dummy identity
            let identity = DPPIdentity(
                id: Data(repeating: 0, count: 32),
                publicKeys: [:],
                balance: 0,
                revision: 0
            )
            
            // Create signer from private key
            let signerResult = key.withUnsafeBytes { keyBytes in
                dash_sdk_signer_create_from_private_key(
                    keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                    UInt(key.count)
                )
            }
            
            guard signerResult.error == nil,
                  let signer = signerResult.data else {
                print("Failed to create signer")
                return
            }
            
            defer {
                dash_sdk_signer_destroy(OpaquePointer(signer)!)
            }
            
            print("Calling transferCredits...")
            _ = try await sdk.transferCredits(
                from: identity,
                toIdentityId: toId,
                amount: amount,
                signer: OpaquePointer(signer)!
            )
            
            print("Method call completed")
        } catch {
            print("Caught error: \(error)")
            print("Error type: \(type(of: error))")
            print("Error localized: \(error.localizedDescription)")
            
            let nsError = error as NSError
            print("NSError domain: \(nsError.domain)")
            print("NSError code: \(nsError.code)")
            print("NSError userInfo: \(nsError.userInfo)")
        }
        
        print("=== Crash debug test completed ===")
    }
    
    func testMethodExistence() {
        print("=== Testing method existence ===")
        
        // Check if the SDK has the method we're trying to call
        let sdkClass: AnyClass? = NSClassFromString("SwiftDashSDK.SDK")
        print("SDK class: \(String(describing: sdkClass))")
        
        if let cls = sdkClass {
            // List all methods
            var methodCount: UInt32 = 0
            let methods = class_copyMethodList(cls, &methodCount)
            
            print("Found \(methodCount) methods in SDK class:")
            if let methods = methods {
                for i in 0..<Int(methodCount) {
                    let method = methods[i]
                    let selector = method_getName(method)
                    let name = NSStringFromSelector(selector)
                    if name.contains("identity") || name.contains("transfer") || name.contains("credit") {
                        print("  - \(name)")
                    }
                }
                free(methods)
            }
        }
        
        print("=== Method existence test completed ===")
    }
}
