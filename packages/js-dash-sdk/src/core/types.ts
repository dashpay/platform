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

// Types for state transitions
export type BlockHeight = number;
export interface StateTransition {
  toBuffer(): Buffer;
  signature?: Buffer;
}

// Context provider for blockchain operations
export interface ContextProvider {
  // Block operations
  getBlockHash(height: BlockHeight): Promise<string>;
  
  // Contract operations
  getDataContract(identifier: string): Promise<any>;
  
  // State transition operations
  waitForStateTransitionResult(stHash: string, prove: boolean): Promise<any>;
  broadcastStateTransition(stateTransition: StateTransition): Promise<string>;
  
  // Protocol information
  getProtocolVersion(): Promise<number>;
  
  // Platform information methods (optional for backwards compatibility)
  getLatestPlatformBlockHeight?(): Promise<number>;
  getLatestPlatformBlockTime?(): Promise<number>;
  getLatestPlatformCoreChainLockedHeight?(): Promise<number>;
  getLatestPlatformVersion?(): Promise<string>;
  getProposerBlockCount?(proposerProTxHash: string): Promise<number | null>;
  getTimePerBlockMillis?(): Promise<number>;
  getBlockProposer?(blockHeight: number): Promise<string | null>;
  isValid?(): Promise<boolean>;
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