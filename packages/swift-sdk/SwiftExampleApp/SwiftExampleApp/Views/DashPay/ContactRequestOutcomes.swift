import Foundation
import SwiftDashSDK

/// How a contact request send or accept that threw ended, for the DashPay
/// views. Follows `SendViewModel`: a shielded spend whose result could not be
/// confirmed (code 18) may already be on chain (Rust keeps its notes
/// reserved), and so may a state transition the SDK could not confirm after
/// it may have been accepted (code 20), so the view keeps either pending and
/// never offers a fresh submission; the next sync settles it. For a public
/// request the wallet holds the pair itself after a 20, the signed transition
/// kept, until a public contact request sync settles it, and refuses every
/// later request or accept to it with code 76: the view shows "may have gone
/// through; it is being checked" until the wallet's per-contact read says
/// settled. A pair Rust reports as held for a
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
    /// The wallet holds the pair for a public request whose outcome is not
    /// proved yet (code 76); nothing was signed. A public contact request
    /// sync settles it.
    case publicRequestUnconfirmed
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

    /// A public request or accept that may have gone through, as the
    /// wallet holds it until a sync settles its outcome.
    static let publicRequestUnconfirmedMessage = "This request may have gone through; "
        + "it is being checked. The next sync settles it."

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
    /// wallet's private payments are not restored, the being-checked notice
    /// while a request to the contact awaits its outcome, the pending notice
    /// while the wallet holds the pair for a private request, the
    /// unavailable notice while the wallet could not be read, and whatever
    /// error an earlier attempt left otherwise.
    static func rowMessage(
        pending: Bool,
        awaitingOutcome: Bool = false,
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
        if awaitingOutcome { return publicRequestUnconfirmedMessage }
        if pending { return privateRequestPendingMessage }
        if unavailable { return PrivateRepliesPendingSnapshot.unavailableMessage }
        return error
    }

    static func of(_ error: Error) -> ContactRequestSubmissionOutcome {
        if let walletError = error as? PlatformWalletError {
            switch walletError {
            case .shieldedSpendUnconfirmed, .transactionBroadcastUnconfirmed:
                return .mayHaveGoneThrough
            case .privateContactRequestPending:
                return .privateRequestPending
            case .publicContactRequestUnconfirmed:
                return .publicRequestUnconfirmed
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

/// Which contacts the wallet holds a pair with for a private request, and
/// which for a public request awaiting its outcome, as the views last read it
/// (`contactRequestHolds`: per contact, `ManagedPlatformWallet.privatePairHeld`,
/// which answers as a send decides it, and
/// `ManagedPlatformWallet.publicContactRequestUnconfirmed`). While either
/// holds a pair, Rust refuses another send or reply
/// to that contact, so the views turn off what would submit one (Accept,
/// Ignore, Send). A failed read is not "nothing pending": the last
/// successful snapshot stays, and those controls stay off for every contact
/// until a read covering it succeeds.
///
/// A send or accept Rust refused as pending (code 71) marks the contact
/// pending at once; the next successful read covering it decides, as it
/// reads what the refusal read. Controls for a pair re-enable only when that
/// read says it is free: a payment that will never land frees it only once
/// the wallet proves so, which the read shows. So does a send or accept that
/// may have gone through (18 or 20), or was refused as awaiting a public
/// request's outcome (76): the contact shows "being checked" at once, and a
/// read decides it the same way, so a reload whose sync did not settle the
/// wallet's hold keeps it so. A read always covers both holds: a read of the
/// private hold alone would offer a public request awaiting its outcome
/// again.
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

    /// The contacts the last successful read found held for a private
    /// request.
    private(set) var pending: Set<Data> = []
    /// The contacts the last successful read covered.
    private(set) var covered: Set<Data> = []
    /// False after a failed read, until a read succeeds.
    private(set) var available = true
    /// Contacts a send or accept was refused for as pending (code 71) since
    /// the last successful read covering them.
    private(set) var refused: Set<Data> = []
    /// The contacts the last successful read found held for a public
    /// request awaiting its outcome.
    private(set) var publicUnconfirmed: Set<Data> = []
    /// Contacts a send or accept may have gone through to (18, 20), or was
    /// refused for as awaiting a public request's outcome (76), since the
    /// last successful read covering them.
    private(set) var awaitingOutcome: Set<Data> = []
    /// The wallet's readiness for the identity, as last read; nil until a
    /// read succeeds.
    private(set) var readiness: PrivateContactReadiness?
    /// Why the last restoration failed, as last read with the readiness;
    /// nil when none failed or the reason could not be read.
    private(set) var restoreFailure: String?
    /// Set by a code 72 refusal; cleared only by a readiness read that says
    /// the private payments are restored.
    private(set) var restoringRefused = false

    /// Record a read of both holds for `contactIds` (`contactRequestHolds`),
    /// or nil when it failed. A successful read decides each contact it
    /// covered, a refusal or a submission that may have gone through
    /// included.
    mutating func record(_ read: ContactRequestHolds?, for contactIds: [Data]) {
        guard let read else {
            available = false
            return
        }
        pending = read.privatelyHeld
        publicUnconfirmed = read.publicUnconfirmed
        covered = Set(contactIds)
        refused.subtract(contactIds)
        awaitingOutcome.subtract(contactIds)
        available = true
    }

    /// Record how a send or accept to `contactId` that threw ended: a 71
    /// refusal marks it pending and a 72 refusal holds every contact, as
    /// `recordRefusal(of:)` and `recordRestoringRefusal()` do; one that may
    /// have gone through, or a 76 refusal, marks it awaiting its outcome.
    /// Nothing else changes a hold.
    mutating func recordOutcome(_ outcome: ContactRequestSubmissionOutcome, for contactId: Data) {
        switch outcome {
        case .privateRequestPending:
            recordRefusal(of: contactId)
        case .mayHaveGoneThrough, .publicRequestUnconfirmed:
            awaitingOutcome.insert(contactId)
        case .privatePaymentsNotRestored:
            recordRestoringRefusal()
        case .privateStoreNotRead, .failed:
            break
        }
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

    /// Whether a request to `contactId` awaits its outcome: the last
    /// successful read found the pair held for a public request awaiting
    /// it, or a send or accept to the contact may have gone through, or was
    /// refused so (76), since.
    func isAwaitingOutcome(_ contactId: Data) -> Bool {
        publicUnconfirmed.contains(contactId) || awaitingOutcome.contains(contactId)
    }

    /// Whether the last read failed.
    var isUnavailable: Bool { !available }

    /// Whether a send or reply to `contactId` may be offered: a successful
    /// read covered it and found the pair free of both holds.
    func allowsSubmission(to contactId: Data) -> Bool {
        available && covered.contains(contactId) && !isPending(contactId)
            && !isAwaitingOutcome(contactId) && !isRestoringPrivatePayments
    }
}

/// One read of the wallet's two per-contact holds.
struct ContactRequestHolds: Equatable {
    /// The contacts whose pair the wallet holds for a private request.
    var privatelyHeld: Set<Data> = []
    /// The contacts whose pair the wallet holds for a public request
    /// awaiting its outcome.
    var publicUnconfirmed: Set<Data> = []
}

/// Read the wallet's two holds for each of `contactIds`: whether the pair is
/// held for a private request (`privatePairHeld`) and whether for a public
/// request awaiting its outcome (`publicUnconfirmed`). Empty when the wallet
/// does not manage the identity (code 7), as it then holds nothing for it;
/// nil when a read failed otherwise, which the views keep apart from
/// "nothing held" (`PrivateRepliesPendingSnapshot`).
func contactRequestHolds(
    contactIds: [Data],
    privatePairHeld: (Data) throws -> Bool,
    publicUnconfirmed: (Data) throws -> Bool
) -> ContactRequestHolds? {
    do {
        var holds = ContactRequestHolds()
        for contactId in contactIds {
            if try privatePairHeld(contactId) {
                holds.privatelyHeld.insert(contactId)
            }
            if try publicUnconfirmed(contactId) {
                holds.publicUnconfirmed.insert(contactId)
            }
        }
        return holds
    } catch PlatformWalletError.identityNotFound(_) {
        return ContactRequestHolds()
    } catch {
        return nil
    }
}

/// The DashPay views' readiness refresh, one flow for the request list and
/// Add Contact. The wallet owns the recovery: on a store already open, a
/// repeat `configureShielded` retries restoring every wallet whose private
/// payments are still held, a restoration that failed after the store
/// opened included, and defers it to a task while a scan holds the store.
/// So a refresh asks for it whenever the payments are not restored, or the
/// store did not open, and only then reads the readiness it shows: the
/// readiness is read after the attempt, never cached from before it. While
/// the payments stay unrestored (a deferred restoration completes later,
/// changing nothing a view observes but the end of the shielded scan that
/// held the store), the view keeps refreshing on a timer for as long as it
/// is shown, and again when a shielded sync pass ends, so the gate lifts
/// without a bind, a restart or a submitted request. The views change no
/// hold themselves.
enum PrivatePaymentsRecovery {
    /// The cadence of the automatic refreshes while the payments stay
    /// unrestored: every `fastRecheckInterval` for the first `fastRechecks`
    /// of them (a minute), then every `slowRecheckInterval`, for as long as
    /// the view is shown. It never stops on a count: a shielded scan can hold
    /// the store for minutes, and the wallet runs a restoration it deferred
    /// only once the scan frees it. Each refresh is local and cheap, reads of
    /// in-memory wallet state and a repeat `configureShielded` that checks
    /// which wallets are still held and queues at most one deferred
    /// restoration per wallet, with no network call, so an open-ended poll at
    /// this cadence costs little. It stops when the readiness reads restored
    /// or the view goes away; a view for another identity starts its own.
    static let fastRechecks = 20
    static let fastRecheckInterval: Duration = .seconds(3)
    static let slowRecheckInterval: Duration = .seconds(15)

    /// Whether a refresh asks the wallet to retry before it reads the
    /// readiness it shows: the store did not open last time, or the last
    /// readiness read says the payments are not restored.
    static func retries(readiness: PrivateContactReadiness?, storeFailure: String?) -> Bool {
        storeFailure != nil || readiness?.privatePaymentsRestored == false
    }

    /// The pause before the next automatic refresh after `attempt` of them,
    /// or nil when there is none: the readiness reads restored, or no read
    /// succeeded yet (left to the refresh the view makes anyway).
    static func recheckDelay(after readiness: PrivateContactReadiness?, attempt: Int) -> Duration? {
        guard readiness?.privatePaymentsRestored == false else { return nil }
        return attempt < fastRechecks ? fastRecheckInterval : slowRecheckInterval
    }

    /// Wait for the next automatic refresh and say whether to make it: false
    /// at once when none is due (`recheckDelay`), false when the wait is cut
    /// short (`sleep` throws, as the view's task does when the view goes
    /// away), true otherwise. The views pass `Task.sleep`.
    @MainActor
    static func waitForRecheck(
        after readiness: PrivateContactReadiness?,
        attempt: Int,
        sleep: (Duration) async throws -> Void
    ) async -> Bool {
        guard let delay = recheckDelay(after: readiness, attempt: attempt) else { return false }
        do {
            try await sleep(delay)
        } catch {
            return false
        }
        return !Task.isCancelled
    }

    /// One refresh. `lastReadiness` is the readiness the view shows now;
    /// `readReadiness` reads the wallet's (with the reason a restoration
    /// failed); `configureAgain` asks the wallet to retry and returns why the
    /// store did not open, or nil. When `retries` says so, the wallet is asked
    /// first and the readiness is read after it; otherwise it is read once.
    @MainActor
    static func refresh(
        lastReadiness: PrivateContactReadiness?,
        storeFailure: String?,
        readReadiness: () -> (readiness: PrivateContactReadiness?, restoreFailure: String?),
        configureAgain: () -> String?
    ) -> (readiness: PrivateContactReadiness?, restoreFailure: String?, storeFailure: String?) {
        var storeFailure = storeFailure
        if retries(readiness: lastReadiness, storeFailure: storeFailure) {
            storeFailure = configureAgain()
            let read = readReadiness()
            return (read.readiness, read.restoreFailure, storeFailure)
        }
        let read = readReadiness()
        if retries(readiness: read.readiness, storeFailure: storeFailure) {
            storeFailure = configureAgain()
            let again = readReadiness()
            return (again.readiness, again.restoreFailure, storeFailure)
        }
        return (read.readiness, read.restoreFailure, storeFailure)
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
