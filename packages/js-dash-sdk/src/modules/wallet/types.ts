export interface WalletAdapter {
  initialize(): Promise<void>;
  getAddresses(accountIndex?: number): Promise<string[]>;
  getBalance(accountIndex?: number): Promise<number>;
  signStateTransition(
    stateTransition: Uint8Array,
    identityId: string,
    keyIndex: number,
    keyType?: 'ECDSA' | 'BLS'
  ): Promise<Uint8Array>;
  createAssetLockProof(request: {
    amount: number;
    accountIndex?: number;
    oneTimePrivateKey?: Uint8Array;
  }): Promise<{
    type: 'instant' | 'chain';
    instantLock?: Uint8Array;
    transaction?: Uint8Array;
    outputIndex?: number;
  }>;
  isReady(): boolean;
  getNetwork(): 'mainnet' | 'testnet' | 'devnet';
}

export interface WalletOptions {
  adapter?: WalletAdapter;
  mnemonic?: string;
  seed?: string;
  privateKey?: string;
}

export interface Account {
  index: number;
  address: string;
  balance: number;
  privateKey?: string;
}

export interface HDWalletOptions {
  mnemonic: string;
  passphrase?: string;
  accountIndex?: number;
  network?: 'mainnet' | 'testnet' | 'devnet';
}