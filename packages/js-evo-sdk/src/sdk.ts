import * as wasm from './wasm.js';
import { ensureInitialized as initWasm } from './wasm.js';
import { AddressesFacade } from './addresses/facade.js';
import { DocumentsFacade } from './documents/facade.js';
import { IdentitiesFacade } from './identities/facade.js';
import { ContractsFacade } from './contracts/facade.js';
import { TokensFacade } from './tokens/facade.js';
import { DpnsFacade } from './dpns/facade.js';
import { EpochFacade } from './epoch/facade.js';
import { ProtocolFacade } from './protocol/facade.js';
import { StateTransitionsFacade } from './state-transitions/facade.js';
import { SystemFacade } from './system/facade.js';
import { GroupFacade } from './group/facade.js';
import { VotingFacade } from './voting/facade.js';
import { ShieldedFacade } from './shielded/facade.js';

export interface ConnectionOptions {
  version?: number;
  proofs?: boolean;
  // Configure tracing/logging emitted from the underlying Wasm SDK.
  // Accepts simple levels: 'off' | 'error' | 'warn' | 'info' | 'debug' | 'trace'
  // or a full EnvFilter string like: 'wasm_sdk=debug,rs_dapi_client=warn'
  logs?: string;
  settings?: {
    connectTimeoutMs?: number;
    timeoutMs?: number;
    retries?: number;
    banFailedAddress?: boolean;
  };
}

export interface EvoSDKOptions extends ConnectionOptions {
  network?: 'testnet' | 'mainnet' | 'local' | 'devnet';
  trusted?: boolean;
  // Custom masternode addresses to seed the SDK with. `network` still
  // controls which Network enum the underlying builder uses (and, for
  // trusted mode, which quorums endpoint is prefetched); the addresses
  // here replace the network's built-in defaults at seed time.
  // Example: ['https://127.0.0.1:1443', 'https://192.168.1.100:1443']
  addresses?: string[];
  // Short name of the devnet (e.g. 'paloma'). Required when network === 'devnet'
  // AND trusted === true (used to derive the quorum URL). When trusted === false,
  // explicit `addresses` are mandatory and `devnetName` alone is not sufficient
  // — no masternode addresses can be discovered without a trusted context.
  devnetName?: string;
  // Optional override for the trusted-context quorum base URL. When omitted,
  // the URL is the network's default (e.g.
  // `https://quorums.<devnetName>.networks.dash.org` for devnet,
  // `https://quorums.testnet.networks.dash.org` for testnet, etc.).
  // Only consulted when trusted === true. Useful for pointing at a staging,
  // self-hosted, or not-yet-deployed quorums endpoint.
  quorumUrl?: string;
}

export class EvoSDK {
  private wasmSdk?: wasm.WasmSdk;
  private options: Required<Pick<EvoSDKOptions, 'network' | 'trusted'>> & ConnectionOptions & { addresses?: string[]; devnetName?: string; quorumUrl?: string };

  public addresses!: AddressesFacade;
  public documents!: DocumentsFacade;
  public identities!: IdentitiesFacade;
  public contracts!: ContractsFacade;
  public tokens!: TokensFacade;
  public dpns!: DpnsFacade;
  public epoch!: EpochFacade;
  public protocol!: ProtocolFacade;
  public stateTransitions!: StateTransitionsFacade;
  public system!: SystemFacade;
  public group!: GroupFacade;
  public voting!: VotingFacade;
  public shielded!: ShieldedFacade;
  constructor(options: EvoSDKOptions = {}) {
    // Apply defaults while preserving any future connection options
    const { network = 'testnet', trusted = false, addresses, devnetName, quorumUrl, ...connection } = options;

    if (network === 'devnet') {
      const hasAddresses = !!(addresses && addresses.length > 0);
      if (trusted) {
        if (!devnetName && !quorumUrl) {
          throw new Error("EvoSDK: trusted devnet requires devnetName (to derive the quorum URL) or an explicit quorumUrl");
        }
      } else if (!hasAddresses) {
        throw new Error("EvoSDK: non-trusted devnet requires explicit addresses (no addresses can be discovered without a trusted context)");
      }
    } else if (devnetName) {
      // Surface a likely typo (e.g. network: 'testent' + devnetName: 'paloma')
      // — devnetName has no effect outside network === 'devnet'.
      throw new Error("EvoSDK: devnetName is only valid when network === 'devnet'");
    }
    if (quorumUrl && !trusted) {
      throw new Error("EvoSDK: quorumUrl is only meaningful when trusted === true");
    }

    this.options = { network, trusted, addresses, devnetName, quorumUrl, ...connection };

    this.addresses = new AddressesFacade(this);
    this.documents = new DocumentsFacade(this);
    this.identities = new IdentitiesFacade(this);
    this.contracts = new ContractsFacade(this);
    this.tokens = new TokensFacade(this);
    this.dpns = new DpnsFacade(this);
    this.epoch = new EpochFacade(this);
    this.protocol = new ProtocolFacade(this);
    this.stateTransitions = new StateTransitionsFacade(this);
    this.system = new SystemFacade(this);
    this.group = new GroupFacade(this);
    this.voting = new VotingFacade(this);
    this.shielded = new ShieldedFacade(this);
  }

  get wasm(): wasm.WasmSdk {
    if (!this.wasmSdk) {
      throw new Error('SDK is not connected. Call EvoSDK#connect() first.');
    }
    return this.wasmSdk;
  }

  get isConnected(): boolean { return !!this.wasmSdk; }

  async getWasmSdkConnected(): Promise<wasm.WasmSdk> {
    if (!this.wasmSdk) {
      await this.connect();
    }
    return this.wasmSdk as wasm.WasmSdk;
  }

  async connect(): Promise<void> {
    if (this.wasmSdk) {
      return; // idempotent
    }
    await initWasm();

    const { network, trusted, version, proofs, settings, logs, addresses, devnetName, quorumUrl } = this.options;

    // Prefetch trusted context only when trusted mode is requested
    let context: wasm.WasmTrustedContext | undefined;
    if (trusted) {
      if (network === 'mainnet') {
        context = quorumUrl
          ? await wasm.WasmTrustedContext.prefetchMainnetWithUrl(quorumUrl)
          : await wasm.WasmTrustedContext.prefetchMainnet();
      } else if (network === 'testnet') {
        context = quorumUrl
          ? await wasm.WasmTrustedContext.prefetchTestnetWithUrl(quorumUrl)
          : await wasm.WasmTrustedContext.prefetchTestnet();
      } else if (network === 'local') {
        context = quorumUrl
          ? await wasm.WasmTrustedContext.prefetchLocalWithUrl(quorumUrl)
          : await wasm.WasmTrustedContext.prefetchLocal();
      } else if (network === 'devnet') {
        if (quorumUrl) {
          context = await wasm.WasmTrustedContext.prefetchDevnetWithUrl(quorumUrl);
        } else if (devnetName) {
          context = await wasm.WasmTrustedContext.prefetchDevnet(devnetName);
        } else {
          throw new Error("EvoSDK: trusted devnet requires devnetName or quorumUrl");
        }
      } else {
        throw new Error(`Unknown network: ${network}`);
      }
    }

    let builder: wasm.WasmSdkBuilder;

    if (addresses && addresses.length > 0) {
      builder = wasm.WasmSdkBuilder.withAddresses(addresses, network);
    } else if (network === 'mainnet') {
      builder = wasm.WasmSdkBuilder.mainnet();
    } else if (network === 'testnet') {
      builder = wasm.WasmSdkBuilder.testnet();
    } else if (network === 'local') {
      builder = wasm.WasmSdkBuilder.local();
    } else if (network === 'devnet') {
      builder = wasm.WasmSdkBuilder.newDevnet();
    } else {
      throw new Error(`Unknown network: ${network}`);
    }

    // Attach trusted context for proof verification and discovered addresses
    if (context) {
      builder = builder.withTrustedContext(context);
    }

    if (version) {
      builder = builder.withVersion(version);
    }
    if (typeof proofs === 'boolean') {
      builder = builder.withProofs(proofs);
    }
    if (logs) {
      builder = builder.withLogs(logs);
    }
    if (settings) {
      const { connectTimeoutMs, timeoutMs, retries, banFailedAddress } = settings;
      builder = builder.withSettings(
        connectTimeoutMs ?? null,
        timeoutMs ?? null,
        retries ?? null,
        banFailedAddress ?? null,
      );
    }

    this.wasmSdk = builder.build();
  }

  static fromWasm(wasmSdk: wasm.WasmSdk): EvoSDK {
    const sdk = new EvoSDK();
    (sdk as any).wasmSdk = wasmSdk;
    return sdk;
  }

  version(): number {
    return this.wasm.version();
  }

  static async setLogLevel(levelOrFilter: string): Promise<void> {
    await initWasm();
    wasm.WasmSdk.setLogLevel(levelOrFilter);
  }

  static async getLatestVersionNumber(): Promise<number> {
    await initWasm();
    return wasm.WasmSdkBuilder.getLatestVersionNumber();
  }

  // Factory helpers that return configured instances (not connected)
  static testnet(options: ConnectionOptions = {}): EvoSDK { return new EvoSDK({ network: 'testnet', ...options }); }
  static mainnet(options: ConnectionOptions = {}): EvoSDK { return new EvoSDK({ network: 'mainnet', ...options }); }
  static testnetTrusted(options: ConnectionOptions = {}): EvoSDK { return new EvoSDK({ network: 'testnet', trusted: true, ...options }); }
  static mainnetTrusted(options: ConnectionOptions = {}): EvoSDK { return new EvoSDK({ network: 'mainnet', trusted: true, ...options }); }
  static local(options: ConnectionOptions = {}): EvoSDK { return new EvoSDK({ network: 'local', ...options }); }
  static localTrusted(options: ConnectionOptions = {}): EvoSDK { return new EvoSDK({ network: 'local', trusted: true, ...options }); }

  /**
   * Create an EvoSDK instance configured for a devnet, without trusted-context
   * proof verification. Requires explicit `addresses` in `options` —
   * `devnetName` alone is not sufficient in non-trusted mode, since no
   * masternode addresses can be discovered without a trusted context.
   * Proof-bearing queries will fail; for proof verification on devnet, use
   * `EvoSDK.devnetTrusted` instead.
   */
  static devnet(devnetName: string, options: ConnectionOptions & { addresses?: string[] } = {}): EvoSDK {
    return new EvoSDK({ network: 'devnet', devnetName, ...options });
  }

  /**
   * Create an EvoSDK instance configured for a devnet with a trusted context.
   *
   * The trusted context is prefetched from
   * `https://quorums.<devnetName>.networks.dash.org` by default. Pass
   * `quorumUrl` to override (useful when the public DNS is not yet deployed).
   *
   * @example
   * ```typescript
   * const sdk = EvoSDK.devnetTrusted('paloma');
   * await sdk.connect();
   * ```
   */
  static devnetTrusted(
    devnetName: string,
    options: ConnectionOptions & { quorumUrl?: string } = {},
  ): EvoSDK {
    return new EvoSDK({ network: 'devnet', devnetName, trusted: true, ...options });
  }

  /**
   * Create an EvoSDK instance configured with specific masternode addresses.
   *
   * @param addresses - Array of HTTPS URLs to masternodes (e.g., ['https://127.0.0.1:1443'])
   * @param network - Network identifier: 'mainnet', 'testnet', 'devnet', or 'local' (default: 'testnet')
   * @param options - Additional connection options
   * @returns A configured EvoSDK instance (not yet connected - call .connect() to establish connection)
   *
   * @example
   * ```typescript
   * const sdk = EvoSDK.withAddresses(['https://52.12.176.90:1443'], 'testnet');
   * await sdk.connect();
   * ```
   */
  static withAddresses(addresses: string[], network: 'mainnet' | 'testnet' | 'local' | 'devnet' = 'testnet', options: ConnectionOptions & { devnetName?: string } = {}): EvoSDK {
    return new EvoSDK({ addresses, network, ...options });
  }
}

export { AddressesFacade } from './addresses/facade.js';
export { DocumentsFacade } from './documents/facade.js';
export { IdentitiesFacade } from './identities/facade.js';
export { ContractsFacade } from './contracts/facade.js';
export { TokensFacade } from './tokens/facade.js';
export { DpnsFacade } from './dpns/facade.js';
export { EpochFacade } from './epoch/facade.js';
export { ProtocolFacade } from './protocol/facade.js';
export { StateTransitionsFacade } from './state-transitions/facade.js';
export { SystemFacade } from './system/facade.js';
export { GroupFacade } from './group/facade.js';
export { VotingFacade } from './voting/facade.js';
export { ShieldedFacade } from './shielded/facade.js';
export { wallet } from './wallet/functions.js';
export * from './wasm.js';
