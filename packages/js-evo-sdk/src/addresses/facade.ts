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
   * This method handles the complete transfer flow:
   * 1. Fetches current nonces for all input addresses
   * 2. Builds and signs the transfer transition
   * 3. Broadcasts and waits for confirmation
   *
   * @param options - Transfer options including inputs, outputs, and signer
   * @returns Promise resolving to transfer result with updated address information
   *
   * @example
   * ```typescript
   * const senderAddr = PlatformAddress.fromBech32m("tdashevo1...");
   * const recipientAddr = PlatformAddress.fromBech32m("tdashevo1...");
   * const privateKey = PrivateKey.fromWIF("cPrivateKeyWif...");
   *
   * const input = new PlatformAddressInput(senderAddr, 0n, 100000n);
   * const output = new PlatformAddressOutput(recipientAddr, 90000n);
   *
   * const signer = new PlatformAddressSigner();
   * signer.addKey(senderAddr, privateKey);
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
   * This method handles the complete top up flow:
   * 1. Fetches the identity from Platform
   * 2. Fetches current nonces for all input addresses
   * 3. Builds and signs the identity top up transition
   * 4. Broadcasts and waits for confirmation
   *
   * @param options - Top up options including identity ID, inputs, and signer
   * @returns Promise resolving to result with updated address infos and new identity balance
   *
   * @example
   * ```typescript
   * const identityId = Identifier.from("...");
   * const sourceAddr = PlatformAddress.fromBech32m("tdashevo1...");
   * const privateKey = PrivateKey.fromWIF("cPrivateKeyWif...");
   *
   * const input = new PlatformAddressInput(sourceAddr, 0n, 50000n);
   *
   * const signer = new PlatformAddressSigner();
   * signer.addKey(sourceAddr, privateKey);
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
   * Withdraws Platform address credits to Core (L1).
   *
   * This method handles the complete withdrawal flow:
   * 1. Fetches current nonces for all input addresses
   * 2. Builds and signs the withdrawal transition
   * 3. Broadcasts and waits for confirmation
   * 4. The withdrawal may be pooled with others depending on the pooling strategy
   *
   * @param options - Withdrawal options including inputs, output script, pooling, and signer
   * @returns Promise resolving to Map of PlatformAddress to PlatformAddressInfo
   *
   * @example
   * ```typescript
   * const platformAddr = PlatformAddress.fromBech32m("tdashevo1...");
   * const privateKey = PrivateKey.fromWIF("cPrivateKeyWif...");
   *
   * // Create Core output script for L1 destination
   * const coreScript = CoreScript.newP2PKH(coreAddressHash);
   *
   * const input = new PlatformAddressInput(platformAddr, 0n, 100000n);
   *
   * const signer = new PlatformAddressSigner();
   * signer.addKey(platformAddr, privateKey);
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
   * This method handles the complete transfer flow:
   * 1. Fetches the identity from Platform
   * 2. Finds the appropriate transfer key to use for signing (if signingTransferKeyId specified)
   * 3. Builds and signs the identity credit transfer to addresses transition
   * 4. Broadcasts and waits for confirmation
   *
   * @param options - Transfer options including identity ID, outputs, and signer
   * @returns Result with updated address information and new identity balance
   *
   * @example
   * ```typescript
   * const identityId = Identifier.from("...");
   * const recipientAddr = PlatformAddress.fromBech32m("tdashevo1...");
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
   * This method handles the complete funding flow:
   * 1. Validates the asset lock proof
   * 2. Builds and signs the address funding transition
   * 3. Broadcasts and waits for confirmation
   *
   * @param options - Funding options including asset lock proof, outputs, and signer
   * @returns Promise resolving to Map of PlatformAddress to PlatformAddressInfo
   *
   * @example
   * ```typescript
   * const platformAddr = PlatformAddress.fromBech32m("tdashevo1...");
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
   * const output = new PlatformAddressOutput(platformAddr, 100000n);
   *
   * const signer = new PlatformAddressSigner();
   * signer.addKey(platformAddr, addressPrivateKey);
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
   * This method handles the complete identity creation flow:
   * 1. Fetches current nonces for all input addresses
   * 2. Builds and signs the identity create from addresses transition
   * 3. Broadcasts and waits for confirmation
   *
   * @param options - Creation options including identity, inputs, and signers
   * @returns Promise resolving to result with created identity and updated address infos
   *
   * @example
   * ```typescript
   * const sourceAddr = PlatformAddress.fromBech32m("tdashevo1...");
   * const addressPrivateKey = PrivateKey.fromWIF("cAddressPrivateKeyWif...");
   * const identityPrivateKey = PrivateKey.fromWIF("cIdentityKeyWif...");
   *
   * // Create identity structure with public keys
   * const identity = new Identity(Identifier.random());
   * identity.addPublicKey(identityPublicKey);
   *
   * const input = new PlatformAddressInput(sourceAddr, 0n, 100000n);
   *
   * // Create signers
   * const addressSigner = new PlatformAddressSigner();
   * addressSigner.addKey(sourceAddr, addressPrivateKey);
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
