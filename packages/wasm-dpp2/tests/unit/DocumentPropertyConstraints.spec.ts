/**
 * Verifies the `propertyConstraints` surface introduced with protocol version
 * 14.
 *
 * A document type names rules its documents' properties must meet: integer
 * comparisons and `in`, string and identifier comparisons (with `$ownerId`),
 * `present` / `absent`, and `anyOf` / `allOf` / `not` over them. Consensus
 * refuses a broken rule with code 10422. What the JS layer offers is
 * discovery (which rules a type declares and what they read) and a pre-check
 * that evaluates a document with the code consensus runs.
 */
import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

let PlatformVersion: typeof wasm.PlatformVersion;

before(async () => {
  await initWasm();
  ({ PlatformVersion } = wasm);
});

const ownerId = 'CXH2kZCATjvDTnQAPVg28EgPg9WySUvwvnR5ZkmNqY5i';
const otherId = '9tSsCqKHTZ8ro16MydChSxgHBukFW36eMLJKKRtebJEn';

const identifierProperty = (position: number) => ({
  type: 'array',
  byteArray: true,
  minItems: 32,
  maxItems: 32,
  contentMediaType: 'application/x.dash.dpp.identifier',
  position,
});

/**
 * An `offer` declaring one rule of each family, next to a `plain` type
 * declaring none.
 */
const schemas = {
  offer: {
    type: 'object',
    properties: {
      price: { type: 'integer', minimum: 0, position: 0 },
      fee: { type: 'integer', minimum: 0, position: 1 },
      discount: { type: 'integer', minimum: 0, position: 2 },
      status: {
        type: 'string', enum: ['open', 'closed'], maxLength: 10, position: 3,
      },
      closedAt: { type: 'integer', minimum: 0, position: 4 },
      sellerId: identifierProperty(5),
    },
    required: ['price', 'fee'],
    additionalProperties: false,
    propertyConstraints: {
      closedNeedsClosedAt: {
        anyOf: [{ notEqual: ['status', { const: 'closed' }] }, { present: 'closedAt' }],
      },
      discountBelowPrice: { lessThan: ['discount', 'price'] },
      perUnitFee: { greaterThanOrEqual: [{ divide: ['price', 'fee'] }, 1] },
      sellerIsOwner: {
        anyOf: [{ absent: 'sellerId' }, { equal: ['sellerId', '$ownerId'] }],
      },
      tieredFee: { in: ['fee', [1, 10, 25]] },
    },
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

function offer(
  contract: InstanceType<typeof wasm.DataContract>,
  properties: Record<string, unknown>,
  owner = ownerId,
) {
  return new wasm.Document({
    properties: { price: 100, fee: 10, ...properties },
    documentTypeName: 'offer',
    dataContractId: contract.id,
    ownerId: owner,
    revision: BigInt(1),
  });
}

describe('DataContract: propertyConstraints (v14)', () => {
  describe('documentTypePropertyConstraints()', () => {
    it('should list every rule in name order with what it reads', () => {
      const contract = buildContract(schemas);

      expect(contract.documentTypePropertyConstraints('offer')).to.deep.equal([
        {
          name: 'closedNeedsClosedAt',
          rule: schemas.offer.propertyConstraints.closedNeedsClosedAt,
          reads: [
            { path: 'status', kind: 'text' },
            { path: 'closedAt', kind: 'presence' },
          ],
          readsOwner: false,
          readsSystem: [],
        },
        {
          name: 'discountBelowPrice',
          rule: schemas.offer.propertyConstraints.discountBelowPrice,
          reads: [
            { path: 'discount', kind: 'value' },
            { path: 'price', kind: 'value' },
          ],
          readsOwner: false,
          readsSystem: [],
        },
        {
          name: 'perUnitFee',
          rule: schemas.offer.propertyConstraints.perUnitFee,
          reads: [
            { path: 'price', kind: 'value' },
            { path: 'fee', kind: 'value' },
          ],
          readsOwner: false,
          readsSystem: [],
        },
        {
          name: 'sellerIsOwner',
          rule: schemas.offer.propertyConstraints.sellerIsOwner,
          reads: [
            { path: 'sellerId', kind: 'presence' },
            { path: 'sellerId', kind: 'identifier' },
          ],
          readsOwner: true,
          readsSystem: [],
        },
        {
          name: 'tieredFee',
          rule: schemas.offer.propertyConstraints.tieredFee,
          reads: [{ path: 'fee', kind: 'value' }],
          readsOwner: false,
          readsSystem: [],
        },
      ]);
    });

    it('should return an empty array for a document type declaring none', () => {
      const contract = buildContract(schemas);

      expect(contract.documentTypePropertyConstraints('plain')).to.deep.equal([]);
    });

    it('should throw for an unknown document type', () => {
      const contract = buildContract(schemas);

      expect(() => contract.documentTypePropertyConstraints('doesNotExist')).to.throw(/not found/);
    });

    it('should key the types declaring rules in documentPropertyConstraints', () => {
      const contract = buildContract(schemas);
      const byType = contract.documentPropertyConstraints;

      expect([...byType.keys()]).to.deep.equal(['offer']);
      expect(byType.get('offer')).to.have.length(5);
    });

    it('should report sizes as length and count reads, and check them', () => {
      const rules = {
        titleBytes: { lessThanOrEqual: [{ byteLength: 'title' }, 12] },
        tagsWithinLimit: { lessThanOrEqual: [{ count: 'tags' }, 'maxTags'] },
      };
      const contract = buildContract({
        listing: {
          type: 'object',
          properties: {
            title: { type: 'string', maxLength: 40, position: 0 },
            tags: {
              type: 'array',
              maxItems: 8,
              items: { type: 'string', maxLength: 16 },
              position: 1,
            },
            maxTags: { type: 'integer', minimum: 0, maximum: 8, position: 2 },
          },
          additionalProperties: false,
          propertyConstraints: rules,
        },
      });

      expect(contract.documentTypePropertyConstraints('listing')).to.deep.equal([
        {
          name: 'tagsWithinLimit',
          rule: rules.tagsWithinLimit,
          reads: [{ path: 'tags', kind: 'count' }, { path: 'maxTags', kind: 'value' }],
          readsOwner: false,
          readsSystem: [],
        },
        {
          name: 'titleBytes',
          rule: rules.titleBytes,
          reads: [{ path: 'title', kind: 'length' }],
          readsOwner: false,
          readsSystem: [],
        },
      ]);

      const listing = (properties: Record<string, unknown>) => new wasm.Document({
        properties,
        documentTypeName: 'listing',
        dataContractId: contract.id,
        ownerId,
        revision: BigInt(1),
      });
      expect(contract.checkDocumentPropertyConstraints(
        listing({ title: 'Café', tags: ['a', 'b'], maxTags: 2 }),
      )).to.equal(undefined);
      expect(contract.checkDocumentPropertyConstraints(
        listing({ title: 'Café', tags: ['a', 'b', 'c'], maxTags: 2 }),
      )).to.deep.include({ rule: 'tagsWithinLimit', violation: 'NotMet' });
      // 8 characters, 16 bytes
      expect(contract.checkDocumentPropertyConstraints(
        listing({ title: 'éééééééé' }),
      )).to.deep.include({ rule: 'titleBytes', violation: 'NotMet' });
    });

    it('should list system times it reads, and check them with the device clock', () => {
      const rules = {
        endsAfterCreation: { greaterThan: ['endsAt', '$createdAt'] },
        listedAfterHeight10: { greaterThanOrEqual: ['$createdAtBlockHeight', 10] },
      };
      const contract = buildContract({
        listing: {
          type: 'object',
          properties: {
            endsAt: { type: 'integer', minimum: 0, position: 0 },
          },
          required: ['endsAt', '$createdAt', '$createdAtBlockHeight'],
          additionalProperties: false,
          propertyConstraints: rules,
        },
      });

      expect(contract.documentTypePropertyConstraints('listing')).to.deep.equal([
        {
          name: 'endsAfterCreation',
          rule: rules.endsAfterCreation,
          reads: [{ path: 'endsAt', kind: 'value' }],
          readsOwner: false,
          readsSystem: ['$createdAt'],
        },
        {
          name: 'listedAfterHeight10',
          rule: rules.listedAfterHeight10,
          reads: [],
          readsOwner: false,
          readsSystem: ['$createdAtBlockHeight'],
        },
      ]);

      const listing = (endsAt: number) => new wasm.Document({
        properties: { endsAt },
        documentTypeName: 'listing',
        dataContractId: contract.id,
        ownerId,
        revision: BigInt(1),
      });
      // Ended in 1970: before a create today, by the device clock
      expect(contract.checkDocumentPropertyConstraints(listing(1)))
        .to.deep.include({ rule: 'endsAfterCreation', violation: 'NotMet' });
      // Ends in 2100; the block height a create records is unknown before its
      // block, so the height rule is not judged
      expect(contract.checkDocumentPropertyConstraints(listing(4102444800000)))
        .to.equal(undefined);
    });

    it('should report the array a contains looks in, and check it', () => {
      const rules = {
        notUsed: { not: { contains: ['labels', { const: 'used' }] } },
      };
      const contract = buildContract({
        listing: {
          type: 'object',
          properties: {
            labels: {
              type: 'array',
              maxItems: 4,
              items: { type: 'string', maxLength: 10, enum: ['new', 'used'] },
              position: 0,
            },
          },
          additionalProperties: false,
          propertyConstraints: rules,
        },
      });

      expect(contract.documentTypePropertyConstraints('listing')).to.deep.equal([
        {
          name: 'notUsed',
          rule: rules.notUsed,
          reads: [{ path: 'labels', kind: 'elements' }],
          readsOwner: false,
          readsSystem: [],
        },
      ]);

      const listing = (labels: string[]) => new wasm.Document({
        properties: { labels },
        documentTypeName: 'listing',
        dataContractId: contract.id,
        ownerId,
        revision: BigInt(1),
      });
      expect(contract.checkDocumentPropertyConstraints(listing(['new']))).to.equal(undefined);
      expect(contract.checkDocumentPropertyConstraints(listing(['new', 'used'])))
        .to.deep.include({ rule: 'notUsed', violation: 'NotMet' });
    });

    it('should check startsWith and endsWith byte for byte', () => {
      const rules = {
        secureUrl: { startsWith: ['url', { const: 'https://' }] },
        dashDomain: { endsWith: ['url', { const: '.dash' }] },
      };
      const contract = buildContract({
        link: {
          type: 'object',
          properties: {
            url: { type: 'string', maxLength: 100, position: 0 },
          },
          required: ['url'],
          additionalProperties: false,
          propertyConstraints: rules,
        },
      });

      expect(contract.documentTypePropertyConstraints('link').map((rule) => rule.reads))
        .to.deep.equal([[{ path: 'url', kind: 'text' }], [{ path: 'url', kind: 'text' }]]);

      const link = (url: string) => new wasm.Document({
        properties: { url },
        documentTypeName: 'link',
        dataContractId: contract.id,
        ownerId,
        revision: BigInt(1),
      });
      expect(contract.checkDocumentPropertyConstraints(link('https://pay.dash'))).to.equal(undefined);
      expect(contract.checkDocumentPropertyConstraints(link('https://pay.com')))
        .to.deep.include({ rule: 'dashDomain', violation: 'NotMet' });
      // No case folding
      expect(contract.checkDocumentPropertyConstraints(link('HTTPS://pay.dash')))
        .to.deep.include({ rule: 'secureUrl', violation: 'NotMet' });
    });

    it('should report integer literals past Number.MAX_SAFE_INTEGER exactly, as bigint', () => {
      const big = 9007199254740993n; // 2 ** 53 + 1, which a number rounds
      const rules = {
        balanceBelowCap: { lessThan: [{ ifAbsent: ['balance', -big] }, big] },
        knownTier: { in: ['tier', [1, big]] },
      };
      const contract = buildContract({
        ledger: {
          type: 'object',
          properties: {
            balance: { type: 'integer', position: 0 },
            tier: { type: 'integer', position: 1 },
          },
          additionalProperties: false,
          propertyConstraints: rules,
        },
      });
      const expected = [
        {
          name: 'balanceBelowCap',
          rule: rules.balanceBelowCap,
          reads: [{ path: 'balance', kind: 'value' }],
          readsOwner: false,
          readsSystem: [],
        },
        {
          name: 'knownTier',
          rule: rules.knownTier,
          reads: [{ path: 'tier', kind: 'value' }],
          readsOwner: false,
          readsSystem: [],
        },
      ];

      expect(contract.documentTypePropertyConstraints('ledger')).to.deep.equal(expected);
      expect(contract.documentPropertyConstraints.get('ledger')).to.deep.equal(expected);
    });

    /**
     * Parsers before protocol version 14 ignore the keyword, so a contract
     * read at such a version reports no rules: exactly what consensus
     * enforced there.
     */
    it('should report no rules on a pre-v14 contract', () => {
      const contract = buildContract(schemas, 13);

      expect(contract.documentTypePropertyConstraints('offer')).to.deep.equal([]);
      expect(contract.checkDocumentPropertyConstraints(offer(contract, { discount: 200 })))
        .to.equal(undefined);
    });
  });

  describe('checkDocumentPropertyConstraints()', () => {
    it('should report nothing for a document meeting every rule', () => {
      const contract = buildContract(schemas);

      expect(contract.checkDocumentPropertyConstraints(offer(contract, {}))).to.equal(undefined);
      expect(contract.checkDocumentPropertyConstraints(offer(contract, {
        status: 'closed', closedAt: 1000, sellerId: ownerId, discount: 5,
      }))).to.equal(undefined);
    });

    it('should report the first rule broken, in name order, as consensus would', () => {
      const contract = buildContract(schemas);
      const violationOf = (properties: Record<string, unknown>, owner = ownerId) => (
        contract.checkDocumentPropertyConstraints(offer(contract, properties, owner))
      );

      expect(violationOf({ status: 'closed' })).to.deep.include({
        rule: 'closedNeedsClosedAt', violation: 'NotMet',
      });
      expect(violationOf({ discount: 200 })).to.deep.include({
        rule: 'discountBelowPrice', violation: 'NotMet',
      });
      // A fee of 0 divides by zero before the tier rule is reached
      expect(violationOf({ fee: 0 })).to.deep.include({
        rule: 'perUnitFee', violation: 'DivisionByZero',
      });
      expect(violationOf({ fee: 5 })).to.deep.include({
        rule: 'tieredFee', violation: 'NotMet',
      });
      expect(violationOf({ fee: 5 }).message).to.be.a('string');
    });

    it('should read the document owner for $ownerId', () => {
      const contract = buildContract(schemas);

      expect(contract.checkDocumentPropertyConstraints(offer(contract, { sellerId: otherId })))
        .to.deep.include({ rule: 'sellerIsOwner', violation: 'NotMet' });
      expect(contract.checkDocumentPropertyConstraints(
        offer(contract, { sellerId: otherId }, otherId),
      )).to.equal(undefined);
    });

    it('should report nothing for a document type declaring no rules', () => {
      const contract = buildContract(schemas);
      const document = new wasm.Document({
        properties: { message: 'hi' },
        documentTypeName: 'plain',
        dataContractId: contract.id,
        ownerId,
        revision: BigInt(1),
      });

      expect(contract.checkDocumentPropertyConstraints(document)).to.equal(undefined);
    });

    it('should throw for a document of another contract or an unknown type', () => {
      const contract = buildContract(schemas);
      const foreign = new wasm.Document({
        properties: { price: 100, fee: 10 },
        documentTypeName: 'offer',
        dataContractId: otherId,
        ownerId,
        revision: BigInt(1),
      });
      const unknownType = new wasm.Document({
        properties: {},
        documentTypeName: 'doesNotExist',
        dataContractId: contract.id,
        ownerId,
        revision: BigInt(1),
      });

      expect(() => contract.checkDocumentPropertyConstraints(foreign)).to.throw(/another contract|not this one/);
      expect(() => contract.checkDocumentPropertyConstraints(unknownType)).to.throw(/not found/);
    });
  });
});
