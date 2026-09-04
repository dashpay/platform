import Foundation
import XCTest
@testable import SwiftDashSDK

final class CoreWalletDiagnosticAnalyzerTests: XCTestCase {
    typealias AccountKey = CoreWalletDatabaseDiagnosticSnapshot.AccountKey
    typealias AssetLock = CoreWalletDatabaseDiagnosticSnapshot.AssetLock
    typealias Txo = CoreWalletDatabaseDiagnosticSnapshot.Txo

    private func account(
        type: UInt32 = 0,
        standardTag: UInt8 = 0,
        index: UInt32 = 0
    ) -> AccountKey {
        AccountKey(
            typeTag: type,
            standardTag: standardTag,
            index: index,
            registrationIndex: 0,
            keyClass: 0,
            userIdentityId: Data(),
            friendIdentityId: Data()
        )
    }

    private func outpoint(_ marker: UInt8) -> Data {
        Data(repeating: marker, count: 32) + Data([0, 0, 0, 0])
    }

    private func txo(
        _ marker: UInt8,
        amount: UInt64 = 100,
        height: UInt32 = 200,
        script: Data = Data([0x51]),
        locked: Bool = false,
        account: AccountKey? = nil
    ) -> Txo {
        Txo(
            outpoint: outpoint(marker),
            amount: amount,
            height: height,
            scriptPubKey: script,
            isLocked: locked,
            account: account ?? self.account()
        )
    }

    private func assetLock(
        _ outpoint: String,
        fundingType: Int = 5,
        status: Int = 1,
        accountIndex: UInt32 = 2,
        registrationIndex: UInt32 = 3,
        amount: UInt64? = 400,
        hasProof: Bool = true
    ) -> AssetLock {
        AssetLock(
            outpointDisplay: outpoint,
            fundingType: fundingType,
            status: status,
            accountIndex: accountIndex,
            registrationIndex: registrationIndex,
            amountDuffs: amount,
            hasProof: hasProof
        )
    }

    func testTxoDiffExactDatabaseOnlyMemoryOnlyAndEveryFieldMismatch() {
        let baseAccount = account()
        let exact = txo(0x01, account: baseAccount)
        let exactResult = CoreWalletDiagnosticAnalyzer.compareTxos(
            database: [exact],
            memory: [exact],
            databaseAccounts: [baseAccount],
            memoryAccounts: [baseAccount]
        )
        XCTAssertEqual(exactResult.commonCount, 1)
        XCTAssertEqual(exactResult.databaseAccountOnlyCount, 0)
        XCTAssertEqual(exactResult.memoryAccountOnlyCount, 0)
        XCTAssertTrue(exactResult.details.isEmpty)

        let database = [
            txo(0x10),
            txo(0x20, amount: 101),
            txo(0x21, height: 201),
            txo(0x22, script: Data([0x52])),
            txo(0x23, locked: true),
            txo(0x24, account: account(type: 1)),
        ]
        let memory = [
            txo(0x11),
            txo(0x20, amount: 102),
            txo(0x21, height: 202),
            txo(0x22, script: Data([0x53])),
            txo(0x23, locked: false),
            txo(0x24, account: account(type: 0)),
        ]
        let result = CoreWalletDiagnosticAnalyzer.compareTxos(
            database: database,
            memory: memory,
            databaseAccounts: [baseAccount, account(type: 1)],
            memoryAccounts: [baseAccount, account(type: 2)]
        )

        XCTAssertEqual(result.commonCount, 5)
        XCTAssertEqual(result.databaseAccountOnlyCount, 1)
        XCTAssertEqual(result.memoryAccountOnlyCount, 1)
        XCTAssertEqual(result.databaseOnlyCount, 1)
        XCTAssertEqual(result.memoryOnlyCount, 1)
        XCTAssertEqual(result.fieldMismatchCount, 5)
        XCTAssertEqual(Set(result.details.map(\.reason)), [
            "account_mismatch",
            "amount_mismatch",
            "database_only",
            "height_mismatch",
            "lock_mismatch",
            "memory_only",
            "script_mismatch",
        ])
    }

    func testTxoDiffLimitsEachReasonToTwentyFiveDetails() {
        let database = (0..<30).map { index in
            txo(UInt8(index + 1))
        }
        let result = CoreWalletDiagnosticAnalyzer.compareTxos(
            database: database,
            memory: [],
            databaseAccounts: [],
            memoryAccounts: []
        )

        XCTAssertEqual(result.details.count, 30)
        XCTAssertEqual(result.emittedDetails.count, 25)
        XCTAssertEqual(result.truncatedCount, 5)
        XCTAssertTrue(result.emittedDetails.allSatisfy { $0.reason == "database_only" })
        XCTAssertEqual(
            result.emittedDetails.map(\.outpoint),
            result.emittedDetails.map(\.outpoint).sorted {
                $0.lexicographicallyPrecedes($1)
            }
        )
    }

    func testAssetLockDiffExactAndEveryMismatchClass() {
        let exact = assetLock("exact:0")
        let exactResult = CoreWalletDiagnosticAnalyzer.compareAssetLocks(
            database: [exact],
            memory: [exact]
        )
        XCTAssertTrue(exactResult.details.isEmpty)

        let result = CoreWalletDiagnosticAnalyzer.compareAssetLocks(
            database: [
                assetLock("database-only:0"),
                assetLock("different:0"),
            ],
            memory: [
                assetLock("memory-only:0"),
                assetLock(
                    "different:0",
                    fundingType: 4,
                    status: 2,
                    accountIndex: 7,
                    registrationIndex: 8,
                    amount: 401,
                    hasProof: false
                ),
            ]
        )

        XCTAssertEqual(Set(result.details.map(\.reason)), [
            "account_index_mismatch",
            "amount_mismatch",
            "database_only",
            "funding_type_mismatch",
            "memory_only",
            "proof_presence_mismatch",
            "registration_index_mismatch",
            "status_mismatch",
        ])
        XCTAssertEqual(result.emittedDetails.count, result.details.count)
        XCTAssertEqual(result.truncatedCount, 0)
    }

    func testMissingAccountIsDatabaseAnomalyAndRejectedFromRestoreBuffer() {
        let missingAccountTxo = Txo(
            outpoint: outpoint(0x30),
            amount: 700,
            height: 900,
            scriptPubKey: Data([0x51]),
            isLocked: false,
            account: nil
        )
        let anomalies = CoreWalletDiagnosticAnalyzer.databaseTxoAnomalies([
            .init(
                txo: missingAccountTxo,
                hasParentTransaction: true,
                walletIdMismatch: false,
                isSpent: false,
                hasSpendingTransaction: false
            ),
        ])
        XCTAssertEqual(anomalies.count(reason: "missing_account"), 1)

        let rejected = CoreWalletDiagnosticAnalyzer.RestoreCandidate(
            amount: missingAccountTxo.amount,
            accountType: nil,
            standardTag: nil,
            rejectionReason: .missingAccount
        )
        let acceptedTxo = txo(0x31, amount: 800)
        let accepted = CoreWalletDiagnosticAnalyzer.RestoreCandidate(
            amount: acceptedTxo.amount,
            accountType: 0,
            standardTag: 0,
            rejectionReason: nil
        )
        let summary = CoreWalletDiagnosticAnalyzer.summarizeRestoreBuffer(
            candidates: [rejected, accepted],
            emittedCount: 1,
            errored: false
        )

        XCTAssertEqual(summary.candidateCount, 2)
        XCTAssertEqual(summary.candidateValueDuffs, 1_500)
        XCTAssertEqual(summary.missingAccountCount, 1)
        XCTAssertEqual(summary.emittedCandidates.count, 1)
        XCTAssertEqual(summary.emittedCandidates.first?.amount, acceptedTxo.amount)
        XCTAssertEqual(summary.emittedValueDuffs, 800)
    }

    func testShieldedStoreSummaryIncludesValuesActivityKeysAndWatermark() {
        let summary = CoreWalletDiagnosticAnalyzer.summarizeShieldedStore(
            notes: [
                .init(value: 7, isSpent: true),
                .init(value: 8, isSpent: true),
                .init(value: 20, isSpent: false),
            ],
            outgoingNoteCount: 2,
            activityStatuses: [0, 1, 2, 0],
            viewingKeyCount: 3,
            syncWatermarks: [5, 99, 40]
        )

        XCTAssertEqual(summary.noteCount, 3)
        XCTAssertEqual(summary.spentNoteCount, 2)
        XCTAssertEqual(summary.spentValueCredits, 15)
        XCTAssertEqual(summary.unspentNoteCount, 1)
        XCTAssertEqual(summary.unspentValueCredits, 20)
        XCTAssertEqual(summary.outgoingNoteCount, 2)
        XCTAssertEqual(summary.activityCount, 4)
        XCTAssertEqual(summary.activityPendingCount, 2)
        XCTAssertEqual(summary.activityFailedCount, 1)
        XCTAssertEqual(summary.viewingKeyCount, 3)
        XCTAssertEqual(summary.subwalletSyncStateCount, 3)
        XCTAssertEqual(summary.maximumSyncWatermark, 99)
    }

    func testFingerprintIsStableUnderReorderAndSensitiveToEveryTxoField() {
        let baseAccount = account()
        let first = txo(0x40, account: baseAccount)
        let second = txo(0x41, amount: 200, account: baseAccount)
        let firstMaterial = fingerprintMaterial(first)
        let secondMaterial = fingerprintMaterial(second)

        XCTAssertEqual(
            diagnosticFingerprint([firstMaterial, secondMaterial]),
            diagnosticFingerprint([secondMaterial, firstMaterial])
        )
        XCTAssertNotEqual(firstMaterial, fingerprintMaterial(txo(0x40, amount: 101)))
        XCTAssertNotEqual(firstMaterial, fingerprintMaterial(txo(0x40, height: 201)))
        XCTAssertNotEqual(firstMaterial, fingerprintMaterial(txo(0x40, script: Data([0x52]))))
        XCTAssertNotEqual(firstMaterial, fingerprintMaterial(txo(0x40, locked: true)))
        XCTAssertNotEqual(
            firstMaterial,
            fingerprintMaterial(txo(0x40, account: account(type: 1)))
        )
    }

    func testRescanDiagnosticResultOnlyReportsArmedForARealRewind() {
        XCTAssertEqual(
            coreRescanDiagnosticResult(
                previousSyncedHeight: 2_500_000,
                requestedStartHeight: 2_484_000
            ),
            .armed
        )
        XCTAssertEqual(
            coreRescanDiagnosticResult(
                previousSyncedHeight: 2_484_000,
                requestedStartHeight: 2_484_000
            ),
            .noOp
        )
        XCTAssertEqual(
            coreRescanDiagnosticResult(
                previousSyncedHeight: 2_480_000,
                requestedStartHeight: 2_484_000
            ),
            .acceptedNoRewind
        )
        XCTAssertEqual(
            coreRescanDiagnosticResult(
                previousSyncedHeight: nil,
                requestedStartHeight: 2_484_000
            ),
            .acceptedNoRewind
        )
    }

    private func fingerprintMaterial(_ txo: Txo) -> Data {
        diagnosticTxoFingerprint(
            outpoint: txo.outpoint,
            amount: txo.amount,
            height: txo.height,
            scriptPubKey: txo.scriptPubKey,
            isLocked: txo.isLocked,
            account: txo.account
        )
    }
}
