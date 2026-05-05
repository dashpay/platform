import Foundation
import SwiftData
import SwiftDashSDK

/// Available send flow types based on source and destination.
enum SendFlow: Equatable {
    case coreToCore              // Standard L1 payment
    case platformToShielded      // Shield credits
    case shieldedToShielded      // Private transfer
    case shieldedToPlatform      // Unshield
    case shieldedToCore          // Withdrawal from shielded to L1

    var displayName: String {
        switch self {
        case .coreToCore: return "Core Payment"
        case .platformToShielded: return "Shield Credits"
        case .shieldedToShielded: return "Shielded Transfer"
        case .shieldedToPlatform: return "Unshield"
        case .shieldedToCore: return "Withdrawal to Core"
        }
    }

    var iconName: String {
        switch self {
        case .coreToCore: return "arrow.right"
        case .platformToShielded: return "lock.shield"
        case .shieldedToShielded: return "arrow.left.arrow.right"
        case .shieldedToPlatform: return "lock.open"
        case .shieldedToCore: return "arrow.down.to.line"
        }
    }

    var estimatedFee: UInt64 {
        switch self {
        case .coreToCore: return 500_000             // ~0.005 DASH
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

    var amount: UInt64? {
        guard let double = Double(amountString), double > 0 else { return nil }
        return UInt64(double * 100_000_000)
    }

    var canSend: Bool {
        detectedFlow != nil && amount != nil && !isSending
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
        modelContext: ModelContext
    ) async {
        guard let flow = detectedFlow, let amount = amount else { return }

        isSending = true
        error = nil
        successMessage = nil
        defer { isSending = false }

        do {
            switch flow {
            case .coreToCore:
                guard let core = coreWallet else {
                    error = "Core wallet not available"
                    return
                }
                let address = recipientAddress.trimmingCharacters(in: .whitespacesAndNewlines)
                let _ = try core.sendToAddresses(
                    recipients: [(address: address, amountDuffs: amount)]
                )
                successMessage = "Payment sent"

            case .shieldedToShielded:
                // Shielded → Shielded: spend notes from this
                // wallet's shielded balance, create a new note
                // for the recipient. Recipient bytes come from
                // the bech32m parser as raw 43-byte Orchard
                // address; matches what the manager's transfer
                // FFI expects.
                let parsed = DashAddress.parse(recipientAddress, network: network)
                guard case .orchard(let recipientRaw) = parsed.type else {
                    error = "Recipient is not a shielded address"
                    return
                }
                try await walletManager.shieldedTransfer(
                    walletId: wallet.walletId,
                    recipientRaw43: recipientRaw,
                    amount: amount
                )
                successMessage = "Shielded transfer complete"

            case .shieldedToPlatform:
                // Shielded → Platform: spend notes, credit the
                // platform address. `addressBytes` is the 21-byte
                // bincode-encoded `PlatformAddress` shape (type
                // byte + 20-byte hash).
                let parsed = DashAddress.parse(recipientAddress, network: network)
                guard case .platform(let addressBytes) = parsed.type else {
                    error = "Recipient is not a platform address"
                    return
                }
                try await walletManager.shieldedUnshield(
                    walletId: wallet.walletId,
                    toPlatformAddress: addressBytes,
                    amount: amount
                )
                successMessage = "Unshield complete"

            case .shieldedToCore:
                // Shielded → Core L1: spend notes, create an L1
                // withdrawal. The manager parses the Base58Check
                // address Rust-side; we just hand the trimmed
                // string through.
                let trimmed = recipientAddress.trimmingCharacters(in: .whitespacesAndNewlines)
                try await walletManager.shieldedWithdraw(
                    walletId: wallet.walletId,
                    toCoreAddress: trimmed,
                    amount: amount,
                    coreFeePerByte: 1
                )
                successMessage = "Withdrawal submitted"

            case .platformToShielded:
                // Platform → Shielded (Type 15): spend credits from
                // the wallet's first Platform Payment account into
                // the bound shielded pool. The KeychainSigner
                // pulls the per-address ECDSA keys via the same
                // mnemonic-resolver path identity-key signing uses;
                // per-input nonces are fetched server-side from
                // Platform inside `ShieldedWallet::shield`.
                _ = platformState
                _ = shieldedService
                _ = sdk
                let signer = KeychainSigner(modelContainer: modelContext.container)
                try await walletManager.shieldedShield(
                    walletId: wallet.walletId,
                    accountIndex: 0,
                    amount: amount,
                    addressSigner: signer
                )
                successMessage = "Shielding complete"
            }

        } catch {
            self.error = error.localizedDescription
        }
    }
}
