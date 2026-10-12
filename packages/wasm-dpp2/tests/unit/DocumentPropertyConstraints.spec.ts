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

/**
 * This package is built with dpp's `validation` feature (for
 * `DataContract.validateUpdate`), so full validation runs the document
 * meta-schema first, as nodes do, and refuses a malformed keyword with
 * `JsonSchemaError` (10101) before the parser sees it. The parser's own
 * checks are reached with full validation off.
 */
const JSON_SCHEMA_ERROR = 10101;

function buildContract(contractSchemas: Record<string, unknown>, platformVersion = 14, fullValidation = true) {
  return new wasm.DataContract({
    ownerId,
    identityNonce: BigInt(2),
    schemas: contractSchemas,
    definitions: null,
    fullValidation,
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
  describe('rules reading a referenced document', () => {
    /**
     * A `vote` whose rules read the `poll` its `pollId` refers to: the
     * platform judges them with the poll its reference check read, and the
     * pre-check, which reads no state, leaves them unjudged.
     */
    const referencingSchemas = {
      poll: {
        type: 'object',
        documentsMutable: false,
        canBeDeleted: true,
        properties: {
          endsAt: { type: 'integer', minimum: 0, position: 0 },
          status: {
            type: 'string', enum: ['open', 'closed'], maxLength: 10, position: 1,
          },
        },
        required: ['$createdAt'],
        additionalProperties: false,
      },
      vote: {
        type: 'object',
        properties: {
          pollId: {
            ...identifierProperty(0),
            refersTo: { type: 'deletableDocument', documentType: 'poll' },
          },
          choice: { type: 'integer', minimum: 0, maximum: 9, position: 1 },
        },
        required: ['pollId', 'choice', '$createdAt'],
        additionalProperties: false,
        propertyConstraints: {
          castBeforeTheEnd: { lessThan: ['$createdAt', { referenced: ['pollId', 'endsAt'] }] },
          pollIsOpen: {
            equal: [{ referenced: ['pollId', 'status'] }, { const: 'open' }],
          },
        },
      },
    };

    it('should list what a rule reads of the referenced document apart from its own reads', () => {
      const contract = buildContract(referencingSchemas);

      expect(contract.documentTypePropertyConstraints('vote')).to.deep.equal([
        {
          name: 'castBeforeTheEnd',
          rule: referencingSchemas.vote.propertyConstraints.castBeforeTheEnd,
          reads: [],
          readsOwner: false,
          readsSystem: ['$createdAt'],
          readsStored: false,
          readsTotals: [],
          readsReferenced: [{ reference: 'pollId', path: 'endsAt', kind: 'value' }],
        },
        {
          name: 'pollIsOpen',
          rule: referencingSchemas.vote.propertyConstraints.pollIsOpen,
          reads: [],
          readsOwner: false,
          readsSystem: [],
          readsStored: false,
          readsTotals: [],
          readsReferenced: [{ reference: 'pollId', path: 'status', kind: 'text' }],
        },
      ]);
    });

    it('should leave a rule reading a referenced document unjudged', () => {
      const contract = buildContract(referencingSchemas);
      const vote = new wasm.Document({
        properties: { pollId: otherId, choice: 0 },
        documentTypeName: 'vote',
        dataContractId: contract.id,
        ownerId,
        revision: BigInt(1),
      });

      expect(contract.checkDocumentPropertyConstraints(vote, null)).to.equal(undefined);
    });
  });

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
          readsStored: false,
          readsTotals: [],
          readsReferenced: [],
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
          readsStored: false,
          readsTotals: [],
          readsReferenced: [],
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
          readsStored: false,
          readsTotals: [],
          readsReferenced: [],
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
          readsStored: false,
          readsTotals: [],
          readsReferenced: [],
        },
        {
          name: 'tieredFee',
          rule: schemas.offer.propertyConstraints.tieredFee,
          reads: [{ path: 'fee', kind: 'value' }],
          readsOwner: false,
          readsSystem: [],
          readsStored: false,
          readsTotals: [],
          readsReferenced: [],
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
          readsStored: false,
          readsTotals: [],
          readsReferenced: [],
        },
        {
          name: 'titleBytes',
          rule: rules.titleBytes,
          reads: [{ path: 'title', kind: 'length' }],
          readsOwner: false,
          readsSystem: [],
          readsStored: false,
          readsTotals: [],
          readsReferenced: [],
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
        null,
      )).to.equal(undefined);
      expect(contract.checkDocumentPropertyConstraints(
        listing({ title: 'Café', tags: ['a', 'b', 'c'], maxTags: 2 }),
        null,
      )).to.deep.include({ rule: 'tagsWithinLimit', violation: 'NotMet' });
      // 8 characters, 16 bytes
      expect(contract.checkDocumentPropertyConstraints(
        listing({ title: 'éééééééé' }),
        null,
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
          readsStored: false,
          readsTotals: [],
          readsReferenced: [],
        },
        {
          name: 'listedAfterHeight10',
          rule: rules.listedAfterHeight10,
          reads: [],
          readsOwner: false,
          readsSystem: ['$createdAtBlockHeight'],
          readsStored: false,
          readsTotals: [],
          readsReferenced: [],
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
      expect(contract.checkDocumentPropertyConstraints(listing(1), null))
        .to.deep.include({ rule: 'endsAfterCreation', violation: 'NotMet' });
      // Ends in 2100; the block height a create records is unknown before its
      // block, so the height rule is not judged
      expect(contract.checkDocumentPropertyConstraints(listing(4102444800000), null))
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
          readsStored: false,
          readsTotals: [],
          readsReferenced: [],
        },
      ]);

      const listing = (labels: string[]) => new wasm.Document({
        properties: { labels },
        documentTypeName: 'listing',
        dataContractId: contract.id,
        ownerId,
        revision: BigInt(1),
      });
      expect(contract.checkDocumentPropertyConstraints(listing(['new']), null)).to.equal(undefined);
      expect(contract.checkDocumentPropertyConstraints(listing(['new', 'used']), null))
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
      expect(contract.checkDocumentPropertyConstraints(link('https://pay.dash'), null)).to.equal(undefined);
      expect(contract.checkDocumentPropertyConstraints(link('https://pay.com'), null))
        .to.deep.include({ rule: 'dashDomain', violation: 'NotMet' });
      // No case folding
      expect(contract.checkDocumentPropertyConstraints(link('HTTPS://pay.dash'), null))
        .to.deep.include({ rule: 'secureUrl', violation: 'NotMet' });
    });

    it('should read the bytes of byte arrays with byteAt, startsWith and endsWith', () => {
      const rules = {
        addressType: { in: [{ byteAt: ['corePaymentAddress', 0] }, [0, 1]] },
        taggedPayload: {
          anyOf: [{ absent: 'payload' }, { startsWith: ['payload', { const: 'cafe' }] }],
        },
      };
      const contract = buildContract({
        profile: {
          type: 'object',
          properties: {
            corePaymentAddress: {
              type: 'array', byteArray: true, minItems: 21, maxItems: 21, position: 0,
            },
            payload: {
              type: 'array', byteArray: true, maxItems: 8, position: 1,
            },
          },
          additionalProperties: false,
          propertyConstraints: rules,
        },
      });

      expect(contract.documentTypePropertyConstraints('profile').map((rule) => rule.reads))
        .to.deep.equal([
          [{ path: 'corePaymentAddress', kind: 'bytes' }],
          [{ path: 'payload', kind: 'presence' }, { path: 'payload', kind: 'bytes' }],
        ]);

      // A DIP-33 address in storage form: the type byte, then a HASH160
      const address = (typeByte: number) => Uint8Array.from([typeByte, ...Array(20).fill(0x5a)]);
      const profile = (properties: Record<string, unknown>) => new wasm.Document({
        properties,
        documentTypeName: 'profile',
        dataContractId: contract.id,
        ownerId,
        revision: BigInt(1),
      });
      const violationOf = (properties: Record<string, unknown>) => (
        contract.checkDocumentPropertyConstraints(profile(properties), null)
      );

      expect(violationOf({ corePaymentAddress: address(1) })).to.equal(undefined);
      // An address left out reads its type byte as 0
      expect(violationOf({})).to.equal(undefined);
      expect(violationOf({ corePaymentAddress: address(2) }))
        .to.deep.include({ rule: 'addressType', violation: 'NotMet' });
      expect(violationOf({ payload: Uint8Array.from([0xca, 0xfe, 1]) })).to.equal(undefined);
      expect(violationOf({ payload: Uint8Array.from([0xbe, 0xef]) }))
        .to.deep.include({ rule: 'taggedPayload', violation: 'NotMet' });
    });

    it('should check ifThen, ifThenElse, notIn, min, max and abs', () => {
      const contract = buildContract({
        order: {
          type: 'object',
          properties: {
            price: { type: 'integer', minimum: 0, position: 0 },
            fee: { type: 'integer', minimum: 0, position: 1 },
            discount: { type: 'integer', minimum: 0, position: 2 },
          },
          required: ['price', 'fee'],
          additionalProperties: false,
          propertyConstraints: {
            discountNeedsPrice: {
              ifThen: [{ greaterThan: ['discount', 0] }, { greaterThanOrEqual: ['price', 100] }],
            },
            feeCapped: { lessThanOrEqual: ['fee', { max: [10, { divide: ['price', 10] }] }] },
            feeNotBanned: { notIn: ['fee', [7, 13]] },
            feeTiers: {
              ifThenElse: [
                { greaterThanOrEqual: ['price', 1000] },
                { lessThanOrEqual: ['fee', 50] },
                { lessThanOrEqual: ['fee', 10] },
              ],
            },
            spreadSmall: { lessThanOrEqual: [{ abs: { subtract: ['price', 'fee'] } }, 1000] },
            termsPositive: { greaterThan: [{ min: ['price', 'fee'] }, 0] },
          },
        },
      });
      const order = (properties: Record<string, number>) => new wasm.Document({
        properties,
        documentTypeName: 'order',
        dataContractId: contract.id,
        ownerId,
        revision: BigInt(1),
      });
      const violationOf = (properties: Record<string, number>) => (
        contract.checkDocumentPropertyConstraints(order(properties), null)
      );

      expect(violationOf({ price: 100, fee: 10 })).to.equal(undefined);
      expect(violationOf({ price: 50, fee: 5, discount: 5 }))
        .to.deep.include({ rule: 'discountNeedsPrice', violation: 'NotMet' });
      expect(violationOf({ price: 100, fee: 11 }))
        .to.deep.include({ rule: 'feeCapped', violation: 'NotMet' });
      expect(violationOf({ price: 100, fee: 7 }))
        .to.deep.include({ rule: 'feeNotBanned', violation: 'NotMet' });
      // Each branch of the ifThenElse
      expect(violationOf({ price: 1000, fee: 40 })).to.equal(undefined);
      expect(violationOf({ price: 1000, fee: 60 }))
        .to.deep.include({ rule: 'feeTiers', violation: 'NotMet' });
      expect(violationOf({ price: 500, fee: 20 }))
        .to.deep.include({ rule: 'feeTiers', violation: 'NotMet' });
      expect(violationOf({ price: 5000, fee: 10 }))
        .to.deep.include({ rule: 'spreadSmall', violation: 'NotMet' });
      expect(violationOf({ price: 100, fee: 0 }))
        .to.deep.include({ rule: 'termsPositive', violation: 'NotMet' });
    });

    it('should report what a countOf or sumOf reads, and leave it to consensus', () => {
      const rules = {
        allListings: { lessThan: [{ countOf: ['listing'] }, 1000] },
        atMostTwoPerOwner: {
          lessThanOrEqual: [{ countOf: ['listing', { $ownerId: '$ownerId' }] }, 2],
        },
        categoryBudget: {
          lessThanOrEqual: [{ sumOf: ['listing', 'price', { category: 'category' }] }, 250],
        },
      };
      const contract = buildContract({
        listing: {
          type: 'object',
          properties: {
            price: {
              type: 'integer', minimum: 0, maximum: 1000000000, position: 0,
            },
            category: {
              type: 'integer', minimum: 0, maximum: 100, position: 1,
            },
          },
          required: ['price', 'category'],
          documentsCountable: true,
          indices: [
            { name: 'byOwner', properties: [{ $ownerId: 'asc' }], countable: 'countable' },
            { name: 'byCategory', properties: [{ category: 'asc' }], summable: 'price' },
          ],
          additionalProperties: false,
          propertyConstraints: rules,
        },
      });

      // The count by owner depends on the owner, the category total on a
      // property of the document written; a whole-type count has no filter
      expect(contract.documentTypePropertyConstraints('listing')).to.deep.equal([
        {
          name: 'allListings',
          rule: rules.allListings,
          reads: [],
          readsOwner: false,
          readsSystem: [],
          readsStored: false,
          readsTotals: [{ kind: 'countOf', documentType: 'listing', filter: [] }],
          readsReferenced: [],
        },
        {
          name: 'atMostTwoPerOwner',
          rule: rules.atMostTwoPerOwner,
          reads: [],
          readsOwner: true,
          readsSystem: [],
          readsStored: false,
          readsTotals: [{ kind: 'countOf', documentType: 'listing', filter: ['$ownerId'] }],
          readsReferenced: [],
        },
        {
          name: 'categoryBudget',
          rule: rules.categoryBudget,
          reads: [{ path: 'category', kind: 'value' }],
          readsOwner: false,
          readsSystem: [],
          readsStored: false,
          readsTotals: [{
            kind: 'sumOf', documentType: 'listing', property: 'price', filter: ['category'],
          }],
          readsReferenced: [],
        },
      ]);

      // A price far past the budget: the total is read from state, which the
      // pre-check does not do, so neither rule is judged
      const listing = new wasm.Document({
        properties: { price: 999999999, category: 1 },
        documentTypeName: 'listing',
        dataContractId: contract.id,
        ownerId,
        revision: BigInt(1),
      });
      expect(contract.checkDocumentPropertyConstraints(listing, null)).to.equal(undefined);
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
          readsStored: false,
          readsTotals: [],
          readsReferenced: [],
        },
        {
          name: 'knownTier',
          rule: rules.knownTier,
          reads: [{ path: 'tier', kind: 'value' }],
          readsOwner: false,
          readsSystem: [],
          readsStored: false,
          readsTotals: [],
          readsReferenced: [],
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
      // Full validation at protocol version 13 refuses the keyword outright:
      // the version 2 meta-schema does not know it.
      expect(() => buildContract(schemas, 13)).to.throw(/propertyConstraints/);

      // A contract read back without full validation parses no rules.
      const contract = buildContract(schemas, 13, false);

      expect(contract.documentTypePropertyConstraints('offer')).to.deep.equal([]);
      expect(contract.checkDocumentPropertyConstraints(offer(contract, { discount: 200 }), null))
        .to.equal(undefined);
    });
  });

  describe('rules reading the stored version ($old.)', () => {
    const rules = {
      atMostThreeWrites: { lessThanOrEqual: ['$revision', 3] },
      priceNeverRises: { lessThanOrEqual: ['price', { ifAbsent: ['$old.price', 1000000] }] },
    };
    const listingSchemas = {
      listing: {
        type: 'object',
        documentsMutable: true,
        properties: {
          price: { type: 'integer', minimum: 0, position: 0 },
        },
        required: ['price'],
        additionalProperties: false,
        propertyConstraints: rules,
      },
    };
    // A fixed id, so that a stored and a written version are the same document
    const listingId = otherId;

    function listing(
      contract: InstanceType<typeof wasm.DataContract>,
      price: number,
      revision: number,
    ) {
      return new wasm.Document({
        id: listingId,
        properties: { price },
        documentTypeName: 'listing',
        dataContractId: contract.id,
        ownerId,
        revision: BigInt(revision),
      });
    }

    it('should report the stored reads and $revision a rule reads', () => {
      const contract = buildContract(listingSchemas);

      expect(contract.documentTypePropertyConstraints('listing')).to.deep.equal([
        {
          name: 'atMostThreeWrites',
          rule: rules.atMostThreeWrites,
          reads: [],
          readsOwner: false,
          readsSystem: ['$revision'],
          readsStored: false,
          readsTotals: [],
          readsReferenced: [],
        },
        {
          name: 'priceNeverRises',
          rule: rules.priceNeverRises,
          reads: [
            { path: 'price', kind: 'value' },
            { path: '$old.price', kind: 'value' },
          ],
          readsOwner: false,
          readsSystem: [],
          readsStored: true,
          readsTotals: [],
          readsReferenced: [],
        },
      ]);
    });

    it('should judge them against the stored document, a create, or not at all', () => {
      const contract = buildContract(listingSchemas);
      const stored = listing(contract, 100, 1);

      // A replace: the price may only drop
      expect(contract.checkDocumentPropertyConstraints(listing(contract, 90, 2), stored))
        .to.equal(undefined);
      expect(contract.checkDocumentPropertyConstraints(listing(contract, 120, 2), stored))
        .to.deep.include({ rule: 'priceNeverRises', violation: 'NotMet' });
      // A create has no stored version: the default stands in for the stored price
      expect(contract.checkDocumentPropertyConstraints(listing(contract, 120, 1), null))
        .to.equal(undefined);
      // Not told which write it is, neither the rule reading the stored version nor
      // the one reading the revision the write records is judged
      expect(contract.checkDocumentPropertyConstraints(listing(contract, 120, 2), undefined))
        .to.equal(undefined);
      expect(contract.checkDocumentPropertyConstraints(listing(contract, 120, 4), undefined))
        .to.equal(undefined);
    });

    it('should judge the revision a replace writes, as the replacement built from the edit sends it', () => {
      const contract = buildContract(listingSchemas);
      // Fetched at revision 3 and edited in place, as an app replaces a document
      const stored = listing(contract, 100, 3);
      const edited = listing(contract, 90, 3);
      const replacement = new wasm.DocumentReplaceTransition({
        document: edited, identityContractNonce: BigInt(1),
      });

      // The replacement writes revision 4, the only one consensus accepts
      expect(replacement.revision).to.equal(BigInt(4));
      expect(contract.checkDocumentPropertyConstraints(edited, stored))
        .to.deep.include({ rule: 'atMostThreeWrites', violation: 'NotMet' });
      // Whatever revision the document checked holds
      expect(contract.checkDocumentPropertyConstraints(listing(contract, 90, 4), stored))
        .to.deep.include({ rule: 'atMostThreeWrites', violation: 'NotMet' });
      // A replace of revision 2 writes revision 3, within the rule
      expect(contract.checkDocumentPropertyConstraints(edited, listing(contract, 100, 2)))
        .to.equal(undefined);
      // A create writes revision 1
      expect(contract.checkDocumentPropertyConstraints(listing(contract, 90, 7), null))
        .to.equal(undefined);
    });

    it('should judge the creation a replace keeps, not the device clock', () => {
      const dayMs = 86400000;
      const contract = buildContract({
        note: {
          type: 'object',
          documentsMutable: true,
          properties: {
            text: { type: 'string', maxLength: 63, position: 0 },
          },
          required: ['text', '$createdAt', '$updatedAt'],
          additionalProperties: false,
          propertyConstraints: {
            editableForADay: { lessThanOrEqual: [{ subtract: ['$updatedAt', '$createdAt'] }, dayMs] },
          },
        },
      });
      const note = (text: string, revision: number, createdAt?: number) => {
        const document = new wasm.Document({
          id: listingId,
          properties: { text },
          documentTypeName: 'note',
          dataContractId: contract.id,
          ownerId,
          revision: BigInt(revision),
        });
        if (createdAt !== undefined) {
          document.createdAt = BigInt(createdAt);
        }
        return document;
      };

      // The edit carries no creation time; the stored document's stands
      expect(contract.checkDocumentPropertyConstraints(
        note('edited', 2),
        note('first', 1, Date.now() - 3 * dayMs),
      )).to.deep.include({ rule: 'editableForADay', violation: 'NotMet' });
      expect(contract.checkDocumentPropertyConstraints(
        note('edited', 2),
        note('first', 1, Date.now() - dayMs / 24),
      )).to.equal(undefined);
      // A create is created now, whatever the document holds
      expect(contract.checkDocumentPropertyConstraints(note('first', 1, 1), null))
        .to.equal(undefined);
    });

    it('should throw for a stored document that is another document', () => {
      const contract = buildContract(listingSchemas);
      const other = new wasm.Document({
        properties: { price: 100 },
        documentTypeName: 'listing',
        dataContractId: contract.id,
        ownerId,
        revision: BigInt(1),
      });

      expect(() => contract.checkDocumentPropertyConstraints(listing(contract, 90, 2), other))
        .to.throw(/another document/);
    });
  });

  describe('checkDocumentPropertyConstraints()', () => {
    it('should report nothing for a document meeting every rule', () => {
      const contract = buildContract(schemas);

      expect(contract.checkDocumentPropertyConstraints(offer(contract, {}), null)).to.equal(undefined);
      expect(contract.checkDocumentPropertyConstraints(offer(contract, {
        status: 'closed', closedAt: 1000, sellerId: ownerId, discount: 5,
      }), null)).to.equal(undefined);
    });

    it('should report the first rule broken, in name order, as consensus would', () => {
      const contract = buildContract(schemas);
      const violationOf = (properties: Record<string, unknown>, owner = ownerId) => (
        contract.checkDocumentPropertyConstraints(offer(contract, properties, owner), null)
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

      expect(contract.checkDocumentPropertyConstraints(offer(contract, { sellerId: otherId }), null))
        .to.deep.include({ rule: 'sellerIsOwner', violation: 'NotMet' });
      expect(contract.checkDocumentPropertyConstraints(
        offer(contract, { sellerId: otherId }, otherId),
        null,
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

      expect(contract.checkDocumentPropertyConstraints(document, null)).to.equal(undefined);
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

      expect(() => contract.checkDocumentPropertyConstraints(foreign, null)).to.throw(/another contract|not this one/);
      expect(() => contract.checkDocumentPropertyConstraints(unknownType, null)).to.throw(/not found/);
    });
  });

  describe('a malformed rule', () => {
    it('should throw with the consensus code Platform would refuse the contract with', () => {
      const malformed = {
        ...schemas,
        offer: { ...schemas.offer, propertyConstraints: { rule: { equal: ['price'] } } },
      };

      // Nodes refuse it at the meta-schema.
      try {
        buildContract(malformed);
        expect.fail('expected to throw');
      } catch (e) {
        expect(e).to.be.instanceOf(wasm.WasmDppError);
        expect(e.code).to.equal(JSON_SCHEMA_ERROR);
      }

      // The parser refuses it too, where the meta-schema does not run.
      try {
        buildContract(malformed, 14, false);
        expect.fail('expected to throw');
      } catch (e) {
        expect(e).to.be.instanceOf(wasm.WasmDppError);
        expect(e.message).to.match(/must list exactly two operands/);
        // InvalidContractStructure
        expect(e.code).to.equal(10231);
      }
    });
  });
});
