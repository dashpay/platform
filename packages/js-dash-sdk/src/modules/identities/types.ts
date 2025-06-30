export interface Identity {
  id: string;
  balance: number;
  revision: number;
  publicKeys: IdentityPublicKey[];
}

export interface IdentityPublicKey {
  id: number;
  type: 'ECDSA_SECP256K1' | 'BLS12_381' | 'ECDSA_HASH160' | 'BIP13_SCRIPT_HASH' | 'EDDSA_25519_HASH160';
  purpose: 'AUTHENTICATION' | 'ENCRYPTION' | 'DECRYPTION' | 'WITHDRAW' | 'SYSTEM' | 'VOTING';
  securityLevel: 'MASTER' | 'HIGH' | 'MEDIUM' | 'CRITICAL';
  data: Uint8Array;
  readOnly: boolean;
  disabledAt?: number;
}

export interface IdentityCreateOptions {
  fundingAmount?: number;
  keys?: IdentityPublicKey[];
}

export interface IdentityTopUpOptions {
  amount: number;
}

export interface IdentityUpdateOptions {
  addKeys?: IdentityPublicKey[];
  disableKeys?: number[];
}

export interface AssetLockProof {
  type: 'instant' | 'chain';
  instantLock?: Uint8Array;
  transaction?: Uint8Array;
  outputIndex?: number;
}

export interface CreditTransferOptions {
  recipientId: string;
  amount: number;
}

export interface CreditWithdrawalOptions {
  amount: number;
  coreFeePerByte: number;
  pooling?: 'never' | 'if-needed' | 'always';
  outputScript?: Uint8Array;
}