import * as wasm from '../../pkg/wasm_sdk.js';
import { DocumentsFacade } from './documents';
import { IdentitiesFacade } from './identities';
import { ContractsFacade } from './contracts';
import { TokensFacade } from './tokens';
import { DpnsFacade } from './dpns';
import { EpochFacade } from './epoch';
import { ProtocolFacade } from './protocol';
import { SystemFacade } from './system';
import { GroupFacade } from './group';
import { VotingFacade } from './voting';

export interface ConnectOptions {
  version?: number;
  proofs?: boolean; // currently ignored (not exposed in JS builder bindings)
  settings?: {
    connectTimeoutMs?: number;
    timeoutMs?: number;
    retries?: number;
    banFailedAddress?: boolean;
  };
}

export interface EvoSDKOptions extends ConnectOptions {
  network?: 'testnet' | 'mainnet';
  trusted?: boolean;
}

export class EvoSDK {
  private _sdk: wasm.WasmSdk;

  public readonly documents: DocumentsFacade;
  public readonly identities: IdentitiesFacade;
  public readonly contracts: ContractsFacade;
  public readonly tokens: TokensFacade;
  public readonly dpns: DpnsFacade;
  public readonly epoch: EpochFacade;
  public readonly protocol: ProtocolFacade;
  public readonly system: SystemFacade;
  public readonly group: GroupFacade;
  public readonly voting: VotingFacade;

  constructor(optionsOrSdk?: EvoSDKOptions | wasm.WasmSdk) {
    if (optionsOrSdk && typeof optionsOrSdk === 'object' && optionsOrSdk instanceof (wasm as any).WasmSdk) {
      this._sdk = optionsOrSdk as wasm.WasmSdk;
    } else {
      const opts = (optionsOrSdk || {}) as EvoSDKOptions;
      const network = opts.network || 'testnet';
      const trusted = !!opts.trusted;

      let builder: wasm.WasmSdkBuilder;
      if (network === 'mainnet') {
        builder = trusted && 'new_mainnet_trusted' in (wasm.WasmSdkBuilder as any)
          ? (wasm.WasmSdkBuilder as any).new_mainnet_trusted()
          : wasm.WasmSdkBuilder.new_mainnet();
      } else {
        builder = trusted && 'new_testnet_trusted' in (wasm.WasmSdkBuilder as any)
          ? (wasm.WasmSdkBuilder as any).new_testnet_trusted()
          : wasm.WasmSdkBuilder.new_testnet();
      }

      if (opts.version) builder = builder.with_version(opts.version);
      if (opts.settings) {
        const { connectTimeoutMs, timeoutMs, retries, banFailedAddress } = opts.settings;
        builder = builder.with_settings(connectTimeoutMs ?? null, timeoutMs ?? null, retries ?? null, banFailedAddress ?? null);
      }
      // proofs is not exposed in current bindings; ignored here
      this._sdk = builder.build();
    }

    this.documents = new DocumentsFacade(this._sdk);
    this.identities = new IdentitiesFacade(this._sdk);
    this.contracts = new ContractsFacade(this._sdk);
    this.tokens = new TokensFacade(this._sdk);
    this.dpns = new DpnsFacade(this._sdk);
    this.epoch = new EpochFacade(this._sdk);
    this.protocol = new ProtocolFacade(this._sdk);
    this.system = new SystemFacade(this._sdk);
    this.group = new GroupFacade(this._sdk);
    this.voting = new VotingFacade(this._sdk);
  }

  get raw(): wasm.WasmSdk { return this._sdk; }

  static builder = {
    mainnet(): wasm.WasmSdkBuilder { return wasm.WasmSdkBuilder.new_mainnet(); },
    testnet(): wasm.WasmSdkBuilder { return wasm.WasmSdkBuilder.new_testnet(); },
    mainnetTrusted(): wasm.WasmSdkBuilder {
      if (!('new_mainnet_trusted' in (wasm.WasmSdkBuilder as any))) throw new Error('Trusted mainnet builder not available');
      return (wasm.WasmSdkBuilder as any).new_mainnet_trusted();
    },
    testnetTrusted(): wasm.WasmSdkBuilder {
      if (!('new_testnet_trusted' in (wasm.WasmSdkBuilder as any))) throw new Error('Trusted testnet builder not available');
      return (wasm.WasmSdkBuilder as any).new_testnet_trusted();
    },
  };

  static async connectTestnet(options: ConnectOptions = {}): Promise<EvoSDK> {
    let b = wasm.WasmSdkBuilder.new_testnet();
    if (options.version) b = b.with_version(options.version);
    if (options.settings) {
      const { connectTimeoutMs, timeoutMs, retries, banFailedAddress } = options.settings;
      b = b.with_settings(connectTimeoutMs ?? null, timeoutMs ?? null, retries ?? null, banFailedAddress ?? null);
    }
    return new EvoSDK(b.build());
  }

  static async connectMainnet(options: ConnectOptions = {}): Promise<EvoSDK> {
    let b = wasm.WasmSdkBuilder.new_mainnet();
    if (options.version) b = b.with_version(options.version);
    if (options.settings) {
      const { connectTimeoutMs, timeoutMs, retries, banFailedAddress } = options.settings;
      b = b.with_settings(connectTimeoutMs ?? null, timeoutMs ?? null, retries ?? null, banFailedAddress ?? null);
    }
    return new EvoSDK(b.build());
  }
}

export { DocumentsFacade } from './documents';
export { IdentitiesFacade } from './identities';
export { ContractsFacade } from './contracts';
export { TokensFacade } from './tokens';
export { DpnsFacade } from './dpns';
export { EpochFacade } from './epoch';
export { ProtocolFacade } from './protocol';
export { SystemFacade } from './system';
export { GroupFacade } from './group';
export { VotingFacade } from './voting';

