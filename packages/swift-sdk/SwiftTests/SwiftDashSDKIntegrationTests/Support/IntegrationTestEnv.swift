import Foundation
import XCTest
import SwiftData
import Security
import SwiftDashSDK

enum IntegrationTestEnvError: LocalizedError {
    case timeout(String)

    var errorDescription: String? {
        switch self {
        case .timeout(let msg): return msg
        }
    }
}

/// Suite-wide handle built once per process from
/// `IntegrationTestCase.setUp` and reused across every test.
final class IntegrationTestEnv: @unchecked Sendable {
    let coreRPC: CoreRPCClient
    let sdk: SDK
    private(set) var modelContainer: ModelContainer

    private(set) var walletManager: PlatformWalletManager
    let spvDataDir: String
    var spvCacheSnapshot: String { spvDataDir + "-snapshot" }

    private let endpoints: LocalDevnet.Endpoints
    private let mineRewardAddress: String
    private let masternodePeers: [String]
    private let masternodeRPCs: [CoreRPCClient]

    var spvConfig: PlatformSpvStartConfig {
        PlatformSpvStartConfig(
            dataDir: spvDataDir,
            network: .regtest,
            userAgent: "swift-sdk-integration-tests/0.1",
            peers: ["127.0.0.1:\(endpoints.coreP2P)"] + masternodePeers,
            restrictToConfiguredPeers: true,
            startFromHeight: 0
        )
    }

    private static func makeSPVDataDir() -> String {
        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("swift-sdk-it-spv-\(UUID().uuidString.prefix(8))", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.path
    }

    /// Sync the SPV data dir up to the chain tip once, before any test
    /// runs
    func createSpvCache() async throws {
        let tip = try await coreRPC.getBlockCount()
        try await walletManager.startSpv(config: spvConfig)

        do {
            try await walletManager.waitUntilUpToDate(height: tip, timeout: 180)
        } catch {
            try? await walletManager.stopSpv()
            throw error
        }

        try await walletManager.stopSpv()
    }

    /// Clone the freshly-synced SPV cache so every test can be handed a
    /// pristine copy of it via `restoreSpvCacheFromSnapshot`.
    func snapshotSpvCache() {
        let fm = FileManager.default
        try? fm.removeItem(atPath: spvCacheSnapshot)
        try? fm.copyItem(atPath: spvDataDir, toPath: spvCacheSnapshot)
    }

    /// Replace the working SPV data dir with a fresh copy of the
    /// snapshot, so each test starts from the same clean, pre-synced
    /// cache instead of whatever the previous test left behind.
    func restoreSpvCacheFromSnapshot() {
        let fm = FileManager.default
        try? fm.removeItem(atPath: spvDataDir)
        try? fm.copyItem(atPath: spvCacheSnapshot, toPath: spvDataDir)
    }

    func cleanupSpvCache() {
        try? FileManager.default.removeItem(atPath: spvDataDir)
        try? FileManager.default.removeItem(atPath: spvCacheSnapshot)
    }

    private init(
        endpoints: LocalDevnet.Endpoints,
        coreRPC: CoreRPCClient,
        sdk: SDK,
        modelContainer: ModelContainer,
        walletManager: PlatformWalletManager,
        spvDataDir: String,
        mineRewardAddress: String,
        masternodePeers: [String],
        masternodeRPCs: [CoreRPCClient]
    ) {
        self.endpoints = endpoints
        self.coreRPC = coreRPC
        self.sdk = sdk
        self.modelContainer = modelContainer
        self.walletManager = walletManager
        self.spvDataDir = spvDataDir
        self.mineRewardAddress = mineRewardAddress
        self.masternodePeers = masternodePeers
        self.masternodeRPCs = masternodeRPCs
    }

    /// Build the env. Assumes `run_integration_tests.sh` has already
    /// brought dashmate online.
    static func bootstrap() async throws -> IntegrationTestEnv {
        cleanupLeftoverSpvDirs()
        sweepKeychain()

        let repoRoot = try locateRepoRoot()
        let configName = ProcessInfo.processInfo.environment["DASHMATE_CONFIG"] ?? "local_seed"
        let endpoints = try LocalDevnet(repoRoot: repoRoot, configName: configName)
            .discoverEndpoints()

        let coreRPC = CoreRPCClient(
            port: endpoints.coreRPC,
            username: endpoints.rpcUsername,
            password: endpoints.rpcPassword
        )

        // The SDK reads `platformDAPIAddresses` from UserDefaults to
        // override its built-in regtest default — matches SwiftExampleApp.
        UserDefaults.standard.set(
            "http://127.0.0.1:\(endpoints.platformDAPI)",
            forKey: "platformDAPIAddresses"
        )
        let sdk = try SDK(network: .regtest)
        let modelContainer = try DashModelContainer.createInMemory()
        let walletManager = try await MainActor.run {
            try PlatformWalletManager(sdk: sdk, modelContainer: modelContainer)
        }

        let masternodePeers = try await coreRPC.masternodeP2PEndpoints()
        let groupPrefix = configName.replacingOccurrences(of: "_seed", with: "")

        let masternodeRPCs = (try? discoverMasternodeRPCs(
            repoRoot: repoRoot, groupPrefix: groupPrefix, p2pEndpoints: masternodePeers
        )) ?? []

        let env = IntegrationTestEnv(
            endpoints: endpoints,
            coreRPC: coreRPC,
            sdk: sdk,
            modelContainer: modelContainer,
            walletManager: walletManager,
            spvDataDir: makeSPVDataDir(),
            mineRewardAddress: try await coreRPC.getNewAddress(),
            masternodePeers: masternodePeers,
            masternodeRPCs: masternodeRPCs
        )

        try await env.createSpvCache()
        env.snapshotSpvCache()

        try await env.assertCleanState()

        return env
    }

    /// Build a `CoreRPCClient` per masternode. `masternodelist` gives the
    /// P2P endpoints in port order (local_1, local_2, …); the RPC port is
    /// the P2P port + 1 in the dashmate local layout, and the password is
    /// read from each MN config. Reads are sequential (yarn's lock).
    private static func discoverMasternodeRPCs(
        repoRoot: URL, groupPrefix: String, p2pEndpoints: [String]
    ) throws -> [CoreRPCClient] {
        try p2pEndpoints.enumerated().compactMap { index, endpoint in
            guard let p2pPort = Int(endpoint.split(separator: ":").last ?? "") else { return nil }

            let password = try LocalDevnet(repoRoot: repoRoot, configName: "\(groupPrefix)_\(index + 1)")
                .rpcPassword()

            return CoreRPCClient(port: p2pPort + 1, username: "dashmate", password: password)
        }
    }

    /// Remove SPV data dirs left behind by previous runs
    private static func cleanupLeftoverSpvDirs() {
        let tmp = FileManager.default.temporaryDirectory

        guard let entries = try? FileManager.default.contentsOfDirectory(
            at: tmp, includingPropertiesForKeys: nil
        ) else { return }

        for url in entries where url.lastPathComponent.hasPrefix("swift-sdk-it-spv-") {
            try? FileManager.default.removeItem(at: url)
        }
    }

    /// Mirror the app's cold-start sequence: stop SPV, drop the
    /// current `PlatformWalletManager`, build a fresh one against the
    /// same `modelContainer`, and re-hydrate it via `loadFromPersistor`.
    /// SPV is left STOPPED so callers can drive state changes
    func restartWalletManager() async throws {
        let stopping = walletManager
        try? await MainActor.run {
            if try stopping.isSpvRunning() { try stopping.stopSpv() }
        }

        let sdk = self.sdk
        let container = self.modelContainer

        walletManager = try await MainActor.run { () -> PlatformWalletManager in
            let mgr = try PlatformWalletManager(sdk: sdk, modelContainer: container)
            _ = try mgr.loadFromPersistor()
            return mgr
        }
    }

    /// Reset the per-test mutable state
    ///  - stop SPV
    ///  - restore the SPV data dir from the pristine snapshot
    ///  - swap in a fresh empty store
    ///  - fresh wallet manager
    ///  - wipe the Keychain.
    func resetState() async throws {
        let stopping = walletManager
        try? await MainActor.run {
            if try stopping.isSpvRunning() { try stopping.stopSpv() }
        }

        restoreSpvCacheFromSnapshot()

        let sdk = self.sdk
        let fresh = try DashModelContainer.createInMemory()
        let mgr = try await MainActor.run { () -> PlatformWalletManager in
            try PlatformWalletManager(sdk: sdk, modelContainer: fresh)
        }

        self.modelContainer = fresh
        self.walletManager = mgr

        Self.sweepKeychain()
    }

    func assertCleanState(file: StaticString = #filePath, line: UInt = #line) async throws {
        let walletCount = await MainActor.run { walletManager.wallets.count }
        XCTAssertEqual(
            walletCount, 0,
            "env not clean: wallet manager still holds \(walletCount) wallet(s)",
            file: file, line: line
        )

        if ProcessInfo.processInfo.environment["DASH_KEYCHAIN_SERVICE"] != nil {
            XCTAssertFalse(
                Self.keychainHasItems(),
                "env not clean: Keychain still holds wallet items",
                file: file, line: line
            )
        }

        XCTAssertEqual(
            spvCacheSignature(spvDataDir), spvCacheSignature(spvCacheSnapshot),
            "env not clean: SPV data dir diverges from the cloned cache",
            file: file, line: line
        )

        let rows = try await MainActor.run { try totalPersistedRows() }
        XCTAssertEqual(
            rows, 0,
            "env not clean: model container still holds \(rows) row(s)",
            file: file, line: line
        )
    }

    private static func sweepKeychain() {
        guard ProcessInfo.processInfo.environment["DASH_KEYCHAIN_SERVICE"] != nil else { return }

        // `SecItemDelete` fails inside the `xctest` runner: item ACLs are
        // bound to the creating binary's code signature, which the runner
        // doesn't satisfy, so deletes are silently denied (items pile up
        // run after run). The Apple-signed `security` tool can delete
        // them. It removes one match per call — loop until none remain.
        let service = WalletStorage.keychainService
        while runSecurityDelete(service: service) {}
    }

    private static func runSecurityDelete(service: String) -> Bool {
#if os(macOS)
        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: "/usr/bin/security")
        proc.arguments = ["delete-generic-password", "-s", service]
        proc.standardOutput = FileHandle.nullDevice
        proc.standardError = FileHandle.nullDevice
        do { try proc.run() } catch { return false }
        proc.waitUntilExit()
        return proc.terminationStatus == 0
#else
        _ = service
        return false
#endif
    }

    private static func keychainHasItems() -> Bool {
        let status = SecItemCopyMatching([
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: WalletStorage.keychainService,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ] as CFDictionary, nil)
        return status == errSecSuccess
    }

    /// Total rows across every persisted model type in the container.
    private func totalPersistedRows() throws -> Int {
        let ctx = ModelContext(modelContainer)
        func count<T: PersistentModel>(_ type: T.Type) throws -> Int {
            try ctx.fetchCount(FetchDescriptor<T>())
        }
        var total = 0
        for type in DashModelContainer.modelTypes {
            total += try count(type)
        }
        return total
    }

    /// Deterministic signature of a dir's regular files (relative path +
    /// byte size) so two SPV caches can be compared for equality.
    /// Paths are relativized via `pathComponents` after resolving
    /// symlinks so the working dir and its `-snapshot` sibling produce
    /// identical signatures — a plain string-prefix strip does not, since
    /// the `/var`↔`/private` symlink and the suffix throw the offset off.
    private func spvCacheSignature(_ path: String) -> String {
        let fm = FileManager.default
        let root = URL(fileURLWithPath: path).resolvingSymlinksInPath()
        let rootCount = root.pathComponents.count

        guard let walker = fm.enumerator(
            at: root,
            includingPropertiesForKeys: [.isRegularFileKey, .fileSizeKey]
        ) else { return "" }

        var entries: [String] = []
        for case let url as URL in walker {
            let values = try? url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
            guard values?.isRegularFile == true else { continue }

            let rel = url.resolvingSymlinksInPath().pathComponents
                .dropFirst(rootCount)
                .joined(separator: "/")

            entries.append("\(rel):\(values?.fileSize ?? 0)")
        }

        return entries.sorted().joined(separator: "\n")
    }

    /// Generates a fresh mnemonic each call and persists it to Keychain.
    func makeTestWallet(name: String? = nil) async throws -> TestWalletWrapper {
        let mnemonic = try Mnemonic.generate(wordCount: 24)

        let wallet = try await MainActor.run {
            try walletManager.createWallet(
                mnemonic: mnemonic,
                network: .regtest,
                name: name,
                createDefaultAccounts: true
            )
        }

        try WalletStorage().storeMnemonic(mnemonic, for: wallet.walletId)

        return TestWalletWrapper(wallet: wallet, core: try wallet.coreWallet())
    }

    /// Mine `count` blocks. When `including` is provided, block first
    /// until dashd has the InstantSend lock for that txid
    @discardableResult
    func mine(_ count: Int, including txid: Data? = nil) async throws -> [String] {
        if let txid {
            let displayHex = Data(txid.reversed()).toHexString()
            try await waitForInstantSendLock(txid: displayHex)
        }

        let preMineHeight = try await coreRPC.getBlockCount()
        let addr = try await coreRPC.generateToAddress(count: count, address: mineRewardAddress)
        let postMineHeight = try await coreRPC.getBlockCount()

        XCTAssertEqual(
            postMineHeight, preMineHeight + count,
            "mine(\(count)) did not advance chain tip: \(preMineHeight) -> \(postMineHeight)"
        )

        return addr
    }

    /// `sendtoaddress` from the dashmate mining wallet, wait for the
    /// InstantSend lock so the next mine actually includes the tx,
    /// then mine 1 block. Returns the Core txid
    @discardableResult
    func fund(address: String, dash: Double) async throws -> String {
        let txid = try await coreRPC.sendToAddress(amount: dash, address: address)
        await broadcastToMasternodes(txid: txid)
        try await waitForInstantSendLock(txid: txid)
        _ = try await mine(1)
        return txid
    }

    /// Hand a freshly-sent tx straight to the masternodes' mempools so the
    /// InstantSend quorum signs it immediately
    private func broadcastToMasternodes(txid: String) async {
        guard !masternodeRPCs.isEmpty,
              let hex = try? await coreRPC.getRawTransaction(txid)
        else { return }
        for mn in masternodeRPCs {
            _ = try? await mn.sendRawTransaction(hex)
        }
    }

    /// Block until dashd reports `instantlock=true` for the given tx
    func waitForInstantSendLock(txid: String, timeout: TimeInterval = 30) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        var lastError: Error?

        while Date() < deadline {
            do {
                let info = try await coreRPC.getRawTransactionInfo(txid)
                if info.instantlock { return }
            } catch {
                lastError = error
            }
            try await Task.sleep(nanoseconds: 50_000_000)
        }

        throw IntegrationTestEnvError.timeout(
            "tx \(txid) did not reach instantlock=true within \(timeout)s" +
            (lastError.map { " (last RPC error: \($0))" } ?? "")
        )
    }

    private static func locateRepoRoot() throws -> URL {
        if let override = ProcessInfo.processInfo.environment["PLATFORM_REPO_ROOT"] {
            return URL(fileURLWithPath: override)
        }

        let fm = FileManager.default

        var candidate = URL(fileURLWithPath: #filePath).deletingLastPathComponent()

        for _ in 0..<8 {
            if fm.fileExists(atPath: candidate.appendingPathComponent("package.json").path),
               fm.fileExists(atPath: candidate.appendingPathComponent("packages/dashmate").path) {
                return candidate
            }
            candidate.deleteLastPathComponent()
        }

        fatalError("Could not locate platform repo root. Set PLATFORM_REPO_ROOT to override.")
    }
}
