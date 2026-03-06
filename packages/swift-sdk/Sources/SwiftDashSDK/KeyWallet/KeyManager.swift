import Foundation
import DashSDKFFI

// MARK: - Key Manager Errors

/// Errors that can occur during key management operations
public enum KeyManagerError: LocalizedError, Sendable {
  case keyNotFound(String)
  case privateKeyNotFound(KeyID)
  case invalidKeyFormat(String)
  case signerCreationFailed(String)
  case keychainError(String)
  case noSuitableKey(String)

  public var errorDescription: String? {
    switch self {
    case .keyNotFound(let message):
      return "Key not found: \(message)"
    case .privateKeyNotFound(let keyId):
      return "Private key not found for key ID \(keyId). Please add the private key first."
    case .invalidKeyFormat(let message):
      return "Invalid key format: \(message)"
    case .signerCreationFailed(let message):
      return "Failed to create signer: \(message)"
    case .keychainError(let message):
      return "Keychain error: \(message)"
    case .noSuitableKey(let message):
      return "No suitable key found: \(message)"
    }
  }
}

// MARK: - Key Manager

/// Centralized key management for Dash Platform identities.
///
/// This class provides a unified interface for:
/// - Finding keys by purpose (transfer, authentication, etc.)
/// - Retrieving private keys from secure storage
/// - Creating signers for SDK operations
/// - Validating keys
///
/// Example usage:
/// ```swift
/// let keyManager = KeyManager(keychainManager: KeychainManager.shared)
///
/// // Get transfer key for an identity
/// if let transferKey = try keyManager.getTransferKey(for: identity) {
///     // Create signer for transfer operation
///     let signer = try keyManager.createSigner(
///         for: identity,
///         keyIndex: transferKey.id
///     )
///     defer { keyManager.destroySigner(signer) }
///     // Use signer...
/// }
/// ```
public final class KeyManager: Sendable {

  /// The keychain manager used for private key storage/retrieval
  private let keychainManager: KeychainManager

  /// Initialize with a keychain manager
  /// - Parameter keychainManager: The keychain manager to use
  /// - Note: If you want to use the shared instance, pass `KeychainManager.shared` explicitly
  public init(keychainManager: KeychainManager) {
    self.keychainManager = keychainManager
  }

  /// Initialize with the shared keychain manager (convenience)
  /// - Note: This must be called from a MainActor context since KeychainManager.shared is @MainActor
  @MainActor
  public static func withSharedKeychain() -> KeyManager {
    return KeyManager(keychainManager: KeychainManager.shared)
  }

  // MARK: - Key Selection

  /// Find a transfer key for an identity
  /// - Parameter identity: The identity to find a transfer key for
  /// - Returns: A transfer key if found, nil otherwise
  /// - Note: This only returns the public key. Use `getPrivateKey(for:keyIndex:from:)` to check if private key is available.
  public func getTransferKey(for identity: DPPIdentity) -> IdentityPublicKey? {
    // Prefer critical transfer key, then any transfer key
    if let criticalKey = identity.publicKeys.values.first(where: {
      $0.purpose == .transfer && $0.securityLevel == .critical && !$0.isDisabled
    }) {
      return criticalKey
    }

    return identity.publicKeys.values.first(where: {
      $0.purpose == .transfer && !$0.isDisabled
    })
  }

  /// Find an authentication key for an identity
  /// - Parameter identity: The identity to find an authentication key for
  /// - Returns: An authentication key if found, nil otherwise
  /// - Note: This only returns the public key. Use `getPrivateKey(for:keyIndex:from:)` to check if private key is available.
  public func getAuthenticationKey(for identity: DPPIdentity) -> IdentityPublicKey? {
    // Prefer critical authentication key, then any authentication key
    if let criticalKey = identity.publicKeys.values.first(where: {
      $0.purpose == .authentication && $0.securityLevel == .critical && !$0.isDisabled
    }) {
      return criticalKey
    }

    return identity.publicKeys.values.first(where: {
      $0.purpose == .authentication && !$0.isDisabled
    })
  }

  /// Find a key by purpose for an identity
  /// - Parameters:
  ///   - identity: The identity to find a key for
  ///   - purpose: The key purpose to find
  /// - Returns: A key with the specified purpose if found, nil otherwise
  /// - Note: This only returns the public key. Use `getPrivateKey(for:keyIndex:from:)` to check if private key is available.
  public func getKeyByPurpose(for identity: DPPIdentity, purpose: KeyPurpose) -> IdentityPublicKey? {
    // Prefer critical key, then any key with the purpose
    if let criticalKey = identity.publicKeys.values.first(where: {
      $0.purpose == purpose && $0.securityLevel == .critical && !$0.isDisabled
    }) {
      return criticalKey
    }

    return identity.publicKeys.values.first(where: {
      $0.purpose == purpose && !$0.isDisabled
    })
  }

  /// Find a key that meets specific requirements
  /// - Parameters:
  ///   - identity: The identity to find a key for
  ///   - purpose: Optional key purpose requirement
  ///   - securityLevel: Optional minimum security level requirement
  ///   - preferCritical: Whether to prefer critical keys (default: true)
  /// - Returns: A key meeting the requirements if found, nil otherwise
  public func findKey(
    for identity: DPPIdentity,
    purpose: KeyPurpose? = nil,
    minimumSecurityLevel: SecurityLevel? = nil,
    preferCritical: Bool = true
  ) -> IdentityPublicKey? {
    let keys = identity.publicKeys.values.filter { !$0.isDisabled }

    // Filter by purpose if specified
    let filteredKeys = purpose != nil
    ? keys.filter { $0.purpose == purpose }
    : keys

    // Filter by security level if specified
    let securityFilteredKeys = minimumSecurityLevel != nil
    ? filteredKeys.filter { $0.securityLevel.rawValue <= minimumSecurityLevel!.rawValue }
    : filteredKeys

    guard !securityFilteredKeys.isEmpty else {
      return nil
    }

    // Prefer critical if requested
    if preferCritical {
      if let criticalKey = securityFilteredKeys.first(where: { $0.securityLevel == .critical }) {
        return criticalKey
      }
    }

    // Return first matching key
    return securityFilteredKeys.first
  }

  // MARK: - Private Key Retrieval

  /// Get private key for a specific key index from an identity
  /// - Parameters:
  ///   - identity: The identity
  ///   - keyIndex: The key index to retrieve
  /// - Returns: The private key data if found
  /// - Throws: `KeyManagerError.privateKeyNotFound` if the key is not in keychain
  /// - Note: This method must be called from a MainActor context
  @MainActor
  public func getPrivateKey(for identity: DPPIdentity, keyIndex: KeyID) throws -> Data {
    guard let privateKeyData = keychainManager.retrievePrivateKey(
      identityId: identity.id,
      keyIndex: Int32(keyIndex)
    ) else {
      throw KeyManagerError.privateKeyNotFound(keyIndex)
    }
    return privateKeyData
  }

  /// Check if a private key is available for a key
  /// - Parameters:
  ///   - identity: The identity
  ///   - keyIndex: The key index to check
  /// - Returns: True if private key is available in keychain
  /// - Note: This method must be called from a MainActor context
  @MainActor
  public func hasPrivateKey(for identity: DPPIdentity, keyIndex: KeyID) -> Bool {
    return keychainManager.hasPrivateKey(identityId: identity.id, keyIndex: Int32(keyIndex))
  }

  /// Find a key with available private key that meets requirements
  /// - Parameters:
  ///   - identity: The identity to find a key for
  ///   - purpose: Optional key purpose requirement
  ///   - minimumSecurityLevel: Optional minimum security level requirement
  ///   - preferCritical: Whether to prefer critical keys (default: true)
  /// - Returns: A key with available private key meeting the requirements, nil otherwise
  /// - Note: This method must be called from a MainActor context
  @MainActor
  public func findKeyWithPrivateKey(
    for identity: DPPIdentity,
    purpose: KeyPurpose? = nil,
    minimumSecurityLevel: SecurityLevel? = nil,
    preferCritical: Bool = true
  ) -> (key: IdentityPublicKey, privateKey: Data)? {
    // First find suitable keys
    guard let suitableKey = findKey(
      for: identity,
      purpose: purpose,
      minimumSecurityLevel: minimumSecurityLevel,
      preferCritical: preferCritical
    ) else {
      return nil
    }

    // Check if private key is available
    guard let privateKeyData = try? getPrivateKey(for: identity, keyIndex: suitableKey.id) else {
      return nil
    }

    return (suitableKey, privateKeyData)
  }

  // MARK: - Signer Creation

  /// Create a signer from private key data
  /// - Parameter privateKeyData: The private key data (32 bytes)
  /// - Returns: An OpaquePointer to the signer handle
  /// - Throws: `KeyManagerError.signerCreationFailed` if signer creation fails
  /// - Note: The returned signer must be destroyed with `destroySigner(_:)` when done
  public func createSigner(from privateKeyData: Data) throws -> OpaquePointer {
    // Validate private key length
    guard privateKeyData.count == 32 else {
      throw KeyManagerError.invalidKeyFormat("Private key must be 32 bytes, got \(privateKeyData.count)")
    }

    let signerResult = privateKeyData.withUnsafeBytes { keyBytes in
      dash_sdk_signer_create_from_private_key(
        keyBytes.bindMemory(to: UInt8.self).baseAddress!,
        UInt(privateKeyData.count)
      )
    }

    guard signerResult.error == nil else {
      let error = signerResult.error!.pointee
      defer { dash_sdk_error_free(signerResult.error) }
      let message = error.message != nil ? String(cString: error.message!) : "Unknown error"
      throw KeyManagerError.signerCreationFailed(message)
    }

    guard let signer = signerResult.data else {
      throw KeyManagerError.signerCreationFailed("No signer data returned")
    }

    return OpaquePointer(signer)
  }

  /// Create a signer for a specific key in an identity
  /// - Parameters:
  ///   - identity: The identity
  ///   - keyIndex: The key index to create a signer for
  /// - Returns: An OpaquePointer to the signer handle
  /// - Throws: `KeyManagerError.privateKeyNotFound` if private key is not available
  /// - Throws: `KeyManagerError.signerCreationFailed` if signer creation fails
  /// - Note: The returned signer must be destroyed with `destroySigner(_:)` when done
  /// - Note: This method must be called from a MainActor context
  @MainActor
  public func createSigner(for identity: DPPIdentity, keyIndex: KeyID) throws -> OpaquePointer {
    let privateKeyData = try getPrivateKey(for: identity, keyIndex: keyIndex)
    return try createSigner(from: privateKeyData)
  }

  /// Create a transfer signer for an identity (convenience method)
  /// - Parameter identity: The identity to create a transfer signer for
  /// - Returns: A tuple containing the transfer key and signer handle
  /// - Throws: `KeyManagerError.noSuitableKey` if no transfer key with private key is found
  /// - Throws: `KeyManagerError.signerCreationFailed` if signer creation fails
  /// - Note: The returned signer must be destroyed with `destroySigner(_:)` when done
  /// - Note: This method must be called from a MainActor context
  @MainActor
  public func createTransferSigner(for identity: DPPIdentity) throws -> (key: IdentityPublicKey, signer: OpaquePointer) {
    guard let transferKey = getTransferKey(for: identity) else {
      throw KeyManagerError.noSuitableKey("No transfer key found for identity")
    }

    let signer = try createSigner(for: identity, keyIndex: transferKey.id)
    return (transferKey, signer)
  }

  /// Create an authentication signer for an identity (convenience method)
  /// - Parameter identity: The identity to create an authentication signer for
  /// - Returns: A tuple containing the authentication key and signer handle
  /// - Throws: `KeyManagerError.noSuitableKey` if no authentication key with private key is found
  /// - Throws: `KeyManagerError.signerCreationFailed` if signer creation fails
  /// - Note: The returned signer must be destroyed with `destroySigner(_:)` when done
  /// - Note: This method must be called from a MainActor context
  @MainActor
  public func createAuthenticationSigner(for identity: DPPIdentity) throws -> (key: IdentityPublicKey, signer: OpaquePointer) {
    guard let authKey = getAuthenticationKey(for: identity) else {
      throw KeyManagerError.noSuitableKey("No authentication key found for identity")
    }

    let signer = try createSigner(for: identity, keyIndex: authKey.id)
    return (authKey, signer)
  }

  /// Create a signer for a key that meets specific requirements
  /// - Parameters:
  ///   - identity: The identity to create a signer for
  ///   - purpose: Optional key purpose requirement
  ///   - minimumSecurityLevel: Optional minimum security level requirement
  ///   - preferCritical: Whether to prefer critical keys (default: true)
  /// - Returns: A tuple containing the selected key and signer handle
  /// - Throws: `KeyManagerError.noSuitableKey` if no suitable key with private key is found
  /// - Throws: `KeyManagerError.signerCreationFailed` if signer creation fails
  /// - Note: The returned signer must be destroyed with `destroySigner(_:)` when done
  /// - Note: This method must be called from a MainActor context
  @MainActor
  public func createSignerForKey(
    for identity: DPPIdentity,
    purpose: KeyPurpose? = nil,
    minimumSecurityLevel: SecurityLevel? = nil,
    preferCritical: Bool = true
  ) throws -> (key: IdentityPublicKey, signer: OpaquePointer) {
    guard let (key, privateKey) = findKeyWithPrivateKey(
      for: identity,
      purpose: purpose,
      minimumSecurityLevel: minimumSecurityLevel,
      preferCritical: preferCritical
    ) else {
      let purposeDesc = purpose != nil ? " with purpose \(purpose!.name)" : ""
      let securityDesc = minimumSecurityLevel != nil ? " with security level \(minimumSecurityLevel!.name) or higher" : ""
      throw KeyManagerError.noSuitableKey("No suitable key\(purposeDesc)\(securityDesc) with available private key found")
    }

    let signer = try createSigner(from: privateKey)
    return (key, signer)
  }

  /// Destroy a signer handle
  /// - Parameter signer: The signer handle to destroy
  public func destroySigner(_ signer: OpaquePointer) {
    let signerPtr = UnsafeMutablePointer<SignerHandle>(signer)
    dash_sdk_signer_destroy(signerPtr)
  }

  // MARK: - Key Validation

  /// Validate that a private key matches a public key
  /// - Parameters:
  ///   - privateKeyData: The private key data
  ///   - publicKey: The public key to validate against
  ///   - isTestnet: Whether this is for testnet (default: true)
  /// - Returns: True if the private key matches the public key
  public func validatePrivateKey(
    _ privateKeyData: Data,
    matches publicKey: IdentityPublicKey,
    isTestnet: Bool = true
  ) -> Bool {
    let privateKeyHex = privateKeyData.toHexString()
    let publicKeyHex = publicKey.data.toHexString()

    return KeyValidation.validatePrivateKeyForPublicKey(
      privateKeyHex: privateKeyHex,
      publicKeyHex: publicKeyHex,
      keyType: publicKey.keyType,
      isTestnet: isTestnet
    )
  }

  /// Validate private key format and length
  /// - Parameter privateKeyData: The private key data to validate
  /// - Returns: True if the private key has valid format (32 bytes)
  public func validatePrivateKeyFormat(_ privateKeyData: Data) -> Bool {
    return privateKeyData.count == 32
  }
}

// MARK: - Convenience Extensions

extension KeyManager {
  /// Find a key suitable for document operations (requires AUTHENTICATION purpose)
  /// - Parameters:
  ///   - identity: The identity to find a key for
  ///   - minimumSecurityLevel: The minimum security level required (default: .high)
  /// - Returns: A key suitable for document operations if found, nil otherwise
  public func findDocumentSigningKey(
    for identity: DPPIdentity,
    minimumSecurityLevel: SecurityLevel = .high
  ) -> IdentityPublicKey? {
    return findKey(
      for: identity,
      purpose: .authentication,
      minimumSecurityLevel: minimumSecurityLevel,
      preferCritical: true
    )
  }

  /// Find a key suitable for contract operations (requires CRITICAL + AUTHENTICATION)
  /// - Parameter identity: The identity to find a key for
  /// - Returns: A key suitable for contract operations if found, nil otherwise
  public func findContractSigningKey(for identity: DPPIdentity) -> IdentityPublicKey? {
    return findKey(
      for: identity,
      purpose: .authentication,
      minimumSecurityLevel: .critical,
      preferCritical: true
    )
  }

  /// Create a signer for document operations
  /// - Parameters:
  ///   - identity: The identity to create a signer for
  ///   - minimumSecurityLevel: The minimum security level required (default: .high)
  /// - Returns: A tuple containing the selected key and signer handle
  /// - Throws: `KeyManagerError.noSuitableKey` if no suitable key with private key is found
  /// - Note: The returned signer must be destroyed with `destroySigner(_:)` when done
  /// - Note: This method must be called from a MainActor context
  @MainActor
  public func createDocumentSigner(
    for identity: DPPIdentity,
    minimumSecurityLevel: SecurityLevel = .high
  ) throws -> (key: IdentityPublicKey, signer: OpaquePointer) {
    return try createSignerForKey(
      for: identity,
      purpose: .authentication,
      minimumSecurityLevel: minimumSecurityLevel,
      preferCritical: true
    )
  }

  /// Create a signer for contract operations (requires CRITICAL + AUTHENTICATION)
  /// - Parameter identity: The identity to create a signer for
  /// - Returns: A tuple containing the selected key and signer handle
  /// - Throws: `KeyManagerError.noSuitableKey` if no suitable key with private key is found
  /// - Note: The returned signer must be destroyed with `destroySigner(_:)` when done
  /// - Note: This method must be called from a MainActor context
  @MainActor
  public func createContractSigner(for identity: DPPIdentity) throws -> (key: IdentityPublicKey, signer: OpaquePointer) {
    return try createSignerForKey(
      for: identity,
      purpose: .authentication,
      minimumSecurityLevel: .critical,
      preferCritical: true
    )
  }
}
