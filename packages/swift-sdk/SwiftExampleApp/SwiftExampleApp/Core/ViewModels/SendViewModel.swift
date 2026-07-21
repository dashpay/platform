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

    var displayName: String {
        switch self {
        case .coreToCore: return "Core Payment"
        case .coreToShielded: return "Shield from Core"
        case .platformToPlatform: return "Platform Transfer"
        case .platformToShielded: return "Shield Credits"
        case .shieldedToShielded: return "Shielded Transfer"
        case .shieldedToPlatform: return "Unshield"
        case .shieldedToCore: return "Withdrawal to Core"
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
        }
    }
}

/// Fund source for sending.
enum FundSource: String, CaseIterable, Identifiable {
    case core = "Core"
    case shielded = "Shielded"
    case platform = "Platform"

    var id: String { rawValue }

    var iconName: String {
        switch self {
        case .core: return "arrow.right"
        case .shielded: return "lock.shield"
        case .platform: return "square.stack.3d.up"
        }
    }

    var color: SwiftUI.Color {
        switch self {
        case .core: return .green
        case .shielded: return .purple
        case .platform: return .blue
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
    @Published var selectedSource: FundSource = .core
    @Published var detectedFlow: SendFlow?
    @Published var estimatedFee: UInt64?
    @Published var isSending = false
    @Published var error: String?
    @Published var successMessage: String?

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
        guard let flow = detectedFlow, !isSending else { return false }
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
        }
    }

    /// Determine which fund sources are available based on destination and balances.
    func availableSources(
        coreBalance: UInt64,
        shieldedBalance: UInt64,
        platformBalance: UInt64
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
        detectedAddressType = parsed.type
        updateFlow()
    }

    func updateFlow() {
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
        case (.platform, .platform):
            detectedFlow = .platformToPlatform
        case (.platform, .shielded):
            detectedFlow = .shieldedToPlatform
        default:
            detectedFlow = nil
        }
        estimatedFee = detectedFlow.map(estimateFee(for:))
    }

    /// Resolve the estimated fee (in the flow's settlement unit) for the
    /// active flow. The shielded flows are consensus-pinned and computed
    /// in Rust (`compute_*_shielded_fee` via the FFI estimator), so this
    /// bridges to that rather than re-deriving the constants in Swift.
    ///
    /// `numActions: 2` — the exact action count isn't known until the
    /// builder selects notes; a single-note spend with change (the common
    /// case) serializes to 2 Orchard actions. The transparent `Shield`
    /// (`platformToShielded`) reserves the same `compute_minimum_shielded_fee(2)`
    /// base as its structure-check minimum, so it shares the transfer kind.
    /// On an FFI error we fall back to the static enum placeholder rather
    /// than surfacing a fee of nil for a flow we can otherwise send.
    private func estimateFee(for flow: SendFlow) -> UInt64 {
        let kind: PlatformWalletManager.ShieldedFeeKind?
        switch flow {
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
        return (try? PlatformWalletManager.estimateShieldedFee(kind: kind, numActions: 2))
            ?? flow.estimatedFee
    }

    // MARK: - Send Execution

    func executeSend(
        sdk: SDK,
        walletManager: PlatformWalletManager,
        shieldedService: ShieldedService,
        platformState: AppState,
        wallet: PersistentWallet,
        platformWallet: ManagedPlatformWallet?,
        platformAddressWallet: ManagedPlatformAddressWallet?,
        signer: KeychainSigner?,
        senderAccountIndex: UInt32,
        modelContext: ModelContext
    ) async {
        guard let flow = detectedFlow else { return }

        isSending = true
        error = nil
        successMessage = nil
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
                // Broadcast lives on the core wallet; grab it locally.
                let _ = try platformWallet.coreWallet().broadcastTransaction(signedTx)
                successMessage = recipients.count > 1
                    ? "Payment sent to \(recipients.count) recipients"
                    : "Payment sent"

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
                // `shieldedShield` has no recipient parameter — Rust
                // always shields into this wallet's own default Orchard
                // address (shieldedAccount 0). If the user typed a
                // *different* Orchard address we'd report success while
                // nothing reached that recipient, so constrain this path
                // to self-shield only.
                let enteredRecipient = recipientAddress
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                // Resolve THIS wallet's own default Orchard address from
                // the engine rather than the single-mirror
                // `shieldedService` (which tracks `firstWallet`). Every
                // loaded wallet is engine-bound, so `shieldedDefaultAddress`
                // resolves for the wallet actually being sent from.
                let ownShieldedAddress = walletManager.shieldedDisplayAddress(
                    walletId: wallet.walletId,
                    network: network
                )
                if !enteredRecipient.isEmpty,
                   enteredRecipient != ownShieldedAddress {
                    // Don't advertise "leave it blank": a blank recipient
                    // clears `detectedFlow` upstream (detectAddressType →
                    // updateFlow), so `canSend` disables the button and
                    // this branch is only reachable with a non-empty
                    // address. Tell the user to enter their own.
                    error = "Shield always sends to your own shielded "
                        + "address; enter your own shielded address as "
                        + "the recipient"
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
            }

        } catch PlatformWalletError.shieldedSpendUnconfirmed {
            // The shielded operation (shield / unshield / transfer / withdraw)
            // was broadcast and accepted, but its execution result couldn't be
            // confirmed — it may already be on chain. Rust intentionally KEEPS
            // any spent notes' reservations, so this must NOT be presented as a
            // retryable failure: retrying would rebuild the bundle and could
            // double-execute if the original landed. Surface it through the
            // non-error (success) path so the UI doesn't invite a retry; the
            // next shielded sync reconciles the outcome.
            successMessage = "Transaction may have gone through — waiting for "
                + "the next shielded sync to confirm. Do not retry."
        } catch {
            self.error = error.localizedDescription
        }
    }
}
