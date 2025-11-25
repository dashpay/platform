import Foundation
import SwiftDashSDK

// Re-export UTXO types from SDK
// Note: UTXO is defined in KeyWalletTypes and extended in Wallet/UTXOExtensions
public typealias UTXO = SwiftDashSDK.UTXO
public typealias UTXOSelection = SwiftDashSDK.UTXOSelection
public typealias UTXOSelector = SwiftDashSDK.UTXOSelector
