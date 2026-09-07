import Foundation
import DashSDKFFI

extension PlatformWalletManager {
    /// Static minimum fee reserve (in credits) for an
    /// `AddressFundingFromAssetLockTransition`, computed at this manager's
    /// network-tracked platform version. This is the consensus admission-floor
    /// estimate, not a state-aware real-fee quote, and performs no network
    /// request.
    public func estimateAddressFundingFee(
        inputCount: Int,
        outputCount: Int
    ) throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle(
                "PlatformWalletManager not configured"
            )
        }
        guard inputCount >= 0 else {
            throw PlatformWalletError.invalidParameter(
                "inputCount must be non-negative, got \(inputCount)"
            )
        }
        guard outputCount >= 0 else {
            throw PlatformWalletError.invalidParameter(
                "outputCount must be non-negative, got \(outputCount)"
            )
        }

        var fee: UInt64 = 0
        try platform_wallet_address_funding_estimate_fee(
            handle,
            UInt(inputCount),
            UInt(outputCount),
            &fee
        ).check()
        return fee
    }
}
