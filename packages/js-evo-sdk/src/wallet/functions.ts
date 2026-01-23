import * as wasm from '../wasm.js';
import type { NetworkLike } from '../wasm.js';

export namespace wallet {
  export async function generateMnemonic(params?: wasm.GenerateMnemonicParams): Promise<string> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.generateMnemonic(params ?? null);
  }

  export async function validateMnemonic(mnemonic: string, languageCode?: string): Promise<boolean> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.validateMnemonic(mnemonic, languageCode ?? null);
  }

  export async function mnemonicToSeed(mnemonic: string, passphrase?: string): Promise<Uint8Array> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.mnemonicToSeed(mnemonic, passphrase ?? null);
  }

  export async function deriveKeyFromSeedPhrase(params: wasm.DeriveKeyFromSeedPhraseParams): Promise<wasm.SeedPhraseKeyInfo> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.deriveKeyFromSeedPhrase(params);
  }

  export async function deriveKeyFromSeedWithPath(params: wasm.DeriveKeyFromSeedWithPathParams): Promise<wasm.PathDerivedKeyInfo> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.deriveKeyFromSeedWithPath(params);
  }

  export async function deriveKeyFromSeedWithExtendedPath(params: wasm.DeriveKeyFromSeedWithExtendedPathParams): Promise<wasm.DerivedKeyInfo> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.deriveKeyFromSeedWithExtendedPath(params);
  }

  export async function deriveDashpayContactKey(params: wasm.DeriveDashpayContactKeyParams): Promise<wasm.DashpayContactKeyInfo> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.deriveDashpayContactKey(params);
  }

  export async function derivationPathBip44Mainnet(account: number, change: number, index: number): Promise<wasm.DerivationPathInfo> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.derivationPathBip44Mainnet(account, change, index);
  }

  export async function derivationPathBip44Testnet(account: number, change: number, index: number): Promise<wasm.DerivationPathInfo> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.derivationPathBip44Testnet(account, change, index);
  }

  export async function derivationPathDip9Mainnet(featureType: number, account: number, index: number): Promise<wasm.DerivationPathInfo> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.derivationPathDip9Mainnet(featureType, account, index);
  }

  export async function derivationPathDip9Testnet(featureType: number, account: number, index: number): Promise<wasm.DerivationPathInfo> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.derivationPathDip9Testnet(featureType, account, index);
  }

  export async function derivationPathDip13Mainnet(account: number): Promise<wasm.Dip13DerivationPathInfo> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.derivationPathDip13Mainnet(account);
  }

  export async function derivationPathDip13Testnet(account: number): Promise<wasm.Dip13DerivationPathInfo> {
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

  export async function generateKeyPair(network: NetworkLike): Promise<wasm.KeyPair> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.generateKeyPair(network);
  }

  export async function generateKeyPairs(network: NetworkLike, count: number): Promise<wasm.KeyPair[]> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.generateKeyPairs(network, count);
  }

  export async function keyPairFromWif(privateKeyWif: string): Promise<wasm.KeyPair> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.keyPairFromWif(privateKeyWif);
  }

  export async function keyPairFromHex(privateKeyHex: string, network: NetworkLike): Promise<wasm.KeyPair> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.keyPairFromHex(privateKeyHex, network);
  }

  export async function pubkeyToAddress(pubkeyHex: string, network: NetworkLike): Promise<string> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.pubkeyToAddress(pubkeyHex, network);
  }

  export async function validateAddress(address: string, network: NetworkLike): Promise<boolean> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.validateAddress(address, network);
  }

  export async function signMessage(message: string, privateKeyWif: string): Promise<string> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.signMessage(message, privateKeyWif);
  }
}
