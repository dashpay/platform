import Foundation
import CommonCrypto
import SwiftDashSDK

/// Available send flow types based on source and destination.
enum SendFlow: Equatable {
    case coreToPlatform       // Asset lock / transfer to platform address
    case coreToCore           // Standard Core transaction
    case platformToShielded   // Shield credits
    case shieldedToShielded   // Private transfer
    case shieldedToPlatform   // Unshield
    case shieldedToCore       // Withdrawal to L1

    var displayName: String {
        switch self {
        case .coreToPlatform: return "Transfer to Platform"
        case .coreToCore: return "Core Transfer"
        case .platformToShielded: return "Shield Credits"
        case .shieldedToShielded: return "Shielded Transfer"
        case .shieldedToPlatform: return "Unshield"
        case .shieldedToCore: return "Withdrawal to Core"
        }
    }

    var iconName: String {
        switch self {
        case .coreToPlatform: return "arrow.up.to.line"
        case .coreToCore: return "arrow.right"
        case .platformToShielded: return "lock.shield"
        case .shieldedToShielded: return "arrow.left.arrow.right"
        case .shieldedToPlatform: return "lock.open"
        case .shieldedToCore: return "arrow.down.to.line"
        }
    }

    /// Approximate fee in duffs for this flow type.
    var estimatedFee: UInt64 {
        switch self {
        case .coreToPlatform: return 100_000     // ~0.001 DASH
        case .coreToCore: return 100_000         // ~0.001 DASH
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

    // Source preference (for demo UI — defaults to Core since shielded requires setup)
    @Published var preferShieldedSource = false

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
            // If we have shielded balance, unshield; otherwise transfer from Core
            detectedFlow = preferShieldedSource ? .shieldedToPlatform : .coreToPlatform
        case .core:
            detectedFlow = preferShieldedSource ? .shieldedToCore : .coreToCore
        case .unknown:
            detectedFlow = nil
        }

        estimatedFee = detectedFlow?.estimatedFee
    }

    // MARK: - Send Execution

    func executeSend(
        sdk: SDK,
        shieldedService: ShieldedService,
        walletService: WalletService,
        platformState: AppState,
        wallet: HDWallet
    ) async {
        guard let flow = detectedFlow, let amount = amount else { return }

        // Shielded flows need pool client; Core flows don't
        let needsPoolClient = flow != .coreToPlatform && flow != .coreToCore
        if needsPoolClient {
            guard shieldedService.poolClient != nil else {
                error = "Shielded pool not initialized"
                return
            }
        }

        isSending = true
        error = nil
        successMessage = nil
        defer { isSending = false }

        do {
            switch flow {
            case .platformToShielded:
                let bundle = try await shieldedService.poolClient!.buildShieldBundle(amount: amount)
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
                let bundle = try await shieldedService.poolClient!.buildTransferBundle(
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
                let bundle = try await shieldedService.poolClient!.buildUnshieldBundle(
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
                let bundle = try await shieldedService.poolClient!.buildWithdrawalBundle(
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

            case .coreToPlatform:
                // Core → Platform via asset lock
                let parsed = DashAddress.parse(recipientAddress, network: network)
                guard case .platform(let addressBytes) = parsed.type else {
                    error = "Invalid platform address"
                    return
                }

                // Convert platform address (21 bytes: type + hash) to P2PKH scriptPubKey (25 bytes)
                // Platform type 0x00 = P2PKH: OP_DUP OP_HASH160 <20-byte-hash> OP_EQUALVERIFY OP_CHECKSIG
                let creditScript: Data
                if addressBytes.count == 21 && addressBytes[0] == 0x00 {
                    let pubkeyHash = addressBytes.dropFirst() // 20-byte hash
                    var script = Data([0x76, 0xa9, 0x14]) // OP_DUP OP_HASH160 PUSH20
                    script.append(contentsOf: pubkeyHash)
                    script.append(contentsOf: [0x88, 0xac]) // OP_EQUALVERIFY OP_CHECKSIG
                    creditScript = script
                } else {
                    // Pass through as-is for other address types
                    creditScript = addressBytes
                }

                // 1. Build the asset lock transaction
                let assetLockResult = try walletService.walletManager.buildAssetLockTransaction(
                    for: wallet,
                    creditOutputs: [(scriptPubKey: creditScript, amount: amount)]
                )

                // 2. Broadcast on Core network
                try walletService.broadcastTransaction(assetLockResult.transactionBytes)

                // Compute txid from transaction bytes (double SHA256, reversed)
                let txid = computeTxid(from: assetLockResult.transactionBytes)

                // 3. Wait for InstantSend lock
                let isLockData = try await walletService.waitForInstantLock(txid: txid, timeout: 30)

                // 4. Submit to Platform
                let outPoint = buildOutPoint(txid: txid, outputIndex: assetLockResult.outputIndex)
                _ = try sdk.addresses.topUpAddressFromAssetLock(
                    proofType: .instant,
                    instantLockData: isLockData,
                    transactionData: assetLockResult.transactionBytes,
                    outputIndex: assetLockResult.outputIndex,
                    coreChainLockedHeight: 0,
                    outPoint: outPoint,
                    assetLockPrivateKey: assetLockResult.privateKey,
                    outputs: [Addresses.AddressTransferOutput(addressBytes: addressBytes, amount: amount)]
                )

                successMessage = "Transfer to Platform complete"

            case .coreToCore:
                // TODO: Implement standard Core → Core transaction
                error = "Core to Core transfer not yet implemented"
            }

            // Refresh shielded balance
            shieldedService.refreshBalance()

        } catch {
            self.error = error.localizedDescription
        }
    }

    // MARK: - Helpers

    /// Compute txid from raw transaction bytes (double SHA256, reversed).
    private func computeTxid(from txBytes: Data) -> Data {
        var hash1 = Data(count: Int(CC_SHA256_DIGEST_LENGTH))
        var hash2 = Data(count: Int(CC_SHA256_DIGEST_LENGTH))
        txBytes.withUnsafeBytes { ptr in
            hash1.withUnsafeMutableBytes { out in
                _ = CC_SHA256(ptr.baseAddress, CC_LONG(txBytes.count), out.bindMemory(to: UInt8.self).baseAddress)
            }
        }
        hash1.withUnsafeBytes { ptr in
            hash2.withUnsafeMutableBytes { out in
                _ = CC_SHA256(ptr.baseAddress, CC_LONG(hash1.count), out.bindMemory(to: UInt8.self).baseAddress)
            }
        }
        // Txid is the reversed double-SHA256
        return Data(hash2.reversed())
    }

    /// Build a 36-byte OutPoint (txid + output index as little-endian u32).
    private func buildOutPoint(txid: Data, outputIndex: UInt32) -> Data {
        // OutPoint = txid (32 bytes, internal byte order) + index (4 bytes LE)
        var outPoint = Data(txid.reversed()) // reversed back to internal order
        var idx = outputIndex.littleEndian
        outPoint.append(Data(bytes: &idx, count: 4))
        return outPoint
    }
}
