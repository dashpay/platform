import * as wasm from './wasm.js';
import { ensureInitialized as initWasm } from './wasm.js';
import { DocumentsFacade } from './documents/facade.js';
import { IdentitiesFacade } from './identities/facade.js';
import { ContractsFacade } from './contracts/facade.js';
import { TokensFacade } from './tokens/facade.js';
import { DpnsFacade } from './dpns/facade.js';
import { EpochFacade } from './epoch/facade.js';
import { ProtocolFacade } from './protocol/facade.js';
import { SystemFacade } from './system/facade.js';
import { GroupFacade } from './group/facade.js';
import { VotingFacade } from './voting/facade.js';

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
  network?: 'testnet' | 'mainnet';
  trusted?: boolean;
  // Custom masternode addresses. When provided, network and trusted options are ignored.
  // Example: ['https://127.0.0.1:1443', 'https://192.168.1.100:1443']
  addresses?: string[];
}

export class EvoSDK {
  private wasmSdk?: wasm.WasmSdk;
  private options: Required<Pick<EvoSDKOptions, 'network' | 'trusted'>> & ConnectionOptions & { addresses?: string[] };

  public documents!: DocumentsFacade;
  public identities!: IdentitiesFacade;
  public contracts!: ContractsFacade;
  public tokens!: TokensFacade;
  public dpns!: DpnsFacade;
  public epoch!: EpochFacade;
  public protocol!: ProtocolFacade;
  public system!: SystemFacade;
  public group!: GroupFacade;
  public voting!: VotingFacade;
  constructor(options: EvoSDKOptions = {}) {
    // Apply defaults while preserving any future connection options
    const { network = 'testnet', trusted = false, addresses, ...connection } = options;
    this.options = { network, trusted, addresses, ...connection };

    this.documents = new DocumentsFacade(this);
    this.identities = new IdentitiesFacade(this);
    this.contracts = new ContractsFacade(this);
    this.tokens = new TokensFacade(this);
    this.dpns = new DpnsFacade(this);
    this.epoch = new EpochFacade(this);
    this.protocol = new ProtocolFacade(this);
    this.system = new SystemFacade(this);
    this.group = new GroupFacade(this);
    this.voting = new VotingFacade(this);
  }

  get wasm(): wasm.WasmSdk {
    if (!this.wasmSdk) throw new Error('SDK is not connected. Call EvoSDK#connect() first.');
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
    if (this.wasmSdk) return; // idempotent
    await initWasm();

    const { network, trusted, version, proofs, settings, logs, addresses } = this.options;

    let builder: wasm.WasmSdkBuilder;

    // If custom addresses are provided, use them instead of network presets
    if (addresses && addresses.length > 0) {
      // Prefetch trusted quorums for the network before creating custom builder
      if (network === 'mainnet') {
        await wasm.WasmSdk.prefetchTrustedQuorumsMainnet();
      } else if (network === 'testnet') {
        await wasm.WasmSdk.prefetchTrustedQuorumsTestnet();
      }
      builder = wasm.WasmSdkBuilder.custom(addresses, network);
    } else if (network === 'mainnet') {
      await wasm.WasmSdk.prefetchTrustedQuorumsMainnet();

      builder = trusted ? wasm.WasmSdkBuilder.mainnetTrusted() : wasm.WasmSdkBuilder.mainnet();
    } else if (network === 'testnet') {
      await wasm.WasmSdk.prefetchTrustedQuorumsTestnet();

      builder = trusted ? wasm.WasmSdkBuilder.testnetTrusted() : wasm.WasmSdkBuilder.testnet();
    } else {
      throw new Error(`Unknown network: ${network}`);
    }

    if (version) builder = builder.withVersion(version);
    if (typeof proofs === 'boolean') builder = builder.withProofs(proofs);
    if (logs) builder = builder.withLogs(logs);
    if (settings) {
      const { connectTimeoutMs, timeoutMs, retries, banFailedAddress } = settings;
      builder = builder.withSettings(connectTimeoutMs ?? null, timeoutMs ?? null, retries ?? null, banFailedAddress ?? null);
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

  /**
   * Create an EvoSDK instance configured with custom masternode addresses.
   *
   * @param addresses - Array of HTTPS URLs to masternodes (e.g., ['https://127.0.0.1:1443'])
   * @param network - Network identifier: 'mainnet', 'testnet' (default: 'testnet')
   * @param options - Additional connection options
   * @returns A configured EvoSDK instance (not yet connected - call .connect() to establish connection)
   *
   * @example
   * ```typescript
   * const sdk = EvoSDK.custom(['https://52.12.176.90:1443'], 'testnet');
   * await sdk.connect();
   * ```
   */
  static custom(addresses: string[], network: 'mainnet' | 'testnet' = 'testnet', options: ConnectionOptions = {}): EvoSDK {
    return new EvoSDK({ addresses, network, ...options });
  }
}

export { DocumentsFacade } from './documents/facade.js';
export { IdentitiesFacade } from './identities/facade.js';
export { ContractsFacade } from './contracts/facade.js';
export { TokensFacade } from './tokens/facade.js';
export { DpnsFacade } from './dpns/facade.js';
export { EpochFacade } from './epoch/facade.js';
export { ProtocolFacade } from './protocol/facade.js';
export { SystemFacade } from './system/facade.js';
export { GroupFacade } from './group/facade.js';
export { VotingFacade } from './voting/facade.js';
export { wallet } from './wallet/functions.js';
export * from './wasm.js';
