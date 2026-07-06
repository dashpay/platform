import Foundation

/// Reads port + credential info from a running dashmate devnet.
/// Lifecycle (starting/stopping dashmate) is the script's job,
/// `run_integration_tests.sh` brings it up before invoking the test
/// binary. This type just reads config out of the running stack.
struct LocalDevnet {
    let repoRoot: URL
    let configName: String

    struct Endpoints {
        let coreRPC: Int
        let coreP2P: Int
        let platformDAPI: Int
        let rpcUsername: String
        let rpcPassword: String
    }

    init(repoRoot: URL, configName: String) {
        self.repoRoot = repoRoot
        self.configName = configName
    }

    // MARK: - Endpoint discovery

    /// Reads ports + RPC creds via `yarn dashmate config get`, which
    /// returns the merged base/local/preset chain — pulling from
    /// `~/.dashmate/config.json` directly would miss preset overrides.
    func discoverEndpoints() throws -> Endpoints {
        let coreRPC = try readConfigInt("core.rpc.port")
        let coreP2P = try readConfigInt("core.p2p.port")
        let platformDAPI = try readConfigInt("platform.gateway.listeners.dapiAndDrive.port")
        let rpcUser = try readConfigString("core.rpc.users.dashmate.username", fallback: "dashmate")
        let rpcPass = try readConfigString("core.rpc.users.dashmate.password", fallback: "rpcpassword")
        return Endpoints(
            coreRPC: coreRPC,
            coreP2P: coreP2P,
            platformDAPI: platformDAPI,
            rpcUsername: rpcUser,
            rpcPassword: rpcPass
        )
    }

    func rpcPassword() throws -> String {
        try readConfigString("core.rpc.users.dashmate.password", fallback: "rpcpassword")
    }

    private func readConfigInt(_ path: String) throws -> Int {
        let raw = try readConfigRaw(path)
        guard let value = Int(raw) else {
            throw DevnetError.malformedConfig("Expected int at `\(path)`, got: \(raw)")
        }
        return value
    }

    private func readConfigString(_ path: String, fallback: String) throws -> String {
        do {
            return try readConfigRaw(path)
        } catch {
            return fallback
        }
    }

    private func readConfigRaw(_ path: String) throws -> String {
        let result = try Shell.runChecked(
            "/usr/bin/env",
            ["yarn", "dashmate", "config", "get", path, "--config=\(configName)"],
            cwd: repoRoot,
            timeout: 30
        )
        // yarn prepends its own banner lines; `dashmate config get`
        // emits the value as the last non-empty line.
        let lines = result.stdout
            .split(separator: "\n", omittingEmptySubsequences: true)
            .map { String($0).trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
        guard let value = lines.last else {
            throw DevnetError.malformedConfig("Empty `dashmate config get \(path)` output")
        }
        return value
    }

    enum DevnetError: Error, CustomStringConvertible {
        case malformedConfig(String)

        var description: String {
            switch self {
            case let .malformedConfig(msg): return "Devnet config malformed: \(msg)"
            }
        }
    }
}
