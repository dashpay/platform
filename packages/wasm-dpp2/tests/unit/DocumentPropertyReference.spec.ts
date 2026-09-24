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
 * 32-byte identifier. The one exception is the `identityPublicKey` form
 * declared on the key id property itself (`senderKeyId` below). The
 * meta-schema rejects it anywhere else.
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
    // `recipientKey` below requires a decryption key bound to `note`, which
    // only a type taking bound decryption keys can be.
    requiresIdentityDecryptionBoundedKey: 2,
    properties: {
      author: identifierProperty(0, { type: 'identity' }),
      sourceContract: identifierProperty(1, { type: 'contract' }),
      paidWith: identifierProperty(2, { type: 'token' }),
      // `contractId` omitted: targets the declaring contract itself.
      // `propertyAgreement` binds the referring document's property to
      // the referenced document's (write-time equality, PV14 #4505).
      parentNoteId: identifierProperty(3, {
        type: 'permanentDocument',
        documentType: 'note',
        propertyAgreement: { signerKeyId: 'signerKeyId' },
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
      // `keyRequirements`: the referenced key must have this purpose and be
      // bound to this contract's `note` type (PV14 #4918).
      recipientKey: identifierProperty(8, {
        type: 'identityPublicKey',
        keyIdProperty: 'recipientKeyId',
        keyRequirements: { purpose: 'decryption', boundTo: 'note' },
      }),
      recipientKeyId: { type: 'integer', position: 9, minimum: 0 },
      // The inverse key reference: the property carries the key id and the
      // declaration names whose key it is (the writer's), so it sits on a
      // u32 integer rather than an identifier; `keyRequirements` apply to it
      // exactly as to the identifier form.
      senderKeyId: {
        type: 'integer',
        minimum: 0,
        maximum: 4294967295,
        position: 10,
        refersTo: {
          type: 'identityPublicKey',
          identityProperty: '$ownerId',
          keyRequirements: { purpose: 'encryption' },
        },
      },
      // The same form naming an identifier property of this type: the key
      // is one of `author`'s.
      authorKeyId: {
        type: 'integer',
        minimum: 0,
        maximum: 4294967295,
        position: 11,
        refersTo: { type: 'identityPublicKey', identityProperty: 'author' },
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
  type?: string;
  anyOf?: Omit<Reference, 'path'>[];
  allOf?: Omit<Reference, 'path'>[];
  contractId?: { toBase58(): string };
  documentType?: string;
  keyIdProperty?: string;
  keyRequirements?: { purpose?: string; boundTo?: string };
  identityProperty?: string;
  propertyAgreement?: Record<string, string>;
  lookup?: { index: string; keys: Record<string, string> };
  inList?: string;
};

/**
 * A `joinRequest` type unique on (`submittedCharterId`, `$ownerId`), and a
 * `charter` whose `memberId` names the owner of a join request for the
 * charter's own `submittedCharterId` rather than holding a request's id.
 */
const plainIdentifier = {
  type: 'array',
  byteArray: true,
  minItems: 32,
  maxItems: 32,
  contentMediaType: 'application/x.dash.dpp.identifier',
  position: 0,
};

const lookupSchemas = {
  joinRequest: {
    type: 'object',
    canBeDeleted: false,
    // A permanentDocument lookup needs a key the join request keeps for good
    documentsMutable: false,
    properties: {
      submittedCharterId: plainIdentifier,
    },
    indices: [
      {
        name: 'bySubmittedCharter',
        properties: [{ submittedCharterId: 'asc' }, { $ownerId: 'asc' }],
        unique: true,
      },
    ],
    required: ['submittedCharterId'],
    additionalProperties: false,
  },
  charter: {
    type: 'object',
    properties: {
      submittedCharterId: plainIdentifier,
      memberId: identifierProperty(1, {
        type: 'permanentDocument',
        documentType: 'joinRequest',
        lookup: {
          index: 'bySubmittedCharter',
          keys: { submittedCharterId: 'submittedCharterId', $ownerId: '.' },
        },
      }),
    },
    required: ['submittedCharterId'],
    additionalProperties: false,
  },
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
        'recipientKey',
        'senderKeyId',
        'authorKeyId',
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
      expect(byPath.get('recipientKey')!.type).to.equal('identityPublicKey');
      expect(byPath.get('senderKeyId')!.type).to.equal('identityPublicKey');
      expect(byPath.get('authorKeyId')!.type).to.equal('identityPublicKey');
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

    it('should carry the propertyAgreement map when declared', () => {
      const contract = buildContract(14);
      const references = contract.documentTypeReferences('note') as Reference[];
      const parent = references.find((reference) => reference.path === 'parentNoteId')!;

      expect(parent.propertyAgreement).to.deep.equal({ signerKeyId: 'signerKeyId' });
    });

    /**
     * Absent — not `{}`-valued — when the declaration carries none,
     * matching the schema's own omission and the absent-field convention
     * of the other optional target fields.
     */
    it('should omit propertyAgreement for a reference declaring none', () => {
      const contract = buildContract(14);
      const references = contract.documentTypeReferences('note') as Reference[];
      const other = references.find((reference) => reference.path === 'otherDoc')!;

      expect(other).to.not.have.property('propertyAgreement');
    });

    it('should carry keyIdProperty for an identityPublicKey reference', () => {
      const contract = buildContract(14);
      const references = contract.documentTypeReferences('note') as Reference[];
      const signerKey = references.find((reference) => reference.path === 'signerKey')!;

      expect(signerKey.keyIdProperty).to.equal('signerKeyId');
      expect(signerKey.identityProperty).to.equal(undefined);
    });

    it('should carry identityProperty for an identityPublicKey reference on the key id property', () => {
      const contract = buildContract(14);
      const references = contract.documentTypeReferences('note') as Reference[];
      const senderKeyId = references.find((reference) => reference.path === 'senderKeyId')!;

      expect(senderKeyId).to.deep.equal({
        path: 'senderKeyId',
        type: 'identityPublicKey',
        identityProperty: '$ownerId',
        keyRequirements: { purpose: 'encryption' },
      });

      const authorKeyId = references.find((reference) => reference.path === 'authorKeyId')!;
      expect(authorKeyId.identityProperty).to.equal('author');
      expect(authorKeyId.keyIdProperty).to.equal(undefined);
      expect(authorKeyId).to.not.have.property('keyRequirements');
    });

    it('should carry keyRequirements when declared and omit them otherwise', () => {
      const contract = buildContract(14);
      const references = contract.documentTypeReferences('note') as Reference[];
      const recipientKey = references.find((reference) => reference.path === 'recipientKey')!;
      const signerKey = references.find((reference) => reference.path === 'signerKey')!;

      expect(recipientKey).to.deep.equal({
        path: 'recipientKey',
        type: 'identityPublicKey',
        keyIdProperty: 'recipientKeyId',
        keyRequirements: { purpose: 'decryption', boundTo: 'note' },
      });
      // Absent, not `{}`-valued, like the schema's own omission.
      expect(signerKey).to.not.have.property('keyRequirements');
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

  describe('lookup', () => {
    it('should carry the lookup of a reference resolved through a unique index', () => {
      const contract = new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: lookupSchemas,
        definitions: null,
        fullValidation: true,
        platformVersion: new PlatformVersion(14),
      });
      const [member] = contract.documentTypeReferences('charter') as Reference[];

      expect(member.path).to.equal('memberId');
      expect(member.type).to.equal('permanentDocument');
      expect(member.documentType).to.equal('joinRequest');
      expect(member.lookup).to.deep.equal({
        index: 'bySubmittedCharter',
        keys: { $ownerId: '.', submittedCharterId: 'submittedCharterId' },
      });
    });

    it('should carry the lookup the elements of a typed array declare', () => {
      const withMembers = structuredClone(lookupSchemas);
      (withMembers.charter.properties as Record<string, object>).members = {
        type: 'array',
        minItems: 0,
        maxItems: 15,
        uniqueItems: true,
        items: {
          type: 'array',
          byteArray: true,
          minItems: 32,
          maxItems: 32,
          contentMediaType: 'application/x.dash.dpp.identifier',
          distinctFrom: '$ownerId',
          refersTo: (withMembers.charter.properties.memberId as { refersTo: object }).refersTo,
        },
        position: 2,
      };
      const contract = new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: withMembers,
        definitions: null,
        fullValidation: true,
        platformVersion: new PlatformVersion(14),
      });
      const members = (contract.documentTypeReferences('charter') as Reference[]).find(
        (reference) => reference.path === 'members[]',
      )!;

      expect(members.type).to.equal('permanentDocument');
      expect(members.lookup).to.deep.equal({
        index: 'bySubmittedCharter',
        keys: { $ownerId: '.', submittedCharterId: 'submittedCharterId' },
      });
    });

    it('should omit lookup for a reference holding the referenced document id', () => {
      const contract = buildContract(14);
      const references = contract.documentTypeReferences('note') as Reference[];
      const other = references.find((reference) => reference.path === 'otherDoc')!;

      expect(other).to.not.have.property('lookup');
    });

    it('should refuse a lookup into an index that is not unique', () => {
      const notUnique = structuredClone(lookupSchemas);
      delete (notUnique.joinRequest.indices[0] as { unique?: boolean }).unique;

      const build = () => new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: notUnique,
        definitions: null,
        fullValidation: true,
        platformVersion: new PlatformVersion(14),
      });

      expect(build).to.throw(/is not unique/);
    });
  });

  describe('listElement', () => {
    /**
     * An `electedCharter` that can be neither deleted nor replaced holds its
     * `members`, and a `resignation` names its charter (`electedCharterId`)
     * and a `memberId` that must be one of that charter's members.
     */
    const listElementSchemas = {
      electedCharter: {
        type: 'object',
        canBeDeleted: false,
        // The list must be fixed once the charter is written
        documentsMutable: false,
        properties: {
          members: {
            type: 'array',
            maxItems: 15,
            items: {
              type: 'array',
              byteArray: true,
              minItems: 32,
              maxItems: 32,
              contentMediaType: 'application/x.dash.dpp.identifier',
            },
            position: 0,
          },
        },
        required: ['members'],
        additionalProperties: false,
      },
      resignation: {
        type: 'object',
        properties: {
          electedCharterId: plainIdentifier,
          memberId: identifierProperty(1, {
            type: 'listElement',
            documentType: 'electedCharter',
            propertyAgreement: { electedCharterId: '$id' },
            inList: 'members',
          }),
        },
        required: ['electedCharterId'],
        additionalProperties: false,
      },
    };
    const buildListElementContract = (documentSchemas: object) => new wasm.DataContract({
      ownerId,
      identityNonce: BigInt(2),
      schemas: documentSchemas,
      definitions: null,
      fullValidation: true,
      platformVersion: new PlatformVersion(14),
    });

    it('should carry the list and the agreement pair naming its document', () => {
      const contract = buildListElementContract(listElementSchemas);
      const member = (contract.documentTypeReferences('resignation') as Reference[]).find(
        (reference) => reference.path === 'memberId',
      )!;

      expect(member.type).to.equal('listElement');
      expect(member.contractId!.toBase58()).to.equal(contract.id.toBase58());
      expect(member.documentType).to.equal('electedCharter');
      expect(member.propertyAgreement).to.deep.equal({ electedCharterId: '$id' });
      expect(member.inList).to.equal('members');
    });

    it('should refuse a list the document holding it can replace', () => {
      const replaceable = structuredClone(listElementSchemas);
      replaceable.electedCharter.documentsMutable = true;

      expect(() => buildListElementContract(replaceable)).to.throw(/can be changed by a replace/);
    });
  });

  describe('reference expressions', () => {
    /**
     * The moderation charter's resignation: the member is either the owner of
     * a join request for the charter, or the moderator an `addedModerator`
     * document names.
     */
    const expressionSchemas = {
      joinRequest: lookupSchemas.joinRequest,
      addedModerator: {
        type: 'object',
        canBeDeleted: false,
        documentsMutable: false,
        properties: {
          submittedCharterId: plainIdentifier,
          moderatorId: { ...plainIdentifier, position: 1 },
        },
        indices: [
          {
            name: 'byModerator',
            properties: [{ submittedCharterId: 'asc' }, { moderatorId: 'asc' }],
            unique: true,
          },
        ],
        required: ['submittedCharterId', 'moderatorId'],
        additionalProperties: false,
      },
      resignation: {
        type: 'object',
        properties: {
          submittedCharterId: plainIdentifier,
          memberId: identifierProperty(1, {
            anyOf: [
              (lookupSchemas.charter.properties.memberId as { refersTo: object }).refersTo,
              {
                type: 'permanentDocument',
                documentType: 'addedModerator',
                lookup: {
                  index: 'byModerator',
                  keys: { submittedCharterId: 'submittedCharterId', moderatorId: '.' },
                },
              },
            ],
          }),
        },
        required: ['submittedCharterId'],
        additionalProperties: false,
      },
    };

    function buildExpressionContract(documentSchemas: object) {
      return new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: documentSchemas,
        definitions: null,
        fullValidation: true,
        platformVersion: new PlatformVersion(14),
      });
    }

    it('should carry the operands of an anyOf in declared order, tagged anyOf', () => {
      const contract = buildExpressionContract(expressionSchemas);
      const [member] = contract.documentTypeReferences('resignation') as Reference[];

      expect(member.path).to.equal('memberId');
      expect(member.type).to.equal('anyOf');
      expect(member).to.not.have.property('allOf');
      expect(member.anyOf).to.have.lengthOf(2);
      const [joinRequest, addedModerator] = member.anyOf!;
      expect(joinRequest.type).to.equal('permanentDocument');
      expect(joinRequest.documentType).to.equal('joinRequest');
      expect(joinRequest.contractId!.toBase58()).to.equal(contract.id.toBase58());
      expect(joinRequest.lookup).to.deep.equal({
        index: 'bySubmittedCharter',
        keys: { $ownerId: '.', submittedCharterId: 'submittedCharterId' },
      });
      expect(addedModerator.documentType).to.equal('addedModerator');
      expect(addedModerator.lookup).to.deep.equal({
        index: 'byModerator',
        keys: { moderatorId: '.', submittedCharterId: 'submittedCharterId' },
      });
      // Each target is a target object of its own, never a nested anyOf
      expect(joinRequest).to.not.have.property('anyOf');
      expect(joinRequest).to.not.have.property('path');
    });

    it('should carry a deletableDocument operand found through a lookup, with its lookup', () => {
      // The leader takes an added moderator off by deleting the addition
      const deletable = structuredClone(expressionSchemas);
      deletable.addedModerator.canBeDeleted = true;
      const memberId = deletable.resignation.properties.memberId as { refersTo: { anyOf: { type: string }[] } };
      memberId.refersTo.anyOf[1].type = 'deletableDocument';
      const contract = buildExpressionContract(deletable);
      const [member] = contract.documentTypeReferences('resignation') as Reference[];

      const [, addedModerator] = member.anyOf!;
      expect(addedModerator.type).to.equal('deletableDocument');
      expect(addedModerator.documentType).to.equal('addedModerator');
      expect(addedModerator.lookup).to.deep.equal({
        index: 'byModerator',
        keys: { moderatorId: '.', submittedCharterId: 'submittedCharterId' },
      });
    });

    it('should refuse a deletableDocument operand found by id', () => {
      const byId = structuredClone(expressionSchemas);
      byId.addedModerator.canBeDeleted = true;
      const memberId = byId.resignation.properties.memberId as { refersTo: { anyOf: object[] } };
      memberId.refersTo.anyOf[1] = { type: 'deletableDocument', documentType: 'addedModerator' };

      expect(() => buildExpressionContract(byId)).to.throw();
    });

    it('should carry an anyOf the elements of a typed array declare', () => {
      const withMembers = structuredClone(expressionSchemas);
      const properties = withMembers.resignation.properties as Record<string, object>;
      properties.members = {
        type: 'array',
        maxItems: 15,
        items: {
          type: 'array',
          byteArray: true,
          minItems: 32,
          maxItems: 32,
          contentMediaType: 'application/x.dash.dpp.identifier',
          refersTo: { anyOf: [{ type: 'identity' }, { type: 'permanentDocument', documentType: 'joinRequest' }] },
        },
        position: 2,
      };
      const contract = buildExpressionContract(withMembers);
      const members = (contract.documentTypeReferences('resignation') as Reference[]).find(
        (reference) => reference.path === 'members[]',
      )!;

      expect(members.type).to.equal('anyOf');
      expect(members.anyOf!.map((target) => target.type)).to.deep.equal(['identity', 'permanentDocument']);
      expect(members.anyOf![1].documentType).to.equal('joinRequest');
    });

    it('should carry an allOf nested in an anyOf, each list under its own key and tagged', () => {
      const nested = structuredClone(expressionSchemas);
      const memberId = nested.resignation.properties.memberId as { refersTo: { anyOf: object[] } };
      const [joinRequest, addedModerator] = memberId.refersTo.anyOf;
      memberId.refersTo = {
        anyOf: [addedModerator, { allOf: [{ type: 'identity' }, joinRequest] }],
      };
      const contract = buildExpressionContract(nested);
      const [member] = contract.documentTypeReferences('resignation') as Reference[];

      expect(member.type).to.equal('anyOf');
      expect(member.anyOf).to.have.lengthOf(2);
      expect(member.anyOf![0].documentType).to.equal('addedModerator');
      const allOf = member.anyOf![1];
      expect(allOf.type).to.equal('allOf');
      expect(allOf).to.not.have.property('anyOf');
      expect(allOf.allOf!.map((operand) => operand.type)).to.deep.equal(['identity', 'permanentDocument']);
      expect(allOf.allOf![1].documentType).to.equal('joinRequest');
    });

    it('should refuse a leaf of a type an expression does not take', () => {
      const withContract = structuredClone(expressionSchemas);
      (withContract.resignation.properties.memberId as { refersTo: object }).refersTo = {
        anyOf: [{ type: 'identity' }, { type: 'contract' }],
      };

      expect(() => buildExpressionContract(withContract)).to.throw(
        /refersTo anyOf\[1\] is a reference of type contract, which a reference expression does not take/,
      );
    });

    it('should refuse an anyOf directly inside an anyOf', () => {
      const flat = structuredClone(expressionSchemas);
      const memberId = flat.resignation.properties.memberId as { refersTo: { anyOf: object[] } };
      memberId.refersTo = {
        anyOf: [{ type: 'identity' }, { anyOf: memberId.refersTo.anyOf }],
      };

      expect(() => buildExpressionContract(flat)).to.throw(
        /refersTo anyOf\[1\] is an anyOf directly inside an anyOf/,
      );
    });
  });

  describe('ownerRefersTo and creatorRefersTo', () => {
    /**
     * A `resignation` may only be written by the owner of a join request for
     * its own `submittedCharterId`: the document type's own reference, whose
     * value is the writer rather than a property's value.
     */
    const ownerSchemas = {
      joinRequest: lookupSchemas.joinRequest,
      resignation: {
        type: 'object',
        ownerRefersTo: {
          type: 'permanentDocument',
          documentType: 'joinRequest',
          lookup: {
            index: 'bySubmittedCharter',
            keys: { submittedCharterId: 'submittedCharterId', $ownerId: '.' },
          },
        },
        properties: {
          submittedCharterId: plainIdentifier,
          author: identifierProperty(1, { type: 'identity' }),
        },
        required: ['submittedCharterId'],
        additionalProperties: false,
      },
    };

    function buildOwnerContract(
      resignation: object,
      platformVersion = 14,
      fullValidation = true,
    ) {
      return new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: { joinRequest: ownerSchemas.joinRequest, resignation },
        definitions: null,
        fullValidation,
        platformVersion: new PlatformVersion(platformVersion),
      });
    }

    it('should list the owner reference first, at the path $ownerId', () => {
      const contract = buildOwnerContract(ownerSchemas.resignation);
      const references = contract.documentTypeReferences('resignation') as Reference[];

      expect(references.map((reference) => reference.path)).to.deep.equal([
        '$ownerId',
        'author',
      ]);
      const [writer] = references;
      expect(writer.type).to.equal('permanentDocument');
      expect(writer.documentType).to.equal('joinRequest');
      expect(writer.contractId!.toBase58()).to.equal(contract.id.toBase58());
      expect(writer.lookup).to.deep.equal({
        index: 'bySubmittedCharter',
        keys: { $ownerId: '.', submittedCharterId: 'submittedCharterId' },
      });
      expect(
        (contract.documentReferences as Map<string, Reference[]>).get('resignation')!
          .map((reference) => reference.path),
      ).to.deep.equal(['$ownerId', 'author']);
    });

    it('should report a document type declaring only an owner reference', () => {
      const onlyOwner = structuredClone(ownerSchemas.resignation) as {
        ownerRefersTo: object;
        properties: Record<string, object>;
      };
      delete onlyOwner.properties.author;
      onlyOwner.ownerRefersTo = { type: 'identity' };
      const contract = buildOwnerContract(onlyOwner);

      expect(contract.documentTypeReferences('resignation')).to.deep.equal([
        { path: '$ownerId', type: 'identity' },
      ]);
    });

    it('should refuse an owner reference to a target the writer can never be', () => {
      for (const ownerRefersTo of [
        { type: 'contract' },
        { type: 'token' },
        { type: 'identityPublicKey', identityProperty: '$ownerId' },
      ]) {
        const refused = { ...ownerSchemas.resignation, ownerRefersTo };
        expect(() => buildOwnerContract(refused)).to.throw();
        // The stored path refuses it too, where no meta-schema runs
        expect(() => buildOwnerContract(refused, 14, false)).to.throw(/ownerRefersTo does not take/);
      }
    });

    it('should list a creator reference first, at the path $creatorId', () => {
      const { ownerRefersTo, ...rest } = ownerSchemas.resignation;
      const badge = { ...rest, transferable: 1, creatorRefersTo: ownerRefersTo };
      const contract = buildOwnerContract(badge);
      const references = contract.documentTypeReferences('resignation') as Reference[];

      expect(references.map((reference) => reference.path)).to.deep.equal([
        '$creatorId',
        'author',
      ]);
      expect(references[0].type).to.equal('permanentDocument');
      expect(references[0].lookup).to.deep.equal({
        index: 'bySubmittedCharter',
        keys: { $ownerId: '.', submittedCharterId: 'submittedCharterId' },
      });
    });

    it('should refuse a creator reference on a type that records no creator ids', () => {
      const { ownerRefersTo, ...rest } = ownerSchemas.resignation;
      const notTransferable = { ...rest, creatorRefersTo: ownerRefersTo };

      expect(() => buildOwnerContract(notTransferable)).to.throw(/records no creator ids/);
    });

    it('should list an owner reference expression at the path $ownerId', () => {
      const { ownerRefersTo, ...rest } = ownerSchemas.resignation;
      const expression = { ...rest, ownerRefersTo: { anyOf: [ownerRefersTo, { type: 'identity' }] } };
      const contract = buildOwnerContract(expression);
      const [writer] = contract.documentTypeReferences('resignation') as Reference[];

      expect(writer.path).to.equal('$ownerId');
      expect(writer.type).to.equal('anyOf');
    });

    it('should report no owner reference on a pre-v14 contract', () => {
      const contract = buildOwnerContract(ownerSchemas.resignation, 13, false);

      expect(contract.documentTypeReferences('resignation')).to.deep.equal([]);
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
      expect(wasm.DocumentReferenceErrorCode.ReferencedDocumentTypeNotDeletable).to.equal(40131);
      expect(wasm.DocumentReferenceErrorCode.ReferencedContractRequirementNotMet).to.equal(40135);
      expect(wasm.DocumentReferenceErrorCode.ReferencedIdentityKeyRequirementNotMet).to.equal(40136);
      expect(wasm.DocumentReferenceErrorCode.ReferencedDocumentLookupInvalid).to.equal(40137);
      expect(wasm.DocumentReferenceErrorCode.ReferencedDocumentListInvalid).to.equal(40138);
    });

    it('should resolve a code back to its name', () => {
      const codes = wasm.DocumentReferenceErrorCode as unknown as Record<number, string>;

      expect(codes[40123]).to.equal('ReferencedIdentityKeyNotFound');
      expect(codes[40125]).to.equal('ReferencedKeyIdPropertyInvalid');
      expect(codes[40136]).to.equal('ReferencedIdentityKeyRequirementNotMet');
    });
  });
});
