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

  /**
   * Transfers credits between Platform addresses.
   *
   * @param options - Transfer options including inputs, outputs, and signer
   * @returns Promise resolving to transfer result with updated address information
   *
   * @example
   * ```typescript
   * const recipientAddr = PlatformAddress.fromBech32m("tdash1...");
   * const privateKey = PrivateKey.fromWIF("cPrivateKeyWif...");
   *
   * const signer = new PlatformAddressSigner();
   * const senderAddr = signer.addKey(privateKey); // Derives P2PKH address from key
   *
   * const input = new PlatformAddressInput(senderAddr, 0n, 100000n);
   * const output = new PlatformAddressOutput(recipientAddr, 90000n);
   *
   * const result = await sdk.addresses.transfer({
   *   inputs: [input],
   *   outputs: [output],
   *   signer
   * });
   * ```
   */
  async transfer(options: wasm.AddressFundsTransferOptions): Promise<Map<wasm.PlatformAddress, wasm.PlatformAddressInfo>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.addressFundsTransfer(options);
  }

  /**
   * Top up an identity from Platform addresses.
   *
   * @param options - Top up options including identity ID, inputs, and signer
   * @returns Promise resolving to result with updated address infos and new identity balance
   *
   * @example
   * ```typescript
   * const identityId = Identifier.from("...");
   * const privateKey = PrivateKey.fromWIF("cPrivateKeyWif...");
   *
   * const signer = new PlatformAddressSigner();
   * const sourceAddr = signer.addKey(privateKey); // Derives P2PKH address from key
   *
   * const input = new PlatformAddressInput(sourceAddr, 0n, 50000n);
   *
   * const result = await sdk.addresses.topUpIdentity({
   *   identityId,
   *   inputs: [input],
   *   signer
   * });
   *
   * console.log('New identity balance:', result.newBalance);
   * ```
   */
  async topUpIdentity(options: wasm.IdentityTopUpFromAddressesOptions): Promise<wasm.IdentityTopUpFromAddressesResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityTopUpFromAddresses(options);
  }

  /**
   * Withdraws Platform address credits to Dash Core.
   *
   * @param options - Withdrawal options including inputs, output script, pooling, and signer
   * @returns Promise resolving to Map of PlatformAddress to PlatformAddressInfo
   *
   * @example
   * ```typescript
   * const privateKey = PrivateKey.fromWIF("cPrivateKeyWif...");
   *
   * // Create Core output script for L1 destination
   * const coreScript = CoreScript.newP2PKH(coreAddressHash);
   *
   * const signer = new PlatformAddressSigner();
   * const platformAddr = signer.addKey(privateKey); // Derives P2PKH address from key
   *
   * const input = new PlatformAddressInput(platformAddr, 0n, 100000n);
   *
   * const result = await sdk.addresses.withdraw({
   *   inputs: [input],
   *   coreFeePerByte: 1,
   *   pooling: PoolingWasm.Standard,
   *   outputScript: coreScript,
   *   signer
   * });
   * ```
   */
  async withdraw(options: wasm.AddressFundsWithdrawOptions): Promise<Map<wasm.PlatformAddress, wasm.PlatformAddressInfo>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.addressFundsWithdraw(options);
  }

  /**
   * Transfer credits from an identity to Platform addresses.
   *
   * @param options - Transfer options including identity ID, outputs, and signer
   * @returns Result with updated address information and new identity balance
   *
   * @example
   * ```typescript
   * const identityId = Identifier.from("...");
   * const recipientAddr = PlatformAddress.fromBech32m("tdash1...");
   * const privateKey = PrivateKey.fromWIF("cPrivateKeyWif..."); // Identity transfer key
   *
   * const output = new PlatformAddressOutput(recipientAddr, 100000n);
   *
   * // Create identity signer and add the transfer key
   * const signer = new IdentitySigner();
   * signer.addKey(privateKey);
   *
   * const result = await sdk.addresses.transferFromIdentity({
   *   identityId,
   *   outputs: [output],
   *   signer
   * });
   *
   * console.log(`New identity balance: ${result.newBalance}`);
   * console.log(`Updated addresses:`, result.addressInfos);
   * ```
   */
  async transferFromIdentity(options: wasm.IdentityTransferToAddressesOptions): Promise<wasm.IdentityTransferToAddressesResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityTransferToAddresses(options);
  }

  /**
   * Fund Platform addresses from an asset lock.
   *
   * @param options - Funding options including asset lock proof, outputs, and signer
   * @returns Promise resolving to Map of PlatformAddress to PlatformAddressInfo
   *
   * @example
   * ```typescript
   * const assetLockPrivateKey = PrivateKey.fromWIF("cPrivateKeyWif...");
   * const addressPrivateKey = PrivateKey.fromWIF("cPrivateKeyWif...");
   *
   * // Create asset lock proof from L1 transaction
   * const assetLockProof = AssetLockProof.createInstantAssetLockProof(
   *   instantLockBytes,
   *   transactionBytes,
   *   outputIndex
   * );
   *
   * const signer = new PlatformAddressSigner();
   * const platformAddr = signer.addKey(addressPrivateKey); // Derives P2PKH address from key
   *
   * const output = new PlatformAddressOutput(platformAddr, 100000n);
   *
   * const result = await sdk.addresses.fundFromAssetLock({
   *   assetLockProof,
   *   assetLockPrivateKey,
   *   outputs: [output],
   *   signer
   * });
   * ```
   */
  async fundFromAssetLock(options: wasm.AddressFundingFromAssetLockOptions): Promise<Map<wasm.PlatformAddress, wasm.PlatformAddressInfo>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.addressFundingFromAssetLock(options);
  }

  /**
   * Create an identity funded from Platform addresses.
   *
   * @param options - Creation options including identity, inputs, and signers
   * @returns Promise resolving to result with created identity and updated address infos
   *
   * @example
   * ```typescript
   * const addressPrivateKey = PrivateKey.fromWIF("cAddressPrivateKeyWif...");
   * const identityPrivateKey = PrivateKey.fromWIF("cIdentityKeyWif...");
   *
   * // Create identity structure with public keys
   * const identity = new Identity(Identifier.random());
   * identity.addPublicKey(identityPublicKey);
   *
   * // Create signers
   * const addressSigner = new PlatformAddressSigner();
   * const sourceAddr = addressSigner.addKey(addressPrivateKey); // Derives P2PKH address from key
   *
   * const input = new PlatformAddressInput(sourceAddr, 0n, 100000n);
   *
   * const identitySigner = new IdentitySigner();
   * identitySigner.addKey(identityPrivateKey);
   *
   * const result = await sdk.addresses.createIdentity({
   *   identity,
   *   inputs: [input],
   *   identitySigner,
   *   addressSigner
   * });
   *
   * console.log('Created identity:', result.identity.id());
   * console.log('Updated addresses:', result.addressInfos);
   * ```
   */
  async createIdentity(options: wasm.IdentityCreateFromAddressesOptions): Promise<wasm.IdentityCreateFromAddressesResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityCreateFromAddresses(options);
  }
}
