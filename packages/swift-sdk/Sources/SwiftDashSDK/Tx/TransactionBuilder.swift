import Foundation

/// Minimal transaction builder facade exposed by SwiftDashSDK.
/// Implementation will be wired to FFI in a follow-up; for now it surfaces a stable API.
public final class SDKTransactionBuilder {
    public struct Input {
        public let txid: Data
        public let vout: UInt32
        public let scriptPubKey: Data
        public let privateKey: Data
        public init(txid: Data, vout: UInt32, scriptPubKey: Data, privateKey: Data) {
            self.txid = txid
            self.vout = vout
            self.scriptPubKey = scriptPubKey
            self.privateKey = privateKey
        }
    }

    public struct Output {
        public let address: String
        public let amount: UInt64
        public init(address: String, amount: UInt64) {
            self.address = address
            self.amount = amount
        }
    }

    private let network: Network
    private let feePerKB: UInt64
    private var inputs: [Input] = []
    private var outputs: [Output] = []
    private var changeAddress: String?

    public init(network: Network, feePerKB: UInt64 = 1000) {
        self.network = network
        self.feePerKB = feePerKB
    }

    public func setChangeAddress(_ address: String) throws {
        // TODO: validate address via SDK once available
        self.changeAddress = address
    }

    public func addInput(_ input: Input) throws {
        inputs.append(input)
    }

    public func addOutput(_ output: Output) throws {
        outputs.append(output)
    }

    public func build() throws -> SDKBuiltTransaction {
        throw SDKTxError.notImplemented("Transaction building is not yet implemented in SwiftDashSDK")
    }
}

