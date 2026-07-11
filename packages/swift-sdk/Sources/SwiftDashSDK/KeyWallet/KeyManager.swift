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
/// - Finding public keys by purpose (transfer, authentication, etc.)
/// - Validating that private material exists for a given key
/// - Inspecting / retrieving private keys for diagnostic UIs
///
/// **Signing happens via `KeychainSigner`, not here.** All callers
/// that need to sign a state transition should construct a
/// `KeychainSigner(modelContainer:)` and pass `signer.handle` to the
/// SDK call — the trampoline pulls private bytes out of Keychain
/// only at sign time and zeroes them immediately. This module's
/// `findSigningKey` helper picks the public key to use as `keyId:`
/// without extracting bytes up-front.
///
/// Example:
/// ```swift
/// let key = keyManager.findSigningKey(
///     for: identity,
///     purpose: .transfer
/// )!
/// let signer = KeychainSigner(modelContainer: container)
/// try await sdk.someOp(..., keyId: key.id, signer: signer.handle)
/// _ = signer  // keepalive — see KeychainSigner lifetime contract
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

  /// Pick a public key on `identity` that has signable private
  /// material on the device, WITHOUT extracting the private-key
  /// bytes. Companion to `findKeyWithPrivateKey` for callers that
  /// only need the `keyId` (because the actual signing happens via
  /// `KeychainSigner`'s callback, not via raw bytes pulled out
  /// here). Same candidate ranking + same dual-scheme keychain
  /// presence check, just discarding the bytes once they're
  /// confirmed to exist.
  ///
  /// Returns `nil` when no candidate has private material the
  /// signer trampoline could later resolve. Surface that as a
  /// "no suitable key found" error at the call site so the user
  /// gets the same shape of feedback the legacy bytes-out path
  /// produced.
  ///
  /// TODO(KeychainSigner v2): once every signer call site has
  /// migrated off the bytes-out path, replace the second
  /// `retrieveIdentityPrivateKey` call with a non-extracting
  /// existence check so the bytes never enter Swift memory at
  /// all during selection.
  @MainActor
  public func findSigningKey(
    for identity: DPPIdentity,
    purpose: KeyPurpose? = nil,
    minimumSecurityLevel: SecurityLevel? = nil,
    preferCritical: Bool = true
  ) -> IdentityPublicKey? {
    let candidates = rankKeys(
      for: identity,
      purpose: purpose,
      minimumSecurityLevel: minimumSecurityLevel,
      preferCritical: preferCritical
    )
    for candidate in candidates {
      if (try? getPrivateKey(for: identity, keyIndex: candidate.id)) != nil {
        return candidate
      }
      let publicKeyHex = candidate.data.toHexString()
      if keychainManager.retrieveIdentityPrivateKey(publicKeyHex: publicKeyHex) != nil {
        return candidate
      }
    }
    return nil
  }

  /// Return every key on `identity` matching the same filters as
  /// `findKey`, ordered the way `findKey` would prefer one — critical
  /// keys first when `preferCritical`, then everything else, with
  /// disabled keys filtered out. Used by `findKeyWithPrivateKey` so
  /// it can iterate through candidates instead of bailing on the
  /// first one whose private material isn't on the device.
  private func rankKeys(
    for identity: DPPIdentity,
    purpose: KeyPurpose?,
    minimumSecurityLevel: SecurityLevel?,
    preferCritical: Bool
  ) -> [IdentityPublicKey] {
    let active = identity.publicKeys.values.filter { !$0.isDisabled }
    let byPurpose = purpose != nil
      ? active.filter { $0.purpose == purpose }
      : Array(active)
    let bySecurity: [IdentityPublicKey]
    if let min = minimumSecurityLevel {
      // SecurityLevel raw values: 0=MASTER, 1=CRITICAL, 2=HIGH,
      // 3=MEDIUM. Lower == stricter, so "minimum X or higher" maps
      // to `rawValue <= X`.
      bySecurity = byPurpose.filter { $0.securityLevel.rawValue <= min.rawValue }
    } else {
      bySecurity = byPurpose
    }
    if preferCritical {
      let critical = bySecurity.filter { $0.securityLevel == .critical }
      let rest = bySecurity.filter { $0.securityLevel != .critical }
      return critical + rest
    }
    return bySecurity
  }

  /// Public ranking entry point. Returns the same purpose/security-ordered
  /// candidate list `findSigningKey` walks, but WITHOUT any private-material
  /// availability check — pure key-selection *policy*. Callers that want to
  /// delegate the availability decision to a specific signer (rather than to
  /// `KeychainManager.shared`) rank candidates here and then ask that signer
  /// directly. This is a read-only filter over `identity.publicKeys`; it does
  /// not touch the Keychain, so it stays out of the `@MainActor` Keychain path.
  public func rankSigningCandidates(
    for identity: DPPIdentity,
    purpose: KeyPurpose? = nil,
    minimumSecurityLevel: SecurityLevel? = nil,
    preferCritical: Bool = true
  ) -> [IdentityPublicKey] {
    rankKeys(
      for: identity,
      purpose: purpose,
      minimumSecurityLevel: minimumSecurityLevel,
      preferCritical: preferCritical
    )
  }

  // MARK: - Signer Creation

  /// Create a signer from private key data
  /// - Parameters:
  ///   - privateKeyData: The private key data (32 bytes)
  ///   - network: The network to associate with the key (affects WIF / address
  ///     derivation only; does not affect signing). Defaults to testnet.
  /// - Returns: An OpaquePointer to the signer handle
  /// - Throws: `KeyManagerError.signerCreationFailed` if signer creation fails
  /// - Note: The returned signer must be destroyed with `destroySigner(_:)` when done
  public func createSigner(
    from privateKeyData: Data,
    network: Network = .testnet
  ) throws -> OpaquePointer {
    // Validate private key length
    guard privateKeyData.count == 32 else {
      throw KeyManagerError.invalidKeyFormat("Private key must be 32 bytes, got \(privateKeyData.count)")
    }

    let signerResult = privateKeyData.withUnsafeBytes { keyBytes in
      dash_sdk_signer_create_from_private_key(
        keyBytes.bindMemory(to: UInt8.self).baseAddress!,
        UInt(privateKeyData.count),
        network.ffiValue
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
  /// Destroy a signer handle
  /// - Parameter signer: The signer handle to destroy
  public func destroySigner(_ signer: OpaquePointer) {
    dash_sdk_signer_destroy(signer)
  }

  // MARK: - Key Validation

  /// Validate that a private key matches a public key
  /// - Parameters:
  ///   - privateKeyData: The private key data
  ///   - publicKey: The public key to validate against
  ///   - network: Which network the keys belong to (default: testnet)
  /// - Returns: True if the private key matches the public key
  public func validatePrivateKey(
    _ privateKeyData: Data,
    matches publicKey: IdentityPublicKey,
    network: Network = .testnet
  ) -> Bool {
    let privateKeyHex = privateKeyData.toHexString()
    let publicKeyHex = publicKey.data.toHexString()

    return KeyValidation.validatePrivateKeyForPublicKey(
      privateKeyHex: privateKeyHex,
      publicKeyHex: publicKeyHex,
      keyType: publicKey.keyType,
      network: network
    )
  }

  /// Validate private key format and length
  /// - Parameter privateKeyData: The private key data to validate
  /// - Returns: True if the private key has valid format (32 bytes)
  public func validatePrivateKeyFormat(_ privateKeyData: Data) -> Bool {
    return privateKeyData.count == 32
  }
}

