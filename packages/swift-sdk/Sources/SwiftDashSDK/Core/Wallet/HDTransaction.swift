import Foundation
import SwiftData

// MARK: - HD Transaction

@Model
public final class HDTransaction {
    @Attribute(.unique) public var id: UUID
    @Attribute(.unique) public var txHash: String
    public var rawTransaction: Data?
    public var blockHeight: Int?
    public var blockHash: String?
    public var timestamp: Date
    public var confirmations: Int
    public var size: Int
    public var fee: UInt64
    public var type: String  // "sent", "received", "self"

    // Inputs and outputs
    public var inputsData: Data?  // Serialized TransactionInput array
    public var outputsData: Data? // Serialized TransactionOutput array

    // Relationships
    @Relationship public var addresses: [HDAddress] = []
    @Relationship public var wallet: HDWallet?

    // Computed amount (positive for received, negative for sent)
    public var amount: Int64

    // Transaction status
    public var isPending: Bool
    public var isInstantSend: Bool
    public var isChainLocked: Bool

    public init(txHash: String, timestamp: Date = Date()) {
        self.id = UUID()
        self.txHash = txHash
        self.timestamp = timestamp
        self.confirmations = 0
        self.size = 0
        self.fee = 0
        self.type = "received"
        self.amount = 0
        self.isPending = true
        self.isInstantSend = false
        self.isChainLocked = false
    }

    public var transactionType: TransactionType {
        return TransactionType(rawValue: type) ?? .received
    }
}

public enum TransactionType: String {
    case sent = "sent"
    case received = "received"
    case `self` = "self"
}

// MARK: - Transaction Components

public struct TransactionInput: Codable {
    public let txHash: String
    public let outputIndex: UInt32
    public let script: Data
    public let sequence: UInt32
    public let amount: UInt64?
    public let address: String?

    public init(txHash: String, outputIndex: UInt32, script: Data, sequence: UInt32 = 0xFFFFFFFF, amount: UInt64? = nil, address: String? = nil) {
        self.txHash = txHash
        self.outputIndex = outputIndex
        self.script = script
        self.sequence = sequence
        self.amount = amount
        self.address = address
    }
}

public struct TransactionOutput: Codable {
    public let amount: UInt64
    public let script: Data
    public let address: String?
    public let isChange: Bool

    public init(amount: UInt64, script: Data, address: String? = nil, isChange: Bool = false) {
        self.amount = amount
        self.script = script
        self.address = address
        self.isChange = isChange
    }
}

// MARK: - Raw Transaction

public struct RawTransaction {
    public let version: UInt32
    public let inputs: [TransactionInput]
    public let outputs: [TransactionOutput]
    public let lockTime: UInt32

    public func serialize() throws -> Data {
        var data = Data()

        // Version
        var versionLE = version.littleEndian
        data.append(Data(bytes: &versionLE, count: 4))

        // Input count (compact size)
        data.append(compactSize(UInt64(inputs.count)))

        // Inputs
        for input in inputs {
            // Previous output
            if let txHashData = Data(hex: input.txHash) {
                data.append(contentsOf: txHashData.reversed()) // Little endian
            }
            var outputIndexLE = input.outputIndex.littleEndian
            data.append(Data(bytes: &outputIndexLE, count: 4))

            // Script
            data.append(compactSize(UInt64(input.script.count)))
            data.append(input.script)

            // Sequence
            var sequenceLE = input.sequence.littleEndian
            data.append(Data(bytes: &sequenceLE, count: 4))
        }

        // Output count
        data.append(compactSize(UInt64(outputs.count)))

        // Outputs
        for output in outputs {
            // Amount
            var amountLE = output.amount.littleEndian
            data.append(Data(bytes: &amountLE, count: 8))

            // Script
            data.append(compactSize(UInt64(output.script.count)))
            data.append(output.script)
        }

        // Lock time
        var lockTimeLE = lockTime.littleEndian
        data.append(Data(bytes: &lockTimeLE, count: 4))

        return data
    }

    private func compactSize(_ value: UInt64) -> Data {
        if value < 0xfd {
            return Data([UInt8(value)])
        } else if value <= 0xffff {
            var data = Data([0xfd])
            var valueLE = UInt16(value).littleEndian
            data.append(Data(bytes: &valueLE, count: 2))
            return data
        } else if value <= 0xffffffff {
            var data = Data([0xfe])
            var valueLE = UInt32(value).littleEndian
            data.append(Data(bytes: &valueLE, count: 4))
            return data
        } else {
            var data = Data([0xff])
            var valueLE = value.littleEndian
            data.append(Data(bytes: &valueLE, count: 8))
            return data
        }
    }
}

// Common hex initializer used by transaction code
extension Data {
    init?(hex: String) {
        let hex = hex.replacingOccurrences(of: " ", with: "")
        guard hex.count % 2 == 0 else { return nil }
        var data = Data(capacity: hex.count / 2)
        var index = hex.startIndex
        while index < hex.endIndex {
            let next = hex.index(index, offsetBy: 2)
            guard next <= hex.endIndex else { return nil }
            let byteString = String(hex[index..<next])
            guard let num = UInt8(byteString, radix: 16) else { return nil }
            data.append(num)
            index = next
        }
        self = data
    }
}
