/**
 * `WasmSdk.encryptDocumentProperty`, `decryptDocumentProperty` and
 * `encryptedPropertyEnvelope`: the `encryptedFor` helpers, keyed off the
 * contract's declaration, run locally with no connection.
 */
import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

const ownerId = '11111111111111111111111111111111';

const identifier = {
  type: 'array',
  byteArray: true,
  minItems: 32,
  maxItems: 32,
  contentMediaType: 'application/x.dash.dpp.identifier',
};

const keyId = { type: 'integer', minimum: 0, maximum: 4294967295 };

/** A `secret` whose message is encrypted to `recipientId`. */
const schemas = {
  secret: {
    type: 'object',
    properties: {
      recipientId: { ...identifier, position: 0 },
      recipientKeyId: { ...keyId, position: 1 },
      senderKeyId: { ...keyId, position: 2 },
      encryptedMessage: {
        type: 'array',
        byteArray: true,
        minItems: 32,
        maxItems: 1040,
        position: 3,
        encryptedFor: {
          recipient: 'recipientId',
          recipientKey: 'recipientKeyId',
          senderKey: 'senderKeyId',
          scheme: 'ecdh-secp256k1-aes256-cbc',
        },
      },
    },
    required: ['recipientId', 'recipientKeyId', 'senderKeyId', 'encryptedMessage'],
    additionalProperties: false,
  },
};

const ENCRYPTION = 1;
const DECRYPTION = 2;
const MEDIUM = 3;
const ECDSA_SECP256K1 = 0;

const leaderId = new Uint8Array(32).fill(7);

describe('encryptedFor helpers', () => {
  let contract: sdk.DataContract;
  let senderPrivateKey: sdk.PrivateKey;
  let recipientPrivateKey: sdk.PrivateKey;
  let senderKey: sdk.IdentityPublicKey;
  let recipientKey: sdk.IdentityPublicKey;

  function identityKey(id: number, purpose: number, privateKey: sdk.PrivateKey) {
    return new sdk.IdentityPublicKey({
      keyId: id,
      purpose,
      securityLevel: MEDIUM,
      keyType: ECDSA_SECP256K1,
      isReadOnly: false,
      data: privateKey.getPublicKey().toBytes(),
    });
  }

  function encrypt(plaintext: Uint8Array | string) {
    return sdk.WasmSdk.encryptDocumentProperty({
      dataContract: contract,
      documentTypeName: 'secret',
      property: 'encryptedMessage',
      plaintext,
      senderKey,
      senderPrivateKey,
      recipientKey,
    }) as Record<string, unknown>;
  }

  function documentWith(fields: Record<string, unknown>) {
    return new sdk.Document({
      properties: { recipientId: leaderId, ...fields },
      documentTypeName: 'secret',
      dataContractId: contract.id,
      ownerId,
    });
  }

  before(async () => {
    await init();
    contract = new sdk.DataContract({
      ownerId,
      identityNonce: BigInt(2),
      schemas,
      definitions: null,
      fullValidation: true,
      platformVersion: new sdk.PlatformVersion(14),
    });
    senderPrivateKey = sdk.PrivateKey.fromHex('21'.repeat(32), 'testnet');
    recipientPrivateKey = sdk.PrivateKey.fromHex('42'.repeat(32), 'testnet');
    senderKey = identityKey(4, ENCRYPTION, senderPrivateKey);
    recipientKey = identityKey(2, DECRYPTION, recipientPrivateKey);
  });

  it('should decrypt what it encrypts and fill the key id properties', () => {
    const message = 'I would like to help moderate';
    const fields = encrypt(message);

    expect(fields.recipientKeyId).to.equal(2);
    expect(fields.senderKeyId).to.equal(4);
    expect(fields.encryptedMessage).to.be.instanceOf(Uint8Array);

    const decrypted = sdk.WasmSdk.decryptDocumentProperty({
      dataContract: contract,
      document: documentWith(fields),
      property: 'encryptedMessage',
      recipientPrivateKey,
      senderKey,
    });
    expect(new TextDecoder().decode(decrypted)).to.equal(message);

    // ECDH is symmetric: the writer reads its own message back
    const bySender = sdk.WasmSdk.decryptDocumentProperty({
      dataContract: contract,
      document: documentWith(fields),
      property: 'encryptedMessage',
      recipientPrivateKey: senderPrivateKey,
      senderKey: recipientKey,
    });
    expect(new TextDecoder().decode(bySender)).to.equal(message);
  });

  it('should write an IV plus whole blocks for every plaintext length', () => {
    [0, 1, 15, 16, 17, 500, 1023].forEach((length) => {
      const fields = encrypt(new Uint8Array(length).fill(0x61));
      const ciphertext = fields.encryptedMessage as Uint8Array;
      expect(ciphertext.length, `${length} bytes`).to.equal(16 + (Math.floor(length / 16) + 1) * 16);
    });
  });

  it('should read whose keys the property is under', () => {
    const envelope = sdk.WasmSdk.encryptedPropertyEnvelope({
      dataContract: contract,
      document: documentWith(encrypt('hello')),
      property: 'encryptedMessage',
    });

    expect(Array.from(envelope.recipientId.toBytes())).to.deep.equal(Array.from(leaderId));
    expect(envelope.recipientKeyId).to.equal(2);
    // The schema declares no key reference, so the sender is the writer
    expect(envelope.senderId.toBase58()).to.equal(ownerId);
    expect(envelope.senderKeyId).to.equal(4);
  });

  it('should not recover the message with a wrong key', () => {
    const message = 'only the leader reads this';
    const document = documentWith(encrypt(message));
    const someoneElse = sdk.PrivateKey.fromHex('43'.repeat(32), 'testnet');

    // The padding check catches a wrong key but about once in 256 tries, and then the
    // bytes are garbage: either way the message does not come back.
    let recovered: string | undefined;
    try {
      recovered = new TextDecoder().decode(sdk.WasmSdk.decryptDocumentProperty({
        dataContract: contract,
        document,
        property: 'encryptedMessage',
        recipientPrivateKey: someoneElse,
        senderKey,
      }));
    } catch (e) {
      expect((e as Error).message).to.match(/decryption failed/);
      expect((e as sdk.WasmSdkError).kind).to.equal(sdk.WasmSdkErrorKind.DecryptionFailed);
    }
    expect(recovered).to.not.equal(message);
  });

  it('should refuse a private key that is not the sender key', () => {
    expect(() => sdk.WasmSdk.encryptDocumentProperty({
      dataContract: contract,
      documentTypeName: 'secret',
      property: 'encryptedMessage',
      plaintext: 'x',
      senderKey,
      senderPrivateKey: recipientPrivateKey,
      recipientKey,
    })).to.throw(/senderPrivateKey is not the private half of senderKey/);
  });

  it('should refuse a property that declares no encryptedFor', () => {
    expect(() => sdk.WasmSdk.encryptDocumentProperty({
      dataContract: contract,
      documentTypeName: 'secret',
      property: 'recipientId',
      plaintext: 'x',
      senderKey,
      senderPrivateKey,
      recipientKey,
    })).to.throw(/declares no encryptedFor/);
  });
});
