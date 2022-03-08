import {keyChainId, DerivableKeyChain} from "../DerivableKeyChain/DerivableKeyChain";

export declare class KeyChainStore {
  constructor();

  keyChains: Map<keyChainId, DerivableKeyChain>
  masterKeyChainId: keyChainId | null;

  addKeyChain(keychain: DerivableKeyChain, opts?: addKeyChainParam): void;
  getKeyChain(keychainId: keyChainId): DerivableKeyChain;
  getKeyChains(): Array<DerivableKeyChain>;
  makeChildKeyChainStore(path: string, opts: DerivableKeyChain.IDerivableKeyChainOptions): KeyChainStore;
  getMasterKeyChain(): DerivableKeyChain;
}

export declare interface addKeyChainParam {
  isMasterKeyChain?: boolean;
}


