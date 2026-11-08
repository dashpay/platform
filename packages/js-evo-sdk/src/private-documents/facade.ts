import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

/**
 * Private document types (protocol version 15) and DashPay's private contact requests.
 *
 * A private type's documents are fixed-size encrypted entries of a store, in append order,
 * readable a page at a time. A DashPay private contact request is one such entry sealed to the
 * recipient's DashPay key: only the recipient learns it exists, who sent it and what it holds.
 *
 * Appending an entry is paid from the shielded pool, which needs an Orchard proof this SDK
 * cannot make yet (see `ShieldedFacade`): `sealContactRequest` builds the entry, and a wallet
 * that can prove appends it.
 */
export class PrivateDocumentsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  /**
   * The proved page of a private type's store from `startPosition`, up to the end of the
   * store's chunk holding it. A reader walks the store by asking again from the position after
   * the page's last entry, until `totalCount`.
   */
  async entries(
    dataContractId: wasm.IdentifierLike,
    documentTypeName: string,
    startPosition: bigint,
  ): Promise<wasm.PrivateDocumentEntriesPage> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getPrivateDocumentEntries(dataContractId, documentTypeName, startPosition);
  }

  /**
   * Seals a DashPay private contact request from the sender's encryption key to one of the
   * recipient's DashPay keys, signed by `options.signer` with one of the sender's signing keys,
   * as its transitions are.
   *
   * @returns The entry, of the type's entry size, to append.
   */
  async sealContactRequest(options: wasm.SealPrivateContactRequestOptions): Promise<Uint8Array> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.sealPrivateContactRequest(options);
  }

  /**
   * Reads one page of DashPay's private contact request store for the wallet's identities: the
   * requests to them, each checked against its sender's signing keys, and the requests they
   * sent. Keep `nextPosition` as the cursor once the page is stored, and scan
   * again from it until `isComplete`.
   */
  async scanContactRequests(
    options: wasm.ScanPrivateContactRequestsOptions,
  ): Promise<wasm.PrivateContactRequestScanResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.scanPrivateContactRequests(options);
  }
}
