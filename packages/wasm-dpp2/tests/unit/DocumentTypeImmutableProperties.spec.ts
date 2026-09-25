/**
 * Verifies the `immutable` / `immutableAllowSetting` metadata surface
 * introduced with protocol version 14.
 *
 * On a mutable document type, `immutable` lists top-level properties frozen
 * at creation and `immutableAllowSetting` the subset a replace may still set
 * while the stored document has no value for them. Consensus enforces both
 * on every replace (code 40128). What the JS layer offers is *discovery*
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
 * A `post` whose author can never change and whose mood can be set once,
 * next to a `plain` type declaring nothing.
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
    required: ['author', 'body'],
    immutable: ['mood', 'author'],
    immutableAllowSetting: ['mood'],
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
};

describe('DataContract — immutable properties (v14)', () => {
  describe('documentTypeImmutableProperties()', () => {
    it('should report both lists, sorted by property name', () => {
      const contract = buildContract(14);

      expect(contract.documentTypeImmutableProperties('post')).to.deep.equal({
        immutable: ['author', 'mood'],
        immutableAllowSetting: ['mood'],
      });
    });

    it('should return empty lists for a document type declaring none', () => {
      const contract = buildContract(14);

      expect(contract.documentTypeImmutableProperties('plain')).to.deep.equal({
        immutable: [],
        immutableAllowSetting: [],
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
      });

      const rawSchemas = contract.schemas as Record<
        string,
        { immutable?: string[]; immutableAllowSetting?: string[] }
      >;
      expect(rawSchemas.post.immutable).to.deep.equal(['mood', 'author']);
      expect(rawSchemas.post.immutableAllowSetting).to.deep.equal(['mood']);
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
  });

  describe('documentImmutableProperties', () => {
    it('should key declarations by document type and omit types freezing nothing', () => {
      const contract = buildContract(14);
      const map = contract.documentImmutableProperties as Map<string, ImmutableProperties>;

      expect([...map.keys()]).to.deep.equal(['post']);
      expect(map.get('post')).to.deep.equal(contract.documentTypeImmutableProperties('post'));
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
