/**
 * Verifies the `encryptedFor` metadata surface introduced with protocol
 * version 14.
 *
 * A byte array property may declare how its ciphertext was produced: for
 * which recipient, under which key ids and under which scheme. Consensus
 * checks only the shape of the bytes on every create and replace (code
 * 10420). What the JS layer offers is *discovery* (which properties of a
 * document type are encrypted, and with what) plus a branchable error code
 * for when a write is rejected.
 */
import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

let PlatformVersion: typeof wasm.PlatformVersion;

before(async () => {
  await initWasm();
  ({ PlatformVersion } = wasm);
});

const ownerId = '11111111111111111111111111111111';

const identifier = {
  type: 'array',
  byteArray: true,
  minItems: 32,
  maxItems: 32,
  contentMediaType: 'application/x.dash.dpp.identifier',
};

const keyId = { type: 'integer', minimum: 0, maximum: 4294967295 };

/**
 * A `secret` whose message is encrypted to a recipient, a `note` whose
 * body is encrypted to its own writer, next to a `plain` type declaring
 * nothing.
 */
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
  note: {
    type: 'object',
    properties: {
      keyId: { ...keyId, position: 0 },
      body: {
        type: 'array',
        byteArray: true,
        minItems: 32,
        maxItems: 4096,
        position: 1,
        encryptedFor: {
          recipient: '$ownerId',
          recipientKey: 'keyId',
          senderKey: 'keyId',
          scheme: 'ecdh-secp256k1-aes256-cbc',
        },
      },
    },
    additionalProperties: false,
  },
  plain: {
    type: 'object',
    properties: {
      message: { type: 'string', position: 0, maxLength: 64 },
    },
    additionalProperties: false,
  },
};

function buildContract(platformVersion: number, fullValidation = true) {
  return new wasm.DataContract({
    ownerId,
    identityNonce: BigInt(2),
    schemas,
    definitions: null,
    fullValidation,
    platformVersion: new PlatformVersion(platformVersion),
  });
}

type Encryption = {
  path: string;
  recipient: string;
  recipientKey: string;
  senderKey: string;
  scheme: string;
};

describe('DataContract: encrypted properties (v14)', () => {
  describe('documentTypeEncryptedProperties()', () => {
    it('should report the declaration with the schema keyword names', () => {
      const contract = buildContract(14);

      expect(contract.documentTypeEncryptedProperties('secret')).to.deep.equal([
        {
          path: 'encryptedMessage',
          recipient: 'recipientId',
          recipientKey: 'recipientKeyId',
          senderKey: 'senderKeyId',
          scheme: 'ecdh-secp256k1-aes256-cbc',
        },
      ]);
    });

    it('should spell an owner recipient as $ownerId', () => {
      const contract = buildContract(14);

      const [body] = contract.documentTypeEncryptedProperties('note') as Encryption[];
      expect(body.path).to.equal('body');
      expect(body.recipient).to.equal('$ownerId');
    });

    it('should return an empty array for a document type declaring none', () => {
      const contract = buildContract(14);

      expect(contract.documentTypeEncryptedProperties('plain')).to.deep.equal([]);
    });

    /**
     * An empty array would conflate "no such type" with "nothing encrypted",
     * which is a difference a caller acting on the result needs.
     */
    it('should throw for an unknown document type', () => {
      const contract = buildContract(14);

      expect(() => contract.documentTypeEncryptedProperties('doesNotExist')).to.throw(/not found/);
    });

    /**
     * The keyword is only parsed from protocol version 14 onward. A contract
     * deserialized against an earlier version reports nothing encrypted even
     * though its raw schema still carries it.
     */
    it('should report nothing on a pre-v14 contract, while the raw schema keeps the keyword', () => {
      const contract = buildContract(13, false);

      expect(contract.documentTypeEncryptedProperties('secret')).to.deep.equal([]);

      const rawSchemas = contract.schemas as Record<
        string,
        { properties: Record<string, { encryptedFor?: Record<string, string> }> }
      >;
      expect(rawSchemas.secret.properties.encryptedMessage.encryptedFor).to.deep.equal({
        recipient: 'recipientId',
        recipientKey: 'recipientKeyId',
        senderKey: 'senderKeyId',
        scheme: 'ecdh-secp256k1-aes256-cbc',
      });
    });

    /**
     * The declaration is consensus-validated at registration: a recipient
     * that is not an identifier property, a key path naming nothing, or an
     * unknown scheme is a contract error, not something the accessor has to
     * guard against.
     */
    it('should refuse a contract whose recipient key names no property', () => {
      const build = () => new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: {
          secret: {
            ...schemas.secret,
            properties: {
              ...schemas.secret.properties,
              encryptedMessage: {
                ...schemas.secret.properties.encryptedMessage,
                encryptedFor: {
                  ...schemas.secret.properties.encryptedMessage.encryptedFor,
                  recipientKey: 'nowhere',
                },
              },
            },
          },
        },
        definitions: null,
        fullValidation: true,
        platformVersion: new PlatformVersion(14),
      });

      expect(build).to.throw(/not a property of the document type/);
    });
  });

  describe('documentEncryptedProperties', () => {
    it('should key declarations by document type and omit types declaring none', () => {
      const contract = buildContract(14);
      const map = contract.documentEncryptedProperties as Map<string, Encryption[]>;

      expect([...map.keys()].sort()).to.deep.equal(['note', 'secret']);
      expect(map.get('secret')).to.deep.equal(contract.documentTypeEncryptedProperties('secret'));
    });

    it('should be empty for a contract encrypting nothing at all', () => {
      const contract = new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: { plain: schemas.plain },
        definitions: null,
        fullValidation: true,
        platformVersion: new PlatformVersion(14),
      });

      const map = contract.documentEncryptedProperties as Map<string, Encryption[]>;
      expect(map.size).to.equal(0);
    });
  });

  describe('DocumentEncryptionErrorCode', () => {
    it('should expose the shape error code both ways', () => {
      const { DocumentEncryptionErrorCode } = wasm;

      expect(DocumentEncryptionErrorCode.InvalidEncryptedPropertyShape).to.equal(10420);
      expect(DocumentEncryptionErrorCode[10420]).to.equal('InvalidEncryptedPropertyShape');
    });
  });
});
