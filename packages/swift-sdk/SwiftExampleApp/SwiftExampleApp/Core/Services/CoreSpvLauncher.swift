import Foundation
import SwiftDashSDK

/// Single source of truth for starting Core SPV sync. Assembles the
/// per-network start config (data dir, peer override, devnet name) and
/// starts the SPV client, so the Sync tab's Start button
/// (`CoreContentView.startSync`) and the launch-time auto-start in
/// `SwiftExampleAppApp.bootstrap` drive identical logic.
///
/// `@MainActor`-isolated because it reads/writes the `@MainActor`
/// `PlatformWalletManager`. Both call sites already run on the main
/// actor (a SwiftUI button action and the `@MainActor` bootstrap).
@MainActor
enum CoreSpvLauncher {
    /// Build the start config for `network` and start SPV on
    /// `walletManager`. `startSpv` itself returns immediately (it spawns
    /// the sync loop on the shared tokio runtime), but resolving the
    /// peer override is done off the main actor first: on devnet
    /// `peerOverride` blocks on a `DispatchSemaphore` up to ~6 s inside
    /// `SDK.discoverActiveMasternodes`, which would freeze the UI on a
    /// cold launch.
    ///
    /// `stillCurrent` is re-checked on the main actor **after** that
    /// (multi-second) peer resolution and immediately before `startSpv`:
    /// if the user switched networks while we were suspended, the
    /// captured `walletManager` is no longer the active one (its network
    /// was stopped and `WalletManagerStore` swapped in a different
    /// per-network instance), and starting it here would revive sync on
    /// an abandoned network with no UI to stop it. On a stale start we
    /// throw `CancellationError` so callers can distinguish supersession
    /// from a real `startSpv` failure. Otherwise rethrows whatever
    /// `startSpv` throws; the caller decides whether to surface or log.
    static func start(
        network: Network,
        on walletManager: PlatformWalletManager,
        stillCurrent: @MainActor () -> Bool
    ) async throws {
        try await start(
            network: network,
            stillCurrent: stillCurrent,
            startSpv: { config in try walletManager.startSpv(config: config) }
        )
    }

    /// Testable core: assembles the config and drives the whole
    /// supersession guard, but the terminal `startSpv` step is injected
    /// rather than calling `PlatformWalletManager` directly. Lets a unit
    /// test assert the guard bails (throws `CancellationError`, injected
    /// closure never runs) on a stale start and passes the expected
    /// config through on a fresh one — no real, network-locked wallet
    /// manager required. The public overload above supplies the real
    /// `startSpv`.
    static func start(
        network: Network,
        stillCurrent: @MainActor () -> Bool,
        startSpv: @MainActor (PlatformSpvStartConfig) throws -> Void
    ) async throws {
        // Network-derived; safe to compute pre-await because we bail on a
        // `stillCurrent` mismatch below before invoking `startSpv` — and
        // creating the data dir alone starts nothing.
        let dataDirURL = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
            .first!
            .appendingPathComponent("SPV")
            .appendingPathComponent(network.networkName)
        try? FileManager.default.createDirectory(at: dataDirURL, withIntermediateDirectories: true)

        // Resolve peers off the main actor — the devnet branch can block
        // for seconds. `network` is `Sendable` and the result is
        // `[String]`; nothing main-actor-bound is captured, so the
        // detached hop is data-race-free.
        let peers = await Task.detached { Self.peerOverride(network: network) }.value

        // Back on the main actor: bail if a network switch / SDK rebuild
        // superseded this start during the peer lookup. Call sites pair a
        // generation token with manager identity (`store.spvStartGeneration
        // == captured && store.activeManager === manager`) so the
        // stop→activate gap is covered, not just the post-swap state.
        guard stillCurrent() else { throw CancellationError() }

        let restrictToConfiguredPeers = !peers.isEmpty

        // Devnet requires a name so `DevnetConfig` can embed
        // `devnet.devnet-<name>` in the SPV user agent (Dash
        // Core devnet peers drop inbound handshakes without it).
        // Read from the same UserDefaults key OptionsView writes.
        let devnetName: String? = network == .devnet
            ? UserDefaults.standard.string(forKey: "platformDevnetName").flatMap {
                let trimmed = $0.trimmingCharacters(in: .whitespaces)
                return trimmed.isEmpty ? nil : trimmed
            }
            : nil

        let config = PlatformSpvStartConfig(
            dataDir: dataDirURL.path,
            network: network,
            peers: peers,
            restrictToConfiguredPeers: restrictToConfiguredPeers,
            devnetName: devnetName
        )
        try startSpv(config)
    }

    /// Resolve the SPV peer override for the given network / docker
    /// combo.
    ///
    /// Three modes coexist on top of the same `useLocalhostCore` /
    /// `localCorePeers` `UserDefaults` keys, which used to bleed into
    /// each other when the user reconfigured between sessions:
    ///
    ///   1. **regtest + docker** — connect to dashmate's `local_seed`
    ///      Core P2P port. The default 3-node setup maps the seed to
    ///      `127.0.0.1:20301` (`getLocalConfigFactory.js` base 20001
    ///      + `setupLocalPresetTaskFactory.js` `+ i*100` with seed
    ///      at index = `nodeCount`, typically 3). Anything sitting
    ///      in `localCorePeers` from a previous testnet / mainnet
    ///      "custom peers" session is ignored — the UI doesn't show
    ///      that knob on regtest+docker so a stale value is always
    ///      bleed-through, never user intent.
    ///   2. **non-regtest + custom peers** — honor `localCorePeers`
    ///      verbatim. The OptionsView "Use Custom SPV Peers" toggle
    ///      seeds and edits this string.
    ///   3. **everything else** — empty list, FFI uses the network's
    ///      built-in seed nodes.
    ///
    /// `nonisolated` so `start` can resolve it on a detached task: it
    /// touches only `UserDefaults` and the (nonisolated) blocking
    /// `SDK.discoverActiveMasternodes` static, both safe off the main
    /// actor.
    nonisolated private static func peerOverride(network: Network) -> [String] {
        let useDocker = UserDefaults.standard.bool(forKey: "useDockerSetup")
        if network == .regtest && useDocker {
            return ["127.0.0.1:20301"]
        }
        // Devnet: auto-discover SPV peers from the quorum-list
        // service's `/masternodes` endpoint. Each masternode reports
        // its own `address` field (`ip:CoreP2PPort`) — use the
        // verbatim values rather than guessing the canonical 29999
        // port (paloma reports 20001 per masternode, for example).
        // No manual SPV input on devnet — the quorum URL is the
        // single source of truth (see `OptionsView`'s devnet branch).
        if network == .devnet {
            guard
                let quorum = UserDefaults.standard.string(forKey: "platformQuorumURL"),
                !quorum.isEmpty,
                let active = SDK.discoverActiveMasternodes(quorumBase: quorum)
            else { return [] }
            return active.map(\.spvPeer)
        }
        let useLocalCore = UserDefaults.standard.bool(forKey: "useLocalhostCore")
        guard useLocalCore else { return [] }
        let raw = UserDefaults.standard.string(forKey: "localCorePeers") ?? ""
        return raw
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
    }
}

/// Pure decision for the launch-time Core SPV auto-start gate, split out
/// from `SwiftExampleAppApp.autoStartCoreSpvIfNeeded` so the gate matrix
/// is unit-testable without a configured wallet manager. Encodes two
/// non-obvious rules the caller relies on:
///
///   * a **no-wallets** launch does NOT latch (leaving the door open for
///     a future call once wallets exist), whereas
///   * an **already-running** client DOES latch (the auto-start is
///     considered "handled" — we don't want to keep re-checking).
enum CoreSpvAutoStart {
    enum Decision: Equatable {
        /// Auto-start already ran this launch — do nothing.
        case skipAlreadyLatched
        /// No wallets on the active network — do nothing, don't latch.
        case skipNoWallets
        /// A client is already running — latch, but don't restart it.
        case latchOnly
        /// Start SPV and latch.
        case startAndLatch

        /// Whether the caller should set its once-per-launch latch.
        var shouldLatch: Bool {
            switch self {
            case .latchOnly, .startAndLatch: return true
            case .skipAlreadyLatched, .skipNoWallets: return false
            }
        }

        /// Whether the caller should actually start the SPV client.
        var shouldStart: Bool { self == .startAndLatch }
    }

    /// Resolve the gate. Order matters: the latch short-circuits first,
    /// then the wallet gate (which must not latch), then the
    /// already-running gate (which latches without starting).
    static func decision(
        alreadyLatched: Bool,
        hasWallets: Bool,
        spvRunning: Bool
    ) -> Decision {
        if alreadyLatched { return .skipAlreadyLatched }
        if !hasWallets { return .skipNoWallets }
        if spvRunning { return .latchOnly }
        return .startAndLatch
    }
}
