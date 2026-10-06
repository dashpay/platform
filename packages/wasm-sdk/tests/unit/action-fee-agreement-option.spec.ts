/**
 * Broadcasting needs the network, so these check only that the option is parsed, before any
 * request is made.
 */
import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

const AUTHENTICATION = 0;
const HIGH = 2;
const ECDSA_SECP256K1 = 0;

describe('actionFeeAgreement option', () => {
  let client: sdk.WasmSdk;
  let document: sdk.Document;
  let identityKey: sdk.IdentityPublicKey;
  let signer: sdk.IdentitySigner;

  before(async () => {
    await init();
    client = await sdk.WasmSdkBuilder.testnet().build();

    const privateKey = sdk.PrivateKey.fromHex('21'.repeat(32), 'testnet');
    identityKey = new sdk.IdentityPublicKey({
      keyId: 1,
      purpose: AUTHENTICATION,
      securityLevel: HIGH,
      keyType: ECDSA_SECP256K1,
      isReadOnly: false,
      data: privateKey.getPublicKey().toBytes(),
    });
    signer = new sdk.IdentitySigner();
    signer.addKey(privateKey);
    document = new sdk.Document({
      properties: { message: 'hello' },
      documentTypeName: 'post',
      dataContractId: new Uint8Array(32).fill(9),
      ownerId: new Uint8Array(32).fill(7),
      revision: BigInt(2),
      entropy: new Uint8Array(32).fill(3),
    });
  });

  after(() => {
    client?.free();
  });

  // The options to build an agreement are not one: it is passed as a DocumentActionFeeAgreement
  const notAnInstance = { owner: BigInt(80000000) };
  const NOT_AN_INSTANCE = /Expected DocumentActionFeeAgreement, provided /;

  it('should refuse agreement options on a create before reading anything', async () => {
    await expect(
      client.documentCreate({
        document, identityKey, signer, actionFeeAgreement: notAnInstance as never,
      }),
    ).to.be.rejectedWith(NOT_AN_INSTANCE);
  });

  it('should refuse agreement options on a replace before reading anything', async () => {
    await expect(
      client.documentReplace({
        document, identityKey, signer, actionFeeAgreement: notAnInstance as never,
      }),
    ).to.be.rejectedWith(NOT_AN_INSTANCE);
  });

  it('should refuse agreement options on a delete before reading anything', async () => {
    await expect(
      client.documentDelete({
        document, identityKey, signer, actionFeeAgreement: notAnInstance as never,
      }),
    ).to.be.rejectedWith(NOT_AN_INSTANCE);
  });

  it('should refuse agreement options on a transfer before reading anything', async () => {
    await expect(
      client.documentTransfer({
        document,
        recipientId: new Uint8Array(32).fill(5) as never,
        identityKey,
        signer,
        actionFeeAgreement: notAnInstance as never,
      }),
    ).to.be.rejectedWith(NOT_AN_INSTANCE);
  });

  it('should refuse agreement options on a purchase before reading anything', async () => {
    await expect(
      client.documentPurchase({
        document,
        buyerId: new Uint8Array(32).fill(5) as never,
        price: BigInt(10),
        identityKey,
        signer,
        actionFeeAgreement: notAnInstance as never,
      }),
    ).to.be.rejectedWith(NOT_AN_INSTANCE);
  });

  it('should refuse agreement options on a price update before reading anything', async () => {
    await expect(
      client.documentSetPrice({
        document,
        price: BigInt(10),
        identityKey,
        signer,
        actionFeeAgreement: notAnInstance as never,
      }),
    ).to.be.rejectedWith(NOT_AN_INSTANCE);
  });

  it('should refuse another wasm class', async () => {
    await expect(
      client.documentReplace({
        document, identityKey, signer, actionFeeAgreement: identityKey as never,
      }),
    ).to.be.rejectedWith(/Expected DocumentActionFeeAgreement, provided IdentityPublicKey/);
  });

  it('should refuse a primitive value', async () => {
    await expect(
      client.documentReplace({
        document, identityKey, signer, actionFeeAgreement: 80000000 as never,
      }),
    ).to.be.rejectedWith(/Value supplied as DocumentActionFeeAgreement is not an object/);
  });

  it('should read a DocumentActionFeeAgreement', async () => {
    // `tokenPaymentInfo` is parsed after the agreement, so reaching its error means the
    // agreement was accepted
    await expect(
      client.documentReplace({
        document,
        identityKey,
        signer,
        actionFeeAgreement: new sdk.DocumentActionFeeAgreement({
          owner: BigInt(80000000),
          moderators: BigInt(16000000),
          feeMultiplier: { knownPermille: BigInt(1000), increaseTolerancePercent: 20 },
        }),
        tokenPaymentInfo: { tokenContractPosition: 'first' } as never,
      }),
    ).to.be.rejectedWith(/invalid type: string "first"/);
  });
});
