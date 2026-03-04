import Foundation

/// Service for managing DashPay contacts and identities
///
/// This service provides high-level operations for DashPay functionality including:
/// - Identity management
/// - Contact requests (sending, accepting, rejecting)
/// - Established contacts management
/// - Contact metadata (aliases, notes, visibility)
public final class DashPayService: Sendable {
    private let platformWallet: SendableBox<PlatformWallet?>
    private let identityManager: SendableBox<IdentityManager?>
    private let currentIdentity: SendableBox<ManagedIdentity?>
    private let network: SendableBox<PlatformNetwork>

    public init() {
        self.platformWallet = SendableBox(nil)
        self.identityManager = SendableBox(nil)
        self.currentIdentity = SendableBox(nil)
        self.network = SendableBox(.testnet)
    }

    // Thread-safe sendable box for reference types
    private final class SendableBox<T>: @unchecked Sendable {
        private let lock = NSLock()
        private var _value: T

        init(_ value: T) {
            self._value = value
        }

        var value: T {
            get {
                lock.lock()
                defer { lock.unlock() }
                return _value
            }
            set {
                lock.lock()
                defer { lock.unlock() }
                _value = newValue
            }
        }
    }

    // MARK: - Initialization

    /// Initialize Platform Wallet from mnemonic
    /// - Parameters:
    ///   - mnemonic: BIP39 mnemonic phrase
    ///   - network: Platform network (mainnet, testnet, devnet)
    /// - Throws: Error if wallet creation fails
    public func initializeWallet(mnemonic: String, network: PlatformNetwork = .testnet) throws {
        // Create platform wallet from mnemonic
        let wallet = try PlatformWallet.fromMnemonic(mnemonic)

        // Get identity manager for the specified network
        let manager = try wallet.getIdentityManager(for: network)

        self.platformWallet.value = wallet
        self.identityManager.value = manager
        self.network.value = network
    }

    // MARK: - Identity Management

    /// Load a managed identity from identity bytes
    /// - Parameter identityBytes: Raw identity data
    /// - Returns: The loaded managed identity
    /// - Throws: Error if identity loading fails
    public func loadIdentity(identityBytes: Data) throws -> ManagedIdentity {
        let managedIdentity = try ManagedIdentity.fromIdentityBytes(identityBytes)

        // Add to identity manager if available
        if let manager = identityManager.value {
            try manager.addIdentity(managedIdentity)
        }

        self.currentIdentity.value = managedIdentity
        return managedIdentity
    }

    /// Get all identities from the manager
    /// - Returns: Array of identity IDs
    /// - Throws: Error if identity manager not initialized
    public func getAllIdentities() throws -> [Identifier] {
        guard let manager = identityManager.value else {
            throw DashPayError.noIdentityManager
        }

        return try manager.getAllIdentityIds()
    }

    /// Set an identity as the primary identity
    /// - Parameter identityId: The identity ID to set as primary
    /// - Throws: Error if operation fails
    public func setPrimaryIdentity(_ identityId: Identifier) throws {
        guard let manager = identityManager.value else {
            throw DashPayError.noIdentityManager
        }

        try manager.setPrimaryIdentity(identityId)
    }

    /// Get the primary identity
    /// - Returns: The primary identity, or nil if none set
    /// - Throws: Error if operation fails
    public func getPrimaryIdentity() throws -> ManagedIdentity? {
        guard let manager = identityManager.value else {
            throw DashPayError.noIdentityManager
        }

        guard let primaryId = try manager.getPrimaryIdentityId() else {
            return nil
        }

        return try manager.getIdentity(primaryId)
    }

    // MARK: - Contact Requests

    /// Send a contact request to another identity
    /// - Parameters:
    ///   - identity: The identity sending the request
    ///   - recipientId: The recipient's identity ID
    ///   - encryptedPublicKey: Encrypted public key for secure communication
    /// - Throws: Error if sending fails
    public func sendContactRequest(
        from identity: ManagedIdentity,
        to recipientId: Identifier,
        encryptedPublicKey: Data
    ) throws {
        // In a real implementation, you would:
        // 1. Derive the appropriate keys
        // 2. Encrypt your public key with recipient's key
        // 3. Create and broadcast the contact request

        try identity.sendContactRequest(
            recipientId: recipientId,
            senderKeyIndex: 0,  // Should be derived from identity keys
            recipientKeyIndex: 0,  // Should be looked up from recipient
            accountReference: 0,
            encryptedPublicKey: encryptedPublicKey
        )
    }

    /// Accept a contact request
    /// - Parameters:
    ///   - identity: The identity accepting the request
    ///   - senderId: The sender's identity ID
    /// - Throws: Error if acceptance fails
    public func acceptContactRequest(identity: ManagedIdentity, from senderId: Identifier) throws {
        try identity.acceptContactRequest(senderId: senderId)
    }

    /// Reject a contact request
    /// - Parameters:
    ///   - identity: The identity rejecting the request
    ///   - senderId: The sender's identity ID
    /// - Throws: Error if rejection fails
    public func rejectContactRequest(identity: ManagedIdentity, from senderId: Identifier) throws {
        try identity.rejectContactRequest(senderId: senderId)
    }

    /// Get all sent contact requests for an identity
    /// - Parameter identity: The identity to query
    /// - Returns: Array of sent contact requests
    /// - Throws: Error if query fails
    public func getSentContactRequests(identity: ManagedIdentity) throws -> [ContactRequest] {
        let requestIds = try identity.getSentContactRequestIds()

        return try requestIds.compactMap { recipientId in
            try identity.getSentContactRequest(recipientId: recipientId)
        }
    }

    /// Get all incoming contact requests for an identity
    /// - Parameter identity: The identity to query
    /// - Returns: Array of incoming contact requests
    /// - Throws: Error if query fails
    public func getIncomingContactRequests(identity: ManagedIdentity) throws -> [ContactRequest] {
        let requestIds = try identity.getIncomingContactRequestIds()

        return try requestIds.compactMap { senderId in
            try identity.getIncomingContactRequest(senderId: senderId)
        }
    }

    // MARK: - Established Contacts

    /// Get all established contacts for an identity
    /// - Parameter identity: The identity to query
    /// - Returns: Array of established contacts
    /// - Throws: Error if query fails
    public func getEstablishedContacts(identity: ManagedIdentity) throws -> [EstablishedContact] {
        let contactIds = try identity.getEstablishedContactIds()

        return try contactIds.compactMap { contactId in
            try identity.getEstablishedContact(contactId: contactId)
        }
    }

    /// Check if a contact is established
    /// - Parameters:
    ///   - identity: The identity to check
    ///   - contactId: The contact's identity ID
    /// - Returns: true if contact is established
    /// - Throws: Error if check fails
    public func isContactEstablished(identity: ManagedIdentity, contactId: Identifier) throws -> Bool {
        return try identity.isContactEstablished(contactId: contactId)
    }

    /// Set alias for a contact
    /// - Parameters:
    ///   - contact: The established contact
    ///   - alias: The alias to set
    /// - Throws: Error if operation fails
    public func setContactAlias(contact: EstablishedContact, alias: String) throws {
        try contact.setAlias(alias)
    }

    /// Set note for a contact
    /// - Parameters:
    ///   - contact: The established contact
    ///   - note: The note to set
    /// - Throws: Error if operation fails
    public func setContactNote(contact: EstablishedContact, note: String) throws {
        try contact.setNote(note)
    }

    /// Hide a contact
    /// - Parameter contact: The contact to hide
    /// - Throws: Error if operation fails
    public func hideContact(_ contact: EstablishedContact) throws {
        try contact.hide()
    }

    /// Unhide a contact
    /// - Parameter contact: The contact to unhide
    /// - Throws: Error if operation fails
    public func unhideContact(_ contact: EstablishedContact) throws {
        try contact.unhide()
    }
}

// MARK: - Errors

/// Errors that can occur in DashPay operations
public enum DashPayError: Error, LocalizedError {
    case noWallet
    case noIdentityManager
    case noCurrentIdentity
    case invalidIdentityBytes
    case contactNotFound

    public var errorDescription: String? {
        switch self {
        case .noWallet:
            return "Platform wallet not initialized"
        case .noIdentityManager:
            return "Identity manager not available"
        case .noCurrentIdentity:
            return "No identity selected"
        case .invalidIdentityBytes:
            return "Invalid identity data"
        case .contactNotFound:
            return "Contact not found"
        }
    }
}
