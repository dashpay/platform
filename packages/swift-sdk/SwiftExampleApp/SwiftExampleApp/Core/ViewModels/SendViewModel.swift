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

    var amount: UInt64? {
        guard let double = Double(amountString), double > 0 else { return nil }
        return UInt64(double * 100_000_000)
    }

    /// Amount in platform credits (1 DASH = 1e11 credits).
    /// Used by platform-credit flows; `amount` (above) is duffs (1e8) for core.
    var amountCredits: UInt64? {
        guard let double = Double(amountString), double > 0 else { return nil }
        return UInt64(double * 100_000_000_000)
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
                guard let core = coreWallet else {
                    error = "Core wallet not available"
                    return
                }
                guard let amount = amount else { return }
                let address = recipientAddress.trimmingCharacters(in: .whitespacesAndNewlines)
                let _ = try core.sendToAddresses(
                    recipients: [(address: address, amountDuffs: amount)]
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
                case 0xb0, 0x00: ffiAddressType = 0  // P2PKH
                case 0x80, 0x01: ffiAddressType = 1  // P2SH
                default:
                    error = "Unknown platform address type byte 0x\(String(bech32mByte, radix: 16))"
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
                let changeOutput: ManagedPlatformAddressWallet.TransferOutput? = changeAddressRow.map {
                    ManagedPlatformAddressWallet.TransferOutput(
                        addressType: $0.addressType,
                        hash: $0.addressHash,
                        credits: 0  // value gets overwritten by the wrapper
                    )
                }
                _ = try await addressWallet.transfer(
                    accountIndex: senderAccountIndex,
                    outputs: [output],
                    changeAddress: changeOutput,
                    signer: signer
                )
                successMessage = "Platform transfer sent"

            case .platformToShielded,
                 .shieldedToShielded,
                 .shieldedToPlatform,
                 .shieldedToCore:
                // Shielded send paths are being moved to the Rust
                // platform-wallet shielded coordinator. The previous
                // SDK-side bundle/build/broadcast surface was deleted
                // along with the duplicate `ShieldedPoolClient` FFI;
                // wiring back up against the new manager-driven path
                // happens in a follow-up PR.
                _ = platformState
                _ = shieldedService
                _ = wallet
                _ = modelContext
                _ = sdk
                error = "Shielded sending is being rebuilt — see follow-up PR"
                return
            }

        } catch {
            self.error = error.localizedDescription
        }
    }
}
