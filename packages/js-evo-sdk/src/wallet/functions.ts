import * as wasm from '../wasm.js';

export namespace wallet {
  export async function generateMnemonic(wordCount?: number, languageCode?: string): Promise<string> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.generateMnemonic(wordCount ?? null, languageCode ?? null);
  }

  export async function validateMnemonic(mnemonic: string, languageCode?: string): Promise<boolean> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.validateMnemonic(mnemonic, languageCode ?? null);
  }

  export async function mnemonicToSeed(mnemonic: string, passphrase?: string): Promise<Uint8Array> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.mnemonicToSeed(mnemonic, passphrase ?? null);
  }

  export async function deriveKeyFromSeedPhrase(mnemonic: string, passphrase: string | null | undefined, network: string): Promise<any> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.deriveKeyFromSeedPhrase(mnemonic, passphrase ?? null, network);
  }

  export async function deriveKeyFromSeedWithPath(mnemonic: string, passphrase: string | null | undefined, path: string, network: string): Promise<any> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.deriveKeyFromSeedWithPath(mnemonic, passphrase ?? null, path, network);
  }

  export async function deriveKeyFromSeedWithExtendedPath(mnemonic: string, passphrase: string | null | undefined, path: string, network: string): Promise<any> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.deriveKeyFromSeedWithExtendedPath(mnemonic, passphrase ?? null, path, network);
  }

  export async function deriveDashpayContactKey(mnemonic: string, passphrase: string | null | undefined, senderIdentityId: string, receiverIdentityId: string, account: number, addressIndex: number, network: string): Promise<any> {
    await wasm.ensureInitialized();
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

  export async function derivationPathBip44Mainnet(account: number, change: number, index: number): Promise<any> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.derivationPathBip44Mainnet(account, change, index);
  }

  export async function derivationPathBip44Testnet(account: number, change: number, index: number): Promise<any> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.derivationPathBip44Testnet(account, change, index);
  }

  export async function derivationPathDip9Mainnet(featureType: number, account: number, index: number): Promise<any> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.derivationPathDip9Mainnet(featureType, account, index);
  }

  export async function derivationPathDip9Testnet(featureType: number, account: number, index: number): Promise<any> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.derivationPathDip9Testnet(featureType, account, index);
  }

  export async function derivationPathDip13Mainnet(account: number): Promise<any> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.derivationPathDip13Mainnet(account);
  }

  export async function derivationPathDip13Testnet(account: number): Promise<any> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.derivationPathDip13Testnet(account);
  }

  export async function deriveChildPublicKey(xpub: string, index: number, hardened: boolean): Promise<string> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.deriveChildPublicKey(xpub, index, hardened);
  }

  export async function xprvToXpub(xprv: string): Promise<string> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.xprvToXpub(xprv);
  }

  export async function generateKeyPair(network: string): Promise<any> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.generateKeyPair(network);
  }

  export async function generateKeyPairs(network: string, count: number): Promise<any[]> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.generateKeyPairs(network, count);
  }

  export async function keyPairFromWif(privateKeyWif: string): Promise<any> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.keyPairFromWif(privateKeyWif);
  }

  export async function keyPairFromHex(privateKeyHex: string, network: string): Promise<any> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.keyPairFromHex(privateKeyHex, network);
  }

  export async function pubkeyToAddress(pubkeyHex: string, network: string): Promise<string> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.pubkeyToAddress(pubkeyHex, network);
  }

  export async function validateAddress(address: string, network: string): Promise<boolean> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.validateAddress(address, network);
  }

  export async function signMessage(message: string, privateKeyWif: string): Promise<string> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.signMessage(message, privateKeyWif);
  }
}
