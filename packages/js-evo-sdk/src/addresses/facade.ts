import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class AddressesFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  /**
   * Fetches information about a Platform address including its nonce and balance.
   *
   * @param address - The platform address to query (PlatformAddress, Uint8Array, or bech32m string)
   * @returns PlatformAddressInfo containing address, nonce, and balance, or undefined if not found
   */
  async get(address: wasm.PlatformAddressLike): Promise<wasm.PlatformAddressInfo | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getAddressInfo(address);
  }

  /**
   * Fetches information about a Platform address with proof.
   *
   * @param address - The platform address to query (PlatformAddress, Uint8Array, or bech32m string)
   * @returns ProofMetadataResponse containing PlatformAddressInfo with proof information
   */
  async getWithProof(address: wasm.PlatformAddressLike): Promise<wasm.ProofMetadataResponseTyped<wasm.PlatformAddressInfo>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getAddressInfoWithProofInfo(address);
  }

  /**
   * Fetches information about multiple Platform addresses.
   *
   * @param addresses - Array of platform addresses to query
   * @returns Map of PlatformAddress to PlatformAddressInfo (or undefined for unfunded addresses)
   */
  async getMany(
    addresses: wasm.PlatformAddressLike[],
  ): Promise<Map<wasm.PlatformAddress, wasm.PlatformAddressInfo | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getAddressesInfos(addresses);
  }

  /**
   * Fetches information about multiple Platform addresses with proof.
   *
   * @param addresses - Array of platform addresses to query
   * @returns ProofMetadataResponse containing Map of PlatformAddress to PlatformAddressInfo
   */
  async getManyWithProof(
    addresses: wasm.PlatformAddressLike[],
  ): Promise<wasm.ProofMetadataResponseTyped<Map<wasm.PlatformAddress, wasm.PlatformAddressInfo | undefined>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getAddressesInfosWithProofInfo(addresses);
  }
}
