import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

/**
 * Encrypting and decrypting the byte properties a document type declares `encryptedFor`.
 *
 * Every method reads the declaration from the contract given, so it works for any contract
 * that declares one. None needs a connection: they run locally.
 */
export class EncryptedForFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  /**
   * Encrypts a message into a property declaring `encryptedFor`.
   *
   * @returns The properties to set on the document: the ciphertext at `property` and the two
   * key ids at the declaration's `recipientKey` and `senderKey` paths. The recipient property
   * is the caller's to set.
   */
  async encrypt(options: wasm.EncryptDocumentPropertyOptions): Promise<Record<string, unknown>> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.encryptDocumentProperty(options);
  }

  /**
   * Decrypts a property declaring `encryptedFor`. The scheme carries no authentication tag:
   * a wrong key is caught only by the padding check, which it passes about once in 256
   * attempts, returning garbage.
   */
  async decrypt(options: wasm.DecryptDocumentPropertyOptions): Promise<Uint8Array> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.decryptDocumentProperty(options);
  }

  /**
   * Whose keys an encrypted property of a document is under: the recipient and sender
   * identities and the ids of their keys, which a reader fetches to decrypt it.
   */
  async envelope(options: wasm.EncryptedPropertyEnvelopeOptions): Promise<wasm.EncryptedPropertyEnvelope> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.encryptedPropertyEnvelope(options);
  }
}
