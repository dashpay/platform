/**
 * Verifies the `immutable` / `immutableAllowSetting` / `immutableAfter`
 * metadata surface introduced with protocol version 14.
 *
 * On a mutable document type, `immutable` lists top-level properties frozen
 * at creation, `immutableAllowSetting` the subset a replace may still set
 * while the stored document has no value for them, and `immutableAfter` the
 * properties a replace may change only for so many seconds after the
 * document's `$createdAt`. Consensus enforces them on every replace (codes
 * 40128 and 40143). What the JS layer offers is *discovery*
 * (which properties are frozen, and which may still be set once) plus a
 * branchable error code for when a replace is rejected.
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
 * A `post` whose author can never change, whose mood can be set once and
 * whose body can be edited for five minutes, a `note` whose text can be
 * edited for a minute, and a `plain` type declaring nothing.
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
    required: ['author', 'body', '$createdAt'],
    immutable: ['mood', 'author'],
    immutableAllowSetting: ['mood'],
    immutableAfter: { body: 300 },
    additionalProperties: false,
  },
  note: {
    type: 'object',
    documentsMutable: true,
    properties: {
      text: { type: 'string', position: 0, maxLength: 200 },
    },
    required: ['text', '$createdAt'],
    immutableAfter: { text: 60 },
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
  immutableAllowSetting: string[];
  immutableAfter: Record<string, number>;
};

describe('DataContract — immutable properties (v14)', () => {
  describe('documentTypeImmutableProperties()', () => {
    it('should report the lists sorted by property name, and the windows', () => {
      const contract = buildContract(14);

      expect(contract.documentTypeImmutableProperties('post')).to.deep.equal({
        immutable: ['author', 'mood'],
        immutableAllowSetting: ['mood'],
        immutableAfter: { body: 300 },
      });
      expect(contract.documentTypeImmutableProperties('note')).to.deep.equal({
        immutable: [],
        immutableAllowSetting: [],
        immutableAfter: { text: 60 },
      });
    });

    it('should return empty lists for a document type declaring none', () => {
      const contract = buildContract(14);

      expect(contract.documentTypeImmutableProperties('plain')).to.deep.equal({
        immutable: [],
        immutableAllowSetting: [],
        immutableAfter: {},
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
     * The keywords are only parsed from protocol version 14 onward. A
     * contract deserialized against an earlier version reports nothing
     * frozen even though its raw schema still carries them.
     */
    it('should report nothing on a pre-v14 contract, while the raw schema keeps the keywords', () => {
      const contract = buildContract(13, false);

      expect(contract.documentTypeImmutableProperties('post')).to.deep.equal({
        immutable: [],
        immutableAllowSetting: [],
        immutableAfter: {},
      });

      const rawSchemas = contract.schemas as Record<
        string,
        {
          immutable?: string[];
          immutableAllowSetting?: string[];
          immutableAfter?: Record<string, bigint>;
        }
      >;
      expect(rawSchemas.post.immutable).to.deep.equal(['mood', 'author']);
      expect(rawSchemas.post.immutableAllowSetting).to.deep.equal(['mood']);
      // The raw schema carries integers as BigInt, as every schema integer
      expect(rawSchemas.post.immutableAfter).to.deep.equal({ body: BigInt(300) });
    });

    /**
     * The lists are consensus-validated at registration: an
     * `immutableAllowSetting` entry outside `immutable`, an unknown
     * property, or a list on a non-mutable type is a contract error, not
     * something the accessor has to guard against.
     */
    it('should refuse a contract whose allow-setting entry is not immutable', () => {
      const build = () => new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: {
          post: { ...schemas.post, immutableAllowSetting: ['body'] },
        },
        definitions: null,
        fullValidation: true,
        platformVersion: new PlatformVersion(14),
      });

      expect(build).to.throw(/not in `immutable`/);
    });

    it('should refuse a contract whose windowed type does not require $createdAt', () => {
      const build = () => new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: {
          note: { ...schemas.note, required: ['text'] },
        },
        definitions: null,
        fullValidation: true,
        platformVersion: new PlatformVersion(14),
      });

      expect(build).to.throw(/does not require `\$createdAt`/);
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
    it('should map the immutable-property errors to their consensus codes', () => {
      expect(wasm.DocumentImmutabilityErrorCode.DocumentImmutablePropertyChanged).to.equal(40128);
      expect(wasm.DocumentImmutabilityErrorCode.DocumentPropertyEditWindowElapsed).to.equal(40143);
    });

    it('should resolve the codes back to their names', () => {
      const codes = wasm.DocumentImmutabilityErrorCode as unknown as Record<number, string>;

      expect(codes[40128]).to.equal('DocumentImmutablePropertyChanged');
      expect(codes[40143]).to.equal('DocumentPropertyEditWindowElapsed');
    });
  });
});
