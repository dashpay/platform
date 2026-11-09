import Foundation
import SwiftDashSDK

/// How a contact request send or accept that threw ended, for the DashPay
/// views. Follows `SendViewModel`: a shielded spend whose result could not be
/// confirmed may already be on chain (Rust keeps its notes reserved), so the
/// view keeps it pending and never offers a fresh submission; the next sync
/// settles it. A private request Rust reports as pending (one to the same
/// contact in flight, or paid and not yet read back), which a public send or
/// accept can also meet, sent nothing new: the view shows it as pending and
/// offers a private sync, which settles it, after which the user may try
/// again. Every other error is an ordinary failure the user may retry.
enum ContactRequestSubmissionOutcome: Equatable {
    /// The private send or reply may have gone through: do not submit again.
    case mayHaveGoneThrough
    /// A private request to the contact is pending in the wallet; nothing
    /// was sent. A private sync settles it.
    case privateRequestPending
    /// It failed; the message is the error's.
    case failed(String)

    static let mayHaveGoneThroughMessage = "This may have gone through. It shows up "
        + "after the next sync confirms it; do not submit it again."

    static let privateRequestPendingMessage = "A private request to this contact is "
        + "pending. Sync private requests to settle it, then try again if it did not "
        + "go through."

    /// The message a request row shows: the pending notice while the wallet
    /// has a private request to the contact pending, the unavailable notice
    /// while the wallet could not be read, whatever error an earlier attempt
    /// left, and that error otherwise.
    static func rowMessage(pending: Bool, unavailable: Bool = false, error: String?) -> String? {
        if pending { return privateRequestPendingMessage }
        if unavailable { return PrivateRepliesPendingSnapshot.unavailableMessage }
        return error
    }

    static func of(_ error: Error) -> ContactRequestSubmissionOutcome {
        if let walletError = error as? PlatformWalletError {
            switch walletError {
            case .shieldedSpendUnconfirmed:
                return .mayHaveGoneThrough
            case .privateContactRequestPending:
                return .privateRequestPending
            default:
                break
            }
        }
        return .failed(error.localizedDescription)
    }
}

/// Which contacts have a private request pending in the wallet, as the views
/// last read it (`privateRepliesPending`). While one is, Rust refuses another
/// send or reply to that contact, so the views turn off what would submit one
/// (Accept, Ignore, Send). A failed read is not "nothing pending": the last
/// successful snapshot stays, and those controls stay off for every contact
/// until a read covering it succeeds.
struct PrivateRepliesPendingSnapshot: Equatable {
    static let unavailableMessage = "Could not read whether a private request to "
        + "this contact is pending. Sync private requests to check again."

    /// The pending contacts the last successful read found.
    private(set) var pending: Set<Data> = []
    /// The contacts the last successful read covered.
    private(set) var covered: Set<Data> = []
    /// False after a failed read, until a read succeeds.
    private(set) var available = true

    /// Record a read of `contactIds`: the pending ones, or nil when the read
    /// failed.
    mutating func record(_ read: Set<Data>?, for contactIds: [Data]) {
        guard let read else {
            available = false
            return
        }
        pending = read
        covered = Set(contactIds)
        available = true
    }

    /// Whether the last successful read found a private request to
    /// `contactId` pending.
    func isPending(_ contactId: Data) -> Bool {
        pending.contains(contactId)
    }

    /// Whether the last read failed.
    var isUnavailable: Bool { !available }

    /// Whether a send or reply to `contactId` may be offered: a successful
    /// read covered it and found nothing pending.
    func allowsSubmission(to contactId: Data) -> Bool {
        available && covered.contains(contactId) && !pending.contains(contactId)
    }
}

/// How Add Contact's collision alert accepts the request the recipient
/// already sent us. With "Send privately" on, the user chose a private
/// request, so the accept replies privately
/// (`PlatformWalletManager.acceptPrivateContactRequest`, paid from the
/// shielded balance as the private send is) even when their request came
/// publicly: a public reply would publish this user as its owner and the
/// contact as its recipient, against that choice. With it off, the wallet
/// replies the way the request came
/// (`PlatformWalletManager.acceptContactRequestReplyingInKind`). The toggle is
/// the user's explicit choice; which reply an in-kind accept takes stays the
/// wallet's.
enum CollisionAccept: Equatable {
    /// Reply privately, paid from the shielded balance.
    case privateReply
    /// Reply the way the request came.
    case replyInKind

    static func of(sendPrivately: Bool) -> CollisionAccept {
        sendPrivately ? .privateReply : .replyInKind
    }

    /// The alert's accept button.
    var buttonTitle: String {
        switch self {
        case .privateReply: return "Accept privately"
        case .replyInKind: return "Accept"
        }
    }

    /// The alert's message.
    var message: String {
        switch self {
        case .privateReply:
            return "This person already sent you a request. Accept it with a private reply "
                + "instead? The reply is paid from this wallet's shielded balance, and no "
                + "public request names you as its sender."
        case .replyInKind:
            return "This person already sent you a request. Accept it instead?"
        }
    }
}

/// What the request list's private-store refresh tells the user. A network
/// that does not serve the store yet (before protocol version 15) is expected
/// and stays quiet; an operational failure, or requests read but not saved,
/// is shown so the user knows to refresh again.
enum PrivateContactRefreshNotice {
    /// The notice a pass that read the store leaves, or nil when every
    /// identity's requests reached disk.
    static func notice(unpersistedIdentityCount: UInt32) -> String? {
        guard unpersistedIdentityCount > 0 else { return nil }
        return "Private requests for \(unpersistedIdentityCount) "
            + (unpersistedIdentityCount == 1 ? "identity were" : "identities were")
            + " read but not saved. Pull to refresh again."
    }

    /// The notice a pass that failed leaves, or nil when the network does not
    /// serve the private store yet.
    static func notice(for error: Error) -> String? {
        if let walletError = error as? PlatformWalletError,
           case .protocolVersionNotActive = walletError {
            return nil
        }
        return "Private requests could not be read: \(error.localizedDescription)"
    }
}
