import Foundation
import SwiftData
import SwiftDashSDK

/// Available send flow types based on source and destination.
enum SendFlow: Equatable {
    case coreToCore              // Standard L1 payment
    case platformToPlatform      // Platform-address → platform-address transfer
    case platformToShielded      // Shield credits
    case shieldedToShielded      // Private transfer
    case shieldedToPlatform      // Unshield
    case shieldedToCore          // Withdrawal from shielded to L1

    var displayName: String {
        switch self {
        case .coreToCore: return "Core Payment"
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
        case .platformToPlatform: return "arrow.right"
        case .platformToShielded: return "lock.shield"
        case .shieldedToShielded: return "arrow.left.arrow.right"
        case .shieldedToPlatform: return "lock.open"
        case .shieldedToCore: return "arrow.down.to.line"
        }
    }

    var estimatedFee: UInt64 {
        switch self {
        case .coreToCore: return 500_000             // ~0.005 DASH
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

/// ViewModel for the Send Transaction screen.
@MainActor
class SendViewModel: ObservableObject {
    @Published var recipientAddress = "" {
        didSet { detectAddressType() }
    }
    @Published var amountString = ""
    @Published var detectedAddressType: DashAddressType = .unknown
    @Published var selectedSource: FundSource = .core
    @Published var detectedFlow: SendFlow?
    @Published var estimatedFee: UInt64?
    @Published var isSending = false
    @Published var error: String?
    @Published var successMessage: String?

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

    var canSend: Bool {
        guard let flow = detectedFlow, !isSending else { return false }
        // Gate on the scaled integer for the *active* flow's unit, not
        // just non-nil. A sub-unit amount (e.g. "0.000000001" in DASH)
        // parses to 0 once scaled; sending that reaches the backend as a
        // zero-value transfer. Core/L1 settles in duffs (1e8); every
        // credits-ledger flow settles in credits (1e11).
        switch flow {
        case .coreToCore:
            return (amountDuffs ?? 0) > 0
        case .platformToPlatform, .platformToShielded,
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
        case (.platform, .platform):
            detectedFlow = .platformToPlatform
        case (.platform, .shielded):
            detectedFlow = .shieldedToPlatform
        default:
            detectedFlow = nil
        }
        estimatedFee = detectedFlow?.estimatedFee
    }

    // MARK: - Send Execution

    func executeSend(
        sdk: SDK,
        walletManager: PlatformWalletManager,
        shieldedService: ShieldedService,
        platformState: AppState,
        wallet: PersistentWallet,
        coreWallet: ManagedCoreWallet?,
        platformAddressWallet: ManagedPlatformAddressWallet?,
        signer: KeychainSigner?,
        senderAccountIndex: UInt32,
        changeAddressRow: PersistentPlatformAddress?,
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
                guard let amountDuffs else {
                    error = "Invalid amount"
                    return
                }
                guard let core = coreWallet else {
                    error = "Core wallet not available"
                    return
                }
                let address = recipientAddress.trimmingCharacters(in: .whitespacesAndNewlines)
                let _ = try core.sendToAddresses(
                    recipients: [(address: address, amountDuffs: amountDuffs)]
                )
                successMessage = "Payment sent"

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
                // conversion (rs-platform-wallet-ffi/src/platform_address_types.rs:42)
                // only accepts P2PKH; sending to a P2SH platform address
                // would surface a raw "Unsupported address type" string
                // from Rust. Fail fast with a user-readable message.
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
                // If the view passed a fresh unused HD address from the
                // pool, use it as the dedicated change destination —
                // matches the Receive screen's lowest-unused selection.
                let change: ManagedPlatformAddressWallet.ChangeAddress? = changeAddressRow.map {
                    ManagedPlatformAddressWallet.ChangeAddress(
                        addressType: $0.addressType,
                        hash: $0.addressHash
                    )
                }
                let updated = try await addressWallet.transfer(
                    accountIndex: senderAccountIndex,
                    outputs: [output],
                    changeAddress: change,
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
                // fetch each row by `addressHash`, update the
                // volatile fields, stamp `lastUpdated`. Every entry
                // returned was touched by the transition, so
                // `isUsed = true` unconditionally. Rows that aren't
                // found are silently skipped — same defensive shape
                // the BLAST handler uses.
                for entry in updated {
                    let entryHash = entry.hash
                    let descriptor = FetchDescriptor<PersistentPlatformAddress>(
                        predicate: #Predicate { $0.addressHash == entryHash }
                    )
                    guard let row = try? modelContext.fetch(descriptor).first else {
                        continue
                    }
                    row.balance = entry.balance
                    row.nonce = entry.nonce
                    row.isUsed = true
                    row.lastUpdated = Date()
                }
                do {
                    try modelContext.save()
                } catch {
                    self.error = "Couldn't persist post-transfer balances: \(error.localizedDescription)"
                    return
                }

                successMessage = "Platform transfer sent"

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
                try await walletManager.shieldedTransfer(
                    walletId: wallet.walletId,
                    account: 0,
                    recipientRaw43: recipientRaw,
                    amount: amountCredits
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
                let ownShieldedAddress =
                    shieldedService.addressesByAccount[0]
                    ?? shieldedService.orchardDisplayAddress
                if !enteredRecipient.isEmpty,
                   enteredRecipient != ownShieldedAddress {
                    error = "Shield always sends to your own shielded "
                        + "address; enter your own address or leave it blank"
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
            }

        } catch {
            self.error = error.localizedDescription
        }
    }
}
