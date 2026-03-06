import Foundation

// MARK: - Stub types for FriendsView
// These types are placeholders - the actual DashPay contact system needs to be implemented

public struct DashPayContact: Identifiable {
    public let id: Data
    public let displayName: String
    public let identityId: Data
    public let dpnsName: String?
    public let note: String?
    public let isHidden: Bool

    public init(id: Data, displayName: String, identityId: Data, dpnsName: String? = nil, note: String? = nil, isHidden: Bool = false) {
        self.id = id
        self.displayName = displayName
        self.identityId = identityId
        self.dpnsName = dpnsName
        self.note = note
        self.isHidden = isHidden
    }
}

public struct DashPayContactRequest: Identifiable {
    public let id: String
    public let senderId: Data
    public let recipientId: Data
    public let createdAt: Date
    public let senderDisplayName: String?

    public init(id: String, senderId: Data, recipientId: Data, createdAt: Date = Date(), senderDisplayName: String? = nil) {
        self.id = id
        self.senderId = senderId
        self.recipientId = recipientId
        self.createdAt = createdAt
        self.senderDisplayName = senderDisplayName
    }
}
