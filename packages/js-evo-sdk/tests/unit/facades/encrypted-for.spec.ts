import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import {
  DataContract,
  Document,
  EvoSDK,
  IdentityPublicKey,
  ensureInitialized,
  PlatformVersion,
  PrivateKey,
} from '../../../dist/sdk.js';

/**
 * The `encryptedFor` facade runs locally, so these run the real helpers. The objects come
 * from the SDK's own exports: the facade calls the bundle the SDK wraps.
 */
describe('EncryptedForFacade', () => {
  let client: EvoSDK;
  let contract: DataContract;
  let senderPrivateKey: PrivateKey;
  let recipientPrivateKey: PrivateKey;
  let senderKey: IdentityPublicKey;
  let recipientKey: IdentityPublicKey;

  const ownerId = '11111111111111111111111111111111';
  const identifier = {
    type: 'array',
    byteArray: true,
    minItems: 32,
    maxItems: 32,
    contentMediaType: 'application/x.dash.dpp.identifier',
  };
  const keyId = { type: 'integer', minimum: 0, maximum: 4294967295 };
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

  function identityKey(id: number, purpose: number, privateKey: PrivateKey): IdentityPublicKey {
    return new IdentityPublicKey({
      keyId: id,
      purpose,
      securityLevel: 3,
      keyType: 0,
      isReadOnly: false,
      data: privateKey.getPublicKey().toBytes(),
    });
  }

  before(async () => {
    await init();
    client = EvoSDK.fromWasm(await wasmSDKPackage.WasmSdkBuilder.testnet().build());
    // The bundle the SDK wraps, whose classes the facade takes
    await ensureInitialized();
    contract = new DataContract({
      ownerId,
      identityNonce: BigInt(2),
      schemas,
      definitions: null,
      fullValidation: true,
      platformVersion: new PlatformVersion(14),
    });
    senderPrivateKey = PrivateKey.fromHex('21'.repeat(32), 'testnet');
    recipientPrivateKey = PrivateKey.fromHex('42'.repeat(32), 'testnet');
    senderKey = identityKey(4, 1, senderPrivateKey);
    recipientKey = identityKey(2, 2, recipientPrivateKey);
  });

  it('should round trip a message through encrypt, envelope and decrypt', async () => {
    const fields = await client.encryptedFor.encrypt({
      dataContract: contract,
      documentTypeName: 'secret',
      property: 'encryptedMessage',
      plaintext: 'hello leader',
      senderKey,
      senderPrivateKey,
      recipientKey,
    });
    expect(fields.recipientKeyId).to.equal(2);
    expect(fields.senderKeyId).to.equal(4);

    const document = new Document({
      properties: { recipientId: new Uint8Array(32).fill(7), ...fields },
      documentTypeName: 'secret',
      dataContractId: contract.id,
      ownerId,
    });
    const envelope = await client.encryptedFor.envelope({
      dataContract: contract,
      document,
      property: 'encryptedMessage',
    });
    expect(envelope.recipientKeyId).to.equal(2);
    expect(envelope.senderKeyId).to.equal(4);

    const message = await client.encryptedFor.decrypt({
      dataContract: contract,
      document,
      property: 'encryptedMessage',
      recipientPrivateKey,
      senderKey,
    });
    expect(new TextDecoder().decode(message)).to.equal('hello leader');
  });
});
