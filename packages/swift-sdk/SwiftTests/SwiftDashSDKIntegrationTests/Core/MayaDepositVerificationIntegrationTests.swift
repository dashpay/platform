import Foundation
import XCTest
@testable import SwiftDashSDK

@MainActor
final class MayaDepositVerificationIntegrationTests: IntegrationTestCase {
    private enum Constants {
        static let bip44TypeTag: UInt8 = 0
        static let bip44StandardTag: UInt8 = 0
        static let bip44AccountIndex: UInt32 = 0
        static let feeRateSatPerKb: UInt64 = 1_000
        static let minimumDepositDuffs: UInt64 = 10_000
        static let maxMemoBytes = 80
        static let longMemo =
            "=:ARB.GLD:0x51a1449b3B6D635EddeC781cD47a99221712De97:344233230e4/1/0:_/def:15/0"
        static let shortMemo =
            "=:r:thor166n4w5039meulfa3p6ydg60ve6ueac7tlt0jws:669458827/1/0:_/def:15/0"
    }

    private struct DepositObservation {
        let name: String
        let txHex: String
        let serializedSize: Int
        let inputCount: Int
        let outputCount: Int
        let vaultAddress: String
        let depositAmount: UInt64
        let actualVaultAmount: UInt64
        let memo: String
        let decodedMemo: String
        let memoBytes: Int
        let outputOneIsOpReturn: Bool
        let inputZeroAddress: String?
        let outputTwoMatchesInputZeroScript: Bool
        let feeDuffs: UInt64
        let feeRateDuffsPerByte: Double
    }

    private struct LegacyFeeObservation {
        let name: String
        let actualFeeDuffs: UInt64
        let legacyExpectedFeeDuffs: UInt64
        let serializedSize: Int
        let inputCount: Int
        let outputCount: Int
        let payloadLength: Int
    }

    private struct ParsedTransactionLayout {
        let inputCount: Int
        let outputCount: Int
        let payloadLength: Int
    }

    func testPrompt04StaticProofAndLegacyFeeParity() async throws {
        try env.walletManager.startSpv(config: env.spvConfig)

        let shortDeposit = try await buildDepositObservation(
            name: "short-memo-single-input",
            fundingDashAmounts: [0.5],
            depositAmountDuffs: 200_000,
            memo: Constants.shortMemo
        )
        let longDeposit = try await buildDepositObservation(
            name: "long-memo-multi-input",
            fundingDashAmounts: [0.2, 0.2],
            depositAmountDuffs: 35_000_000,
            memo: Constants.longMemo
        )

        assertDepositObservation(shortDeposit)
        assertDepositObservation(longDeposit)

        let ordinary = try await legacyFeeObservationForOrdinarySend()
        let multiRecipient = try await legacyFeeObservationForMultiRecipientSend()
        let selectedInput = try await legacyFeeObservationForSelectedInputShape()
        let sweep = try await legacyFeeObservationForDrainShape()
        let addressFunding = try await legacyFeeObservationForAssetLockShape(
            name: "asset-lock-address-top-up",
            fundingType: .assetLockAddressTopUp
        )
        let identityFunding = try await legacyFeeObservationForAssetLockShape(
            name: "asset-lock-identity-registration",
            fundingType: .identityRegistration
        )

        let feeObservations = [
            ordinary,
            multiRecipient,
            selectedInput,
            sweep,
            addressFunding,
            identityFunding,
        ]

        for observation in feeObservations {
            XCTAssertEqual(
                observation.actualFeeDuffs,
                observation.legacyExpectedFeeDuffs,
                "\(observation.name) fee moved: actual \(observation.actualFeeDuffs) vs legacy \(observation.legacyExpectedFeeDuffs)"
            )
        }

        print(renderDepositObservation(shortDeposit))
        print(renderDepositObservation(longDeposit))
        for observation in feeObservations {
            print(renderLegacyFeeObservation(observation))
        }
    }

    private func buildDepositObservation(
        name: String,
        fundingDashAmounts: [Double],
        depositAmountDuffs: UInt64,
        memo: String
    ) async throws -> DepositObservation {
        let wallet = try await env.makeTestWallet(name: "maya-\(name)")
        let coreWallet = wallet.getCoreWallet()
        let platformWallet = wallet.getPlatformWallet()

        for amount in fundingDashAmounts {
            let address = try coreWallet.nextReceiveAddress()
            _ = try await fundByMining(address: address, dash: amount)
        }

        let expectedSpendable = fundingDashAmounts.reduce(UInt64(0)) { partial, amount in
            partial + UInt64((amount * 100_000_000).rounded())
        }
        try await wallet.waitForSpendable(exactly: expectedSpendable, timeout: 90)

        let utxosBeforeBuild = try bip44Utxos(for: platformWallet)
        let vaultAddress = try await env.coreRPC.getNewAddress()
        let memoData = Data(memo.utf8)

        let builder = try CoreTransactionBuilder(network: .regtest)
        try builder.addOutput(address: vaultAddress, amountDuffs: depositAmountDuffs)
        try builder.addOpReturn(memoData)
        try builder.preserveOutputOrder()
        try builder.changeToFirstInput()
        let tx = try builder.finalizeAtomic(
            wallet: platformWallet,
            accountType: .bip44,
            accountIndex: Constants.bip44AccountIndex
        )
        let txData = try tx.serializedData()

        let decoded = try TransactionDecoder.decode(txData, network: .regtest)
        let memoOutput = decoded.outputs[1]
        let decodedMemoData = try XCTUnwrap(opReturnPayload(from: memoOutput.scriptPubkey))
        let decodedMemo = try XCTUnwrap(String(data: decodedMemoData, encoding: .utf8))

        let inputZeroMatch = try findMatchedUTXO(for: decoded.inputs[0], in: utxosBeforeBuild)
        let outputTwoMatchesInputZeroScript: Bool
        if decoded.outputs.count == 3 {
            outputTwoMatchesInputZeroScript = decoded.outputs[2].scriptPubkey == inputZeroMatch.scriptPubkey
        } else {
            outputTwoMatchesInputZeroScript = false
        }

        return DepositObservation(
            name: name,
            txHex: hex(txData),
            serializedSize: txData.count,
            inputCount: decoded.inputs.count,
            outputCount: decoded.outputs.count,
            vaultAddress: vaultAddress,
            depositAmount: depositAmountDuffs,
            actualVaultAmount: decoded.outputs[0].valueDuffs,
            memo: memo,
            decodedMemo: decodedMemo,
            memoBytes: memoData.count,
            outputOneIsOpReturn: memoOutput.scriptPubkey.first == 0x6a,
            inputZeroAddress: decoded.inputs.first?.address,
            outputTwoMatchesInputZeroScript: outputTwoMatchesInputZeroScript,
            feeDuffs: tx.fee,
            feeRateDuffsPerByte: Double(tx.fee) / Double(txData.count)
        )
    }

    private func assertDepositObservation(_ observation: DepositObservation) {
        XCTAssertGreaterThanOrEqual(observation.outputCount, 2, "\(observation.name) output count below Maya minimum")
        XCTAssertLessThanOrEqual(observation.outputCount, 3, "\(observation.name) output count above Maya maximum")
        XCTAssertEqual(observation.actualVaultAmount, observation.depositAmount, "\(observation.name) VOUT0 amount mismatch")
        XCTAssertTrue(observation.outputOneIsOpReturn, "\(observation.name) VOUT1 is not OP_RETURN")
        XCTAssertEqual(observation.decodedMemo, observation.memo, "\(observation.name) memo payload mismatch")
        XCTAssertGreaterThanOrEqual(observation.depositAmount, Constants.minimumDepositDuffs, "\(observation.name) deposit fell below Maya dust floor")
        XCTAssertLessThanOrEqual(observation.memoBytes, Constants.maxMemoBytes, "\(observation.name) memo exceeded 80 bytes")
        XCTAssertGreaterThanOrEqual(observation.feeDuffs, UInt64(observation.serializedSize), "\(observation.name) fee fell below 1 duff/byte")
        if observation.outputCount == 3 {
            XCTAssertTrue(observation.outputTwoMatchesInputZeroScript, "\(observation.name) VOUT2 does not return change to VIN0 scriptPubKey")
        }
    }

    private func legacyFeeObservationForOrdinarySend() async throws -> LegacyFeeObservation {
        let wallet = try await env.makeTestWallet(name: "legacy-ordinary")
        let coreWallet = wallet.getCoreWallet()
        let platformWallet = wallet.getPlatformWallet()

        let fundingAddress = try coreWallet.nextReceiveAddress()
        _ = try await fundByMining(address: fundingAddress, dash: 0.5)
        try await wallet.waitForSpendable(exactly: 50_000_000, timeout: 90)

        let utxosBeforeBuild = try bip44Utxos(for: platformWallet)
        let recipient = try await env.coreRPC.getNewAddress()

        let builder = try CoreTransactionBuilder(network: .regtest)
        try builder.addOutput(address: recipient, amountDuffs: 1_000_000)
        let tx = try builder.finalizeAtomic(
            wallet: platformWallet,
            accountType: .bip44,
            accountIndex: Constants.bip44AccountIndex
        )
        let txData = try tx.serializedData()

        return try makeLegacyFeeObservation(
            name: "ordinary-single-recipient-send",
            txData: txData,
            actualFee: tx.fee,
            utxosBeforeBuild: utxosBeforeBuild
        )
    }

    private func legacyFeeObservationForMultiRecipientSend() async throws -> LegacyFeeObservation {
        let wallet = try await env.makeTestWallet(name: "legacy-multi-recipient")
        let coreWallet = wallet.getCoreWallet()
        let platformWallet = wallet.getPlatformWallet()

        let fundingAddress = try coreWallet.nextReceiveAddress()
        _ = try await fundByMining(address: fundingAddress, dash: 0.5)
        try await wallet.waitForSpendable(exactly: 50_000_000, timeout: 90)

        let utxosBeforeBuild = try bip44Utxos(for: platformWallet)
        let recipientA = try await env.coreRPC.getNewAddress()
        let recipientB = try await env.coreRPC.getNewAddress()

        let builder = try CoreTransactionBuilder(network: .regtest)
        try builder.addOutput(address: recipientA, amountDuffs: 1_000_000)
        try builder.addOutput(address: recipientB, amountDuffs: 2_000_000)
        let tx = try builder.finalizeAtomic(
            wallet: platformWallet,
            accountType: .bip44,
            accountIndex: Constants.bip44AccountIndex
        )
        let txData = try tx.serializedData()

        return try makeLegacyFeeObservation(
            name: "multi-recipient-bip70-shape",
            txData: txData,
            actualFee: tx.fee,
            utxosBeforeBuild: utxosBeforeBuild
        )
    }

    private func legacyFeeObservationForSelectedInputShape() async throws -> LegacyFeeObservation {
        let wallet = try await env.makeTestWallet(name: "legacy-selected-input")
        let coreWallet = wallet.getCoreWallet()
        let platformWallet = wallet.getPlatformWallet()

        let fundingAddress = try coreWallet.nextReceiveAddress()
        _ = try await fundByMining(address: fundingAddress, dash: 0.5)
        try await wallet.waitForSpendable(exactly: 50_000_000, timeout: 90)

        let selectedUtxos = try bip44Utxos(for: platformWallet)
        let recipient = try await env.coreRPC.getNewAddress()

        let builder = try CoreTransactionBuilder(network: .regtest)
        try builder.addInputs(
            wallet: platformWallet,
            accountType: .bip44,
            accountIndex: Constants.bip44AccountIndex,
            utxos: selectedUtxos
        )
        try builder.addOutput(address: recipient, amountDuffs: 1_000_000)
        try builder.setChangeAddress(fundingAddress)
        try builder.setFeeRate(satPerKb: Constants.feeRateSatPerKb)
        let tx = try builder.finalizeAtomic(
            wallet: platformWallet,
            accountType: .bip44,
            accountIndex: Constants.bip44AccountIndex
        )
        let txData = try tx.serializedData()

        return try makeLegacyFeeObservation(
            name: "selected-input-send-shape",
            txData: txData,
            actualFee: tx.fee,
            utxosBeforeBuild: selectedUtxos
        )
    }

    private func legacyFeeObservationForDrainShape() async throws -> LegacyFeeObservation {
        let wallet = try await env.makeTestWallet(name: "legacy-drain")
        let coreWallet = wallet.getCoreWallet()
        let platformWallet = wallet.getPlatformWallet()

        let fundingAddress = try coreWallet.nextReceiveAddress()
        _ = try await fundByMining(address: fundingAddress, dash: 0.5)
        try await wallet.waitForSpendable(exactly: 50_000_000, timeout: 90)

        let utxosBeforeBuild = try bip44Utxos(for: platformWallet)
        let recipient = try await env.coreRPC.getNewAddress()

        let builder = try CoreTransactionBuilder(network: .regtest)
        try builder.addInputs(
            wallet: platformWallet,
            accountType: .bip44,
            accountIndex: Constants.bip44AccountIndex,
            utxos: utxosBeforeBuild
        )
        try builder.setSelectionStrategy(.all)
        try builder.setFeeRate(satPerKb: Constants.feeRateSatPerKb)
        try builder.addOutput(address: recipient, amountDuffs: 0)
        let tx = try builder.finalizeAtomic(
            wallet: platformWallet,
            accountType: .bip44,
            accountIndex: Constants.bip44AccountIndex
        )
        let txData = try tx.serializedData()

        return try makeLegacyFeeObservation(
            name: "drain-shape-coinjoin-sweep-equivalent",
            txData: txData,
            actualFee: tx.fee,
            utxosBeforeBuild: utxosBeforeBuild
        )
    }

    private func legacyFeeObservationForAssetLockShape(
        name: String,
        fundingType: ManagedAssetLockManager.FundingType
    ) async throws -> LegacyFeeObservation {
        let wallet = try await env.makeTestWallet(name: name)
        let coreWallet = wallet.getCoreWallet()
        let platformWallet = wallet.getPlatformWallet()

        let fundingAddress = try coreWallet.nextReceiveAddress()
        _ = try await fundByMining(address: fundingAddress, dash: 0.5)
        try await wallet.waitForSpendable(exactly: 50_000_000, timeout: 90)

        let utxosBeforeBuild = try bip44Utxos(for: platformWallet)
        let manager = try platformWallet.assetLockManager()
        let resolver = MnemonicResolver()
        let built = try manager.buildTransaction(
            amountDuffs: 10_000_000,
            accountIndex: Constants.bip44AccountIndex,
            fundingType: fundingType,
            identityIndex: 0,
            resolver: resolver
        )

        let decoded = try TransactionDecoder.decode(built.transaction, network: .regtest)
        let actualFee = try sumSelectedInputs(decoded.inputs, from: utxosBeforeBuild)
            - decoded.outputs.reduce(UInt64(0)) { $0 + $1.valueDuffs }

        return try makeLegacyFeeObservation(
            name: name,
            txData: built.transaction,
            actualFee: actualFee,
            utxosBeforeBuild: utxosBeforeBuild
        )
    }

    private func makeLegacyFeeObservation(
        name: String,
        txData: Data,
        actualFee: UInt64,
        utxosBeforeBuild: [PlatformWalletManager.AccountUtxo]
    ) throws -> LegacyFeeObservation {
        let decoded = try TransactionDecoder.decode(txData, network: .regtest)
        let layout = try parseTransactionLayout(txData)
        let selectedInputsValue = try sumSelectedInputs(decoded.inputs, from: utxosBeforeBuild)
        let paidOutputs = decoded.outputs.reduce(UInt64(0)) { $0 + $1.valueDuffs }
        XCTAssertEqual(
            actualFee,
            selectedInputsValue - paidOutputs,
            "\(name) reported fee does not match selected-input minus output value"
        )

        let outputsLegacyBytes = layout.outputCount * 34
        let legacyExpected = 8
            + varIntSize(layout.inputCount)
            + layout.inputCount * 148
            + varIntSize(layout.outputCount)
            + outputsLegacyBytes
            + (layout.payloadLength > 0 ? varIntSize(layout.payloadLength) + layout.payloadLength : 0)

        return LegacyFeeObservation(
            name: name,
            actualFeeDuffs: actualFee,
            legacyExpectedFeeDuffs: UInt64(legacyExpected),
            serializedSize: txData.count,
            inputCount: layout.inputCount,
            outputCount: layout.outputCount,
            payloadLength: layout.payloadLength
        )
    }

    @discardableResult
    private func fundByMining(address: String, dash: Double) async throws -> String {
        let txid = try await env.coreRPC.sendToAddress(amount: dash, address: address)
        _ = try await env.mine(1)
        return txid
    }

    private func bip44Utxos(for wallet: ManagedPlatformWallet) throws -> [PlatformWalletManager.AccountUtxo] {
        let walletId = wallet.walletId
        guard let balance = env.walletManager.accountBalances(for: walletId).first(where: {
            $0.typeTag == Constants.bip44TypeTag
                && $0.standardTag == Constants.bip44StandardTag
                && $0.index == Constants.bip44AccountIndex
        }) else {
            throw XCTSkip("BIP44 account balance was not materialized")
        }
        return env.walletManager.accountUtxos(for: walletId, balance: balance).filter { !$0.isLocked }
    }

    private func findMatchedUTXO(
        for input: DecodedTransaction.Input,
        in utxos: [PlatformWalletManager.AccountUtxo]
    ) throws -> PlatformWalletManager.AccountUtxo {
        guard let match = utxos.first(where: {
            $0.outpointTxid == input.prevTxid && $0.outpointVout == input.prevVout
        }) else {
            throw NSError(domain: "MayaVerification", code: 1, userInfo: [
                NSLocalizedDescriptionKey: "Could not match input \(hex(input.prevTxid)):\(input.prevVout) to a pre-build UTXO"
            ])
        }
        return match
    }

    private func sumSelectedInputs(
        _ inputs: [DecodedTransaction.Input],
        from utxos: [PlatformWalletManager.AccountUtxo]
    ) throws -> UInt64 {
        var total: UInt64 = 0
        for input in inputs {
            total += try findMatchedUTXO(for: input, in: utxos).valueDuffs
        }
        return total
    }

    private func parseTransactionLayout(_ txData: Data) throws -> ParsedTransactionLayout {
        var offset = 0

        guard txData.count >= 4 else {
            throw NSError(domain: "MayaVerification", code: 2, userInfo: [
                NSLocalizedDescriptionKey: "Serialized transaction too short"
            ])
        }
        offset += 4

        let inputCount = try Int(readVarInt(from: txData, offset: &offset))
        for _ in 0..<inputCount {
            offset += 32 + 4
            let scriptLength = try Int(readVarInt(from: txData, offset: &offset))
            offset += scriptLength + 4
        }

        let outputCount = try Int(readVarInt(from: txData, offset: &offset))
        for _ in 0..<outputCount {
            offset += 8
            let scriptLength = try Int(readVarInt(from: txData, offset: &offset))
            offset += scriptLength
        }

        offset += 4

        let payloadLength: Int
        if offset < txData.count {
            payloadLength = try Int(readVarInt(from: txData, offset: &offset))
            offset += payloadLength
        } else {
            payloadLength = 0
        }

        guard offset == txData.count else {
            throw NSError(domain: "MayaVerification", code: 3, userInfo: [
                NSLocalizedDescriptionKey: "Unexpected trailing bytes while parsing transaction layout"
            ])
        }

        return ParsedTransactionLayout(
            inputCount: inputCount,
            outputCount: outputCount,
            payloadLength: payloadLength
        )
    }

    private func readVarInt(from data: Data, offset: inout Int) throws -> UInt64 {
        guard offset < data.count else {
            throw NSError(domain: "MayaVerification", code: 4, userInfo: [
                NSLocalizedDescriptionKey: "Unexpected end of transaction while reading varint"
            ])
        }

        let prefix = data[offset]
        offset += 1

        switch prefix {
        case 0x00...0xfc:
            return UInt64(prefix)
        case 0xfd:
            guard offset + 2 <= data.count else { throw truncatedVarIntError() }
            let value = UInt64(data[offset]) | (UInt64(data[offset + 1]) << 8)
            offset += 2
            return value
        case 0xfe:
            guard offset + 4 <= data.count else { throw truncatedVarIntError() }
            let value = UInt64(data[offset])
                | (UInt64(data[offset + 1]) << 8)
                | (UInt64(data[offset + 2]) << 16)
                | (UInt64(data[offset + 3]) << 24)
            offset += 4
            return value
        default:
            guard offset + 8 <= data.count else { throw truncatedVarIntError() }
            var value: UInt64 = 0
            for shift in 0..<8 {
                value |= UInt64(data[offset + shift]) << (8 * UInt64(shift))
            }
            offset += 8
            return value
        }
    }

    private func truncatedVarIntError() -> NSError {
        NSError(domain: "MayaVerification", code: 5, userInfo: [
            NSLocalizedDescriptionKey: "Unexpected end of transaction while reading extended varint"
        ])
    }

    private func opReturnPayload(from script: Data) -> Data? {
        guard script.count >= 2, script[0] == 0x6a else { return nil }
        let pushOpcode = script[1]

        switch pushOpcode {
        case 0x01...0x4b:
            let length = Int(pushOpcode)
            guard script.count == 2 + length else { return nil }
            return script.subdata(in: 2..<(2 + length))
        case 0x4c:
            guard script.count >= 3 else { return nil }
            let length = Int(script[2])
            guard script.count == 3 + length else { return nil }
            return script.subdata(in: 3..<(3 + length))
        case 0x4d:
            guard script.count >= 4 else { return nil }
            let length = Int(script[2]) | (Int(script[3]) << 8)
            guard script.count == 4 + length else { return nil }
            return script.subdata(in: 4..<(4 + length))
        default:
            return nil
        }
    }

    private func varIntSize(_ value: Int) -> Int {
        switch value {
        case 0...0xfc:
            return 1
        case 0xfd...0xffff:
            return 3
        case 0x1_0000...0xffff_ffff:
            return 5
        default:
            return 9
        }
    }

    private func hex(_ data: Data) -> String {
        data.map { String(format: "%02x", $0) }.joined()
    }

    private func renderDepositObservation(_ observation: DepositObservation) -> String {
        [
            "PROMPT04_DEPOSIT \(observation.name)",
            "  tx_hex=\(observation.txHex)",
            "  serialized_size=\(observation.serializedSize)",
            "  input_count=\(observation.inputCount)",
            "  output_count=\(observation.outputCount)",
            "  vault_address=\(observation.vaultAddress)",
            "  deposit_amount=\(observation.depositAmount)",
            "  actual_vout0_amount=\(observation.actualVaultAmount)",
            "  memo_bytes=\(observation.memoBytes)",
            "  input0_address=\(observation.inputZeroAddress ?? "nil")",
            "  output2_matches_input0_script=\(observation.outputTwoMatchesInputZeroScript)",
            "  fee_duffs=\(observation.feeDuffs)",
            String(format: "  fee_rate_duffs_per_byte=%.6f", observation.feeRateDuffsPerByte),
        ].joined(separator: "\n")
    }

    private func renderLegacyFeeObservation(_ observation: LegacyFeeObservation) -> String {
        [
            "PROMPT04_FEE \(observation.name)",
            "  actual_fee_duffs=\(observation.actualFeeDuffs)",
            "  legacy_expected_fee_duffs=\(observation.legacyExpectedFeeDuffs)",
            "  serialized_size=\(observation.serializedSize)",
            "  input_count=\(observation.inputCount)",
            "  output_count=\(observation.outputCount)",
            "  payload_length=\(observation.payloadLength)",
        ].joined(separator: "\n")
    }
}
