#!/usr/bin/env python3
"""Compile the real Settings erase route against a single-operation wallet stub."""

from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
VIEWS = ROOT / "SwiftExampleApp/SwiftExampleApp/Views"


@unittest.skipUnless(shutil.which("swiftc"), "Swift compiler unavailable")
class DocumentErasePreflightTests(unittest.TestCase):
    def test_should_display_rust_outcomes_without_orchestrating_an_erase(self):
        source = (VIEWS / "TransitionDetailView.swift").read_text()
        start = source.index("  private func executeDocumentErase(")
        end = source.index("\n  private func executeDocumentTransfer(", start)
        display = (VIEWS / "DocumentsView.swift").read_text()
        display_start = display.index("func describeDocumentEraseOutcome(")
        display_end = display.index("struct EraseDocumentView:", display_start)
        harness = r'''
import Foundation

enum SDKError: Error { case invalidParameter(String) }
enum PlatformWalletError: Error { case transactionBroadcastUnconfirmed(String) }
enum LifecycleState: String { case active, deleted, erasing, absent }
struct DocumentHistoryLifecycle {
    let state: LifecycleState
    let remainingRevisions: UInt64
    static func describeEraseProgress(before: Self?, after: Self) -> String {
        after.state == .absent ? "Erasure complete" : "Retained revisions remain"
    }
}
enum DocumentEraseNotSubmittedReason {
    case active, absent, preflightUnavailable, notNewestOwner(Data)
}
enum DocumentEraseOutcome {
    case notSubmitted(reason: DocumentEraseNotSubmittedReason, before: DocumentHistoryLifecycle?)
    case submitted(before: DocumentHistoryLifecycle?, after: DocumentHistoryLifecycle?)
    var before: DocumentHistoryLifecycle? {
        switch self { case .notSubmitted(_, let b), .submitted(let b, _): return b }
    }
    var after: DocumentHistoryLifecycle? {
        if case .submitted(_, let a) = self { return a }; return nil
    }
}
extension Data {
    static func identifier(fromBase58: String) -> Data? { Data(repeating: 1, count: 32) }
    func toBase58String() -> String { "owner" }
}
struct Identity {
    let identityIdBase58 = "signer"
    let identityId = Data(repeating: 1, count: 32)
    var wallet: WalletLink? = WalletLink()
}
struct WalletLink { let walletId = Data() }
struct ModelContext { let container = 0 }
struct KeychainSigner { init(modelContainer: Int) {} }
struct SDK {}
@MainActor final class Wallet {
    let outcome: DocumentEraseOutcome
    var calls = 0
    init(_ outcome: DocumentEraseOutcome) { self.outcome = outcome }
    func eraseDocumentAndObserve(submittingIdentityId: Data, contractId: Data,
        documentType: String, documentId: Data, signer: KeychainSigner) async throws -> DocumentEraseOutcome {
        calls += 1
        return outcome
    }
}
@MainActor struct WalletManager {
    let loaded: Wallet
    func wallet(for: Data) -> Wallet? { loaded }
}
''' + display[display_start:display_end] + r'''
@MainActor struct SettingsRoute {
    let selectedIdentityId = "signer"
    let identities: [Identity]
    let formInputs = ["contractId": "contract", "documentType": "note", "documentId": "doc"]
    let modelContext = ModelContext()
    let walletManager: WalletManager
    func run() async throws -> [String: Any] {
        try await executeDocumentErase(sdk: SDK()) as! [String: Any]
    }
''' + source[start:end] + r'''
}
@main struct Main {
    @MainActor static func main() async throws {
        let erasing = DocumentHistoryLifecycle(state: .erasing, remainingRevisions: 50)
        let outcomes: [DocumentEraseOutcome] = [
            .notSubmitted(reason: .active, before: nil),
            .notSubmitted(reason: .absent, before: nil),
            .notSubmitted(reason: .preflightUnavailable, before: nil),
            .notSubmitted(reason: .notNewestOwner(Data()), before: erasing),
            .submitted(before: erasing, after: nil),
            .submitted(before: erasing, after: .init(state: .absent, remainingRevisions: 0)),
        ]
        for outcome in outcomes {
            let wallet = Wallet(outcome)
            let route = SettingsRoute(identities: [Identity()], walletManager: .init(loaded: wallet))
            let result = try await route.run()
            precondition(wallet.calls == 1, "host must call the single Rust operation")
            let message = result["message"] as! String
            switch outcome {
            case .notSubmitted(let reason, _):
                precondition(message.hasPrefix("Erase not submitted"))
                if case .absent = reason { precondition(!message.contains("Erasure complete")) }
            case .submitted(_, let after):
                precondition(message.hasPrefix("Erase submitted"))
                if after == nil { precondition(message.contains("Lifecycle not read")) }
            }
        }
        let wallet = Wallet(outcomes[0])
        var unlinked = Identity(); unlinked.wallet = nil
        do {
            _ = try await SettingsRoute(identities: [unlinked], walletManager: .init(loaded: wallet)).run()
            fatalError("orphaned identity must not reach Rust")
        } catch SDKError.invalidParameter(let message) {
            precondition(message == "Identity has no wallet linkage; cannot erase document history")
        }
        precondition(wallet.calls == 0)
        print("Six typed outcomes and wallet-linkage control passed")
    }
}
'''
        with tempfile.TemporaryDirectory() as directory:
            source_path = Path(directory) / "EraseRoute.swift"
            binary = Path(directory) / "erase-route"
            source_path.write_text(harness)
            compiled = subprocess.run(
                ["swiftc", "-parse-as-library", str(source_path), "-o", str(binary)],
                capture_output=True, text=True, check=False,
            )
            self.assertEqual(compiled.returncode, 0, compiled.stdout + compiled.stderr)
            result = subprocess.run([str(binary)], capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
