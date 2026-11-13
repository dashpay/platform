#!/usr/bin/env python3
"""Run the Settings erase route against an offline SDK, without an iOS build."""

from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


VIEW = (Path(__file__).resolve().parents[1] / "SwiftExampleApp" /
        "SwiftExampleApp/Views/TransitionDetailView.swift")


@unittest.skipUnless(shutil.which("swiftc"), "Swift compiler unavailable")
class DocumentErasePreflightTests(unittest.TestCase):
    def test_should_skip_active_and_absent_without_claiming_an_erase_completed(self):
        source = VIEW.read_text()
        start = source.index("  private func executeDocumentErase(")
        end = source.index("\n  private func executeDocumentTransfer(", start)
        # Compile the actual route, including its reads and broadcast call.
        # The stubs isolate presentation behavior from SwiftUI and the FFI.
        harness = r'''
import Foundation

enum SDKError: Error { case invalidParameter(String) }
enum LifecycleState: String, CaseIterable { case active, deleted, erasing, absent }
struct DocumentHistoryLifecycle {
    let state: LifecycleState
    let remainingRevisions: UInt64
    static func describeEraseProgress(
        before: Self?, after: Self
    ) -> String {
        after.state == .absent ? "Erasure complete: no retained revisions remain." : "Retained revisions remain."
    }
}
struct PublicKey { let id: UInt32 = 0 }
struct Identity {
    let identityIdBase58 = "signer"
    let identityId = "signer"
    let identityPublicKeys = [PublicKey()]
    let balance: Int64 = 100
}
struct DPPIdentity {
    let id: String
    let publicKeys: [UInt32: PublicKey]
    let balance: UInt64
    let revision: UInt32
}
struct ModelContext { let container = 0 }
struct KeychainSigner {
    init(modelContainer: Int) {}
    let handle = 0
}
@MainActor final class SDK {
    let initialState: LifecycleState?
    var reads = 0
    var broadcasts = 0
    init(_ state: LifecycleState?) { initialState = state }
    func documentGetLifecycle(
        dataContractId: String, documentType: String, documentId: String
    ) async throws -> DocumentHistoryLifecycle {
        reads += 1
        if reads == 1 {
            guard let initialState else { throw SDKError.invalidParameter("Read unavailable") }
            return DocumentHistoryLifecycle(state: initialState, remainingRevisions: 0)
        }
        return DocumentHistoryLifecycle(state: .absent, remainingRevisions: 0)
    }
    func documentErase(
        contractId: String, documentType: String, documentId: String,
        ownerIdentity: DPPIdentity, signer: Int
    ) async throws {
        broadcasts += 1
    }
}
@MainActor struct SettingsRoute {
    let selectedIdentityId = "signer"
    let identities = [Identity()]
    let formInputs = ["contractId": "contract", "documentType": "note", "documentId": "doc"]
    let modelContext = ModelContext()
    func run(_ sdk: SDK) async throws -> [String: Any] {
        try await executeDocumentErase(sdk: sdk) as! [String: Any]
    }
''' + source[start:end] + r'''
}
@main struct Main {
    @MainActor static func main() async throws {
        var failures: [String] = []
        func require(_ condition: Bool, _ message: String) {
            if !condition { failures.append(message) }
        }
        let states: [LifecycleState?] = [.active, .absent, .deleted, .erasing, nil]
        for state in states {
            let sdk = SDK(state)
            let result = try await SettingsRoute().run(sdk)
            let label = state?.rawValue ?? "unreadable"
            if state == .active || state == .absent {
                require(sdk.broadcasts == 0, "\(label): must not broadcast a paid erase")
                require(sdk.reads == 1, "\(label): no closing read without a broadcast")
                require(result["message"] as? String == "Erase not submitted", "\(label): explain the skip")
                require(result["lifecycle"] as? String == label, "\(label): show the observed lifecycle")
                require(result["remainingRevisions"] as? UInt64 == 0, "\(label): show retained revisions")
                let progress = result["progress"] as? String ?? ""
                require(!progress.isEmpty, "\(label): show progress")
                if state == .absent {
                    require(progress.contains("No retained revisions exist"), "absent: describe the observation")
                    require(!progress.contains("Erasure complete"), "absent: an observation does not prove an erase completed")
                }
            } else {
                require(sdk.broadcasts == 1, "\(label): continue the requested erase")
                require(sdk.reads == 2, "\(label): describe the result after broadcast")
                require(result["message"] as? String == "Erase submitted; document absence observed under proof",
                        "\(label): preserve submission status")
            }
        }
        if !failures.isEmpty {
            failures.forEach { print($0) }
            exit(1)
        }
        print("Five erase preflight controls passed")
    }
}
'''
        with tempfile.TemporaryDirectory() as directory:
            source_path = Path(directory) / "ErasePreflight.swift"
            binary = Path(directory) / "erase-preflight"
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
