import Foundation
import SwiftDashSDK
import Combine

// Re-export core types
public typealias DashPayError = SwiftDashSDK.DashPayError

/// UI-friendly wrapper around SDK's DashPayService with ObservableObject support
@MainActor
class DashPayService: ObservableObject {
    @Published var platformWallet: PlatformWallet?
    @Published var identityManager: IdentityManager?
    @Published var currentIdentity: ManagedIdentity?
    @Published var isLoading = false
    @Published var error: String?

    private let sdkService = SwiftDashSDK.DashPayService()
    private var network: PlatformNetwork = .testnet

    /// Initialize Platform Wallet from mnemonic
    func initializeWallet(mnemonic: String, network: PlatformNetwork = .testnet) async throws {
        isLoading = true
        defer { isLoading = false }

        do {
            try sdkService.initializeWallet(mnemonic: mnemonic, network: network)

            // Create platform wallet from mnemonic for local reference
            let wallet = try PlatformWallet.fromMnemonic(mnemonic)
            let manager = try wallet.getIdentityManager(for: network)

            self.platformWallet = wallet
            self.identityManager = manager
            self.network = network
            self.error = nil
        } catch {
            self.error = "Failed to initialize Platform Wallet: \(error.localizedDescription)"
            throw error
        }
    }

    /// Load a managed identity from identity bytes
    func loadIdentity(identityBytes: Data) async throws -> ManagedIdentity {
        isLoading = true
        defer { isLoading = false }

        do {
            let managedIdentity = try sdkService.loadIdentity(identityBytes: identityBytes)

            // Add to identity manager if available
            if let manager = identityManager {
                try manager.addIdentity(managedIdentity)
            }

            self.currentIdentity = managedIdentity
            self.error = nil

            return managedIdentity
        } catch {
            self.error = "Failed to load identity: \(error.localizedDescription)"
            throw error
        }
    }

    /// Get all identities from the manager
    func getAllIdentities() throws -> [Identifier] {
        guard let manager = identityManager else {
            throw DashPayError.noIdentityManager
        }

        return try manager.getAllIdentityIds()
    }

    /// Set an identity as the primary identity
    func setPrimaryIdentity(_ identityId: Identifier) throws {
        guard let manager = identityManager else {
            throw DashPayError.noIdentityManager
        }

        try manager.setPrimaryIdentity(identityId)
    }

    /// Get the primary identity
    func getPrimaryIdentity() throws -> ManagedIdentity? {
        guard let manager = identityManager else {
            throw DashPayError.noIdentityManager
        }

        guard let primaryId = try manager.getPrimaryIdentityId() else {
            return nil
        }

        return try manager.getIdentity(primaryId)
    }

    // MARK: - Contact Requests

    /// Send a contact request to another identity
    func sendContactRequest(
        from identity: ManagedIdentity,
        to recipientId: Identifier,
        encryptedPublicKey: Data
    ) async throws {
        isLoading = true
        defer { isLoading = false }

        do {
            try sdkService.sendContactRequest(
                from: identity,
                to: recipientId,
                encryptedPublicKey: encryptedPublicKey
            )

            self.error = nil
        } catch {
            self.error = "Failed to send contact request: \(error.localizedDescription)"
            throw error
        }
    }

    /// Accept a contact request
    func acceptContactRequest(identity: ManagedIdentity, from senderId: Identifier) async throws {
        isLoading = true
        defer { isLoading = false }

        do {
            try sdkService.acceptContactRequest(identity: identity, from: senderId)

            self.error = nil
        } catch {
            self.error = "Failed to accept contact request: \(error.localizedDescription)"
            throw error
        }
    }

    /// Reject a contact request
    func rejectContactRequest(identity: ManagedIdentity, from senderId: Identifier) async throws {
        isLoading = true
        defer { isLoading = false }

        do {
            try sdkService.rejectContactRequest(identity: identity, from: senderId)

            self.error = nil
        } catch {
            self.error = "Failed to reject contact request: \(error.localizedDescription)"
            throw error
        }
    }

    /// Get all sent contact requests for an identity
    func getSentContactRequests(identity: ManagedIdentity) throws -> [ContactRequest] {
        return try sdkService.getSentContactRequests(identity: identity)
    }

    /// Get all incoming contact requests for an identity
    func getIncomingContactRequests(identity: ManagedIdentity) throws -> [ContactRequest] {
        return try sdkService.getIncomingContactRequests(identity: identity)
    }

    // MARK: - Established Contacts

    /// Get all established contacts for an identity
    func getEstablishedContacts(identity: ManagedIdentity) throws -> [EstablishedContact] {
        return try sdkService.getEstablishedContacts(identity: identity)
    }

    /// Check if a contact is established
    func isContactEstablished(identity: ManagedIdentity, contactId: Identifier) throws -> Bool {
        return try sdkService.isContactEstablished(identity: identity, contactId: contactId)
    }

    /// Set alias for a contact
    func setContactAlias(contact: EstablishedContact, alias: String) throws {
        try sdkService.setContactAlias(contact: contact, alias: alias)
    }

    /// Set note for a contact
    func setContactNote(contact: EstablishedContact, note: String) throws {
        try sdkService.setContactNote(contact: contact, note: note)
    }

    /// Hide a contact
    func hideContact(_ contact: EstablishedContact) throws {
        try sdkService.hideContact(contact)
    }

    /// Unhide a contact
    func unhideContact(_ contact: EstablishedContact) throws {
        try sdkService.unhideContact(contact)
    }
}

// MARK: - Contact Model for UI

struct DashPayContact: Identifiable {
    let id: Identifier
    let alias: String?
    let note: String?
    let isHidden: Bool
    let dpnsName: String?  // Could be fetched from Platform

    init(from establishedContact: EstablishedContact) throws {
        self.id = try establishedContact.getContactIdentityId()
        self.alias = try establishedContact.getAlias()
        self.note = try establishedContact.getNote()
        self.isHidden = try establishedContact.isHidden()
        self.dpnsName = nil  // Would need to query Platform for DPNS name
    }

    var displayName: String {
        alias ?? dpnsName ?? id.hexString.prefix(12) + "..."
    }
}

// MARK: - Contact Request Model for UI

struct DashPayContactRequest: Identifiable {
    let id: UUID = UUID()
    let senderId: Identifier
    let recipientId: Identifier
    let createdAt: Date
    let encryptedPublicKey: Data

    init(from contactRequest: ContactRequest) throws {
        self.senderId = try contactRequest.getSenderId()
        self.recipientId = try contactRequest.getRecipientId()
        let timestamp = try contactRequest.getCreatedAt()
        self.createdAt = Date(timeIntervalSince1970: Double(timestamp) / 1000.0)
        self.encryptedPublicKey = try contactRequest.getEncryptedPublicKey()
    }
}
