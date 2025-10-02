import * as wasm from '../wasm.js';

export namespace wallet {
  export function generateMnemonic(wordCount?: number, languageCode?: string): string {
    return wasm.WasmSdk.generateMnemonic(wordCount ?? null, languageCode ?? null);
  }

  export function validateMnemonic(mnemonic: string, languageCode?: string): boolean {
    return wasm.WasmSdk.validateMnemonic(mnemonic, languageCode ?? null);
  }

  export function mnemonicToSeed(mnemonic: string, passphrase?: string): Uint8Array {
    return wasm.WasmSdk.mnemonicToSeed(mnemonic, passphrase ?? null);
  }

  export function deriveKeyFromSeedPhrase(mnemonic: string, passphrase: string | null | undefined, network: string): any {
    return wasm.WasmSdk.deriveKeyFromSeedPhrase(mnemonic, passphrase ?? null, network);
  }

  export function deriveKeyFromSeedWithPath(mnemonic: string, passphrase: string | null | undefined, path: string, network: string): any {
    return wasm.WasmSdk.deriveKeyFromSeedWithPath(mnemonic, passphrase ?? null, path, network);
  }

  export function deriveKeyFromSeedWithExtendedPath(mnemonic: string, passphrase: string | null | undefined, path: string, network: string): any {
    return wasm.WasmSdk.deriveKeyFromSeedWithExtendedPath(mnemonic, passphrase ?? null, path, network);
  }

  export function deriveDashpayContactKey(mnemonic: string, passphrase: string | null | undefined, senderIdentityId: string, receiverIdentityId: string, account: number, addressIndex: number, network: string): any {
    return wasm.WasmSdk.deriveDashpayContactKey(
      mnemonic,
      passphrase ?? null,
      senderIdentityId,
      receiverIdentityId,
      account,
      addressIndex,
      network,
    );
  }

  export function derivationPathBip44Mainnet(account: number, change: number, index: number): any {
    return wasm.WasmSdk.derivationPathBip44Mainnet(account, change, index);
  }

  export function derivationPathBip44Testnet(account: number, change: number, index: number): any {
    return wasm.WasmSdk.derivationPathBip44Testnet(account, change, index);
  }

  export function derivationPathDip9Mainnet(featureType: number, account: number, index: number): any {
    return wasm.WasmSdk.derivationPathDip9Mainnet(featureType, account, index);
  }

  export function derivationPathDip9Testnet(featureType: number, account: number, index: number): any {
    return wasm.WasmSdk.derivationPathDip9Testnet(featureType, account, index);
  }

  export function derivationPathDip13Mainnet(account: number): any {
    return wasm.WasmSdk.derivationPathDip13Mainnet(account);
  }

  export function derivationPathDip13Testnet(account: number): any {
    return wasm.WasmSdk.derivationPathDip13Testnet(account);
  }

  export function deriveChildPublicKey(xpub: string, index: number, hardened: boolean): string {
    return wasm.WasmSdk.deriveChildPublicKey(xpub, index, hardened);
  }

  export function xprvToXpub(xprv: string): string {
    return wasm.WasmSdk.xprvToXpub(xprv);
  }

  export function generateKeyPair(network: string): any {
    return wasm.WasmSdk.generateKeyPair(network);
  }

  export function generateKeyPairs(network: string, count: number): any[] {
    return wasm.WasmSdk.generateKeyPairs(network, count);
  }

  export function keyPairFromWif(privateKeyWif: string): any {
    return wasm.WasmSdk.keyPairFromWif(privateKeyWif);
  }

  export function keyPairFromHex(privateKeyHex: string, network: string): any {
    return wasm.WasmSdk.keyPairFromHex(privateKeyHex, network);
  }

  export function pubkeyToAddress(pubkeyHex: string, network: string): string {
    return wasm.WasmSdk.pubkeyToAddress(pubkeyHex, network);
  }

  export function validateAddress(address: string, network: string): boolean {
    return wasm.WasmSdk.validateAddress(address, network);
  }

  export function signMessage(message: string, privateKeyWif: string): string {
    return wasm.WasmSdk.signMessage(message, privateKeyWif);
  }
}
