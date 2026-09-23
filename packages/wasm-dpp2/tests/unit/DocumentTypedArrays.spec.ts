/**
 * Verifies the typed array metadata surface introduced with protocol
 * version 14.
 *
 * A typed array is `type: 'array'` with an `items` schema naming what every
 * element is, where a byte array declares `byteArray: true` instead. It is
 * stored inline in the document and validated against the JSON schema like
 * any other property. What the JS layer offers is *discovery*: which
 * properties of a document type are lists, and of what.
 */
import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

let PlatformVersion: typeof wasm.PlatformVersion;

before(async () => {
  await initWasm();
  ({ PlatformVersion } = wasm);
});

const ownerId = '11111111111111111111111111111111';

/**
 * A `charter` with typed arrays of identifiers, strings and byte arrays
 * (one nested in an object), next to a `plain` type declaring none.
 */
const schemas = {
  charter: {
    type: 'object',
    properties: {
      reasons: {
        type: 'array',
        minItems: 0,
        maxItems: 64,
        uniqueItems: true,
        items: {
          type: 'array',
          byteArray: true,
          minItems: 32,
          maxItems: 32,
          contentMediaType: 'application/x.dash.dpp.identifier',
        },
        position: 0,
      },
      labels: {
        type: 'array',
        maxItems: 5,
        items: {
          type: 'string', minLength: 1, maxLength: 20, enum: ['spam', 'abuse', 'offTopic'],
        },
        position: 1,
      },
      scores: {
        type: 'array',
        maxItems: 4,
        items: { type: 'integer', minimum: 0, maximum: 100 },
        position: 3,
      },
      team: {
        type: 'object',
        position: 2,
        properties: {
          digests: {
            type: 'array',
            maxItems: 3,
            items: {
              type: 'array', byteArray: true, minItems: 4, maxItems: 8,
            },
            position: 0,
          },
        },
        additionalProperties: false,
      },
    },
    required: ['reasons'],
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

function buildContract(contractSchemas: Record<string, unknown>, platformVersion = 14) {
  return new wasm.DataContract({
    ownerId,
    identityNonce: BigInt(2),
    schemas: contractSchemas,
    definitions: null,
    fullValidation: true,
    platformVersion: new PlatformVersion(platformVersion),
  });
}

describe('DataContract: typed arrays (v14)', () => {
  describe('documentTypeTypedArrays()', () => {
    it('should report every typed array with its element type and bounds', () => {
      const contract = buildContract(schemas);

      expect(contract.documentTypeTypedArrays('charter')).to.deep.equal([
        {
          path: 'reasons',
          items: { type: 'identifier' },
          minItems: 0,
          maxItems: 64,
          uniqueItems: true,
        },
        {
          path: 'labels',
          items: {
            type: 'string', minLength: 1, maxLength: 20, enum: ['spam', 'abuse', 'offTopic'],
          },
          maxItems: 5,
          uniqueItems: false,
        },
        {
          path: 'team.digests',
          items: { type: 'byteArray', minItems: 4, maxItems: 8 },
          maxItems: 3,
          uniqueItems: false,
        },
        {
          path: 'scores',
          items: { type: 'integer', minimum: 0, maximum: 100 },
          maxItems: 4,
          uniqueItems: false,
        },
      ]);
    });

    it('should return an empty array for a document type declaring none', () => {
      const contract = buildContract(schemas);

      expect(contract.documentTypeTypedArrays('plain')).to.deep.equal([]);
    });

    it('should throw for an unknown document type', () => {
      const contract = buildContract(schemas);

      expect(() => contract.documentTypeTypedArrays('doesNotExist')).to.throw(/not found/);
    });

    /**
     * The meta-schemas before protocol version 14 only have byte arrays.
     */
    it('should refuse a typed array on a pre-v14 contract', () => {
      expect(() => buildContract(schemas, 13)).to.throw();
    });
  });

  /**
   * An identifier element may carry `refersTo`, which every element then
   * declares and consensus checks element by element.
   */
  describe('element references', () => {
    const referencingSchemas = {
      reason: {
        type: 'object',
        canBeDeleted: false,
        properties: {
          topic: { type: 'string', position: 0, maxLength: 63 },
        },
        additionalProperties: false,
      },
      submittedCharter: {
        type: 'object',
        properties: {
          reasons: {
            type: 'array',
            minItems: 0,
            maxItems: 64,
            uniqueItems: true,
            items: {
              type: 'array',
              byteArray: true,
              minItems: 32,
              maxItems: 32,
              contentMediaType: 'application/x.dash.dpp.identifier',
              refersTo: {
                type: 'permanentDocument',
                documentType: 'reason',
                propertyAgreement: { topic: 'topic' },
              },
            },
            position: 0,
          },
          topic: { type: 'string', position: 1, maxLength: 63 },
        },
        additionalProperties: false,
      },
    };

    type ElementReference = {
      type: string;
      contractId: { toBase58(): string };
      documentType: string;
      propertyAgreement?: Record<string, string>;
    };

    it('should report the element reference on the typed array items', () => {
      const contract = buildContract(referencingSchemas);
      const [reasons] = contract.documentTypeTypedArrays('submittedCharter') as {
        path: string;
        items: { type: string; refersTo: ElementReference };
      }[];

      expect(reasons.path).to.equal('reasons');
      expect(reasons.items.type).to.equal('identifier');
      expect(reasons.items.refersTo.type).to.equal('permanentDocument');
      expect(reasons.items.refersTo.contractId.toBase58()).to.equal(contract.id.toBase58());
      expect(reasons.items.refersTo.documentType).to.equal('reason');
      expect(reasons.items.refersTo.propertyAgreement).to.deep.equal({ topic: 'topic' });
    });

    it('should list the element reference among the references at its list path', () => {
      const contract = buildContract(referencingSchemas);
      const references = contract.documentTypeReferences('submittedCharter') as {
        path: string;
        type: string;
      }[];

      expect(references.map((reference) => [reference.path, reference.type])).to.deep.equal([
        ['reasons[]', 'permanentDocument'],
      ]);
    });

    it('should refuse an identityPublicKey reference on the elements', () => {
      const schemasWithKeyReference = {
        submittedCharter: {
          type: 'object',
          properties: {
            reasons: {
              type: 'array',
              maxItems: 4,
              items: {
                type: 'array',
                byteArray: true,
                minItems: 32,
                maxItems: 32,
                contentMediaType: 'application/x.dash.dpp.identifier',
                refersTo: { type: 'identityPublicKey', keyIdProperty: 'keyId' },
              },
              position: 0,
            },
            keyId: {
              type: 'integer', minimum: 0, maximum: 4294967295, position: 1,
            },
          },
          additionalProperties: false,
        },
      };

      expect(() => buildContract(schemasWithKeyReference)).to.throw(
        /identityPublicKey refersTo is not allowed on the elements of a typed array/,
      );
    });
  });

  describe('documentTypedArrays', () => {
    it('should key typed arrays by document type and omit types declaring none', () => {
      const contract = buildContract(schemas);
      const map = contract.documentTypedArrays as Map<string, unknown[]>;

      expect([...map.keys()]).to.deep.equal(['charter']);
      expect(map.get('charter')).to.deep.equal(contract.documentTypeTypedArrays('charter'));
    });

    it('should be empty for a contract declaring no typed arrays at all', () => {
      const contract = buildContract({ plain: schemas.plain });

      expect((contract.documentTypedArrays as Map<string, unknown[]>).size).to.equal(0);
    });
  });
});
