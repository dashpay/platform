import XCTest
@testable import SwiftDashSDK

/// Pure-function tests for `TransactionDecoder` — no wallet handle or network
/// access. The fixture is the same synthetic transaction exercised by the
/// Rust-side unit tests in `rs-platform-wallet-ffi/src/tx_decode.rs`:
/// one P2PKH input (spending 1111…11:3, scriptSig = push(sig) push(pubkey)),
/// one P2PKH output of 151 072 duffs, and one OP_RETURN output.
final class TransactionDecoderTests: XCTestCase {
    private static let fixtureHex =
        "01000000011111111111111111111111111111111111111111111111111111111111111111"
        + "030000006a4730303030303030303030303030303030303030303030303030303030303030"
        + "30303030303030303030303030303030303030303030303030303030303030303030303030"
        + "303030210324653eac434488002cc06bbfb7f10fe18991e35f9fe4302dbea6d2353dc0ab1c"
        + "ffffffff02204e0200000000001976a91414db4138d56a2ecfb10881a9be394d9f321985b2"
        + "88ac0000000000000000066a04aaaaaaaa00000000"

    private static let fixtureAddress = "yNDj28QBMm5sY6bLjFcNdWRNef24KLQNuQ"
    private static let fixtureTxidDisplay =
        "bf7479216e5ba76f60bf11654c881824c6f9cdbb64eebe332cf835a3391cb5d5"

    private var fixtureData: Data {
        var data = Data()
        let hex = Self.fixtureHex
        var index = hex.startIndex
        while index < hex.endIndex {
            let next = hex.index(index, offsetBy: 2)
            data.append(UInt8(hex[index..<next], radix: 16)!)
            index = next
        }
        return data
    }

    func testDecodesOutputsWithAddressesAndValues() throws {
        let decoded = try TransactionDecoder.decode(fixtureData, network: .testnet)

        XCTAssertEqual(decoded.outputs.count, 2)

        // P2PKH output: exact signal amount, testnet address.
        XCTAssertEqual(decoded.outputs[0].address, Self.fixtureAddress)
        XCTAssertEqual(decoded.outputs[0].valueDuffs, 151_072)
        XCTAssertFalse(decoded.outputs[0].scriptPubkey.isEmpty)

        // OP_RETURN output: no address, script still exposed.
        XCTAssertNil(decoded.outputs[1].address)
        XCTAssertFalse(decoded.outputs[1].scriptPubkey.isEmpty)
    }

    func testRecoversP2PKHInputAddressAndOutpoint() throws {
        let decoded = try TransactionDecoder.decode(fixtureData, network: .testnet)

        XCTAssertEqual(decoded.inputs.count, 1)
        XCTAssertEqual(decoded.inputs[0].prevTxid, Data(repeating: 0x11, count: 32))
        XCTAssertEqual(decoded.inputs[0].prevVout, 3)
        XCTAssertEqual(decoded.inputs[0].address, Self.fixtureAddress)
    }

    func testTxidDisplayHexMatchesExplorerOrder() throws {
        let decoded = try TransactionDecoder.decode(fixtureData, network: .testnet)
        XCTAssertEqual(decoded.txidDisplayHex, Self.fixtureTxidDisplay)
        XCTAssertEqual(decoded.txid, Data(decoded.txid.reversed().reversed()))
    }

    func testNetworkChangesRenderedAddresses() throws {
        let mainnet = try TransactionDecoder.decode(fixtureData, network: .mainnet)
        XCTAssertNotEqual(mainnet.outputs[0].address, Self.fixtureAddress)
        XCTAssertTrue(mainnet.outputs[0].address?.hasPrefix("X") ?? false)
    }

    func testMalformedBytesThrowDeserialization() {
        var bytes = fixtureData
        bytes.append(contentsOf: [0xDE, 0xAD]) // trailing garbage
        XCTAssertThrowsError(try TransactionDecoder.decode(bytes, network: .testnet))
        XCTAssertThrowsError(try TransactionDecoder.decode(Data([0xFF, 0xFF, 0xFF]), network: .testnet))
    }

    func testEmptyInputThrowsInvalidParameter() {
        XCTAssertThrowsError(try TransactionDecoder.decode(Data(), network: .testnet))
    }
}
