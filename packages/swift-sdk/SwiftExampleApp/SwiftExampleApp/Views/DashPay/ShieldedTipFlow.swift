import Foundation
import SwiftDashSDK

// The shielded tip screens show what the wallet decides: it parses the
// amount, resolves the username, compares the recipient with the one this
// wallet last confirmed for the name, refuses a tip while an earlier payment
// to the recipient is unresolved, and checks the memo. Nothing here adds a
// rule of its own: these types map the wallet's results and errors to what
// the screens show and which controls they offer.
//
// Restart safety is the wallet's too. After a relaunch an unresolved earlier
// tip comes back from the review (`unresolved`) and from a send (code 85), so
// the app keeps no record of its own of a tip in flight.
//
// Neither code 85 nor `unresolved` says what became of the earlier payment:
// it may be this same tip and may pay the recipient, and the wallet stops
// refusing once a sync resolves it, whether it executed or was proven unable
// to. The app cannot tell which attempt that payment was, so its outcome is
// unknown to the app, and the screens never steer the user to send again.

/// How a shielded tip send ended.
enum ShieldedTipSendOutcome: Equatable {
    /// The wallet sent the tip.
    case submitted
    /// The tip may have left the wallet and may still execute (code 18 or
    /// 20). Never offered again: a shielded sync resolves it.
    case mayHaveGoneThrough(String)
    /// Refused with nothing sent by this attempt (code 85): another tip is
    /// being sent, or an earlier payment to the recipient is unresolved and
    /// may pay them. Never offered again, as for 18 and 20.
    case earlierTipPending(String)
    /// Not sent because the name of the contact the tip started from now
    /// belongs to another identity (code 86). The message is the wallet's.
    case recipientMismatch(String)
    /// Not sent (code 7, a recipient that changed since the review, a memo
    /// the wallet refused, and every other error). The message is the
    /// wallet's.
    case notSent(String)

    /// The outcome of a send that threw `error`.
    static func of(_ error: Error) -> ShieldedTipSendOutcome {
        if let walletError = error as? PlatformWalletError {
            switch walletError {
            case .shieldedSpendUnconfirmed, .transactionBroadcastUnconfirmed:
                return .mayHaveGoneThrough(walletError.localizedDescription)
            case .shieldedTipPending:
                return .earlierTipPending(walletError.localizedDescription)
            case .shieldedTipRecipientMismatch:
                return .recipientMismatch(walletError.localizedDescription)
            default:
                break
            }
        }
        return .notSent(error.localizedDescription)
    }

    var title: String {
        switch self {
        case .submitted: return "Tip sent"
        case .mayHaveGoneThrough: return "This tip may have gone through"
        case .earlierTipPending: return "Not sent: an earlier payment is unresolved"
        case .recipientMismatch: return "Not sent: this name moved to another identity"
        case .notSent: return "Not sent"
        }
    }

    var message: String {
        switch self {
        case .submitted:
            return "The wallet sent the tip. It shows in shielded activity once a shielded sync sees it."
        case .mayHaveGoneThrough:
            return "Do not send it again. A shielded sync tells whether it went through; "
                + "until then the wallet refuses another tip to this recipient."
        case .earlierTipPending:
            return "Nothing was sent by this attempt. "
                + ShieldedTipPresentation.unresolvedCaution
        case .recipientMismatch:
            return "Nothing was sent. " + ShieldedTipPresentation.recipientMismatchMessage
        case .notSent:
            return "Nothing was sent."
        }
    }

    /// The wallet's own words, shown under the message.
    var detail: String? {
        switch self {
        case .submitted: return nil
        case .mayHaveGoneThrough(let detail), .earlierTipPending(let detail),
             .recipientMismatch(let detail), .notSent(let detail):
            return detail
        }
    }

    /// Accessibility identifier of the outcome row.
    var identifier: String {
        switch self {
        case .submitted: return "dashpay.tip.outcome.submitted"
        case .mayHaveGoneThrough: return "dashpay.tip.outcome.mayHaveGoneThrough"
        case .earlierTipPending: return "dashpay.tip.outcome.pending"
        case .recipientMismatch: return "dashpay.tip.outcome.recipientMismatch"
        case .notSent: return "dashpay.tip.outcome.notSent"
        }
    }
}

/// The typed amount as the wallet parsed it.
enum ShieldedTipAmount: Equatable {
    /// Nothing typed yet.
    case empty
    /// Exact credits.
    case parsed(UInt64)
    /// The wallet refused the text; its reason.
    case refused(String)

    var credits: UInt64? {
        if case .parsed(let credits) = self { return credits }
        return nil
    }
}

/// Text for the parts of a review.
enum ShieldedTipPresentation {
    static func changeText(_ change: ShieldedTipRecipientChange) -> String {
        switch change {
        case .new:
            return "No recipient was confirmed for this name from this wallet before."
        case .unchanged:
            return "Same recipient as the one last confirmed for this name from this wallet."
        case .changed:
            return "This name now pays a different identity or shielded address than the one "
                + "last confirmed for it from this wallet. Check with the recipient before sending."
        }
    }

    /// Whether the change is shown as a warning.
    static func changeIsWarning(_ change: ShieldedTipRecipientChange) -> Bool {
        if case .changed = change { return true }
        return false
    }

    /// The recipient last confirmed for the name, when it changed.
    static func previousRecipient(_ change: ShieldedTipRecipientChange) -> ShieldedTipRecipient? {
        if case .changed(let previous) = change { return previous }
        return nil
    }

    /// Why nothing can be sent while an earlier payment to the recipient is
    /// unresolved, and what to check before tipping them again.
    static let unresolvedCaution = "An earlier payment to this recipient has not resolved and "
        + "may pay them. Check shielded activity before tipping this recipient again; the "
        + "wallet stops refusing once a sync resolves it, whether or not it paid."

    /// The review's notice when the wallet marks it unresolved.
    static let unresolvedMessage = "The wallet refuses a tip to this recipient for now. "
        + unresolvedCaution

    /// Why a tip started from a contact was refused with code 86, and where
    /// a tip to the name's new owner starts.
    static let recipientMismatchMessage = "This name now belongs to another identity, not "
        + "this contact. To tip the name's new owner, start from the name with Tip a "
        + "username on your profile, not from this contact."

    /// The text a failed review shows: the wallet's message, after the
    /// explanation for a name that moved away from the contact (code 86).
    static func reviewFailureText(_ error: Error) -> String {
        if let walletError = error as? PlatformWalletError,
           case .shieldedTipRecipientMismatch = walletError {
            return recipientMismatchMessage + " " + walletError.localizedDescription
        }
        return error.localizedDescription
    }

    static let noUsernameMessage = "A shielded tip is sent to a username, and no username "
        + "is known for this contact on this device."
}

/// The paying identity's shielded balance as last read.
///
/// `shieldedBalanceBusy` (a scan or a spend held the shielded store past the
/// read's wait) keeps the last state shown with a short busy note and reads
/// again after `busyRetryDelay`. Every other error, `walletOperation`
/// included, replaces the balance with its message and is not retried.
///
/// Every read is numbered by `beginRead()`; only the latest one may apply
/// its result, so a read that finishes after a newer one changes nothing.
struct ShieldedTipBalanceState: Equatable {
    enum Shown: Equatable {
        case notRead
        case balance(ShieldedIdentityBalance)
        case failed(String)
    }

    /// The pause before reading again while the store is busy.
    static let busyRetryDelay: Duration = .seconds(2)

    static let busyNote = "The shielded store is busy. Reading again shortly."

    private(set) var shown: Shown = .notRead
    /// The latest read found the store busy; `shown` is what the read
    /// before it said.
    private(set) var busy = false
    /// The number of the latest read started.
    private(set) var latestRead = 0
    /// A prepare succeeded and no current read that was not busy has been
    /// applied since: Prepare stays off, so a busy read that keeps the old
    /// `.unverified` does not offer it (and the seed prompt) again.
    private(set) var awaitingReadAfterPrepare = false

    /// Start a read and return its number.
    mutating func beginRead() -> Int {
        latestRead += 1
        return latestRead
    }

    /// Record a prepare that succeeded. Reads started before it no longer
    /// apply, and Prepare stays off until a read started after it is applied
    /// with a result that is not busy.
    mutating func prepareSucceeded() {
        latestRead += 1
        awaitingReadAfterPrepare = true
    }

    /// Apply the result of read `request`. Returns false, changing nothing,
    /// when a newer read was started since.
    @discardableResult
    mutating func apply(_ result: Result<ShieldedIdentityBalance, Error>, for request: Int) -> Bool {
        guard request == latestRead else { return false }
        switch result {
        case .success(let balance):
            shown = .balance(balance)
            busy = false
            awaitingReadAfterPrepare = false
        case .failure(let error):
            if let walletError = error as? PlatformWalletError, case .shieldedBalanceBusy = walletError {
                busy = true
            } else {
                shown = .failed(error.localizedDescription)
                busy = false
                awaitingReadAfterPrepare = false
            }
        }
        return true
    }

    /// The pause before reading again, or nil when the latest read needs
    /// none.
    var retryDelay: Duration? { busy ? Self.busyRetryDelay : nil }

    /// Whether to offer proving the identity's account
    /// (`prepareShieldedIdentityAddress`).
    var offersPrepare: Bool { !awaitingReadAfterPrepare && shown == .balance(.unverified) }

    var text: String {
        switch shown {
        case .notRead:
            return busy ? "Not read yet." : "Reading the shielded balance…"
        case .balance(.unverified):
            return "Not verified yet. The wallet proves this identity's shielded account with "
                + "its seed once per session; prepare it to read the balance."
        case .balance(.unavailable(let account)):
            return "Not known yet (account \(account)). A shielded sync establishes it. "
                + "Not known is not zero."
        case .balance(.available(let account, let credits)):
            return "\(formatCreditsAsDash(credits)) (account \(account))"
        case .failed(let message):
            return "Could not read the shielded balance: \(message)"
        }
    }

    /// The note under the kept balance while the store is busy.
    var busyText: String? { busy ? Self.busyNote : nil }

    var isFailure: Bool {
        if case .failed = shown { return true }
        return false
    }
}

/// One tip the user reviewed: what is sent if they confirm it.
struct ShieldedTipReviewed: Equatable {
    let username: String
    let amount: UInt64
    let memo: String?
    let review: ShieldedTipReview
    /// The contact the tip started from, passed to the send as to the
    /// review so the wallet refuses a name that moved to another identity;
    /// nil for a typed username.
    var contactIdentityId: Data? = nil
}

/// The tip sheet's state: amount, memo, review, send and its outcome. The
/// wallet calls come in as closures, so the mapping is tested without the
/// native library.
@MainActor
final class ShieldedTipFlow: ObservableObject {
    struct Actions {
        /// `PlatformWalletManager.parseShieldedTipAmount`.
        var parseAmount: @MainActor (String) throws -> UInt64
        /// `PlatformWalletManager.reviewShieldedTip` for the wallet, with the
        /// username and the contact's identity id (nil for a typed name).
        var review: @MainActor (String, Data?) async throws -> ShieldedTipReview
        /// `PlatformWalletManager.sendShieldedTip` from the identity the
        /// user acts as, with the reviewed recipient.
        var send: @MainActor (ShieldedTipReviewed) async throws -> Void
    }

    enum Phase: Equatable {
        case editing
        case reviewing
        case reviewFailed(String)
        case reviewed(ShieldedTipReviewed)
        case sending(ShieldedTipReviewed)
        case finished(ShieldedTipSendOutcome)
    }

    @Published var username: String {
        didSet { if username != oldValue { inputChanged() } }
    }
    @Published var amountText: String = "" {
        didSet {
            if amountText != oldValue {
                amount = parse(amountText)
                inputChanged()
            }
        }
    }
    @Published var memo: String = "" {
        didSet { if memo != oldValue { inputChanged() } }
    }
    @Published private(set) var amount: ShieldedTipAmount = .empty
    @Published private(set) var phase: Phase = .editing

    /// The username came with the entry point (a contact) and is not edited.
    let usernameIsFixed: Bool
    /// The contact the tip starts from, or nil for a typed username.
    let contactIdentityId: Data?

    private let actions: Actions
    /// Bumped by every input change and review, so a review that finishes
    /// after the inputs changed is dropped.
    private var generation = 0

    /// `contactIdentityId` is the contact's identity id when the tip starts
    /// from a contact, whose `username` it then carries.
    init(username: String?, contactIdentityId: Data? = nil, actions: Actions) {
        self.username = username ?? ""
        self.usernameIsFixed = username != nil
        self.contactIdentityId = contactIdentityId
        self.actions = actions
    }

    // MARK: Derived

    /// While a send runs, and after one that went, may have gone through, or
    /// met an unresolved earlier payment, the inputs stay as they were sent.
    var inputsLocked: Bool {
        switch phase {
        case .sending, .finished(.submitted), .finished(.mayHaveGoneThrough),
             .finished(.earlierTipPending):
            return true
        default:
            return false
        }
    }

    var reviewed: ShieldedTipReviewed? {
        switch phase {
        case .reviewed(let tip), .sending(let tip): return tip
        default: return nil
        }
    }

    var isBusy: Bool {
        switch phase {
        case .reviewing, .sending: return true
        default: return false
        }
    }

    var canReview: Bool {
        !inputsLocked && phase != .reviewing && !username.isEmpty && amount.credits != nil
    }

    /// Send is offered only for a review the wallet did not mark unresolved.
    var canSend: Bool {
        if case .reviewed(let tip) = phase { return !tip.review.unresolved }
        return false
    }

    /// A shielded sync is offered where one resolves what holds the tip.
    var offersShieldedSync: Bool {
        switch phase {
        case .reviewed(let tip): return tip.review.unresolved
        case .finished(.earlierTipPending), .finished(.mayHaveGoneThrough): return true
        default: return false
        }
    }

    var offersTipAgain: Bool { phase == .finished(.submitted) }

    // MARK: Actions

    func review() async {
        guard canReview, let credits = amount.credits else { return }
        generation += 1
        let current = generation
        let name = username
        let memoText = memo.isEmpty ? nil : memo
        phase = .reviewing
        do {
            let review = try await actions.review(name, contactIdentityId)
            guard current == generation else { return }
            phase = .reviewed(ShieldedTipReviewed(
                username: name, amount: credits, memo: memoText, review: review,
                contactIdentityId: contactIdentityId))
        } catch {
            guard current == generation else { return }
            phase = .reviewFailed(ShieldedTipPresentation.reviewFailureText(error))
        }
    }

    func send() async {
        guard canSend, case .reviewed(let tip) = phase else { return }
        phase = .sending(tip)
        do {
            try await actions.send(tip)
            phase = .finished(.submitted)
        } catch {
            phase = .finished(ShieldedTipSendOutcome.of(error))
        }
    }

    /// Start another tip after one was sent.
    func tipAgain() {
        guard offersTipAgain else { return }
        phase = .editing
        amountText = ""
        memo = ""
    }

    // MARK: Private

    private func parse(_ text: String) -> ShieldedTipAmount {
        guard !text.isEmpty else { return .empty }
        do {
            return .parsed(try actions.parseAmount(text))
        } catch {
            return .refused(error.localizedDescription)
        }
    }

    /// An edit drops the review, a review in progress and a refusal shown.
    private func inputChanged() {
        guard !inputsLocked else { return }
        generation += 1
        if phase != .editing {
            phase = .editing
        }
    }
}
