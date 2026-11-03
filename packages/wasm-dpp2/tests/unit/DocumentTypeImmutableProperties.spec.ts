/**
 * Verifies the `immutable` metadata surface introduced with protocol
 * version 14.
 *
 * On a mutable document type, `immutable` lists top-level properties a
 * replace may not change: by name, frozen at creation, or as
 * `{ property, when }`, frozen for any replace its condition holds for
 * (judged on the document the replace writes, with the stored one read
 * through `$old.`). Consensus enforces both on every replace (code 40128).
 * What the JS layer offers is *discovery* (which properties are frozen, and
 * under which condition) plus a branchable error code for when a replace is
 * rejected.
 */
import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

let PlatformVersion: typeof wasm.PlatformVersion;

before(async () => {
  await initWasm();
  ({ PlatformVersion } = wasm);
});

const ownerId = '11111111111111111111111111111111';

/** Frozen five minutes after creation: `$updatedAt` is the replace's time. */
const fiveMinutesAfterCreation = {
  greaterThan: [{ subtract: ['$updatedAt', '$createdAt'] }, 300000],
};

/**
 * A `post` whose author can never change, whose mood can be set once and
 * whose body can be edited for five minutes, a `note` whose text is frozen
 * once published, and a `plain` type declaring nothing.
 */
const schemas = {
  post: {
    type: 'object',
    documentsMutable: true,
    properties: {
      author: { type: 'string', position: 0, maxLength: 63 },
      body: { type: 'string', position: 1, maxLength: 500 },
      mood: { type: 'string', position: 2, maxLength: 30 },
    },
    required: ['author', 'body', '$createdAt', '$updatedAt'],
    immutable: [
      { property: 'mood', when: { present: '$old.mood' } },
      'author',
      { property: 'body', when: fiveMinutesAfterCreation },
    ],
    additionalProperties: false,
  },
  note: {
    type: 'object',
    documentsMutable: true,
    properties: {
      text: { type: 'string', position: 0, maxLength: 200 },
      status: { type: 'string', position: 1, enum: ['draft', 'published'] },
    },
    required: ['text'],
    immutable: [
      { property: 'text', when: { equal: ['$old.status', { const: 'published' }] } },
    ],
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

type ImmutableProperties = {
  immutable: string[];
  immutableWhen: Record<string, unknown>;
};

describe('DataContract — immutable properties (v14)', () => {
  describe('documentTypeImmutableProperties()', () => {
    it('should report the frozen properties and the conditions, sorted by property name', () => {
      const contract = buildContract(14);

      // Integers of a condition come back as BigInt, as every schema integer
      expect(contract.documentTypeImmutableProperties('post')).to.deep.equal({
        immutable: ['author'],
        immutableWhen: {
          body: { greaterThan: [{ subtract: ['$updatedAt', '$createdAt'] }, BigInt(300000)] },
          mood: { present: '$old.mood' },
        },
      });
      expect(contract.documentTypeImmutableProperties('note')).to.deep.equal({
        immutable: [],
        immutableWhen: {
          text: { equal: ['$old.status', { const: 'published' }] },
        },
      });
    });

    it('should return nothing for a document type declaring none', () => {
      const contract = buildContract(14);

      expect(contract.documentTypeImmutableProperties('plain')).to.deep.equal({
        immutable: [],
        immutableWhen: {},
      });
    });

    /**
     * Empty lists would conflate "no such type" with "nothing frozen",
     * which is a difference a caller acting on the result needs.
     */
    it('should throw for an unknown document type', () => {
      const contract = buildContract(14);

      expect(() => contract.documentTypeImmutableProperties('doesNotExist')).to.throw(/not found/);
    });

    /**
     * The keyword is only parsed from protocol version 14 onward. A
     * contract deserialized against an earlier version reports nothing
     * frozen even though its raw schema still carries it.
     */
    it('should report nothing on a pre-v14 contract, while the raw schema keeps the keyword', () => {
      const contract = buildContract(13, false);

      expect(contract.documentTypeImmutableProperties('post')).to.deep.equal({
        immutable: [],
        immutableWhen: {},
      });

      const rawSchemas = contract.schemas as Record<string, { immutable?: unknown[] }>;
      expect(rawSchemas.post.immutable).to.have.length(3);
    });

    /**
     * The declarations are consensus-validated at registration: a
     * condition reading a property the type does not declare is a contract
     * error, not something the accessor has to guard against.
     */
    it('should refuse a contract whose condition reads an unknown property', () => {
      const build = () => new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: {
          note: {
            ...schemas.note,
            immutable: [{ property: 'text', when: { present: '$old.nope' } }],
          },
        },
        definitions: null,
        fullValidation: true,
        platformVersion: new PlatformVersion(14),
      });

      expect(build).to.throw(/tests the presence of "\$old\.nope"/);
    });

    it('should refuse immutableAllowSetting, which a condition replaces', () => {
      const build = () => new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: {
          post: { ...schemas.plain, documentsMutable: true, immutableAllowSetting: ['message'] },
        },
        definitions: null,
        fullValidation: false,
        platformVersion: new PlatformVersion(14),
      });

      expect(build).to.throw(/is replaced by a conditional `immutable` entry/);
    });
  });

  describe('documentImmutableProperties', () => {
    it('should key declarations by document type and omit types freezing nothing', () => {
      const contract = buildContract(14);
      const map = contract.documentImmutableProperties as Map<string, ImmutableProperties>;

      expect([...map.keys()].sort()).to.deep.equal(['note', 'post']);
      expect(map.get('post')).to.deep.equal(contract.documentTypeImmutableProperties('post'));
      expect(map.get('note')).to.deep.equal(contract.documentTypeImmutableProperties('note'));
    });

    it('should be empty for a contract freezing nothing at all', () => {
      const contract = new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: { plain: schemas.plain },
        definitions: null,
        fullValidation: true,
        platformVersion: new PlatformVersion(14),
      });

      expect((contract.documentImmutableProperties as Map<string, ImmutableProperties>).size).to.equal(0);
    });
  });

  describe('DocumentImmutabilityErrorCode', () => {
    /**
     * The number a caller compares `WasmSdkError.code` against after a
     * rejected replace. Renumbering it silently breaks every `switch` in
     * the wild.
     */
    it('should map the immutable-property error to its consensus code', () => {
      expect(wasm.DocumentImmutabilityErrorCode.DocumentImmutablePropertyChanged).to.equal(40128);
    });

    it('should resolve the code back to its name', () => {
      const codes = wasm.DocumentImmutabilityErrorCode as unknown as Record<number, string>;

      expect(codes[40128]).to.equal('DocumentImmutablePropertyChanged');
    });
  });
});
