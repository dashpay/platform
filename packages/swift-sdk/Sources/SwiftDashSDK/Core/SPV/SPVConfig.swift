import Foundation
import DashSDKFFI

public class SPVConfig: @unchecked Sendable {
    internal var config: UnsafeMutablePointer<FFIClientConfig>

    public init(_ network: Network) {
        config = {
            switch network.rawValue {
            case 0:
                return dash_spv_ffi_config_mainnet()
            case 1:
                return dash_spv_ffi_config_testnet()
            case 2:
                // Regtest (local Docker)
                return dash_spv_ffi_config_new(FFINetwork(rawValue: 2))
            case 3:
                // Map devnet to custom FFINetwork value 3
                return dash_spv_ffi_config_new(FFINetwork(rawValue: 3))
            default:
                return dash_spv_ffi_config_testnet()
            }
        }()

        // If requested, prefer local core peers (defaults to 127.0.0.1 with network default port)
        let useLocalCore = UserDefaults.standard.bool(forKey: "useLocalhostCore")
            || UserDefaults.standard.bool(forKey: "useDockerSetup")
        // Only restrict to configured peers when using local core, if not, allow DNS discovery

        self.restrictToConfiguredPeers(useLocalCore)

        if useLocalCore {
            let raw = UserDefaults.standard.string(forKey: "corePeerAddresses")?.trimmingCharacters(in: .whitespacesAndNewlines)
            let peers = (raw?.isEmpty == false ? raw! : "127.0.0.1:20001")
                .split(separator: ",")
                .map { $0.trimmingCharacters(in: .whitespaces) }
                .filter { !$0.isEmpty }

            self.clearPeers();

            for peer in peers {
                self.addPeer(peer)
            }
        }

        // Set user agent to include SwiftDashSDK version from the framework bundle
        do {
            let bundle = Bundle(for: SPVClient.self)
            let version = (bundle.infoDictionary?["CFBundleShortVersionString"] as? String)
                ?? (bundle.infoDictionary?["CFBundleVersion"] as? String)
                ?? "dev"
            let ua = "SwiftDashSDK/\(version)"

            self.setUserAgent(ua)
        }
    }

    deinit {
        dash_spv_ffi_config_destroy(config)
    }

    public func setMasternodeSyncEnabled(_ enabled: Bool) {
        dash_spv_ffi_config_set_masternode_sync_enabled(config, enabled)
    }

    public func setMempoolStrategy(_ strategy: FFIMempoolStrategy) {
        dash_spv_ffi_config_set_mempool_tracking(config, true)
        dash_spv_ffi_config_set_mempool_strategy(config, strategy)
        dash_spv_ffi_config_set_fetch_mempool_transactions(config, true)
    }

    public func setUserAgent(_ userAgent: String) {
        dash_spv_ffi_config_set_user_agent(config, userAgent)
    }

    public func setStartHeight(_ height: UInt32) {
        dash_spv_ffi_config_set_start_from_height(config, height)
    }

    public func setDataDir(_ dataDir: String) {
        dash_spv_ffi_config_set_data_dir(config, dataDir)
    }

    public func clearPeers() {
        dash_spv_ffi_config_clear_peers(config)
    }

    // Supports "ip:port" or bare IP for network-default port
    public func addPeer(_ peer: String) {
        dash_spv_ffi_config_add_peer(config, peer)
    }

    public func restrictToConfiguredPeers(_ enabled: Bool) {
        dash_spv_ffi_config_set_restrict_to_configured_peers(config, enabled)
    }
}
