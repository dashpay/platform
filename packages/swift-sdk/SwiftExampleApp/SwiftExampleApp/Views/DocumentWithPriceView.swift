import SwiftUI
import SwiftDashSDK

struct DocumentWithPriceView: View {
    @Binding var documentId: String
    let contractId: String
    let documentType: String
    let currentIdentityId: String? // Pass from parent to check ownership
    
    @EnvironmentObject var appState: UnifiedAppState
    @State private var isLoading = false
    @State private var documentPrice: UInt64?
    @State private var documentExists = false
    @State private var errorMessage: String?
    @State private var fetchedDocument: [String: Any]?
    @State private var debounceTimer: Timer?
    @State private var isOwnedByCurrentIdentity = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Document ID Input
            HStack {
                TextField("Enter document ID", text: $documentId)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .onChange(of: documentId) { newValue in
                        handleDocumentIdChange(newValue)
                    }
                
                if isLoading {
                    ProgressView()
                        .scaleEffect(0.8)
                }
            }
            
            // Status/Price Display
            if let error = errorMessage {
                HStack {
                    Image(systemName: "exclamationmark.circle.fill")
                        .foregroundColor(.red)
                    Text(error)
                        .font(.caption)
                        .foregroundColor(.red)
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(Color.red.opacity(0.1))
                .cornerRadius(6)
            } else if documentExists {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundColor(.green)
                        Text("Document found")
                            .font(.caption)
                            .foregroundColor(.green)
                    }
                    
                    if isOwnedByCurrentIdentity {
                        // Show ownership message
                        HStack {
                            Image(systemName: "person.fill.checkmark")
                                .foregroundColor(.purple)
                            Text("You are already the owner")
                                .font(.caption)
                                .foregroundColor(.purple)
                        }
                        .padding()
                        .background(Color.purple.opacity(0.1))
                        .cornerRadius(8)
                    } else if let price = documentPrice {
                        HStack {
                            Label("Price", systemImage: "tag.fill")
                                .font(.subheadline)
                                .foregroundColor(.blue)
                            Spacer()
                            Text(formatPrice(price))
                                .font(.headline)
                                .foregroundColor(.blue)
                        }
                        .padding()
                        .background(Color.blue.opacity(0.1))
                        .cornerRadius(8)
                    } else {
                        HStack {
                            Image(systemName: "xmark.circle")
                                .foregroundColor(.orange)
                            Text("This document is not for sale")
                                .font(.caption)
                                .foregroundColor(.orange)
                        }
                        .padding()
                        .background(Color.orange.opacity(0.1))
                        .cornerRadius(8)
                    }
                    
                    // Show document owner if available
                    if let doc = fetchedDocument,
                       let ownerId = (doc["$ownerId"] ?? doc["ownerId"]) as? String {
                        HStack {
                            Text("Owner:")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(String(ownerId.prefix(16)) + "...")
                                .font(.caption)
                                .font(.system(.caption, design: .monospaced))
                                .foregroundColor(.secondary)
                        }
                    }
                }
            }
            
            // Help text
            if !documentExists && errorMessage == nil && !documentId.isEmpty && !isLoading {
                Text("Enter a valid document ID to see its price")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
    }
    
    private func handleDocumentIdChange(_ newValue: String) {
        // Cancel previous timer
        debounceTimer?.invalidate()
        
        // Reset state
        documentExists = false
        documentPrice = nil
        fetchedDocument = nil
        errorMessage = nil
        isOwnedByCurrentIdentity = false
        
        // Also reset the app state
        appState.transitionState.canPurchaseDocument = false
        appState.transitionState.documentPrice = nil
        appState.transitionState.documentPurchaseError = nil
        
        // Only proceed if we have all required fields
        guard !newValue.isEmpty,
              !contractId.isEmpty,
              !documentType.isEmpty else {
            if !newValue.isEmpty {
                errorMessage = "Please select a contract and document type first"
            }
            return
        }
        
        // Validate document ID format (should be base58 or hex)
        guard isValidDocumentId(newValue) else {
            errorMessage = "Invalid document ID format"
            // Make sure to reset purchase state for invalid IDs
            appState.transitionState.canPurchaseDocument = false
            appState.transitionState.documentPrice = nil
            return
        }
        
        // Set up debounce timer to fetch after user stops typing
        debounceTimer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: false) { _ in
            Task {
                await fetchDocument()
            }
        }
    }
    
    private func isValidDocumentId(_ id: String) -> Bool {
        // Check if it's a valid base58 string (32 bytes when decoded)
        if let data = Data.identifier(fromBase58: id) {
            return data.count == 32
        }
        
        // Check if it's a valid hex string (64 characters)
        if id.count == 64 {
            return id.allSatisfy { $0.isHexDigit }
        }
        
        return false
    }
    
    @MainActor
    private func fetchDocument() async {
        isLoading = true
        defer { isLoading = false }
        
        guard let sdk = appState.sdk else {
            errorMessage = "SDK not initialized"
            return
        }
        
        do {
            // Normalize document ID to base58
            let normalizedId = normalizeDocumentId(documentId)
            
            // Fetch the document
            let document = try await sdk.documentGet(
                dataContractId: contractId,
                documentType: documentType,
                documentId: normalizedId
            )
            
            // Document exists
            documentExists = true
            fetchedDocument = document
            
            // Debug: Log the entire document to see what fields are available
            print("DEBUG: Document fetched successfully")
            print("DEBUG: Document keys: \(document.keys)")
            for (key, value) in document {
                print("DEBUG: \(key) = \(value) (type: \(type(of: value)))")
                // If it's a dictionary, log its contents too
                if let dict = value as? [String: Any] {
                    print("DEBUG:   \(key) contents:")
                    for (subKey, subValue) in dict {
                        print("DEBUG:     \(subKey) = \(subValue) (type: \(type(of: subValue)))")
                    }
                }
            }
            
            // Check ownership (try both with and without $ prefix)
            let ownerId = (document["$ownerId"] ?? document["ownerId"]) as? String
            if let ownerId = ownerId,
               let currentId = currentIdentityId {
                isOwnedByCurrentIdentity = (ownerId == currentId)
                print("DEBUG: Owner ID: \(ownerId), Current ID: \(currentId), Is owner: \(isOwnedByCurrentIdentity)")
            } else {
                isOwnedByCurrentIdentity = false
            }
            
            // Check for price field - it might be in a 'data' subdictionary
            var priceValue: Any? = nil
            
            // First try to get price from data field
            if let data = document["data"] as? [String: Any] {
                priceValue = data["$price"]
                print("DEBUG: Found data field, checking for $price: \(priceValue != nil)")
            }
            
            // Fallback to checking root level (in case SDK structure varies)
            if priceValue == nil {
                priceValue = document["$price"]
            }
            
            if let priceValue = priceValue {
                print("DEBUG: Found price value: \(priceValue) (type: \(type(of: priceValue)))")
                
                if let priceNum = priceValue as? NSNumber {
                    documentPrice = priceNum.uint64Value
                    print("DEBUG: Price as NSNumber: \(documentPrice!)")
                } else if let priceString = priceValue as? String,
                          let price = UInt64(priceString) {
                    documentPrice = price
                    print("DEBUG: Price as String: \(documentPrice!)")
                } else if let priceInt = priceValue as? Int {
                    documentPrice = UInt64(priceInt)
                    print("DEBUG: Price as Int: \(documentPrice!)")
                } else if let priceUInt = priceValue as? UInt64 {
                    documentPrice = priceUInt
                    print("DEBUG: Price as UInt64: \(documentPrice!)")
                } else {
                    print("DEBUG: Could not convert price value to UInt64")
                }
            } else {
                // Document exists but has no price set
                print("DEBUG: No price field found in document")
                documentPrice = nil
            }
            
            // Update transition state on main thread
            await MainActor.run {
                appState.transitionState.documentPrice = documentPrice
                
                // Determine if document can be purchased
                if isOwnedByCurrentIdentity {
                    appState.transitionState.canPurchaseDocument = false
                    appState.transitionState.documentPurchaseError = "You already own this document"
                    print("DEBUG: Cannot purchase - already owned")
                } else if documentPrice == nil || documentPrice == 0 {
                    appState.transitionState.canPurchaseDocument = false
                    appState.transitionState.documentPurchaseError = "This document is not for sale"
                    print("DEBUG: Cannot purchase - no price or price is 0. Price: \(String(describing: documentPrice))")
                } else {
                    appState.transitionState.canPurchaseDocument = true
                    appState.transitionState.documentPurchaseError = nil
                    print("DEBUG: Can purchase! Price: \(documentPrice!), canPurchase: \(appState.transitionState.canPurchaseDocument)")
                    
                    // Force the TransitionDetailView to update its button state
                    // by triggering an objectWillChange on the main app state
                    appState.objectWillChange.send()
                }
            }
            
        } catch {
            // Check if it's a not found error
            if error.localizedDescription.contains("not found") ||
               error.localizedDescription.contains("does not exist") {
                errorMessage = "Document not found"
            } else {
                errorMessage = "Error: \(error.localizedDescription)"
            }
            documentExists = false
            documentPrice = nil
            
            // Clear transition state when document fetch fails
            appState.transitionState.documentPrice = nil
            appState.transitionState.canPurchaseDocument = false
            appState.transitionState.documentPurchaseError = nil
        }
    }
    
    private func normalizeDocumentId(_ id: String) -> String {
        // If it's already base58, return as is
        if Data.identifier(fromBase58: id) != nil {
            return id
        }
        
        // If it's hex, convert to base58
        if let data = Data(hexString: id), data.count == 32 {
            return data.toBase58String()
        }
        
        return id
    }
    
    private func formatPrice(_ credits: UInt64) -> String {
        let dashAmount = Double(credits) / 100_000_000_000 // 1 DASH = 100B credits
        
        if dashAmount < 0.00001 {
            return "\(credits) credits"
        } else {
            return String(format: "%.8f DASH", dashAmount)
        }
    }
}

// Extension to check if character is hex digit
extension Character {
    var isHexDigit: Bool {
        return "0123456789abcdefABCDEF".contains(self)
    }
}