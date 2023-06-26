export declare interface WalletStoreState {
  mnemonic: string;
  paths: Map<string, any>
  identities: Map<string, any>
}

type walletId = string;
type ExportedState = any;

export declare class WalletStore {
  constructor(walletId: walletId);

  walletId: walletId;
  state: WalletStoreState;

  createPathState(path: string): void;
  exportState(): ExportedState;
  getIdentityIdByIndex(identityIndex: number): string;
  getIndexedIdentityIds(identityIndex: number): Array<string>;
  getPathState(path: string): any;

  importState(exportedState: ExportedState): void;
  insertIdentityIdAtIndex(identityId: string, identityIndex: number): void;
}


