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
// tip comes back from the review (`unresolved`) and from a send (code 84), so
// the app keeps no record of its own of a tip in flight.

/// How a shielded tip send ended.
enum ShieldedTipSendOutcome: Equatable {
    /// The wallet sent the tip.
    case submitted
    /// The tip may have left the wallet and may still execute (code 18 or
    /// 20). Never offered again: a shielded sync resolves it.
    case mayHaveGoneThrough(String)
    /// Refused with nothing sent (code 84): another tip is being sent, or an
    /// earlier payment to the recipient is unresolved. It clears by itself
    /// after a shielded sync; then the user reviews again.
    case earlierTipPending(String)
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
        case .earlierTipPending: return "Not sent: an earlier tip is still pending"
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
            return "Nothing was sent. Another tip from this wallet is being sent, or an earlier "
                + "payment to this recipient has not resolved yet. It clears by itself after a "
                + "shielded sync; then review again."
        case .notSent:
            return "Nothing was sent."
        }
    }

    /// The wallet's own words, shown under the message.
    var detail: String? {
        switch self {
        case .submitted: return nil
        case .mayHaveGoneThrough(let detail), .earlierTipPending(let detail), .notSent(let detail):
            return detail
        }
    }

    /// Accessibility identifier of the outcome row.
    var identifier: String {
        switch self {
        case .submitted: return "dashpay.tip.outcome.submitted"
        case .mayHaveGoneThrough: return "dashpay.tip.outcome.mayHaveGoneThrough"
        case .earlierTipPending: return "dashpay.tip.outcome.pending"
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
            return "This wallet has not tipped this name before."
        case .unchanged:
            return "Same recipient as the last tip from this wallet to this name."
        case .changed:
            return "This name now pays a different identity or shielded address than the "
                + "last tip from this wallet to it. Check with the recipient before sending."
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

    static let unresolvedMessage = "An earlier shielded payment from this wallet to this "
        + "recipient has not resolved yet, so the wallet would refuse another. A shielded "
        + "sync resolves it; then review again."

    static let noUsernameMessage = "A shielded tip is sent to a username, and no username "
        + "is known for this contact on this device."
}

/// The paying identity's shielded balance as last read. A busy shielded store
/// (`walletOperation`) keeps the last state shown and asks for another read.
struct ShieldedTipBalanceState: Equatable {
    enum Shown: Equatable {
        case notRead
        case balance(ShieldedIdentityBalance)
        case failed(String)
    }

    private(set) var shown: Shown = .notRead
    /// The last read found the store busy; `shown` is what the read before
    /// it said.
    private(set) var busy = false

    mutating func record(_ balance: ShieldedIdentityBalance) {
        shown = .balance(balance)
        busy = false
    }

    mutating func record(error: Error) {
        if let walletError = error as? PlatformWalletError, case .walletOperation = walletError {
            busy = true
            return
        }
        shown = .failed(error.localizedDescription)
        busy = false
    }

    /// Whether to read again shortly.
    var readsAgain: Bool { busy }

    /// Whether to offer proving the identity's account
    /// (`prepareShieldedIdentityAddress`).
    var offersPrepare: Bool { shown == .balance(.unverified) }

    var text: String {
        switch shown {
        case .notRead:
            return busy
                ? "The shielded store is busy. Reading again shortly."
                : "Reading the shielded balance…"
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
}

/// The tip sheet's state: amount, memo, review, send and its outcome. The
/// wallet calls come in as closures, so the mapping is tested without the
/// native library.
@MainActor
final class ShieldedTipFlow: ObservableObject {
    struct Actions {
        /// `PlatformWalletManager.parseShieldedTipAmount`.
        var parseAmount: @MainActor (String) throws -> UInt64
        /// `PlatformWalletManager.reviewShieldedTip` for the wallet.
        var review: @MainActor (String) async throws -> ShieldedTipReview
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

    private let actions: Actions
    /// Bumped by every input change and review, so a review that finishes
    /// after the inputs changed is dropped.
    private var generation = 0

    init(username: String?, actions: Actions) {
        self.username = username ?? ""
        self.usernameIsFixed = username != nil
        self.actions = actions
    }

    // MARK: Derived

    /// While a send runs, and after one that went or may have gone through,
    /// the inputs stay as they were sent.
    var inputsLocked: Bool {
        switch phase {
        case .sending, .finished(.submitted), .finished(.mayHaveGoneThrough):
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

    /// "Review again" is offered where a shielded sync may have cleared what
    /// held the tip. Never after a tip that may have gone through.
    var offersReviewAgain: Bool {
        switch phase {
        case .reviewed(let tip): return tip.review.unresolved
        case .finished(.earlierTipPending): return true
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
            let review = try await actions.review(name)
            guard current == generation else { return }
            phase = .reviewed(
                ShieldedTipReviewed(username: name, amount: credits, memo: memoText, review: review))
        } catch {
            guard current == generation else { return }
            phase = .reviewFailed(error.localizedDescription)
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
