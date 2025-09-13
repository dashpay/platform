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
    const { network = 'testnet', trusted = false, version, proofs, settings } = options;
    this.options = { network, trusted, version, proofs, settings } as any;
  }

  get wasm(): wasm.WasmSdk {
    if (!this.wasmSdk) throw new Error('EvoSDK is not connected. Call await sdk.connect() first.');
    return this.wasmSdk;
  }

  get isConnected(): boolean { return !!this.wasmSdk; }

  async connect(): Promise<void> {
    if (this.wasmSdk) return; // idempotent
    await initWasm();

    const { network, trusted, version, proofs, settings } = this.options;

    let b: wasm.WasmSdkBuilder;
    if (network === 'mainnet') {
      b = trusted ? (wasm.WasmSdkBuilder as any).new_mainnet_trusted() : wasm.WasmSdkBuilder.new_mainnet();
    } else {
      b = trusted ? (wasm.WasmSdkBuilder as any).new_testnet_trusted() : wasm.WasmSdkBuilder.new_testnet();
    }

    if (version) b = b.with_version(version);
    if (typeof proofs === 'boolean') b = b.with_proofs(proofs);
    if (settings) {
      const { connectTimeoutMs, timeoutMs, retries, banFailedAddress } = settings;
      b = b.with_settings(connectTimeoutMs ?? null, timeoutMs ?? null, retries ?? null, banFailedAddress ?? null);
    }

    this.wasmSdk = b.build();

    // Initialize facades
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

  static fromWasm(raw: wasm.WasmSdk): EvoSDK {
    const sdk = new EvoSDK();
    (sdk as any).wasmSdk = raw;
    sdk.documents = new DocumentsFacade(sdk);
    sdk.identities = new IdentitiesFacade(sdk);
    sdk.contracts = new ContractsFacade(sdk);
    sdk.tokens = new TokensFacade(sdk);
    sdk.dpns = new DpnsFacade(sdk);
    sdk.epoch = new EpochFacade(sdk);
    sdk.protocol = new ProtocolFacade(sdk);
    sdk.system = new SystemFacade(sdk);
    sdk.group = new GroupFacade(sdk);
    sdk.voting = new VotingFacade(sdk);
    return sdk;
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
// For error types, import directly from '@dashevo/wasm-sdk/errors'
