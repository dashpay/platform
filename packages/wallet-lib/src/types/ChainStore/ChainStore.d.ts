export declare interface feeState {
  minRelay: number
}

export declare interface HeaderMetadata {
  height: number,
  time: number
}

export declare interface ChainStoreState {
  fees: feeState;
  blockHeight: number;
  lastSyncedHeaderHeight: number,
  lastSyncedBlockHeight: number,
  blockHeaders: any[],
  headersMetadata: Map<string, HeaderMetadata>,
  hashesByHeight: Map<number, string>,
  transactions: Map<string, any>
  instantLocks: Map<string, any>
  addresses: Map<string, any>
}

type NetworkIdentifier = string;
type ExportedState = any;

export declare class ChainStore {
  constructor(networkIdentifier: NetworkIdentifier);
  network: NetworkIdentifier;

  state: ChainStoreState;

  considerTransaction(transactionHash: string): any;
  exportState(): ExportedState;
  importState(state: ExportedState): void;

  getAddress(address: string): any;
  getAddresses(address: string): Map<string, any>

  getBlockHeader(blockHeaderHash: string): any;
  getInstantLock(transactionHash: string): any;
  getTransaction(transactionHash: string): any;

  importAddress(address: any): void;
  importAddress(blockHeader: any): void;
  importInstantLock(instantLock: any): void;
  importTransaction(transaction: any, metadata: any): any;
}
