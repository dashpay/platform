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
import { ContractGroupsFacade } from './contract-groups/facade.js';
import { VotingFacade } from './voting/facade.js';
import { ShieldedFacade } from './shielded/facade.js';
import { EncryptedForFacade } from './encrypted-for/facade.js';
import { ModerationChartersFacade } from './moderation-charters/facade.js';
import { DecisionProposalsFacade } from './decision-proposals/facade.js';

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
  // here replace the network's built-in defaults at seed time. In trusted
  // mode they also take precedence over the addresses the quorum service
  // advertises, so `connect()` skips that discovery request when they are set.
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
  // Prove every Platform quorum key with a DIP 175 proof from a checkpoint
  // instead of taking it from the quorum service on trust; the service then
  // only relays proofs. Mainnet and testnet only, and exclusive with
  // `trusted`. Nothing is fetched at connect time: each key is proven the
  // first time a response signed by its quorum is verified.
  verified?: boolean;
  // Where proofs start from: a ChainLocked block you checked independently,
  // in Dash Core's `getquorumproofchain` `target` form. Defaults to the
  // checkpoint built into the SDK. To resume where a previous session got to,
  // pass `continuation` instead. Only valid when verified === true.
  checkpoint?: wasm.QuorumProofState;
  // Where a previous session got to, as `verifiedContinuation()` returned it,
  // so proofs stay short. Like a checkpoint it is trust configuration: only
  // pass back what an SDK you trust saved. Exclusive with `checkpoint`; only
  // valid when verified === true.
  continuation?: wasm.QuorumProofContinuation;
  // 1 to 8 relay base URLs whose `/proofs` endpoint serves the proofs; each
  // must be https (plain http only on a loopback host). Defaults to the
  // network's public quorum service. Only valid when verified === true.
  quorumRelays?: string[];
}

type VerifiedOptions = Pick<EvoSDKOptions, 'checkpoint' | 'continuation' | 'quorumRelays'>;

export class EvoSDK {
  private wasmSdk?: wasm.WasmSdk;
  private verifiedContext?: wasm.WasmVerifiedContext;
  private options: Required<Pick<EvoSDKOptions, 'network' | 'trusted' | 'verified'>> & ConnectionOptions & VerifiedOptions & { addresses?: string[]; devnetName?: string; quorumUrl?: string };

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
  public contractGroups!: ContractGroupsFacade;
  public voting!: VotingFacade;
  public shielded!: ShieldedFacade;
  public encryptedFor!: EncryptedForFacade;
  public moderationCharters!: ModerationChartersFacade;
  public decisionProposals!: DecisionProposalsFacade;
  constructor(options: EvoSDKOptions = {}) {
    // Apply defaults while preserving any future connection options
    const {
      network = 'testnet',
      trusted = false,
      verified = false,
      checkpoint,
      continuation,
      quorumRelays,
      addresses,
      devnetName,
      quorumUrl,
      ...connection
    } = options;

    if (verified) {
      if (trusted) {
        throw new Error('EvoSDK: trusted and verified are mutually exclusive; pick one source of quorum keys');
      }
      if (network !== 'mainnet' && network !== 'testnet') {
        throw new Error(`EvoSDK: verified quorum keys are only available on mainnet and testnet, not ${network}`);
      }
      if (checkpoint != null && continuation != null) {
        throw new Error('EvoSDK: checkpoint and continuation are mutually exclusive; pass one place to start proofs from');
      }
    } else if (checkpoint != null || continuation != null || quorumRelays != null) {
      throw new Error('EvoSDK: checkpoint, continuation and quorumRelays are only meaningful when verified === true');
    }

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

    this.options = {
      network,
      trusted,
      verified,
      checkpoint,
      continuation,
      quorumRelays,
      addresses,
      devnetName,
      quorumUrl,
      ...connection,
    };

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
    this.contractGroups = new ContractGroupsFacade(this);
    this.voting = new VotingFacade(this);
    this.shielded = new ShieldedFacade(this);
    this.encryptedFor = new EncryptedForFacade(this);
    this.moderationCharters = new ModerationChartersFacade(this);
    this.decisionProposals = new DecisionProposalsFacade(this);
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

    const {
      network, trusted, verified, checkpoint, continuation, quorumRelays, version, proofs, settings, logs,
      addresses, devnetName, quorumUrl,
    } = this.options;

    // Prefetch trusted context only when trusted mode is requested. Explicit
    // addresses win over discovered ones in `withTrustedContext`, so the
    // masternode discovery request is skipped when they are given.
    const discoverAddresses = !(addresses && addresses.length > 0);
    let context: wasm.WasmTrustedContext | undefined;
    if (trusted) {
      if (network === 'mainnet') {
        context = quorumUrl
          ? await wasm.WasmTrustedContext.prefetchMainnetWithUrl(quorumUrl, discoverAddresses)
          : await wasm.WasmTrustedContext.prefetchMainnet(discoverAddresses);
      } else if (network === 'testnet') {
        context = quorumUrl
          ? await wasm.WasmTrustedContext.prefetchTestnetWithUrl(quorumUrl, discoverAddresses)
          : await wasm.WasmTrustedContext.prefetchTestnet(discoverAddresses);
      } else if (network === 'local') {
        context = quorumUrl
          ? await wasm.WasmTrustedContext.prefetchLocalWithUrl(quorumUrl, discoverAddresses)
          : await wasm.WasmTrustedContext.prefetchLocal(discoverAddresses);
      } else if (network === 'devnet') {
        if (quorumUrl) {
          context = await wasm.WasmTrustedContext.prefetchDevnetWithUrl(quorumUrl, discoverAddresses);
        } else if (devnetName) {
          context = await wasm.WasmTrustedContext.prefetchDevnet(devnetName, discoverAddresses);
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

    // A verified context discovers no addresses, so the builder keeps the
    // network's or the caller's. The constructor admits mainnet and testnet only.
    let verifiedContext: wasm.WasmVerifiedContext | undefined;
    if (verified) {
      if (network === 'mainnet') {
        if (continuation) {
          verifiedContext = wasm.WasmVerifiedContext.mainnetFromContinuation(continuation, quorumRelays);
        } else if (checkpoint) {
          verifiedContext = wasm.WasmVerifiedContext.mainnetWithCheckpoint(checkpoint, quorumRelays);
        } else {
          verifiedContext = wasm.WasmVerifiedContext.mainnet(quorumRelays);
        }
      } else if (continuation) {
        verifiedContext = wasm.WasmVerifiedContext.testnetFromContinuation(continuation, quorumRelays);
      } else if (checkpoint) {
        verifiedContext = wasm.WasmVerifiedContext.testnetWithCheckpoint(checkpoint, quorumRelays);
      } else {
        verifiedContext = wasm.WasmVerifiedContext.testnet(quorumRelays);
      }
      builder = builder.withVerifiedContext(verifiedContext);
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
    this.verifiedContext = verifiedContext;
  }

  /**
   * Where quorum proofs got to, for the next start to resume from;
   * `undefined` unless this EvoSDK connected with `verified: true` (an SDK
   * from `fromWasm` has none to report). Save it once queries have run
   * (for example on page hide) and pass it back as `continuation`, so later
   * proofs stay short.
   *
   * @example
   * ```typescript
   * const continuation = sdk.verifiedContinuation();
   * if (continuation) localStorage.setItem('dash-continuation', JSON.stringify(continuation));
   * // next start:
   * const saved = localStorage.getItem('dash-continuation');
   * const sdk = EvoSDK.testnetVerified({ continuation: saved ? JSON.parse(saved) : undefined });
   * ```
   */
  verifiedContinuation(): wasm.QuorumProofContinuation | undefined {
    return this.verifiedContext?.continuation();
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

  /**
   * Hard ceiling on a ranked / having-range `limit`. A request above it is
   * rejected, not truncated.
   */
  static async maxRankedLimit(): Promise<number> {
    await initWasm();
    return wasm.WasmSdk.maxRankedLimit();
  }

  /**
   * Hard ceiling on the element count of a branching `in` prefix pin on a
   * ranked / having-range query. Each element is its own secondary walk and
   * its own proof branch, so this bounds the fan-out one request can ask
   * for. A pin above it is rejected, not truncated.
   */
  static async maxPrefixInBranches(): Promise<number> {
    await initWasm();
    return wasm.WasmSdk.maxPrefixInBranches();
  }

  /**
   * Fixed-point divisor for the `avg` axis of a ranked / having-range
   * result. Exposed so a caller who persisted a `DocumentsGroupEntry.value`
   * can re-render it without holding on to the result that produced it.
   * Never hardcode the number.
   */
  static async rankedAverageScale(): Promise<bigint> {
    await initWasm();
    return wasm.WasmSdk.rankedAverageScale();
  }

  // Factory helpers that return configured instances (not connected)
  static testnet(options: ConnectionOptions = {}): EvoSDK { return new EvoSDK({ network: 'testnet', ...options }); }
  static mainnet(options: ConnectionOptions = {}): EvoSDK { return new EvoSDK({ network: 'mainnet', ...options }); }
  static testnetTrusted(options: ConnectionOptions = {}): EvoSDK { return new EvoSDK({ network: 'testnet', trusted: true, ...options }); }
  static mainnetTrusted(options: ConnectionOptions = {}): EvoSDK { return new EvoSDK({ network: 'mainnet', trusted: true, ...options }); }
  static local(options: ConnectionOptions = {}): EvoSDK { return new EvoSDK({ network: 'local', ...options }); }
  static localTrusted(options: ConnectionOptions = {}): EvoSDK { return new EvoSDK({ network: 'local', trusted: true, ...options }); }
  /** Testnet with quorum keys proven from a checkpoint; see `EvoSDKOptions.verified`. */
  static testnetVerified(options: ConnectionOptions & VerifiedOptions = {}): EvoSDK { return new EvoSDK({ network: 'testnet', verified: true, ...options }); }
  /** Mainnet with quorum keys proven from a checkpoint; see `EvoSDKOptions.verified`. */
  static mainnetVerified(options: ConnectionOptions & VerifiedOptions = {}): EvoSDK { return new EvoSDK({ network: 'mainnet', verified: true, ...options }); }

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
export { ContractGroupsFacade } from './contract-groups/facade.js';
export { VotingFacade } from './voting/facade.js';
export { ShieldedFacade } from './shielded/facade.js';
export { EncryptedForFacade } from './encrypted-for/facade.js';
export { ModerationChartersFacade } from './moderation-charters/facade.js';
export { DecisionProposalsFacade } from './decision-proposals/facade.js';
export { wallet } from './wallet/functions.js';
export * from './wasm.js';
