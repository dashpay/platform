import Foundation
import SwiftData
import SwiftDashSDK

/// Available send flow types based on source and destination.
enum SendFlow: Equatable {
    case coreToCore              // Standard L1 payment
    case coreToShielded          // Shield Core L1 funds to a shielded recipient
    case platformToPlatform      // Platform-address → platform-address transfer
    case platformToShielded      // Shield credits
    case shieldedToShielded      // Private transfer
    case shieldedToPlatform      // Unshield
    case shieldedToCore          // Withdrawal from shielded to L1
    case identityToShielded      // Shield from an identity's credit balance (Type 21)

    /// Whether the flow spends this wallet's account-0 shielded notes.
    var spendsShieldedNotes: Bool {
        switch self {
        case .shieldedToShielded, .shieldedToPlatform, .shieldedToCore:
            return true
        // An identity shield spends identity credits, not notes.
        case .coreToCore, .coreToShielded, .platformToPlatform, .platformToShielded,
             .identityToShielded:
            return false
        }
    }

    var displayName: String {
        switch self {
        case .coreToCore: return "Core Payment"
        case .coreToShielded: return "Shield from Core"
        case .platformToPlatform: return "Platform Transfer"
        case .platformToShielded: return "Shield Credits"
        case .shieldedToShielded: return "Shielded Transfer"
        case .shieldedToPlatform: return "Unshield"
        case .shieldedToCore: return "Withdrawal to Core"
        case .identityToShielded: return "Shield from Identity"
        }
    }

    var iconName: String {
        switch self {
        case .coreToCore: return "arrow.right"
        case .coreToShielded: return "lock.shield"
        case .platformToPlatform: return "arrow.right"
        case .platformToShielded: return "lock.shield"
        case .shieldedToShielded: return "arrow.left.arrow.right"
        case .shieldedToPlatform: return "lock.open"
        case .shieldedToCore: return "arrow.down.to.line"
        case .identityToShielded: return "lock.shield"
        }
    }

    /// Static fee estimate. Authoritative only for the non-shielded
    /// flows (`coreToCore`, `platformToPlatform`); the shielded flows are
    /// consensus-pinned and resolved through the Rust FFI estimator in
    /// `SendViewModel.estimateFee(for:)`, so their values here are only
    /// the FFI-unavailable fallback (the real fee is ~500x larger).
    var estimatedFee: UInt64 {
        switch self {
        case .coreToCore: return 500_000             // ~0.005 DASH
        case .coreToShielded: return 200_000
        case .platformToPlatform: return 100_000_000 // ~0.001 DASH in credits
        case .platformToShielded: return 200_000
        case .shieldedToShielded: return 300_000
        case .shieldedToPlatform: return 300_000
        case .shieldedToCore: return 500_000
        // Never shown: this flow has no fee row and no Send until the
        // resolved floor is known (see `estimateFee(for:)`).
        case .identityToShielded: return 200_000
        }
    }
}

/// Fund source for sending.
enum FundSource: String, CaseIterable, Identifiable {
    case core = "Core"
    case shielded = "Shielded"
    case platform = "Platform"
    case identity = "Identity"

    var id: String { rawValue }

    var iconName: String {
        switch self {
        case .core: return "arrow.right"
        case .shielded: return "lock.shield"
        case .platform: return "square.stack.3d.up"
        case .identity: return "person.crop.circle"
        }
    }

    var color: SwiftUI.Color {
        switch self {
        case .core: return .green
        case .shielded: return .purple
        case .platform: return .blue
        case .identity: return .orange
        }
    }
}

import SwiftUI

/// One *extra* Core output row beyond the primary recipient. The
/// primary recipient stays on `recipientAddress` / `amountString` (it
/// drives `detectAddressType()` → `detectedFlow`, i.e. the Core /
/// Platform / Shielded routing); these are the additional outputs that
/// only the multi-recipient `coreToCore` flow appends. `id` is a stable
/// identity for `ForEach`/`@Published` diffing; `Equatable` keeps the
/// array cheap to diff.
struct CoreRecipient: Identifiable, Equatable {
    let id = UUID()
    var address: String = ""
    var amountString: String = ""
}

/// Per-source balances of the Send screen, the one input to
/// `SendViewModel.availableSources(_:)`. `platformAddresses` is the sum of
/// the wallet's platform-address balances only. The screen's "Platform"
/// balance row may fall back to identity credits before the first address
/// sync, but those credits can only be sent through the Identity source.
struct SendSourceBalances: Equatable {
    var core: UInt64
    /// Account 0's spendable credits as the wallet pushes them
    /// (`spendableShieldedCredits(walletId:account: 0)`); a balance that is
    /// not known yet is passed as 0, so the source is not offered.
    var shielded: UInt64
    var platformAddresses: UInt64
    /// Largest balance of any one identity: one identity funds a shield.
    var largestIdentity: UInt64
}

/// ViewModel for the Send Transaction screen.
@MainActor
class SendViewModel: ObservableObject {
    @Published var recipientAddress = "" {
        didSet { detectAddressType() }
    }
    @Published var amountString = ""
    /// Extra Core outputs beyond the primary recipient. Only populated
    /// (and only consulted) on the `coreToCore` flow — every other flow
    /// ignores it. Empty by default so the screen looks identical to the
    /// single-recipient form until the user taps "Add recipient".
    @Published var additionalCoreRecipients: [CoreRecipient] = []
    /// Optional UTF-8 memo for a shielded → shielded transfer. Only
    /// surfaced when the recipient is an Orchard address; Rust caps it
    /// at 32 UTF-8 bytes and does the 36-byte encoding.
    @Published var memoText = ""
    @Published var detectedAddressType: DashAddressType = .unknown
    /// The source the send is funded from. Changed only through
    /// [`selectSource(_:)`] (the user's tap) and
    /// [`reconcileSource(available:)`] (the screen's offered sources).
    @Published private(set) var selectedSource: FundSource = .core
    /// The sources the screen currently offers, as last passed to
    /// [`reconcileSource(available:)`]. A send is allowed only from one
    /// of these.
    @Published private(set) var offeredSources: [FundSource] = []
    /// Whether `selectedSource` is the user's explicit choice rather than
    /// the automatic default. An explicit choice is never replaced by
    /// another source: when it stops being offered it stays selected and
    /// Send is disabled until the user picks again. It ends only when the
    /// user picks another source or the recipient changes to a different
    /// known kind (Core, Orchard, Platform). Another address of the same
    /// kind keeps it, and so does passing through an unparsable or empty
    /// recipient while the user edits the address.
    private(set) var sourceChosenByUser = false
    /// The kind of the last recipient that parsed to a known kind.
    private var lastRecipientKind: DashAddressType.Kind?
    @Published var detectedFlow: SendFlow?
    @Published var estimatedFee: UInt64?
    @Published var isSending = false
    @Published var error: String?
    @Published var successMessage: String?
    /// A send that failed, presented by Rust's payment class
    /// (`PaymentFailure`): its title and text, and whether it may be
    /// retried. Never shown as a success. One that holds its slot
    /// (`PaymentFailure.holdsSlot`) stays stored after its alert is
    /// acknowledged, so `canSend` and `executeSend` refuse a second payment
    /// until the user leaves the form.
    @Published var paymentFailure: PaymentFailure?

    /// Whether the stored failure forbids another submission from this form.
    var isHeldByPaymentFailure: Bool { paymentFailure?.holdsSlot == true }

    /// Acknowledge the failure alert. Returns whether the form must close:
    /// a failure that holds its slot stays stored (the form stays refused)
    /// and the form closes; a `.notSent` one is cleared, so a corrected
    /// request may go.
    func acknowledgePaymentFailure() -> Bool {
        if isHeldByPaymentFailure { return true }
        paymentFailure = nil
        return false
    }
    /// The send was accepted but its effect is not confirmed yet (the
    /// identity shield: the wallet only verified the identity's balance at
    /// a block). Shown in its own "Submitted" alert, never as a success.
    @Published var submittedMessage: String?

    /// Whether the last send was an identity shield call after which the
    /// wallet may have saved a debit record (it saves one before it
    /// broadcasts). The screen then tells every open recovery list for the
    /// wallet to re-read the Rust records.
    private(set) var identityDebitMayHaveBeenSaved = false

    /// Per-output minimum credit amount (`min_output_amount`) the chain
    /// enforces for address-funds transitions, resolved on the Rust side from
    /// the wallet's current platform version and pushed in by the VIEW
    /// (`SendTransactionView.resolvePlatformLimits()`) on appear — the view
    /// model has no wallet handle of its own. A `platformToPlatform` transfer
    /// sends a single output, and DPP rejects any output below this floor, so
    /// `canSend` requires the requested credits to reach it.
    ///
    /// `nil` until the view resolves it (or if resolution fails). An
    /// unresolved floor keeps the `.platformToPlatform` Send gate CLOSED
    /// (never *under*-gates) — the same conservative treatment the dedicated
    /// `TransferPlatformAddressView` gives a nil `minOutputAmount`. Resolved
    /// via `ManagedPlatformAddressWallet.minOutputAmount()` rather than
    /// mirroring the protocol constant in Swift, which would drift if the
    /// version changed it. Only the platform path consults this; the
    /// core/shielded flows are unaffected.
    @Published var platformMinOutputAmount: UInt64?

    /// Consensus-pinned shielded fee estimates (credits, 2 actions) per fee
    /// kind, resolved through the wallet manager's network-tracked platform
    /// version and pushed in by the VIEW
    /// (`SendTransactionView.resolveShieldedFees()`) on appear, when the
    /// async protocol-version refresh publishes, and again from the Send
    /// action — the view model has no wallet handle of its own, same as
    /// `platformMinOutputAmount` above. A missing entry falls back to the
    /// static `SendFlow.estimatedFee` placeholder in `estimateFee(for:)`.
    @Published var shieldedFeeEstimates: [PlatformWalletManager.ShieldedFeeKind: UInt64] = [:]

    /// Identity funding an `.identityToShielded` send, and its credit
    /// balance. The view owns the identity rows, so it pushes both in
    /// (`SendTransactionView.syncSelectedIdentity()`), the same way it
    /// pushes `platformMinOutputAmount`.
    @Published var selectedIdentityId: Data?
    @Published var selectedIdentityBalance: UInt64 = 0

    private let network: Network

    init(network: Network) {
        self.network = network
    }

    /// Amount in duffs (1 DASH = 1e8). Used by core/L1 flows.
    /// Backed by `Decimal` parsing — typing 0.0001 deterministically
    /// yields exactly 10_000 duffs, not 9_999 or 10_001 depending on
    /// binary-float rounding.
    var amount: UInt64? {
        parseTokenAmount(amountString, decimals: 8)
    }

    /// Amount in platform credits (1 DASH = 1e11 credits). Used by
    /// every flow that touches the credits ledger
    /// (`platformToShielded`, `shieldedToShielded`,
    /// `shieldedToPlatform`, `shieldedToCore`,
    /// `platformToPlatform`). Same `Decimal`-backed parsing as
    /// `amount`; the divisor difference is just the `decimals` arg.
    var amountCredits: UInt64? {
        parseTokenAmount(amountString, decimals: 11)
    }

    /// Unit-explicit alias for [`amount`] — kept so the Core-side
    /// shielded send flows that read `amountDuffs` stay self-documenting
    /// (Core uses duffs; Platform / shielded use credits).
    var amountDuffs: UInt64? { amount }

    /// The recipient's 20-byte platform address hash, when the typed/scanned
    /// recipient resolves to a platform address (`detectedAddressType ==
    /// .platform`). `nil` for every other address type or a malformed
    /// payload.
    ///
    /// This is the SAME already-decoded payload `executeSend`'s
    /// `.platformToPlatform` branch reads — `detectedAddressType` is
    /// populated by `DashAddress.parse` (via the `recipientAddress` `didSet`),
    /// and the 21-byte platform payload is `[type byte] + [20-byte hash]`
    /// (see rs-dpp/src/address_funds/platform_address.rs). We slice the hash
    /// out here rather than re-running any address decoding, so the view can
    /// exclude an own-wallet recipient that collides with a candidate source
    /// input — mirroring `TransferPlatformAddressView.sourceInputHashes` and
    /// the Rust Auto selector, which forbid an address being both an input
    /// and an output of the same transfer.
    var platformRecipientHash: Data? {
        guard case .platform(let payload) = detectedAddressType,
              payload.count == 21 else { return nil }
        return payload.subdata(in: 1..<21)
    }

    // MARK: - Multi-recipient (coreToCore only)

    /// Append an empty extra Core output. The Rust coin-selector handles
    /// however many outputs we hand it, so there's no UI-side cap.
    func addCoreRecipient() {
        additionalCoreRecipients.append(CoreRecipient())
    }

    /// Remove the extra output with the given identity. No-op if it was
    /// already removed (e.g. a double-tap on the delete control).
    func removeCoreRecipient(_ id: CoreRecipient.ID) {
        additionalCoreRecipients.removeAll { $0.id == id }
    }

    /// Parse one extra row's amount to duffs through the same
    /// `Decimal`-backed path as the primary `amount` (1 DASH = 1e8). Kept
    /// as a single helper so every Core output — primary and extra — is
    /// scaled identically; reinventing the decimal parse per row would
    /// risk a row that rounds differently than the summary total.
    func duffs(forRecipientAmount amountString: String) -> UInt64? {
        parseTokenAmount(amountString, decimals: 8)
    }

    /// The validated Core batch for the `coreToCore` flow, built once: the
    /// ordered output list (primary + extras) AND its running duffs total.
    /// `coreRecipients` and `coreSendTotalDuffs` both derive from this so
    /// the gated/sent list and the displayed "Total" can never disagree —
    /// they come from a single iteration over the same rows.
    ///
    /// Returns `nil` (whole batch invalid) when ANY row is invalid OR the
    /// running total would overflow `UInt64`, so callers (gating + send)
    /// treat the batch atomically — a single bad extra row, or an
    /// aggregate that exceeds the duffs range, blocks the whole send
    /// rather than silently dropping an output or wrapping the total.
    ///
    /// "Valid" per row = the address parses as a `.core` address on
    /// `self.network` (same `DashAddress.parse` the primary row's
    /// detection uses, so a Platform/Orchard string in an extra row is
    /// rejected here) AND its duffs amount is `> 0` (a sub-unit amount
    /// scales to 0 and would reach Rust as a zero-value output).
    ///
    /// Overflow rationale: each row's amount is independently a valid
    /// `UInt64`, but two valid amounts can still sum past `UInt64.max`.
    /// Dash's total supply is far below that, so this is only reachable
    /// from raw user input, but a bare `+` would trap in debug / wrap in
    /// release the moment the summary renders. Accumulating with
    /// `addingReportingOverflow` and treating overflow as "batch invalid"
    /// keeps the failure mode identical to a bad address or zero amount.
    private var coreRecipientPlan:
        (outputs: [(address: String, amountDuffs: UInt64)], total: UInt64)? {
        var outputs: [(address: String, amountDuffs: UInt64)] = []
        var total: UInt64 = 0

        // Accumulate `duffs` into `total`, rejecting the whole batch on
        // overflow (see the property's overflow rationale).
        func add(_ address: String, _ duffs: UInt64) -> Bool {
            let (sum, overflow) = total.addingReportingOverflow(duffs)
            if overflow { return false }
            total = sum
            outputs.append((address: address, amountDuffs: duffs))
            return true
        }

        // Primary row. The trim mirrors the existing `.coreToCore` send
        // case so the marshalled address matches what the badge validated.
        let primaryAddress = recipientAddress
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard isCoreAddress(primaryAddress),
              let primaryDuffs = amountDuffs, primaryDuffs > 0,
              add(primaryAddress, primaryDuffs)
        else { return nil }

        // Extra rows, in order.
        for row in additionalCoreRecipients {
            let address = row.address
                .trimmingCharacters(in: .whitespacesAndNewlines)
            guard isCoreAddress(address),
                  let rowDuffs = duffs(forRecipientAmount: row.amountString),
                  rowDuffs > 0,
                  add(address, rowDuffs)
            else { return nil }
        }

        return (outputs, total)
    }

    /// The full, ordered Core output list for the `coreToCore` flow:
    /// the PRIMARY row (`recipientAddress` + `amountDuffs`) followed by
    /// each `additionalCoreRecipients` row, in display order. `nil` when
    /// the batch isn't fully valid — see `coreRecipientPlan`.
    var coreRecipients: [(address: String, amountDuffs: UInt64)]? {
        coreRecipientPlan?.outputs
    }

    /// Sum of every valid Core output (primary + additional), in duffs —
    /// the "Total" shown in the multi-output summary. Returns 0 when the
    /// batch isn't fully valid (`coreRecipientPlan` is `nil`, including the
    /// overflow case), matching the disabled-Send state so the summary
    /// never shows a total for a batch that can't be sent. Computed with
    /// checked addition (no bare `+`) in `coreRecipientPlan`.
    var coreSendTotalDuffs: UInt64 {
        coreRecipientPlan?.total ?? 0
    }

    /// Whether a trimmed string parses as a `.core` address on this
    /// view model's network. Wraps the same `DashAddress.parse` the
    /// primary-row detection uses so Core-address validity is judged
    /// identically for every output.
    private func isCoreAddress(_ trimmed: String) -> Bool {
        guard !trimmed.isEmpty else { return false }
        if case .core = DashAddress.parse(trimmed, network: network).type {
            return true
        }
        return false
    }

    /// Minimum L1 lock size (in duffs) for a `coreToShielded` send.
    /// Mirrors `ShieldedFundFromAssetLockView.minDuffs` (1 mDASH): an
    /// asset lock smaller than the Platform pool fee would build a lock
    /// and a ~30s Halo 2 proof only to be rejected at submission, so we
    /// gate it up front rather than burn the work.
    static let minShieldFromCoreDuffs: UInt64 = 100_000

    /// Maximum UTF-8 byte length of a shielded memo (the 32-byte
    /// payload of the 36-byte `DashMemo`; the other 4 bytes are the
    /// kind tag). Mirrors `dpp::shielded::MEMO_PAYLOAD_SIZE`.
    static let memoByteLimit = 32

    /// UTF-8 byte length of the trimmed memo — what Rust validates
    /// against the 32-byte limit, not the character count.
    var memoByteCount: Int {
        memoText.trimmingCharacters(in: .whitespacesAndNewlines).utf8.count
    }

    /// Whether the memo exceeds the 32-byte payload limit. Blocks Send.
    var isMemoOverLimit: Bool {
        memoByteCount > Self.memoByteLimit
    }

    var canSend: Bool {
        guard let flow = detectedFlow, !isSending, !isHeldByPaymentFailure else { return false }
        // A source the screen no longer offers (its balance went away, or
        // it was never offered) must not be sent from.
        guard offeredSources.contains(selectedSource) else { return false }
        // An over-limit memo would be rejected by Rust; block here so
        // the user sees the red counter rather than a backend error.
        // Only the shielded → shielded path carries a memo, so a stale
        // over-limit memo must not block the other flows (where the
        // field is hidden and the text is ignored).
        if flow == .shieldedToShielded && isMemoOverLimit { return false }
        // Gate on the scaled integer for the *active* flow's unit, not
        // just non-nil. A sub-unit amount (e.g. "0.000000001" in DASH)
        // parses to 0 once scaled; sending that reaches the backend as a
        // zero-value transfer. Core/L1 settles in duffs (1e8); every
        // credits-ledger flow settles in credits (1e11).
        switch flow {
        case .coreToCore:
            // Gate on the *whole* batch, not just the primary row: the
            // multi-output send is atomic (one tx), so every extra row's
            // address must be a Core address on-network and every extra
            // amount > 0. `coreRecipients` already encodes "all rows
            // valid (incl. the primary's amountDuffs > 0), else nil", so a
            // non-nil list means the batch is sendable. With zero extra
            // rows this reduces to the prior primary-only check.
            return coreRecipients != nil
        case .coreToShielded:
            // Funded by an L1 asset lock denominated in duffs; gate on
            // the lock floor so a doomed (sub-fee) amount can't kick off
            // the lock-build + proof pipeline.
            return (amountDuffs ?? 0) >= Self.minShieldFromCoreDuffs
        case .platformToPlatform:
            // An address-funds transfer sends exactly one output, and DPP
            // rejects any output below `min_output_amount`. Gate on the
            // version-locked floor (resolved Rust-side and pushed in by the
            // view) so the button reflects what DPP will accept, rather than
            // only `> 0`, which would enable a sub-minimum amount that fails
            // structure validation after submit — matching the dedicated
            // `TransferPlatformAddressView`. An unresolved floor (`nil`) keeps
            // the gate CLOSED (never *under*-gates); it loads on appear.
            guard let minOutput = platformMinOutputAmount else { return false }
            return (amountCredits ?? 0) >= minOutput
        case .platformToShielded,
             .shieldedToShielded, .shieldedToPlatform, .shieldedToCore:
            return (amountCredits ?? 0) > 0
        case .identityToShielded:
            return identityShieldIsAffordable
        }
    }

    /// Send gate for `.identityToShielded`: an identity is selected, the
    /// amount is > 0, the fee floor is resolved, and amount + floor fits the
    /// identity's balance (overflow-checked). The wallet arms its retry guard
    /// for a Type 21 shield before the chain checks the balance, so a request
    /// the chain refuses would block further shields from that identity until
    /// the user abandons it. An unresolved floor keeps the gate closed.
    var identityShieldIsAffordable: Bool {
        guard selectedIdentityId != nil,
              !identityShieldAmountTooPrecise,
              let feeFloor = shieldedFeeEstimates[.shieldFromIdentity],
              let amount = amountCredits, amount > 0
        else { return false }
        let (total, overflow) = amount.addingReportingOverflow(feeFloor)
        return !overflow && total <= selectedIdentityBalance
    }

    /// Sources offered for the screen's balances. The Platform source counts
    /// platform-address funds only: identity credits fund the Identity source.
    func availableSources(_ balances: SendSourceBalances) -> [FundSource] {
        availableSources(
            coreBalance: balances.core,
            shieldedBalance: balances.shielded,
            platformBalance: balances.platformAddresses,
            identityBalance: balances.largestIdentity
        )
    }

    /// The typed amount has a nonzero digit beyond one credit, which the
    /// credits parser would round down. Only the identity shield refuses it;
    /// the other flows keep their existing parsing.
    var identityShieldAmountTooPrecise: Bool {
        hasDigitsBeyondPrecision(amountString, decimals: 11)
    }

    /// Determine which fund sources are available based on destination and balances.
    /// `identityBalance` is the largest balance of any one identity of the
    /// wallet, since a single identity funds an identity shield.
    private func availableSources(
        coreBalance: UInt64,
        shieldedBalance: UInt64,
        platformBalance: UInt64,
        identityBalance: UInt64
    ) -> [FundSource] {
        var sources: [FundSource] = []
        switch detectedAddressType {
        case .core:
            if coreBalance > 0 { sources.append(.core) }
            if shieldedBalance > 0 { sources.append(.shielded) }
        case .orchard:
            if shieldedBalance > 0 { sources.append(.shielded) }
            if platformBalance > 0 { sources.append(.platform) }
            // Core L1 → shielded recipient via a Type 18
            // ShieldFromAssetLock. Appended last so the private
            // shielded → shielded path stays the auto-selected default.
            if coreBalance > 0 { sources.append(.core) }
            // Type 21 ShieldFromIdentity, last for the same reason.
            if identityBalance > 0 { sources.append(.identity) }
        case .platform:
            if platformBalance > 0 { sources.append(.platform) }
            if shieldedBalance > 0 { sources.append(.shielded) }
        case .unknown:
            break
        }
        return sources
    }

    // MARK: - Address Detection

    func detectAddressType() {
        let trimmed = recipientAddress.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            detectedAddressType = .unknown
            detectedFlow = nil
            estimatedFee = nil
            return
        }

        let parsed = DashAddress.parse(trimmed, network: network)
        noteRecipientKind(parsed.type.kind)
        detectedAddressType = parsed.type
        updateFlow()
    }

    /// A recipient of a different known kind offers different sources,
    /// so the next reconcile picks that kind's default again. An unknown
    /// recipient is an address being edited, not a new kind.
    private func noteRecipientKind(_ kind: DashAddressType.Kind) {
        guard kind != .unknown else { return }
        if let lastRecipientKind, lastRecipientKind != kind {
            sourceChosenByUser = false
        }
        lastRecipientKind = kind
    }

    // MARK: - Source selection

    /// The user picked `source` from the offered list.
    func selectSource(_ source: FundSource) {
        guard offeredSources.contains(source) else { return }
        selectedSource = source
        sourceChosenByUser = true
        updateFlow()
    }

    /// Apply the screen's currently offered sources (in their default
    /// order). Call it whenever the offered list changes, for example when
    /// a balance that gates a source arrives after the recipient was
    /// typed. Without an explicit choice the first offered source is
    /// selected. An explicit choice stays selected whether or not it is
    /// offered: a send never moves to a source the user did not pick (a
    /// shielded payment silently becoming a transparent one).
    func reconcileSource(available: [FundSource]) {
        offeredSources = available
        if !sourceChosenByUser, let first = available.first {
            selectedSource = first
        }
        updateFlow()
    }

    /// Whether the user's explicit choice is not offered right now for a
    /// recipient the screen understands. The screen says so and Send
    /// stays disabled until the user picks an offered source.
    var chosenSourceUnavailable: Bool {
        sourceChosenByUser
            && detectedAddressType.kind != .unknown
            && !offeredSources.contains(selectedSource)
    }

    static let routeChangedMessage =
        "The payment route or recipient changed after Send was tapped. Review it and tap Send again."

    static let identityChangedMessage =
        "The source identity changed after Send was tapped. Review it and tap Send again."

    static let amountChangedMessage =
        "The amount changed after Send was tapped. Review it and tap Send again."

    /// The flow a tap may dispatch, or `nil` after setting the reason.
    /// A send already running admits nothing and changes no state (a second
    /// tap before the disabled state renders must not reset the first). The
    /// route, recipient and amount must still be the ones tapped, so every
    /// flow sends the tapped amount.
    func admitSend(
        tappedFlow: SendFlow?,
        tappedRecipient: String,
        tappedAmountText: String,
        accountZeroShieldedBalance: UInt64?
    ) -> SendFlow? {
        guard !isSending else { return nil }
        guard let flow = flowToSend(tappedFlow: tappedFlow, tappedRecipient: tappedRecipient),
              Self.mayDispatch(flow, accountZeroShieldedBalance: accountZeroShieldedBalance)
        else {
            error = Self.routeChangedMessage
            return nil
        }
        guard amountString == tappedAmountText else {
            error = Self.amountChangedMessage
            return nil
        }
        return flow
    }

    /// The identity an identity shield debits: the one selected when Send
    /// was tapped, and only while it is still the selected one, so a pick
    /// made after the tap never changes which identity pays.
    func identityToSend(tappedIdentityId: Data?) -> Data? {
        guard let tappedIdentityId, tappedIdentityId == selectedIdentityId else { return nil }
        return tappedIdentityId
    }

    /// Whether `flow` may be dispatched given the account-0 shielded
    /// balance read from the published wallet state at dispatch time. A
    /// shielded spend needs a known, non-zero balance; the offered list
    /// can lag behind a balance update that landed after the tap. Other
    /// flows do not spend shielded notes and are not affected.
    nonisolated static func mayDispatch(_ flow: SendFlow, accountZeroShieldedBalance: UInt64?) -> Bool {
        guard flow.spendsShieldedNotes else { return true }
        guard let balance = accountZeroShieldedBalance else { return false }
        return balance > 0
    }

    /// The flow to send when Send was tapped while `tappedFlow` was shown
    /// for `tappedRecipient`: `nil` unless that is still the current flow,
    /// its source is still offered, and the recipient text is unchanged,
    /// so an asynchronous send never runs a route or pays a recipient
    /// other than the one the user confirmed.
    func flowToSend(tappedFlow: SendFlow?, tappedRecipient: String) -> SendFlow? {
        guard let tappedFlow, tappedFlow == detectedFlow,
              recipientAddress == tappedRecipient,
              offeredSources.contains(selectedSource) else { return nil }
        return tappedFlow
    }

    func updateFlow() {
        guard offeredSources.contains(selectedSource) else {
            // A source that is not offered has no route to show or send.
            detectedFlow = nil
            estimatedFee = nil
            return
        }
        switch (detectedAddressType, selectedSource) {
        case (.core, .core):
            detectedFlow = .coreToCore
        case (.core, .shielded):
            detectedFlow = .shieldedToCore
        case (.orchard, .shielded):
            detectedFlow = .shieldedToShielded
        case (.orchard, .platform):
            detectedFlow = .platformToShielded
        case (.orchard, .core):
            detectedFlow = .coreToShielded
        case (.orchard, .identity):
            detectedFlow = .identityToShielded
        case (.platform, .platform):
            detectedFlow = .platformToPlatform
        case (.platform, .shielded):
            detectedFlow = .shieldedToPlatform
        default:
            detectedFlow = nil
        }
        estimatedFee = detectedFlow.flatMap(estimateFee(for:))
    }

    /// Resolve the estimated fee (in the flow's settlement unit) for the
    /// active flow. The shielded flows are consensus-pinned and computed
    /// in Rust (`compute_*_shielded_fee` via the FFI estimator, at the
    /// manager's network-tracked platform version), so this reads the
    /// view-pushed `shieldedFeeEstimates` rather than re-deriving the
    /// constants in Swift.
    ///
    /// The estimates are for 2 Orchard actions — the exact action count
    /// isn't known until the builder selects notes; a single-note spend
    /// with change (the common case) serializes to 2 actions. The
    /// transparent `Shield` (`platformToShielded`) reserves the same
    /// `compute_minimum_shielded_fee(2)` base as its structure-check
    /// minimum, so it shares the transfer kind. When the view hasn't
    /// resolved a fee (or the FFI errored) we fall back to the static enum
    /// placeholder rather than surfacing a fee of nil for a flow we can
    /// otherwise send. `.identityToShielded` is the exception: its floor
    /// gates Send, so it has no placeholder and stays `nil` until resolved.
    private func estimateFee(for flow: SendFlow) -> UInt64? {
        let kind: PlatformWalletManager.ShieldedFeeKind?
        switch flow {
        case .identityToShielded:
            return shieldedFeeEstimates[.shieldFromIdentity]
        case .shieldedToShielded, .platformToShielded, .coreToShielded:
            // Type 18 ShieldFromAssetLock carves the same
            // `compute_minimum_shielded_fee` base as the Shield/transfer
            // path; its fee is denominated in credits (deducted from the
            // locked value), so the view renders it with the credits unit.
            kind = .transfer
        case .shieldedToPlatform:
            kind = .unshield
        case .shieldedToCore:
            kind = .withdrawal
        case .coreToCore, .platformToPlatform:
            kind = nil
        }
        guard let kind else { return flow.estimatedFee }
        return shieldedFeeEstimates[kind] ?? flow.estimatedFee
    }

    // MARK: - Send Execution

    func executeSend(
        sdk: SDK,
        walletManager: PlatformWalletManager,
        platformState: AppState,
        wallet: PersistentWallet,
        platformWallet: ManagedPlatformWallet?,
        platformAddressWallet: ManagedPlatformAddressWallet?,
        signer: KeychainSigner?,
        senderAccountIndex: UInt32,
        modelContext: ModelContext,
        tappedFlow: SendFlow?,
        tappedRecipient: String,
        tappedIdentityId: Data?,
        tappedAmountText: String
    ) async {
        // A failure that may have left the wallet or holds a lock stays
        // stored: never pay again from this form.
        guard !isHeldByPaymentFailure else { return }
        guard let flow = admitSend(
            tappedFlow: tappedFlow,
            tappedRecipient: tappedRecipient,
            tappedAmountText: tappedAmountText,
            accountZeroShieldedBalance: walletManager.spendableShieldedCredits(
                walletId: wallet.walletId, account: 0)
        ) else { return }

        isSending = true
        error = nil
        successMessage = nil
        paymentFailure = nil
        submittedMessage = nil
        identityDebitMayHaveBeenSaved = false
        defer { isSending = false }

        do {
            switch flow {
            case .coreToCore:
                guard let platformWallet else {
                    error = "Wallet not available"
                    return
                }
                // `coreRecipients` is nil unless every row is a valid
                // on-network Core address with a > 0 duffs amount — the
                // same condition `canSend` gates on, re-checked here so a
                // stale enabled-Send tap can't slip an invalid batch
                // through.
                guard let recipients = coreRecipients else {
                    error = "Invalid recipient or amount"
                    return
                }
                // Coin selection, funding, and signing are Rust-side; we
                // marshal the outputs into the builder, fund + sign from
                // the sender account, then broadcast the signed tx. The
                // signed tx carries its funding account, so a failed
                // broadcast releases its UTXO reservation for retry. The
                // builder validates addresses against `self.network`, which
                // the Rust side re-checks against the wallet's own network.
                let builder = try CoreTransactionBuilder(network: network)
                for recipient in recipients {
                    try builder.addOutput(
                        address: recipient.address,
                        amountDuffs: recipient.amountDuffs
                    )
                }
                let signedTx = try builder.finalizeAtomic(
                    wallet: platformWallet,
                    accountType: .bip44,
                    accountIndex: senderAccountIndex
                )
                // Core acceptance, rather than a successful peer socket write,
                // is the boundary for showing payment success.
                let outcome = try platformWallet.coreWallet()
                    .broadcastTransactionWithOutcome(signedTx)
                applyCoreBroadcastOutcome(outcome, recipientCount: recipients.count)

            case .platformToPlatform:
                guard let addressWallet = platformAddressWallet else {
                    error = "Platform address wallet not available"
                    return
                }
                guard let signer = signer else {
                    error = "Signer not available"
                    return
                }
                guard case .platform(let payload) = detectedAddressType else {
                    error = "Recipient is not a platform address"
                    return
                }
                guard payload.count == 21 else {
                    error = "Platform address must be 21 bytes (got \(payload.count))"
                    return
                }
                guard let credits = amountCredits else {
                    error = "Invalid amount"
                    return
                }
                // Map bech32m wire byte → FFI storage discriminant.
                // See rs-dpp/src/address_funds/platform_address.rs:41-47.
                let bech32mByte = payload[0]
                let ffiAddressType: UInt8
                switch bech32mByte {
                case 0xb0: ffiAddressType = 0  // P2PKH
                case 0x80: ffiAddressType = 1  // P2SH
                default:
                    error = "Unknown platform address type byte 0x\(String(bech32mByte, radix: 16))"
                    return
                }
                // The Rust FFI's `PlatformAddressFFI → PlatformAddress`
                // conversion (rs-platform-wallet-ffi/src/platform_address_types.rs,
                // `impl TryFrom<PlatformAddressFFI>`) accepts P2PKH only;
                // sending to a P2SH platform address would surface a
                // P2SH-specific rejection from Rust. Fail fast here with a
                // user-readable message instead.
                guard ffiAddressType == 0 else {
                    error = "P2SH platform addresses aren't supported yet. Use a P2PKH recipient."
                    return
                }
                let hash = payload.subdata(in: 1..<21)
                let output = ManagedPlatformAddressWallet.TransferOutput(
                    addressType: ffiAddressType,
                    hash: hash,
                    credits: credits
                )
                // Input selection, fee strategy, and the surplus (left on
                // the source addresses in the credit-balance model) are all
                // owned by the Rust Auto path — no change address to pass.
                let updated = try await addressWallet.transfer(
                    accountIndex: senderAccountIndex,
                    outputs: [output],
                    signer: signer
                )

                // Belt-and-suspenders: apply the post-broadcast
                // balances/nonces returned by `transfer` to SwiftData
                // directly. The Rust side already pushes the same
                // changeset through the persister, so this loop is
                // idempotent (same hash → same balance/nonce), but
                // doing it here too keeps the @Query-bound
                // PersistentPlatformAddress rows fresh even if the
                // persister callback ordering ever changes.
                //
                // Mirrors PlatformWalletPersistenceHandler.persistAddressBalances:
                // fetch each row by `walletId + addressHash`, update the
                // volatile fields, stamp `lastUpdated`. Scope by `walletId`
                // too (mirroring the dedicated transfer sheet): a hash-only
                // predicate can match another wallet's row in a multi-wallet
                // store. Every entry returned was touched by the transition,
                // so `isUsed = true` unconditionally. Rows that aren't found
                // are silently skipped — same defensive shape the BLAST
                // handler uses.
                let walletId = wallet.walletId
                for entry in updated {
                    let entryHash = entry.hash
                    let descriptor = FetchDescriptor<PersistentPlatformAddress>(
                        predicate: #Predicate {
                            $0.walletId == walletId && $0.addressHash == entryHash
                        }
                    )
                    guard let row = try? modelContext.fetch(descriptor).first else {
                        continue
                    }
                    row.balance = entry.balance
                    row.nonce = entry.nonce
                    row.isUsed = true
                    row.lastUpdated = Date()
                }
                // The transfer has ALREADY succeeded on-chain by this point,
                // and a DIP-17 resync corrects balances regardless. So a
                // local SwiftData `save()` failure must NOT be reported as
                // the transfer having failed (that would make the user
                // think credits didn't move when they did) — but it also
                // must not be silently swallowed. Keep the SUCCESS message
                // and append a non-fatal caveat noting balances will refresh
                // on the next sync.
                do {
                    try modelContext.save()
                    successMessage = "Platform transfer sent"
                } catch {
                    successMessage = "Platform transfer sent. Local balances "
                        + "couldn't be updated — they'll refresh on the next "
                        + "sync: \(error.localizedDescription)"
                }

            case .shieldedToShielded:
                // Shielded → Shielded: spend notes from this
                // wallet's shielded balance, create a new note
                // for the recipient. Amount is in **credits**
                // (1 DASH = 1e11) — the entire shielded ledger
                // works on the credits scale.
                guard let amountCredits else {
                    error = "Invalid amount"
                    return
                }
                let trimmed = recipientAddress.trimmingCharacters(in: .whitespacesAndNewlines)
                let parsed = DashAddress.parse(trimmed, network: network)
                guard case .orchard(let recipientRaw) = parsed.type else {
                    error = "Recipient is not a shielded address"
                    return
                }
                let trimmedMemo = memoText.trimmingCharacters(in: .whitespacesAndNewlines)
                // Per-operation spend authority: the resolver fires
                // exactly for this spend (launch binds are
                // viewing-key only and never read the mnemonic).
                try await walletManager.shieldedTransfer(
                    walletId: wallet.walletId,
                    resolver: MnemonicResolver(),
                    account: 0,
                    recipientRaw43: recipientRaw,
                    amount: amountCredits,
                    memo: trimmedMemo.isEmpty ? nil : trimmedMemo
                )
                successMessage = "Shielded transfer complete"

            case .shieldedToPlatform:
                // Shielded → Platform: spend notes, credit the
                // platform address (also credits scale). The
                // bech32m string is forwarded as-is — Rust parses
                // it via `PlatformAddress::from_bech32m_string`
                // and verifies the network.
                guard let amountCredits else {
                    error = "Invalid amount"
                    return
                }
                let trimmed = recipientAddress.trimmingCharacters(in: .whitespacesAndNewlines)
                try await walletManager.shieldedUnshield(
                    walletId: wallet.walletId,
                    resolver: MnemonicResolver(),
                    account: 0,
                    toPlatformAddress: trimmed,
                    amount: amountCredits
                )
                successMessage = "Unshield complete"

            case .shieldedToCore:
                // Shielded → Core L1: spend notes (credits), create
                // an L1 withdrawal. The shielded-side amount is in
                // credits; the network converts to L1 duffs at the
                // 1000:1 conversion rate.
                guard let amountCredits else {
                    error = "Invalid amount"
                    return
                }
                let trimmed = recipientAddress.trimmingCharacters(in: .whitespacesAndNewlines)
                try await walletManager.shieldedWithdraw(
                    walletId: wallet.walletId,
                    resolver: MnemonicResolver(),
                    account: 0,
                    toCoreAddress: trimmed,
                    amount: amountCredits,
                    coreFeePerByte: 1
                )
                successMessage = "Withdrawal submitted"

            case .platformToShielded:
                // Platform → Shielded (Type 15): spend credits from
                // the wallet's first Platform Payment account into
                // the bound shielded pool. Credits scale.
                guard let amountCredits else {
                    error = "Invalid amount"
                    return
                }
                _ = platformState
                _ = sdk
                if let message = selfShieldRecipientError(
                    walletManager: walletManager,
                    walletId: wallet.walletId
                ) {
                    error = message
                    return
                }
                let signer = KeychainSigner(modelContainer: modelContext.container)
                try await walletManager.shieldedShield(
                    walletId: wallet.walletId,
                    shieldedAccount: 0,
                    paymentAccount: 0,
                    amount: amountCredits,
                    addressSigner: signer
                )
                successMessage = "Shielding complete"

            case .coreToShielded:
                // Core L1 → Shielded recipient (Type 18
                // ShieldFromAssetLock): lock L1 duffs, then the
                // network mints the locked value (minus the pool fee)
                // as an Orchard note for the recipient. The typed
                // amount is the **L1 lock size in duffs** — the funds
                // leaving the wallet — and the recipient receives
                // `lock_value − pool_fee` in credits. We mirror
                // ShieldedFundFromAssetLockView's "lock size, not net
                // amount" convention rather than grossing up the fee:
                // Type 18's Orchard value_balance is baked into the
                // Halo 2 proof at build time and can't be re-derived
                // afterward.
                //
                // Note: this awaits the whole asset-lock pipeline
                // (build lock → wait IS-lock/ChainLock → ~30s proof →
                // submit ST), so the "Sending…" overlay can sit for a
                // minute+. The dedicated ShieldedFundFromAssetLockView
                // exposes the staged progress UI; this is the simple
                // inline path. It bypasses the shield coordinator, but
                // the Rust `shield_guard` mutex still serializes
                // shield-class ops per wallet, so concurrent attempts
                // block rather than corrupt.
                guard let amountDuffs else {
                    error = "Invalid amount"
                    return
                }
                let trimmed = recipientAddress
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                let parsed = DashAddress.parse(trimmed, network: network)
                guard case .orchard(let recipientRaw) = parsed.type else {
                    error = "Recipient is not a shielded address"
                    return
                }
                // Asset locks draw UTXOs from a SINGLE Core account, so
                // fund from the standard BIP44 account (typeTag/standardTag
                // 0) with the largest confirmed balance. The screen's
                // "Core" total sums across accounts, so an amount under
                // the displayed total can still fail here when the balance
                // is split across several accounts.
                let funding = walletManager.accountBalances(for: wallet.walletId)
                    .filter { $0.typeTag == 0 && $0.standardTag == 0 }
                    .max(by: { $0.confirmed < $1.confirmed })
                guard let funding, funding.confirmed > 0 else {
                    error = "No spendable Core account to fund the shield"
                    return
                }
                try await walletManager.shieldedFundFromAssetLock(
                    walletId: wallet.walletId,
                    fundingAccountIndex: funding.index,
                    amountDuffs: amountDuffs,
                    recipients: [
                        ShieldedFundFromAssetLockRecipient(recipientRaw43: recipientRaw)
                    ]
                )
                successMessage = "Shield submitted — the note arrives on "
                    + "the recipient's next shielded sync."

            case .identityToShielded:
                guard let identityId = identityToSend(tappedIdentityId: tappedIdentityId) else {
                    error = Self.identityChangedMessage
                    return
                }
                // Re-checked here so a stale enabled tap cannot reach the
                // wallet with an amount the identity cannot cover.
                guard identityShieldIsAffordable,
                      let amountCredits
                else {
                    error = "Select an identity whose balance covers the "
                        + "amount plus the estimated fee"
                    return
                }
                if let message = selfShieldRecipientError(
                    walletManager: walletManager,
                    walletId: wallet.walletId
                ) {
                    error = message
                    return
                }
                // Signs with the identity's TRANSFER key.
                let identitySigner = KeychainSigner(modelContainer: modelContext.container)
                do {
                    try await walletManager.shieldedShieldFromIdentity(
                        walletId: wallet.walletId,
                        shieldedAccount: 0,
                        identityId: identityId,
                        amount: amountCredits,
                        identitySigner: identitySigner
                    )
                    identityDebitMayHaveBeenSaved = Self.identityShieldMayHaveSavedDebit(after: nil)
                } catch {
                    identityDebitMayHaveBeenSaved = Self.identityShieldMayHaveSavedDebit(after: error)
                    throw error
                }
                applyIdentityShieldSubmitted()
            }

        } catch {
            applySendFailure(error, flow: flow)
        }
    }

    /// Whether the wallet may have saved an identity debit record after a
    /// `shieldedShieldFromIdentity` call that threw `error` (`nil`: it
    /// returned normally, so a record was saved). Only the refusals the
    /// wallet makes before saving anything say no; every other outcome may
    /// follow a saved record, so the recovery lists must re-read.
    nonisolated static func identityShieldMayHaveSavedDebit(after error: Error?) -> Bool {
        guard let error else { return true }
        guard let walletError = error as? PlatformWalletError else { return true }
        switch walletError {
        case .shieldedIdentityDebitPending, .insufficientIdentityCredits,
             .identityBalanceUnavailable, .protocolVersionNotActive,
             .sdkPinnedBelowProtocolVersion, .invalidParameter:
            return false
        default:
            return true
        }
    }

    /// Outcome of a normal return from `shieldedShieldFromIdentity`. That
    /// return is not proof the debit executed: the wallet verified the
    /// identity's balance at a block, and the activity row stays pending
    /// until the scan sees the note. So it is "submitted", not a success.
    func applyIdentityShieldSubmitted() {
        error = nil
        successMessage = nil
        paymentFailure = nil
        submittedMessage = Self.identityShieldSubmittedMessage
    }

    /// Route a failed send to the right outcome.
    ///
    /// Every flow (Core, Platform, shielded, the identity shield) presents
    /// its failure by Rust's payment class (`PaymentFailure`), never by the
    /// error's case or code: a retry only for a retryable `.notSent`, the
    /// held lock's resume for `.lockHeld`, and a wait for a payment that may
    /// have left (never "failed"). An identity shield refused with nothing
    /// sent keeps this flow's wording (`identityShieldRefusalMessage`) as the
    /// failure's text. An error no payment raised (an input check before any
    /// send) shows as its own message.
    func applySendFailure(_ failure: Error, flow: SendFlow) {
        successMessage = nil
        submittedMessage = nil
        guard let classified = PaymentFailure.ofPayment(failure) else {
            error = failure.localizedDescription
            return
        }
        if flow == .identityToShielded,
           let walletError = failure as? PlatformWalletError,
           case .notSent? = walletError.delivery,
           let message = Self.identityShieldRefusalMessage(walletError) {
            paymentFailure = PaymentFailure(payment: walletError.payment, detail: message)
            return
        }
        paymentFailure = classified
    }

    /// Wallet refusals of an identity shield that happen before anything is
    /// saved, built or sent, with wording for this flow. `nil` for any other
    /// error, whose own description is shown.
    nonisolated static func identityShieldRefusalMessage(_ error: PlatformWalletError) -> String? {
        switch error {
        case .shieldedIdentityDebitPending:
            return identityDebitPendingMessage
        case .insufficientIdentityCredits(_, let required, let available, _):
            return "The identity's balance on Platform is too low for this shield: "
                + "it needs \(formatCredits(required)) including the fee and has "
                + "\(formatCredits(available)). Nothing was sent."
        case .identityBalanceUnavailable:
            return "The identity's balance could not be read from Platform, so "
                + "the shield was not attempted. Nothing was sent."
        case .protocolVersionNotActive:
            return "This network has not activated shielding from an identity yet. "
                + "Nothing was sent."
        case .sdkPinnedBelowProtocolVersion:
            return "This app's SDK is pinned below the protocol version that "
                + "shields from an identity. Nothing was sent."
        default:
            return nil
        }
    }

    nonisolated static func formatCredits(_ credits: UInt64) -> String {
        formatCreditsAsDash(credits)
    }

    /// Whether a shielded operation's error means it may already be on
    /// chain, so no retry is offered: Rust classed the payment as maybe sent
    /// (a spend whose result is not proved, a state transition the SDK could
    /// not confirm, any other verdict no proof backs) or as holding an asset
    /// lock. Read from the error's payment class, never from its case.
    nonisolated static func mayHaveGoneThrough(_ error: PlatformWalletError) -> Bool {
        switch error.delivery {
        case .maybeSent, .lockHeld: return true
        case .notSent, nil: return false
        }
    }

    // MARK: - Self-shield recipient

    /// Error for a self-shield (`.platformToShielded`, `.identityToShielded`)
    /// whose recipient is not this wallet's own default Orchard address, or
    /// `nil` when it is. Neither wrapper takes a recipient: Rust always
    /// shields into shielded account 0's default address, so another typed
    /// address would report success while nothing reached it.
    ///
    /// The own address is resolved per wallet from the engine, not from the
    /// single-mirror `shieldedService` (which tracks `firstWallet`). A blank
    /// recipient never gets here (it clears `detectedFlow`), so the message
    /// asks for the own address instead of suggesting leaving it blank.
    private func selfShieldRecipientError(
        walletManager: PlatformWalletManager,
        walletId: Data
    ) -> String? {
        Self.selfShieldRecipientProblem(
            entered: recipientAddress,
            ownAddress: walletManager.shieldedDisplayAddress(
                walletId: walletId,
                network: network
            )
        )
    }

    nonisolated static let selfShieldRecipientMessage =
        "Shield always sends to your own shielded address; enter your own "
        + "shielded address as the recipient"

    /// Why `entered` is not this wallet's own shielded address, or `nil`.
    /// Bech32m decodes case-insensitively, so case does not matter; an empty
    /// recipient and an unknown own address are refused.
    nonisolated static func selfShieldRecipientProblem(entered: String, ownAddress: String?) -> String? {
        let enteredRecipient = entered.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !enteredRecipient.isEmpty,
              let ownAddress,
              enteredRecipient.lowercased() == ownAddress.lowercased()
        else { return selfShieldRecipientMessage }
        return nil
    }

    // MARK: - Identity shield messages

    nonisolated static let identityShieldSubmittedMessage =
        "Shield from identity submitted. Its Shielded Activity row stays "
        + "pending until the shielded sync sees the new note."

    /// Wallet-scoped on purpose: an unreadable record blocks shields from
    /// every identity in the wallet, and the error then names the requested
    /// identity, not the record's.
    nonisolated static let identityDebitPendingMessage =
        "An earlier shield from an identity in this wallet is still unresolved, "
        + "or its record could not be read, so a new one was not sent. See "
        + "In-Progress Identity Shields in Shielded Activity: the wallet resolves "
        + "readable ones on a shielded sync, and you can stop retrying any of them there."

    /// Apply the authoritative Core broadcast outcome to the send UI. Kept as
    /// a small pure state transition so rejected/unknown can be regression
    /// tested without constructing a live wallet or transaction.
    func applyCoreBroadcastOutcome(
        _ outcome: CoreTransactionBroadcastOutcome,
        recipientCount: Int
    ) {
        switch outcome {
        case .accepted:
            error = nil
            successMessage = recipientCount > 1
                ? "Payment sent to \(recipientCount) recipients"
                : "Payment sent"
        case .rejected(_, let reason, _):
            successMessage = nil
            error = "Payment rejected by Dash Core: \(reason)"
        case .unknown(let txid, let reason, let payment):
            // The broadcast may have reached the network. The hold follows
            // Rust's class of the result, as for any other payment failure
            // (one without a class also holds the form).
            successMessage = nil
            paymentFailure = PaymentFailure(
                payment: payment,
                detail: "Payment status could not be verified (\(txid)). "
                    + "It may already have been accepted; do not retry. \(reason)"
            )
        }
    }
}
