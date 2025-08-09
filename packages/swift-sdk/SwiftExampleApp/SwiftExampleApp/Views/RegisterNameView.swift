import SwiftUI
import SwiftDashSDK

struct RegisterNameView: View {
    let identity: IdentityModel
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    
    @State private var username = ""
    @State private var isChecking = false
    @State private var isAvailable: Bool? = nil
    @State private var isContested = false
    @State private var errorMessage = ""
    @State private var showingError = false
    @State private var checkTimer: Timer? = nil
    @State private var lastCheckedName = ""
    @State private var isRegistering = false
    @State private var registrationSuccess = false
    
    private var normalizedUsername: String {
        // Use the FFI function to normalize the username
        let trimmed = username.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return "" }
        
        return trimmed.withCString { namePtr in
            let result = dash_sdk_dpns_normalize_username(namePtr)
            defer { 
                if let error = result.error {
                    dash_sdk_error_free(error)
                }
                if let dataPtr = result.data {
                    dash_sdk_string_free(dataPtr.assumingMemoryBound(to: CChar.self))
                }
            }
            
            if result.error == nil, let dataPtr = result.data {
                return String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
            }
            return trimmed.lowercased() // Fallback to simple lowercasing
        }
    }
    
    private enum ValidationStatus {
        case valid
        case notLongEnough
        case tooLong
        case invalidCharacters
        case invalidHyphenPlacement
    }
    
    private var validationStatus: ValidationStatus {
        let name = normalizedUsername
        
        // Check basic length first
        if name.count < 3 {
            return .notLongEnough
        }
        if name.count > 63 {
            return .tooLong
        }
        
        // Use FFI function to validate
        let isValid = name.withCString { namePtr in
            let result = dash_sdk_dpns_is_valid_username(namePtr)
            return result == 1
        }
        
        if isValid {
            return .valid
        }
        
        // If not valid, determine the specific reason
        // Check for invalid characters
        let validCharsPattern = "^[a-z0-9-]+$"
        let validCharsRegex = try? NSRegularExpression(pattern: validCharsPattern, options: [])
        let range = NSRange(location: 0, length: name.utf16.count)
        if validCharsRegex?.firstMatch(in: name, options: [], range: range) == nil {
            return .invalidCharacters
        }
        
        // Check hyphen rules
        if name.hasPrefix("-") || name.hasSuffix("-") || name.contains("--") {
            return .invalidHyphenPlacement
        }
        
        return .invalidCharacters // Default for any other invalid case
    }
    
    private var isValidUsername: Bool {
        // Use the FFI function directly
        guard !normalizedUsername.isEmpty else { return false }
        
        return normalizedUsername.withCString { namePtr in
            let result = dash_sdk_dpns_is_valid_username(namePtr)
            return result == 1
        }
    }
    
    private var validationMessage: String {
        // Use the FFI function to get validation message
        guard !normalizedUsername.isEmpty else { return "" }
        
        return normalizedUsername.withCString { namePtr in
            let result = dash_sdk_dpns_get_validation_message(namePtr)
            defer { 
                if let error = result.error {
                    dash_sdk_error_free(error)
                }
                if let dataPtr = result.data {
                    dash_sdk_string_free(dataPtr.assumingMemoryBound(to: CChar.self))
                }
            }
            
            if result.error == nil, let dataPtr = result.data {
                let message = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
                return message == "valid" ? "" : message
            }
            
            // Fallback to our own messages
            switch validationStatus {
            case .valid:
                return ""
            case .notLongEnough:
                return "Name must be at least 3 characters long"
            case .tooLong:
                return "Name must be 63 characters or less"
            case .invalidCharacters:
                return "Name can only contain letters, numbers, and hyphens"
            case .invalidHyphenPlacement:
                return "Hyphens cannot be at the start/end or consecutive"
            }
        }
    }
    
    private var isNameContested: Bool {
        // Only check if name is valid
        guard isValidUsername else { return false }
        
        // Use the FFI function to check if the name is contested
        return normalizedUsername.withCString { namePtr in
            let result = dash_sdk_dpns_is_contested_username(namePtr)
            return result == 1
        }
    }
    
    var body: some View {
        NavigationView {
            Form {
                Section("Choose Your Username") {
                    TextField("Enter username", text: $username)
                        .textContentType(.username)
                        .autocapitalization(.none)
                        .autocorrectionDisabled(true)
                        .onChange(of: username) { _ in
                            // Cancel any existing timer
                            checkTimer?.invalidate()
                            
                            // Reset availability if name changed
                            if normalizedUsername != lastCheckedName {
                                isAvailable = nil
                                isChecking = false
                            }
                            
                            errorMessage = ""
                            // Update contested status
                            isContested = isNameContested
                            
                            // Start new timer if name is valid
                            if isValidUsername && normalizedUsername != lastCheckedName {
                                checkTimer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: false) { _ in
                                    Task {
                                        await checkAvailabilityAutomatically()
                                    }
                                }
                            }
                        }
                    
                    if !normalizedUsername.isEmpty {
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text("Normalized: ")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                                Text("\(normalizedUsername).dash")
                                    .font(.caption)
                                    .foregroundColor(.blue)
                            }
                            
                            // Show validation status
                            if validationStatus != .valid {
                                HStack {
                                    Image(systemName: "exclamationmark.circle.fill")
                                        .foregroundColor(.red)
                                        .font(.caption)
                                    Text(validationMessage)
                                        .font(.caption)
                                        .foregroundColor(.red)
                                }
                            }
                        }
                    }
                }
                
                Section("Name Information") {
                    HStack {
                        Text("Validity")
                        Spacer()
                        if !normalizedUsername.isEmpty {
                            switch validationStatus {
                            case .valid:
                                Label("Valid", systemImage: "checkmark.circle.fill")
                                    .foregroundColor(.green)
                            case .notLongEnough:
                                Label("Not Long Enough", systemImage: "xmark.circle.fill")
                                    .foregroundColor(.red)
                            case .tooLong:
                                Label("Too Long", systemImage: "xmark.circle.fill")
                                    .foregroundColor(.red)
                            case .invalidCharacters, .invalidHyphenPlacement:
                                Label("Not Valid", systemImage: "xmark.circle.fill")
                                    .foregroundColor(.red)
                            }
                        } else {
                            Text("Enter a name")
                                .foregroundColor(.secondary)
                        }
                    }
                    
                    if isValidUsername {
                        HStack {
                            Text("Availability")
                            Spacer()
                            if isChecking {
                                ProgressView()
                                    .scaleEffect(0.8)
                            } else if let available = isAvailable {
                                if available {
                                    Label("Available", systemImage: "checkmark.circle.fill")
                                        .foregroundColor(.green)
                                } else {
                                    Label("Taken", systemImage: "xmark.circle.fill")
                                        .foregroundColor(.red)
                                }
                            } else {
                                Text("Not checked")
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                    
                    HStack {
                        Text("Contest Status")
                        Spacer()
                        if isContested {
                            Label("Contested", systemImage: "flag.fill")
                                .foregroundColor(.orange)
                        } else {
                            Label("Regular", systemImage: "checkmark.circle")
                                .foregroundColor(.green)
                        }
                    }
                }
                
                if isContested && !normalizedUsername.isEmpty {
                    Section("Contest Warning") {
                        HStack {
                            Image(systemName: "exclamationmark.triangle.fill")
                                .foregroundColor(.orange)
                            VStack(alignment: .leading, spacing: 4) {
                                Text("Contested Name")
                                    .font(.headline)
                                    .foregroundColor(.orange)
                                Text("This name is less than 20 characters with only letters (a-z, A-Z), digits (0, 1), and hyphens. It requires a masternode vote contest to register.")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                        .padding(.vertical, 4)
                    }
                }
                
                Section {
                    Button(action: registerName) {
                        HStack {
                            if isRegistering {
                                ProgressView()
                                    .progressViewStyle(CircularProgressViewStyle(tint: .white))
                                    .scaleEffect(0.8)
                                Text("Registering...")
                            } else {
                                Image(systemName: "plus.circle.fill")
                                Text("Register Name")
                            }
                        }
                        .foregroundColor(.white)
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(isValidUsername && isAvailable == true && !isRegistering ? Color.blue : Color.gray)
                        .cornerRadius(10)
                    }
                    .disabled(!isValidUsername || isAvailable != true || isRegistering)
                    .listRowInsets(EdgeInsets())
                    .listRowBackground(Color.clear)
                }
            }
            .navigationTitle("Register DPNS Name")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
            }
            .alert("Error", isPresented: $showingError) {
                Button("OK") { }
            } message: {
                Text(errorMessage)
            }
            .onDisappear {
                // Clean up timer when view disappears
                checkTimer?.invalidate()
                checkTimer = nil
            }
        }
    }
    
    private func checkAvailabilityAutomatically() async {
        // Store the name we're checking
        lastCheckedName = normalizedUsername
        
        // Start showing the checking indicator
        await MainActor.run {
            isChecking = true
        }
        
        // Use the SDK to check availability
        guard let sdk = appState.sdk else {
            await MainActor.run {
                errorMessage = "SDK not initialized"
                showingError = true
                isChecking = false
            }
            return
        }
        
        do {
            let available = try await sdk.dpnsCheckAvailability(name: normalizedUsername)
            
            await MainActor.run {
                isAvailable = available
                isChecking = false
                if !available {
                    errorMessage = "This name is already registered"
                }
            }
        } catch {
            await MainActor.run {
                // If we get an error, assume unavailable
                isAvailable = false
                isChecking = false
                errorMessage = "Failed to check availability: \(error.localizedDescription)"
                // Don't show error alert for automatic checks
            }
        }
    }
    
    private func registerName() {
        guard let sdk = appState.sdk,
              let handle = sdk.handle else {
            errorMessage = "SDK not initialized"
            showingError = true
            return
        }
        
        // Find a suitable authentication key with a private key available
        // DPNS registration requires HIGH or CRITICAL security level authentication keys
        var selectedKey: IdentityPublicKey? = nil
        var privateKeyData: Data? = nil
        
        // Try to find a suitable authentication key with private key
        for publicKey in identity.publicKeys {
            // Check if this is an authentication key with proper security level
            if publicKey.purpose == .authentication && 
               (publicKey.securityLevel == .high || publicKey.securityLevel == .critical) {
                // Try to retrieve the private key from keychain
                if let keyData = KeychainManager.shared.retrievePrivateKey(
                    identityId: identity.id,
                    keyIndex: Int32(publicKey.id)
                ) {
                    selectedKey = publicKey
                    privateKeyData = keyData
                    print("✅ Found private key for authentication key #\(publicKey.id) with security level: \(publicKey.securityLevel)")
                    break
                }
            }
        }
        
        guard let privateKey = privateKeyData,
              let publicKey = selectedKey else {
            errorMessage = "No HIGH or CRITICAL security authentication key with private key available. DPNS registration requires a HIGH or CRITICAL security level authentication key."
            showingError = true
            return
        }
        
        isRegistering = true
        
        Task {
            do {
                // Create identity handle from components
                let identityHandle = identity.id.withUnsafeBytes { idBytes in
                    // Create public keys array
                    var pubKeys: [DashSDKPublicKeyData] = []
                    for key in identity.publicKeys {
                        // Get the raw key data
                        let keyData = key.data
                        keyData.withUnsafeBytes { keyBytes in
                            let keyStruct = DashSDKPublicKeyData(
                                id: UInt8(key.id),
                                purpose: key.purpose.rawValue,
                                security_level: key.securityLevel.rawValue,
                                key_type: key.keyType.rawValue,
                                read_only: key.readOnly,
                                data: keyBytes.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                data_len: UInt(keyBytes.count),
                                disabled_at: key.disabledAt ?? 0
                            )
                            pubKeys.append(keyStruct)
                        }
                    }
                    
                    return pubKeys.withUnsafeBufferPointer { keysPtr in
                        dash_sdk_identity_create_from_components(
                            idBytes.baseAddress?.assumingMemoryBound(to: UInt8.self),
                            keysPtr.baseAddress,
                            UInt(keysPtr.count),
                            identity.balance,
                            0  // revision
                        )
                    }
                }
                
                guard identityHandle.error == nil,
                      let identityPtr = identityHandle.data else {
                    if let error = identityHandle.error {
                        let errorMsg = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Failed to create identity"
                        dash_sdk_error_free(error)
                        throw SDKError.internalError(errorMsg)
                    }
                    throw SDKError.internalError("Failed to create identity from components")
                }
                
                let identityOpaquePtr = OpaquePointer(identityPtr)
                defer {
                    // Clean up identity - need to find the destroy function
                    // dash_sdk_identity_destroy(identityOpaquePtr)
                }
                
                // Create public key handle
                let publicKeyHandle = publicKey.data.withUnsafeBytes { keyBytes in
                    dash_sdk_identity_public_key_create_from_data(
                        UInt32(publicKey.id),
                        publicKey.keyType.rawValue,
                        publicKey.purpose.rawValue,
                        publicKey.securityLevel.rawValue,
                        keyBytes.baseAddress?.assumingMemoryBound(to: UInt8.self),
                        UInt(keyBytes.count),
                        publicKey.readOnly,
                        publicKey.disabledAt ?? 0
                    )
                }
                
                guard publicKeyHandle.error == nil,
                      let publicKeyPtr = publicKeyHandle.data else {
                    if let error = publicKeyHandle.error {
                        let errorMsg = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Failed to create public key"
                        dash_sdk_error_free(error)
                        throw SDKError.internalError(errorMsg)
                    }
                    throw SDKError.internalError("Failed to create public key from data")
                }
                
                let publicKeyOpaquePtr = OpaquePointer(publicKeyPtr)
                defer {
                    dash_sdk_identity_public_key_destroy(publicKeyOpaquePtr)
                }
                
                // Create signer from private key
                let signerResult = privateKey.withUnsafeBytes { bytes in
                    dash_sdk_signer_create_from_private_key(
                        bytes.baseAddress?.assumingMemoryBound(to: UInt8.self),
                        UInt(privateKey.count)
                    )
                }
                
                guard signerResult.error == nil,
                      let signerData = signerResult.data else {
                    if let error = signerResult.error {
                        let errorMsg = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Failed to create signer"
                        dash_sdk_error_free(error)
                        throw SDKError.internalError(errorMsg)
                    }
                    throw SDKError.internalError("Failed to create signer")
                }
                
                let signerHandle = OpaquePointer(signerData)
                defer {
                    dash_sdk_signer_destroy(signerHandle)
                }
                
                // Register the DPNS name
                let result = normalizedUsername.withCString { namePtr in
                    dash_sdk_dpns_register_name(
                        handle,
                        namePtr,
                        UnsafeRawPointer(identityOpaquePtr),
                        UnsafeRawPointer(publicKeyOpaquePtr),
                        UnsafeRawPointer(signerHandle)
                    )
                }
                
                // Handle the result
                if let error = result.error {
                    let errorMsg = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Registration failed"
                    dash_sdk_error_free(error)
                    throw SDKError.internalError(errorMsg)
                }
                
                guard let dataPtr = result.data else {
                    throw SDKError.internalError("No registration result returned")
                }
                
                // The result contains the registration info
                let registrationResult = dataPtr.assumingMemoryBound(to: DpnsRegistrationResult.self)
                defer {
                    dash_sdk_dpns_registration_result_free(registrationResult)
                }
                
                // Success! Update the identity with the new DPNS name
                let registeredName = "\(normalizedUsername).dash"
                
                await MainActor.run {
                    // Update the identity's dpnsName
                    if let index = appState.identities.firstIndex(where: { $0.id == identity.id }) {
                        var updatedIdentity = appState.identities[index]
                        updatedIdentity.dpnsName = normalizedUsername // Store just the username part
                        appState.identities[index] = updatedIdentity
                    }
                    
                    registrationSuccess = true
                    errorMessage = "Successfully registered \(registeredName)!"
                    showingError = true
                    isRegistering = false
                }
                
                // Dismiss the view after a short delay
                try? await Task.sleep(nanoseconds: 2_000_000_000)
                await MainActor.run {
                    dismiss()
                }
                
            } catch {
                await MainActor.run {
                    errorMessage = "Registration failed: \(error.localizedDescription)"
                    showingError = true
                    isRegistering = false
                }
            }
        }
    }
}

// Preview
struct RegisterNameView_Previews: PreviewProvider {
    static var previews: some View {
        RegisterNameView(identity: IdentityModel(
            id: Data(repeating: 0, count: 32),
            balance: 1000000,
            isLocal: false
        ))
        .environmentObject(AppState())
    }
}
