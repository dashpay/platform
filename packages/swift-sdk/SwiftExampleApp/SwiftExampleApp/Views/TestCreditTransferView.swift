import SwiftUI
import SwiftDashSDK

struct TestCreditTransferView: View {
    @EnvironmentObject var appState: UnifiedAppState
    @State private var isRunning = false
    @State private var resultMessage = ""
    @State private var isError = false
    
    var body: some View {
        VStack(spacing: 20) {
            Text("Credit Transfer Test")
                .font(.largeTitle)
                .padding()
            
            Text("This will transfer 10,000,000 credits (0.0001 DASH) from the test identity to HccabTZZpMEDAqU4oQFk3PE47kS6jDDmCjoxR88gFttA")
                .multilineTextAlignment(.center)
                .padding()
            
            if isRunning {
                ProgressView("Executing transfer...")
                    .padding()
            }
            
            if !resultMessage.isEmpty {
                Text(resultMessage)
                    .foregroundColor(isError ? .red : .green)
                    .padding()
                    .background(Color.gray.opacity(0.2))
                    .cornerRadius(8)
            }
            
            Button(action: runTransfer) {
                Text("Run Transfer")
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(isRunning ? Color.gray : Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(10)
            }
            .disabled(isRunning)
            .padding()
            
            Spacer()
        }
        .navigationTitle("Test Transfer")
    }
    
    private func runTransfer() {
        Task {
            await executeTransfer()
        }
    }
    
    @MainActor
    private func executeTransfer() async {
        isRunning = true
        resultMessage = ""
        isError = false
        
        defer {
            isRunning = false
        }
        
        // Load credentials from .env
        EnvLoader.loadEnvFile()
        
        guard let testIdentityId = EnvLoader.get("TEST_IDENTITY_ID"),
              let key3WIF = EnvLoader.get("TEST_KEY_3_PRIVATE") else {
            resultMessage = "Error: Missing test credentials in .env file"
            isError = true
            return
        }
        
        // Decode private key
        let privateKey: Data
        do {
            privateKey = try decodeWIFPrivateKey(key3WIF)
        } catch {
            resultMessage = "Error decoding private key: \(error)"
            isError = true
            return
        }
        
        guard let sdk = appState.platformState.sdk else {
            resultMessage = "Error: SDK not initialized"
            isError = true
            return
        }
        
        // Transfer parameters
        let recipientId = "HccabTZZpMEDAqU4oQFk3PE47kS6jDDmCjoxR88gFttA"
        let amount: UInt64 = 10_000_000 // 0.0001 DASH (10M credits = 10K duffs = 0.0001 DASH)
        
        do {
            // Check balance first
            let identity = try await sdk.identityGet(identityId: testIdentityId)
            if let balance = identity["balance"] as? UInt64 {
                let dashAmount = Double(balance) / 100_000_000_000 // 1 DASH = 100B credits
                print("Current balance: \(balance) credits (\(dashAmount) DASH)")
            }
            
            // Execute transfer
            let (senderBalance, receiverBalance) = try await sdk.identityTransferCredits(
                fromIdentityId: testIdentityId,
                toIdentityId: recipientId,
                amount: amount,
                signerPrivateKey: privateKey
            )
            
            resultMessage = """
            ✅ Transfer successful!
            
            Sender new balance: \(senderBalance) credits
            Receiver new balance: \(receiverBalance) credits
            """
            
        } catch {
            resultMessage = "❌ Transfer failed: \(error)"
            isError = true
        }
    }
    
    private func decodeWIFPrivateKey(_ wif: String) throws -> Data {
        // Base58 alphabet
        let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
        var result = Data()
        var multi = Data([0])
        
        for char in wif {
            guard let index = alphabet.firstIndex(of: char) else {
                throw NSError(domain: "Invalid base58 character", code: 1)
            }
            
            // Multiply by 58
            var carry = 0
            for i in 0..<multi.count {
                carry += Int(multi[i]) * 58
                multi[i] = UInt8(carry % 256)
                carry /= 256
            }
            while carry > 0 {
                multi.append(UInt8(carry % 256))
                carry /= 256
            }
            
            // Add index
            carry = alphabet.distance(from: alphabet.startIndex, to: index)
            for i in 0..<multi.count {
                carry += Int(multi[i])
                multi[i] = UInt8(carry % 256)
                carry /= 256
            }
            while carry > 0 {
                multi.append(UInt8(carry % 256))
                carry /= 256
            }
        }
        
        // Skip leading zeros
        for char in wif {
            if char != "1" { break }
            result.append(0)
        }
        
        // Append in reverse
        for byte in multi.reversed() {
            if result.count > 0 || byte != 0 {
                result.append(byte)
            }
        }
        
        // Extract private key (skip version byte and checksum)
        guard result.count >= 37 else {
            throw NSError(domain: "Invalid WIF format", code: 2)
        }
        
        return Data(result[1..<33])
    }
}