import Foundation
import SwiftDashSDK

/// How a contact request send or accept that threw ended, for the DashPay
/// views. Follows `SendViewModel`: a shielded spend whose result could not be
/// confirmed (code 18) may already be on chain (Rust keeps its notes
/// reserved), so the view keeps it pending and never offers a fresh
/// submission; the next sync settles it. A pair Rust reports as held for a
/// private request (code 71), which a public send or accept can also meet,
/// sent nothing new: the view shows it as pending until the wallet's
/// per-contact read says the pair is free. Unrestored private payments
/// (code 72) hold the wallet, not one contact: the view shows "setting up"
/// until the wallet's readiness reads restored, or why the last restoration
/// failed; no private sync lifts it. A private store read that did not
/// finish (code 74) sent nothing and is not pending: the user may try again.
/// Every other error is an ordinary failure the user may retry; a storage
/// backend that cannot protect private payments (73) and an SDK pinned below
/// the protocol version (75) are shown as configuration problems.
enum ContactRequestSubmissionOutcome: Equatable {
    /// The private send or reply may have gone through: do not submit again.
    case mayHaveGoneThrough
    /// The wallet holds the pair for a private request; nothing was sent.
    case privateRequestPending
    /// The wallet's private payments are not restored yet; nothing was sent.
    /// Restoring them, not a private sync, lifts it.
    case privatePaymentsNotRestored
    /// The private store read every contact request runs first did not
    /// finish; nothing was sent, and trying again continues it.
    case privateStoreNotRead
    /// It failed; the message is the error's.
    case failed(String)

    static let mayHaveGoneThroughMessage = "This may have gone through. It shows up "
        + "after the next sync confirms it; do not submit it again."

    static let privateRequestPendingMessage = "A private request to this contact is "
        + "pending. It clears once the request is read back from the private store, "
        + "or once its payment can no longer land, which can take a while. Sync "
        + "private requests to check."

    static let settingUpPrivatePaymentsMessage = "Setting up this wallet's private "
        + "payments. Contact requests wait until it finishes."

    /// The wallet's last restoration of its private payments failed.
    static func restoreFailedMessage(_ reason: String) -> String {
        "Restoring this wallet's private payments failed: \(reason). Contact requests "
            + "wait until a restore succeeds."
    }

    /// The app could not open the shielded store, which restores the
    /// wallet's private payments.
    static func shieldedStoreFailedMessage(_ reason: String) -> String {
        "The shielded store could not be opened: \(reason). Contact requests wait "
            + "until it opens."
    }

    static let privateStoreNotReadMessage = "Still reading private contact requests. "
        + "Nothing was sent; try again in a moment."

    /// Code 73 is a configuration error of the app's storage, which both
    /// declares what a private payment needs; shown as such.
    static let recoveryUnsupportedMessage = "This storage backend can't protect "
        + "private payments, so private contact requests are off."

    /// Code 75 is the developer's setting: only raising or removing the SDK's
    /// protocol version pin lifts it.
    static let sdkPinnedMessage = "This build's SDK is pinned below the protocol "
        + "version private contact requests need. Raise or remove the SDK's protocol "
        + "version pin."

    /// The notice shown while the wallet's private payments are not
    /// restored: why opening the shielded store failed, why the last
    /// restoration failed, or that they are being set up.
    static func restoringMessage(restoreFailure: String?, shieldedStoreFailure: String?) -> String {
        if let shieldedStoreFailure { return shieldedStoreFailedMessage(shieldedStoreFailure) }
        if let restoreFailure { return restoreFailedMessage(restoreFailure) }
        return settingUpPrivatePaymentsMessage
    }

    /// The message a request row shows: the restoring notice while the
    /// wallet's private payments are not restored, the pending notice while
    /// the wallet holds the pair, the unavailable notice while the wallet
    /// could not be read, and whatever error an earlier attempt left
    /// otherwise.
    static func rowMessage(
        pending: Bool,
        restoring: Bool = false,
        restoreFailure: String? = nil,
        shieldedStoreFailure: String? = nil,
        unavailable: Bool = false,
        error: String?
    ) -> String? {
        if restoring {
            return restoringMessage(
                restoreFailure: restoreFailure, shieldedStoreFailure: shieldedStoreFailure)
        }
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
            case .privateContactPaymentsNotRestored:
                return .privatePaymentsNotRestored
            case .privateContactStoreNotRead:
                return .privateStoreNotRead
            case .privateContactRecoveryUnsupported:
                return .failed(recoveryUnsupportedMessage)
            case .sdkPinnedBelowProtocolVersion:
                return .failed(sdkPinnedMessage)
            default:
                break
            }
        }
        return .failed(error.localizedDescription)
    }
}

/// Which contacts the wallet holds a pair with for a private request, as the
/// views last read it (`privateRepliesPending`, one
/// `ManagedPlatformWallet.privatePairHeld` read per contact, which answers as
/// a send decides it). While one is held, Rust refuses another send or reply
/// to that contact, so the views turn off what would submit one (Accept,
/// Ignore, Send). A failed read is not "nothing pending": the last
/// successful snapshot stays, and those controls stay off for every contact
/// until a read covering it succeeds.
///
/// A send or accept Rust refused as pending (code 71) marks the contact
/// pending at once; the next successful read covering it decides, as it
/// reads what the refusal read. Controls for a pair re-enable only when that
/// read says it is free: a payment that will never land frees it only once
/// the wallet proves so, which the read shows.
///
/// Whether the wallet's private payments are restored is the wallet's
/// readiness (`ManagedPlatformWallet.privateContactReadiness`), not
/// anything the views conclude: a code 72 refusal, or a readiness read that
/// says not restored, shows "setting up" (or why the last restoration
/// failed) for every contact, and only a readiness read that says restored
/// lifts it. A private sync changes neither on its own.
struct PrivateRepliesPendingSnapshot: Equatable {
    static let unavailableMessage = "Could not read whether a private request to "
        + "this contact is pending. Sync private requests to check again."

    /// The held contacts the last successful read found.
    private(set) var pending: Set<Data> = []
    /// The contacts the last successful read covered.
    private(set) var covered: Set<Data> = []
    /// False after a failed read, until a read succeeds.
    private(set) var available = true
    /// Contacts a send or accept was refused for as pending (code 71) since
    /// the last successful read covering them.
    private(set) var refused: Set<Data> = []
    /// The wallet's readiness for the identity, as last read; nil until a
    /// read succeeds.
    private(set) var readiness: PrivateContactReadiness?
    /// Why the last restoration failed, as last read with the readiness;
    /// nil when none failed or the reason could not be read.
    private(set) var restoreFailure: String?
    /// Set by a code 72 refusal; cleared only by a readiness read that says
    /// the private payments are restored.
    private(set) var restoringRefused = false

    /// Record a read of `contactIds`: the held ones, or nil when the read
    /// failed. A successful read decides each contact it covered, a code 71
    /// refusal included.
    mutating func record(_ read: Set<Data>?, for contactIds: [Data]) {
        guard let read else {
            available = false
            return
        }
        pending = read
        covered = Set(contactIds)
        refused.subtract(contactIds)
        available = true
    }

    /// Record that a send or accept to `contactId` was refused as pending
    /// (code 71).
    mutating func recordRefusal(of contactId: Data) {
        refused.insert(contactId)
    }

    /// Record a send or accept refused because the wallet's private payments
    /// are not restored (code 72).
    mutating func recordRestoringRefusal() {
        restoringRefused = true
    }

    /// Record a read of the wallet's readiness and, when it flags a failed
    /// restoration, the reason read with it; a nil readiness (the read
    /// failed) keeps what the last read said. A readiness that says restored
    /// lifts a code 72 refusal and its failure.
    mutating func recordReadiness(_ read: PrivateContactReadiness?, restoreFailure: String? = nil) {
        guard let read else { return }
        readiness = read
        self.restoreFailure = read.privatePaymentsRestoreFailed ? restoreFailure : nil
        if read.privatePaymentsRestored {
            restoringRefused = false
        }
    }

    /// Whether the wallet's private payments are being restored: a send or
    /// accept was refused with code 72, or the last readiness read says they
    /// are not restored. Every contact waits while they are.
    var isRestoringPrivatePayments: Bool {
        restoringRefused || readiness?.privatePaymentsRestored == false
    }

    /// Whether `contactId` is pending: the last successful read found the
    /// pair held, or a send or accept to it was refused as pending since.
    func isPending(_ contactId: Data) -> Bool {
        pending.contains(contactId) || refused.contains(contactId)
    }

    /// Whether the last read failed.
    var isUnavailable: Bool { !available }

    /// Whether a send or reply to `contactId` may be offered: a successful
    /// read covered it and found the pair free.
    func allowsSubmission(to contactId: Data) -> Bool {
        available && covered.contains(contactId) && !isPending(contactId)
            && !isRestoringPrivatePayments
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
        if let walletError = error as? PlatformWalletError {
            switch walletError {
            case .protocolVersionNotActive:
                return nil
            case .sdkPinnedBelowProtocolVersion:
                return ContactRequestSubmissionOutcome.sdkPinnedMessage
            default:
                break
            }
        }
        return "Private requests could not be read: \(error.localizedDescription)"
    }
}
