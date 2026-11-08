import Foundation
import SwiftDashSDK

/// How a contact request send or accept that threw ended, for the DashPay
/// views. Follows `SendViewModel`: a shielded spend whose result could not be
/// confirmed may already be on chain (Rust keeps its notes reserved), so the
/// view keeps it pending and never offers a fresh submission; the next sync
/// settles it. A private request Rust reports as pending (one to the same
/// contact in flight, or paid and not yet read back) is the same outcome:
/// nothing new was paid, and the next private sync settles it. Every other
/// error is an ordinary failure the user may retry.
enum ContactRequestSubmissionOutcome: Equatable {
    /// The private send or reply may have gone through: do not submit again.
    case mayHaveGoneThrough
    /// It failed; the message is the error's.
    case failed(String)

    static let mayHaveGoneThroughMessage = "This may have gone through. It shows up "
        + "after the next sync confirms it; do not submit it again."

    /// The message a request row shows: the pending notice while the wallet
    /// has a private reply to the contact pending, whatever error an earlier
    /// attempt left, and that error otherwise.
    static func rowMessage(pending: Bool, error: String?) -> String? {
        pending ? mayHaveGoneThroughMessage : error
    }

    static func of(_ error: Error) -> ContactRequestSubmissionOutcome {
        if let walletError = error as? PlatformWalletError {
            switch walletError {
            case .shieldedSpendUnconfirmed, .privateContactRequestPending:
                return .mayHaveGoneThrough
            default:
                break
            }
        }
        return .failed(error.localizedDescription)
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
