export interface Network {
  name: 'mainnet' | 'testnet' | 'devnet' | string;
  type: 'mainnet' | 'testnet' | 'devnet';
}

export interface SDKOptions {
  network?: Network | string;
  contextProvider?: ContextProvider;
  wallet?: WalletOptions;
  apps?: Record<string, AppDefinition>;
  retries?: number;
  timeout?: number;
}

export interface WalletOptions {
  mnemonic?: string;
  seed?: string;
  privateKey?: string;
  adapter?: any; // WalletAdapter interface
  bluetooth?: boolean; // Use Bluetooth wallet
}

export interface AppDefinition {
  contractId: string;
  contract?: any; // DataContract type from wasm-sdk
}

export interface ContextProvider {
  getLatestPlatformBlockHeight(): Promise<number>;
  getLatestPlatformBlockTime(): Promise<number>;
  getLatestPlatformCoreChainLockedHeight(): Promise<number>;
  getLatestPlatformVersion(): Promise<string>;
  getProposerBlockCount(proposerProTxHash: string): Promise<number | null>;
  getTimePerBlockMillis(): Promise<number>;
  getBlockProposer(blockHeight: number): Promise<string | null>;
  isValid(): Promise<boolean>;
}

export interface StateTransitionResult {
  stateTransition: any; // StateTransition type from wasm-sdk
  metadata?: {
    height?: number;
    coreChainLockedHeight?: number;
    epoch?: number;
    timeMs?: number;
    protocolVersion?: number;
    fee?: number;
  };
}

export interface QueryOptions {
  limit?: number;
  startAt?: number;
  startAfter?: number;
  orderBy?: Array<[string, 'asc' | 'desc']>;
  where?: Array<WhereClause>;
}

export type WhereClause = 
  | ['=', string, any]
  | ['>', string, any]
  | ['>=', string, any]
  | ['<', string, any]
  | ['<=', string, any]
  | ['in', string, any[]]
  | ['startsWith', string, string]
  | ['contains', string, any]
  | ['exists', string]
  | ['elementMatch', string, WhereClause[]];

export interface BroadcastOptions {
  skipValidation?: boolean;
  retries?: number;
}

export interface ProofOptions {
  verify?: boolean;
}