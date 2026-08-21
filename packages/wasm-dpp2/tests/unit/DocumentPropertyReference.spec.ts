/**
 * Verifies the `refersTo` document-reference metadata surface introduced
 * with protocol version 14.
 *
 * `refersTo` is a write-time consensus constraint: it declares what an
 * identifier property points at, and consensus checks the target exists
 * whenever a document carrying it is written. Nothing resolves a reference
 * for a reader, so what the JS layer offers is *discovery* — which
 * properties are references, and to what — plus branchable error codes for
 * when a write is rejected.
 */
import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

let PlatformVersion: typeof wasm.PlatformVersion;

before(async () => {
  await initWasm();
  ({ PlatformVersion } = wasm);
});

const ownerId = '11111111111111111111111111111111';
const foreignContractId = '4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6';

/**
 * `refersTo` is only allowed on properties with exactly this shape — a
 * 32-byte identifier. The meta-schema rejects it anywhere else.
 */
function identifierProperty(position: number, refersTo: object): object {
  return {
    type: 'array',
    byteArray: true,
    minItems: 32,
    maxItems: 32,
    contentMediaType: 'application/x.dash.dpp.identifier',
    position,
    refersTo,
  };
}

/**
 * One document type covering every reference target, plus a nested
 * declaration to exercise dotted paths, and a second type declaring none.
 */
const schemas = {
  note: {
    type: 'object',
    // A `permanentDocument` target must not be deletable, and `note`
    // references itself below.
    canBeDeleted: false,
    properties: {
      author: identifierProperty(0, { type: 'identity' }),
      sourceContract: identifierProperty(1, { type: 'contract' }),
      paidWith: identifierProperty(2, { type: 'token' }),
      // `contractId` omitted: targets the declaring contract itself.
      parentNoteId: identifierProperty(3, {
        type: 'permanentDocument',
        documentType: 'note',
      }),
      otherDoc: identifierProperty(4, {
        type: 'permanentDocument',
        contractId: foreignContractId,
        documentType: 'thing',
      }),
      signerKey: identifierProperty(5, {
        type: 'identityPublicKey',
        keyIdProperty: 'signerKeyId',
      }),
      signerKeyId: { type: 'integer', position: 6, minimum: 0 },
      meta: {
        type: 'object',
        position: 7,
        properties: {
          ownerRef: identifierProperty(0, { type: 'identity' }),
        },
        additionalProperties: false,
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

type Reference = {
  path: string;
  type: string;
  contractId?: { toBase58(): string };
  documentType?: string;
  keyIdProperty?: string;
};

describe('DataContract — refersTo declarations (v14)', () => {
  describe('documentTypeReferences()', () => {
    it('should report every reference declaration in schema property order', () => {
      const contract = buildContract(14);
      const references = contract.documentTypeReferences('note') as Reference[];

      expect(references.map((reference) => reference.path)).to.deep.equal([
        'author',
        'sourceContract',
        'paidWith',
        'parentNoteId',
        'otherDoc',
        'signerKey',
        'meta.ownerRef',
      ]);
    });

    it('should tag each declaration with its target kind', () => {
      const contract = buildContract(14);
      const references = contract.documentTypeReferences('note') as Reference[];
      const byPath = new Map(references.map((reference) => [reference.path, reference]));

      expect(byPath.get('author')!.type).to.equal('identity');
      expect(byPath.get('sourceContract')!.type).to.equal('contract');
      expect(byPath.get('paidWith')!.type).to.equal('token');
      expect(byPath.get('parentNoteId')!.type).to.equal('permanentDocument');
      expect(byPath.get('otherDoc')!.type).to.equal('permanentDocument');
      expect(byPath.get('signerKey')!.type).to.equal('identityPublicKey');
      expect(byPath.get('meta.ownerRef')!.type).to.equal('identity');
    });

    it('should carry no target fields for the bare kinds', () => {
      const contract = buildContract(14);
      const references = contract.documentTypeReferences('note') as Reference[];
      const author = references.find((reference) => reference.path === 'author')!;

      expect(author).to.deep.equal({ path: 'author', type: 'identity' });
    });

    /**
     * An omitted `contractId` means "the declaring contract". Consensus
     * computes `contract_id.unwrap_or(contract.id())` and treats an
     * explicit self-id identically, so the accessor resolves it rather
     * than handing JS a null to re-derive.
     */
    it('should resolve an omitted contractId to the declaring contract', () => {
      const contract = buildContract(14);
      const references = contract.documentTypeReferences('note') as Reference[];
      const parent = references.find((reference) => reference.path === 'parentNoteId')!;

      expect(parent.contractId!.toBase58()).to.equal(contract.id.toBase58());
      expect(parent.documentType).to.equal('note');
    });

    it('should keep an explicit foreign contractId', () => {
      const contract = buildContract(14);
      const references = contract.documentTypeReferences('note') as Reference[];
      const other = references.find((reference) => reference.path === 'otherDoc')!;

      expect(other.contractId!.toBase58()).to.equal(foreignContractId);
      expect(other.documentType).to.equal('thing');
    });

    it('should carry keyIdProperty for an identityPublicKey reference', () => {
      const contract = buildContract(14);
      const references = contract.documentTypeReferences('note') as Reference[];
      const signerKey = references.find((reference) => reference.path === 'signerKey')!;

      expect(signerKey.keyIdProperty).to.equal('signerKeyId');
    });

    it('should return an empty array for a document type declaring none', () => {
      const contract = buildContract(14);

      expect(contract.documentTypeReferences('plain')).to.deep.equal([]);
    });

    /**
     * An empty array would conflate "no such type" with "no references",
     * which is a difference a caller acting on the result needs.
     */
    it('should throw for an unknown document type', () => {
      const contract = buildContract(14);

      expect(() => contract.documentTypeReferences('doesNotExist')).to.throw(/not found/);
    });

    /**
     * The version gate, and the trap that comes with it: `refersTo` is only
     * parsed from protocol version 14 onward, so a contract deserialized
     * against an earlier version reports no references even though its raw
     * schema still carries the keyword.
     */
    it('should report no references on a pre-v14 contract, while the raw schema keeps the keyword', () => {
      const contract = buildContract(13, false);

      expect(contract.documentTypeReferences('note')).to.deep.equal([]);

      const rawSchemas = contract.schemas as Record<
        string,
        { properties: Record<string, { refersTo?: object }> }
      >;
      expect(rawSchemas.note.properties.author.refersTo).to.deep.equal({ type: 'identity' });
    });
  });

  describe('documentReferences', () => {
    it('should key declarations by document type and omit types with none', () => {
      const contract = buildContract(14);
      const map = contract.documentReferences as Map<string, Reference[]>;

      expect([...map.keys()]).to.deep.equal(['note']);
      expect(map.get('note')!.map((reference) => reference.path)).to.deep.equal(
        (contract.documentTypeReferences('note') as Reference[]).map((r) => r.path),
      );
    });

    it('should be empty for a contract declaring no references at all', () => {
      const contract = new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: { plain: schemas.plain },
        definitions: null,
        fullValidation: true,
        platformVersion: new PlatformVersion(14),
      });

      expect((contract.documentReferences as Map<string, Reference[]>).size).to.equal(0);
    });
  });

  describe('DocumentReferenceErrorCode', () => {
    /**
     * These are the numbers a caller compares `WasmSdkError.code` against
     * after a rejected write. Renumbering any of them silently breaks every
     * `switch` in the wild.
     */
    it('should map each reference-validation error to its consensus code', () => {
      expect(wasm.DocumentReferenceErrorCode.ReferencedEntityNotFound).to.equal(40120);
      expect(wasm.DocumentReferenceErrorCode.ReferencedDocumentTypeNotFound).to.equal(40121);
      expect(wasm.DocumentReferenceErrorCode.ReferencedDocumentTypeDeletable).to.equal(40122);
      expect(wasm.DocumentReferenceErrorCode.ReferencedIdentityKeyNotFound).to.equal(40123);
      expect(wasm.DocumentReferenceErrorCode.ReferencedIdentityKeyDisabled).to.equal(40124);
      expect(wasm.DocumentReferenceErrorCode.ReferencedKeyIdPropertyInvalid).to.equal(40125);
    });

    it('should resolve a code back to its name', () => {
      const codes = wasm.DocumentReferenceErrorCode as unknown as Record<number, string>;

      expect(codes[40123]).to.equal('ReferencedIdentityKeyNotFound');
      expect(codes[40125]).to.equal('ReferencedKeyIdPropertyInvalid');
    });
  });
});
