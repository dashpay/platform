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
}

export class EvoSDK {
  private wasmSdk?: wasm.WasmSdk;
  private options: Required<Pick<EvoSDKOptions, 'network' | 'trusted'>> & ConnectionOptions;

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
    const { network = 'testnet', trusted = false, ...connection } = options;
    this.options = { network, trusted, ...connection };

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

    const { network, trusted, version, proofs, settings, logs } = this.options;

    let builder: wasm.WasmSdkBuilder;
    if (network === 'mainnet') {
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
export { verifyIdentityResponse, verifyDataContract, verifyDocuments, start } from './wasm.js';
export { DataContract } from './wasm.js';
