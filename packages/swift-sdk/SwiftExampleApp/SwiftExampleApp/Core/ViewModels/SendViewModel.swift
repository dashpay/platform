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

            case .platformToShielded:
                _ = platformState // quiet unused-param warnings
                guard let poolClient = shieldedService.poolClient else {
                    error = "Shielded pool not initialized"
                    return
                }
                let bundle = try await poolClient.buildShieldBundle(amount: amount)

                // Fetch a PersistentIdentity on this wallet/network
                // that has enough platform balance to cover `amount`.
                // `balance` is stored as Int64 (bit-pattern cast of
                // the UInt64 DPP credits), so we compare against the
                // same bit-pattern cast of the requested amount.
                let walletId = wallet.walletId
                // Identities are scoped to a network; match the
                // wallet's resolved network directly. The `?? .testnet`
                // keeps the predicate well-formed when the wallet row
                // hasn't had its network stamped yet — a wallet in
                // that state has no identities to find anyway.
                //
                // Filter against `networkRaw` (the UInt32-backed shadow
                // field) because Foundation's predicate engine can't
                // capture `Network`.
                let walletNetworkRaw = (wallet.network ?? .testnet).rawValue
                let amountThreshold = Int64(bitPattern: amount)
                let descriptor = FetchDescriptor<PersistentIdentity>(
                    predicate: #Predicate<PersistentIdentity> { identity in
                        identity.wallet?.walletId == walletId &&
                        identity.networkRaw == walletNetworkRaw &&
                        identity.balance >= amountThreshold
                    }
                )
                guard let identity = try? modelContext.fetch(descriptor).first else {
                    error = "No identity with sufficient platform balance"
                    return
                }

                // Pick the first public key that has an associated
                // private key in the keychain. Private keys no
                // longer live on the identity row.
                guard let privateKey = identity.publicKeys.lazy
                    .compactMap({ key -> Data? in
                        KeychainManager.shared.retrievePrivateKey(
                            identityId: identity.identityId,
                            keyIndex: key.keyId
                        )
                    })
                    .first else {
                    error = "No private key available for identity"
                    return
                }

                let addressBytes = identity.identityId.prefix(21)
                let input = ShieldFundsInput(
                    address: Data(addressBytes),
                    amount: amount,
                    privateKey: privateKey
                )
                try await sdk.shieldFunds(
                    inputs: [input],
                    bundle: bundle,
                    amount: amount,
                    feeFromInputIndex: 0
                )
                successMessage = "Shielding complete"

            case .shieldedToShielded:
                guard let poolClient = shieldedService.poolClient else {
                    error = "Shielded pool not initialized"
                    return
                }
                let parsed = DashAddress.parse(recipientAddress, network: network)
                guard case .orchard(let rawAddress) = parsed.type else { return }
                let bundle = try await poolClient.buildTransferBundle(
                    recipientAddress: rawAddress,
                    amount: amount
                )
                try await sdk.shieldedTransfer(
                    bundle: bundle,
                    valueBalance: flow.estimatedFee
                )
                successMessage = "Shielded transfer complete"

            case .shieldedToPlatform:
                guard let poolClient = shieldedService.poolClient else {
                    error = "Shielded pool not initialized"
                    return
                }
                let parsed = DashAddress.parse(recipientAddress, network: network)
                guard case .platform(let addressBytes) = parsed.type else { return }
                let bundle = try await poolClient.buildUnshieldBundle(
                    outputAddress: addressBytes,
                    amount: amount
                )
                try await sdk.unshieldFunds(
                    outputAddress: addressBytes,
                    amount: amount,
                    bundle: bundle
                )
                successMessage = "Unshield complete"

            case .shieldedToCore:
                guard let poolClient = shieldedService.poolClient else {
                    error = "Shielded pool not initialized"
                    return
                }
                let parsed = DashAddress.parse(recipientAddress, network: network)
                guard case .core(let outputScript) = parsed.type else { return }
                let bundle = try await poolClient.buildWithdrawalBundle(
                    outputScript: outputScript,
                    amount: amount,
                    coreFeePerByte: 1,
                    pooling: .never
                )
                try await sdk.shieldedWithdraw(
                    amount: amount,
                    bundle: bundle,
                    coreFeePerByte: 1,
                    pooling: .never,
                    outputScript: outputScript
                )
                successMessage = "Withdrawal submitted"
            }

            // Refresh balances
            shieldedService.refreshBalance()

        } catch {
            self.error = error.localizedDescription
        }
    }
}
