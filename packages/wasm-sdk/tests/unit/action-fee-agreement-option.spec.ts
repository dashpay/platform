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

  const malformed = { owner: 'eighty million' };
  const MALFORMED = /invalid type: string "eighty million", expected u64/;

  it('should refuse a malformed agreement on a create before reading anything', async () => {
    await expect(
      client.documentCreate({
        document, identityKey, signer, actionFeeAgreement: malformed as never,
      }),
    ).to.be.rejectedWith(MALFORMED);
  });

  it('should refuse a malformed agreement on a replace before reading anything', async () => {
    await expect(
      client.documentReplace({
        document, identityKey, signer, actionFeeAgreement: malformed as never,
      }),
    ).to.be.rejectedWith(MALFORMED);
  });

  it('should refuse a malformed agreement on a delete before reading anything', async () => {
    await expect(
      client.documentDelete({
        document, identityKey, signer, actionFeeAgreement: malformed as never,
      }),
    ).to.be.rejectedWith(MALFORMED);
  });

  it('should refuse a malformed agreement on a transfer before reading anything', async () => {
    await expect(
      client.documentTransfer({
        document,
        recipientId: new Uint8Array(32).fill(5) as never,
        identityKey,
        signer,
        actionFeeAgreement: malformed as never,
      }),
    ).to.be.rejectedWith(MALFORMED);
  });

  it('should refuse a malformed agreement on a purchase before reading anything', async () => {
    await expect(
      client.documentPurchase({
        document,
        buyerId: new Uint8Array(32).fill(5) as never,
        price: BigInt(10),
        identityKey,
        signer,
        actionFeeAgreement: malformed as never,
      }),
    ).to.be.rejectedWith(MALFORMED);
  });

  it('should refuse a malformed agreement on a price update before reading anything', async () => {
    await expect(
      client.documentSetPrice({
        document,
        price: BigInt(10),
        identityKey,
        signer,
        actionFeeAgreement: malformed as never,
      }),
    ).to.be.rejectedWith(MALFORMED);
  });

  it('should refuse another wasm class, which would read as an agreement to pay nothing', async () => {
    await expect(
      client.documentReplace({
        document, identityKey, signer, actionFeeAgreement: identityKey as never,
      }),
    ).to.be.rejectedWith(/must be a DocumentActionFeeAgreement or its options, not an instance of IdentityPublicKey/);
  });

  it('should refuse an agreement naming an unknown key before reading anything', async () => {
    await expect(
      client.documentCreate({
        document, identityKey, signer, actionFeeAgreement: { ownr: BigInt(80000000) } as never,
      }),
    ).to.be.rejectedWith(/unknown DocumentActionFeeAgreement option "ownr"/);
  });

  it('should refuse a primitive value', async () => {
    await expect(
      client.documentReplace({
        document, identityKey, signer, actionFeeAgreement: 80000000 as never,
      }),
    ).to.be.rejectedWith(/must be a DocumentActionFeeAgreement or its options, not a primitive value/);
  });

  it('should read a well-formed agreement, as an instance or as options', async () => {
    // `tokenPaymentInfo` is parsed after the agreement, so reaching its error means the
    // agreement was accepted
    const agreements = [
      new sdk.DocumentActionFeeAgreement({ owner: BigInt(80000000), moderators: BigInt(16000000) }),
      {
        owner: BigInt(80000000),
        moderators: BigInt(16000000),
        feeMultiplier: { knownPermille: BigInt(1000), increaseTolerancePercent: 20 },
      },
    ];
    for (const actionFeeAgreement of agreements) {
      await expect(
        client.documentReplace({
          document,
          identityKey,
          signer,
          actionFeeAgreement,
          tokenPaymentInfo: { tokenContractPosition: 'first' } as never,
        }),
      ).to.be.rejectedWith(/invalid type: string "first"/);
    }
  });
});
