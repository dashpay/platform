export declare interface feeState {
  minRelay: number
}

export declare interface ChainStoreState {
  fees: feeState;
  blockHeight: number;
  blockHeaders: Map<string, any>
  transactions: Map<string, any>
  instantLocks: Map<string, any>
  addresses: Map<string, any>
}

type networkIdentifier = string;
type exportedState = any;

export declare class ChainStore {
  constructor(networkIdentifier: networkIdentifier);
  network: networkIdentifier;

  state: ChainStoreState;

  considerTransaction(transactionHash: string): any;
  exportState(): exportedState;
  importState(exportedState): void;

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
