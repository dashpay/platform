import Foundation
import SwiftDashSDK

/// Available send flow types based on source and destination.
enum SendFlow: Equatable {
    case platformToShielded   // Shield credits
    case shieldedToShielded   // Private transfer
    case shieldedToPlatform   // Unshield
    case shieldedToCore       // Withdrawal to L1

    var displayName: String {
        switch self {
        case .platformToShielded: return "Shield Credits"
        case .shieldedToShielded: return "Shielded Transfer"
        case .shieldedToPlatform: return "Unshield"
        case .shieldedToCore: return "Withdrawal to Core"
        }
    }

    var iconName: String {
        switch self {
        case .platformToShielded: return "lock.shield"
        case .shieldedToShielded: return "arrow.left.arrow.right"
        case .shieldedToPlatform: return "lock.open"
        case .shieldedToCore: return "arrow.down.to.line"
        }
    }

    /// Approximate fee in credits for this flow type.
    var estimatedFee: UInt64 {
        switch self {
        case .platformToShielded: return 200_000
        case .shieldedToShielded: return 300_000
        case .shieldedToPlatform: return 300_000
        case .shieldedToCore: return 500_000
        }
    }
}

/// Fund source for sending.
enum FundSource: String, CaseIterable {
    case shielded = "Shielded"
    case platform = "Platform"
    case core = "Core"
}

/// ViewModel for the Send Transaction screen.
@MainActor
class SendViewModel: ObservableObject {
    @Published var recipientAddress = "" {
        didSet { detectAddressType() }
    }
    @Published var amountString = ""
    @Published var detectedAddressType: DashAddressType = .unknown
    @Published var detectedFlow: SendFlow?
    @Published var estimatedFee: UInt64?
    @Published var isSending = false
    @Published var error: String?
    @Published var successMessage: String?

    // Source preference (for demo UI)
    @Published var preferShieldedSource = true

    private let network: AppNetwork

    init(network: AppNetwork) {
        self.network = network
    }

    var amount: UInt64? {
        guard let double = Double(amountString), double > 0 else { return nil }
        return UInt64(double * 100_000_000)
    }

    var canSend: Bool {
        detectedFlow != nil && amount != nil && !isSending
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

        // Determine flow based on destination type and source preference
        switch parsed.type {
        case .orchard:
            detectedFlow = preferShieldedSource ? .shieldedToShielded : .platformToShielded
        case .platform:
            detectedFlow = .shieldedToPlatform
        case .core:
            detectedFlow = .shieldedToCore
        case .unknown:
            detectedFlow = nil
        }

        estimatedFee = detectedFlow?.estimatedFee
    }

    // MARK: - Send Execution

    func executeSend(
        sdk: SDK,
        shieldedService: ShieldedService,
        platformState: AppState,
        wallet: HDWallet
    ) async {
        guard let flow = detectedFlow, let amount = amount else { return }
        guard let poolClient = shieldedService.poolClient else {
            error = "Shielded pool not initialized"
            return
        }

        isSending = true
        error = nil
        successMessage = nil
        defer { isSending = false }

        do {
            switch flow {
            case .platformToShielded:
                let bundle = try await poolClient.buildShieldBundle(amount: amount)
                // Find an identity with sufficient balance to fund the shield
                guard let identity = platformState.identities.first(where: {
                    $0.walletId == wallet.walletId &&
                    $0.network == wallet.network.rawValue &&
                    $0.balance >= amount
                }) else {
                    error = "No identity with sufficient platform balance"
                    return
                }
                // Need the identity's private key for the shield input
                guard let privateKey = identity.privateKeys.first else {
                    error = "No private key available for identity"
                    return
                }
                let addressBytes = identity.id.prefix(21)
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

            // Refresh shielded balance
            shieldedService.refreshBalance()

        } catch {
            self.error = error.localizedDescription
        }
    }
}
