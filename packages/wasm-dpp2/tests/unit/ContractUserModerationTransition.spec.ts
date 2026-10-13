import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

const OWNER_ID = '11111111111111111111111111111111';
const CONTRACT_ID = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';
const TARGET_ID = '2QjL594djCH2NyDsn45vd6yQjEDHupMKo7CEGVTHtQxU';
const DOCUMENT_ID = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
const DOCUMENT_TYPE_NAME = 'post';
/** A team action another member of the seated team proposed */
const ACTION_ID = 'cGfHiC6Kgg3FpFZvgwGcswsCRtp4aBP2fzuXRQPizuN';

interface ModerationOptions {
  action?: 'ban' | 'unban' | 'suspend' | 'unsuspend' | 'warn' | 'clearWarnings' | 'deleteDocument' | 'restoreDocument' | 'changeDocumentFields' | 'deleteSettledDocument' | 'approveTeamAction';
  /** `null` leaves the identity out; left undefined, every action but one naming a document or a team action gets `TARGET_ID` */
  identityId?: string | null;
  /** `null` leaves it out; left undefined, every action naming a document gets `DOCUMENT_TYPE_NAME` */
  documentTypeName?: string | null;
  /** `null` leaves it out; left undefined, a deleteDocument, a changeDocumentFields or a deleteSettledDocument gets `DOCUMENT_ID` */
  documentId?: string | null;
  /** `null` leaves it out; left undefined, a restoreDocument gets `DOCUMENT_BYTES` */
  document?: Uint8Array | null;
  /** `null` leaves them out; left undefined, a changeDocumentFields gets `FIELDS` */
  fields?: Record<string, unknown> | null;
  /** `null` leaves it out; left undefined, an approveTeamAction gets `ACTION_ID` */
  actionId?: string | null;
  until?: bigint;
  /** `null` leaves the reason out; left undefined, a ban, a suspend, a warn and a deleteSettledDocument get `REASON` */
  reason?: {
    code?: number;
    text: string;
    documents?: { documentTypeName: string; documentId: string }[];
    reasonDocumentId?: string;
  } | null;
  identityContractNonce?: bigint;
  userFeeIncrease?: number;
}

const REASON = { text: 'spam' };
/** What a restore carries: the document as it was serialized when it was deleted */
const DOCUMENT_BYTES = new Uint8Array(70).fill(7);
/** What a field change sets: a status, and a resolution removed */
const FIELDS = { status: 2, resolution: null };

function createTransition(options: ModerationOptions = {}) {
  const action = options.action ?? 'ban';
  const addsAnEntry = action === 'ban' || action === 'suspend' || action === 'warn';
  // The proposal of a settled document's deletion names the reason its approvals approve.
  const proposesASettledDeletion = action === 'deleteSettledDocument';
  let { reason } = options;
  if (reason === undefined && (addsAnEntry || proposesASettledDeletion)) {
    reason = REASON;
  }
  // A deletion, a restore, a field change and the proposal of a settled document's deletion
  // name a document, an approval a team action, and every other action an identity.
  const deletesADocument = action === 'deleteDocument';
  const restoresADocument = action === 'restoreDocument';
  const changesADocument = action === 'changeDocumentFields';
  const approvesATeamAction = action === 'approveTeamAction';
  const namesADocument = deletesADocument || restoresADocument || changesADocument
    || proposesASettledDeletion;
  let {
    identityId, documentTypeName, documentId, document, fields, actionId,
  } = options;
  if (identityId === undefined && !namesADocument && !approvesATeamAction) {
    identityId = TARGET_ID;
  }
  if (documentTypeName === undefined && namesADocument) {
    documentTypeName = DOCUMENT_TYPE_NAME;
  }
  if (documentId === undefined && (deletesADocument || changesADocument || proposesASettledDeletion)) {
    documentId = DOCUMENT_ID;
  }
  if (document === undefined && restoresADocument) {
    document = DOCUMENT_BYTES;
  }
  if (fields === undefined && changesADocument) {
    fields = FIELDS;
  }
  if (actionId === undefined && approvesATeamAction) {
    actionId = ACTION_ID;
  }

  return new wasm.ContractUserModeration({
    ownerId: OWNER_ID,
    dataContractId: CONTRACT_ID,
    identityContractNonce: options.identityContractNonce ?? BigInt(7),
    action,
    identityId: identityId ?? undefined,
    documentTypeName: documentTypeName ?? undefined,
    documentId: documentId ?? undefined,
    document: document ?? undefined,
    fields: fields ?? undefined,
    actionId: actionId ?? undefined,
    until: options.until,
    reason: reason ?? undefined,
    userFeeIncrease: options.userFeeIncrease,
  });
}

describe('ContractUserModeration', () => {
  describe('constructor()', () => {
    it('should create a ban', () => {
      const transition = createTransition();

      expect(transition).to.be.an.instanceof(wasm.ContractUserModeration);
      expect(transition.action).to.equal('ban');
      expect(transition.ownerId.toString()).to.equal(OWNER_ID);
      expect(transition.dataContractId.toString()).to.equal(CONTRACT_ID);
      expect(transition.identityId?.toString()).to.equal(TARGET_ID);
      expect(transition.documentTypeName).to.equal(undefined);
      expect(transition.documentId).to.equal(undefined);
      expect(transition.identityContractNonce).to.equal(BigInt(7));
      expect(transition.until).to.equal(undefined);
      expect(transition.reason).to.deep.equal({ code: null, text: 'spam' });
      expect(transition.userFeeIncrease).to.equal(0);
    });

    it('should create a document deletion, which names a document and no identity', () => {
      const transition = createTransition({
        action: 'deleteDocument',
        reason: { code: 2, text: 'spam' },
      });

      expect(transition.action).to.equal('deleteDocument');
      expect(transition.documentTypeName).to.equal(DOCUMENT_TYPE_NAME);
      expect(transition.documentId?.toString()).to.equal(DOCUMENT_ID);
      expect(transition.identityId).to.equal(undefined);
      expect(transition.until).to.equal(undefined);
      expect(transition.reason).to.deep.equal({ code: 2, text: 'spam' });
    });

    it('should create a document deletion without a reason, stored as no code and an empty text', () => {
      const transition = createTransition({ action: 'deleteDocument' });

      expect(transition.reason).to.deep.equal({ code: null, text: '' });
      // Left out or written empty, the reason signed is the same.
      expect(transition.toBytes()).to.deep.equal(
        createTransition({ action: 'deleteDocument', reason: { text: '' } }).toBytes(),
      );
    });

    it('should refuse a document deletion without its document type or its document', () => {
      expect(() => createTransition({ action: 'deleteDocument', documentTypeName: null })).to.throw();
      expect(() => createTransition({ action: 'deleteDocument', documentId: null })).to.throw();
    });

    it('should refuse an identity or an end on a document deletion', () => {
      expect(() => createTransition({ action: 'deleteDocument', identityId: TARGET_ID })).to.throw();
      expect(() => createTransition({ action: 'deleteDocument', until: BigInt(5) })).to.throw();
    });

    it('should create a document restore, which carries the document and names no identity', () => {
      const transition = createTransition({ action: 'restoreDocument' });

      expect(transition.action).to.equal('restoreDocument');
      expect(transition.documentTypeName).to.equal(DOCUMENT_TYPE_NAME);
      expect(transition.document).to.deep.equal(DOCUMENT_BYTES);
      expect(transition.documentId).to.equal(undefined);
      expect(transition.identityId).to.equal(undefined);
      expect(transition.until).to.equal(undefined);
      expect(transition.reason).to.equal(undefined);
    });

    it('should refuse a document restore without its document type or its document', () => {
      expect(() => createTransition({ action: 'restoreDocument', documentTypeName: null })).to.throw();
      expect(() => createTransition({ action: 'restoreDocument', document: null })).to.throw();
    });

    it('should refuse on a document restore what it does not carry', () => {
      expect(() => createTransition({ action: 'restoreDocument', identityId: TARGET_ID })).to.throw();
      expect(() => createTransition({ action: 'restoreDocument', documentId: DOCUMENT_ID })).to.throw();
      expect(() => createTransition({ action: 'restoreDocument', reason: REASON })).to.throw();
      expect(() => createTransition({ action: 'restoreDocument', until: BigInt(5) })).to.throw();
      expect(() => createTransition({ action: 'deleteDocument', document: DOCUMENT_BYTES })).to.throw();
    });

    it('should create a field change, which names a document and the fields it sets', () => {
      const transition = createTransition({
        action: 'changeDocumentFields',
        reason: { code: 4, text: 'handled' },
      });

      expect(transition.action).to.equal('changeDocumentFields');
      expect(transition.documentTypeName).to.equal(DOCUMENT_TYPE_NAME);
      expect(transition.documentId?.toString()).to.equal(DOCUMENT_ID);
      // Read back as a document's properties are, integers as bigints; a removal stays null.
      expect(transition.fields).to.deep.equal({ status: BigInt(2), resolution: null });
      expect(transition.identityId).to.equal(undefined);
      expect(transition.document).to.equal(undefined);
      expect(transition.reason).to.deep.equal({ code: 4, text: 'handled' });
    });

    it('should create a field change without a reason, stored as no code and an empty text', () => {
      const transition = createTransition({ action: 'changeDocumentFields' });

      expect(transition.reason).to.deep.equal({ code: null, text: '' });
    });

    it('should refuse a field change without what it names, and fields beside another action', () => {
      expect(() => createTransition({ action: 'changeDocumentFields', fields: null })).to.throw();
      expect(() => createTransition({ action: 'changeDocumentFields', documentId: null })).to.throw();
      expect(() => createTransition({ action: 'changeDocumentFields', documentTypeName: null })).to.throw();
      expect(() => createTransition({ action: 'changeDocumentFields', identityId: TARGET_ID })).to.throw();
      expect(() => createTransition({ action: 'changeDocumentFields', document: DOCUMENT_BYTES })).to.throw();
      expect(() => createTransition({ action: 'deleteDocument', fields: FIELDS })).to.throw();
      expect(() => createTransition({ action: 'ban', fields: FIELDS })).to.throw();
      expect(createTransition({ action: 'ban' }).fields).to.equal(undefined);
    });

    it('should leave out a field set to undefined, and remove only one set to null', () => {
      const transition = createTransition({
        action: 'changeDocumentFields',
        fields: { status: 2, resolution: undefined, note: null },
      });

      expect(transition.fields).to.deep.equal({ status: BigInt(2), note: null });
    });

    it('should give back a field named __proto__ as a field', () => {
      const fields = JSON.parse('{"__proto__":{"status":1},"resolution":"done"}');
      const transition = createTransition({ action: 'changeDocumentFields', fields });

      const returned = transition.fields as Record<string, unknown>;
      expect(Object.keys(returned)).to.have.members(['__proto__', 'resolution']);
      expect(Object.getPrototypeOf(returned)).to.equal(Object.prototype);
    });

    it('should sign the same bytes once read back from its object', () => {
      const transition = createTransition({
        action: 'changeDocumentFields',
        fields: {
          status: 2,
          reviewer: new Uint8Array(32).fill(7),
          attachment: new Uint8Array([1, 2, 3]),
        },
      });

      const restored = wasm.ContractUserModeration.fromObject(transition.toObject());

      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });

    it('should create a proposal of the deletion of a settled document, which names a document and no identity', () => {
      const reason = { code: 2, text: 'doxxing', reasonDocumentId: TARGET_ID };
      const transition = createTransition({ action: 'deleteSettledDocument', reason });

      expect(transition.action).to.equal('deleteSettledDocument');
      expect(transition.documentTypeName).to.equal(DOCUMENT_TYPE_NAME);
      expect(transition.documentId?.toString()).to.equal(DOCUMENT_ID);
      expect(transition.identityId).to.equal(undefined);
      expect(transition.document).to.equal(undefined);
      expect(transition.fields).to.equal(undefined);
      expect(transition.until).to.equal(undefined);
      expect(transition.reason).to.deep.equal(reason);
    });

    it('should give a proposal of the deletion of a settled document the id of the team action it opens', () => {
      const proposal = createTransition({ action: 'deleteSettledDocument' });
      const actionId = proposal.actionId?.toString();

      expect(actionId).to.be.a('string');
      // Computed from the contract, the proposer, its nonce, the document and the reason:
      // the same proposal read back opens the same action, and the next nonce another.
      expect(wasm.ContractUserModeration.fromBytes(proposal.toBytes()).actionId?.toString())
        .to.equal(actionId);
      expect(createTransition({ action: 'deleteSettledDocument', identityContractNonce: BigInt(8) })
        .actionId?.toString()).to.not.equal(actionId);
      // Every other action but an approval names no team action.
      expect(createTransition({ action: 'deleteDocument' }).actionId).to.equal(undefined);
      expect(createTransition().actionId).to.equal(undefined);
    });

    it('should refuse a proposal of the deletion of a settled document without its reason, its document type or its document', () => {
      // Unlike a deletion's, the reason is needed: the approvals approve the deletion for it.
      expect(() => createTransition({ action: 'deleteSettledDocument', reason: null })).to.throw();
      expect(() => createTransition({ action: 'deleteSettledDocument', documentTypeName: null })).to.throw();
      expect(() => createTransition({ action: 'deleteSettledDocument', documentId: null })).to.throw();
    });

    it('should refuse on a proposal of the deletion of a settled document what it does not carry', () => {
      expect(() => createTransition({ action: 'deleteSettledDocument', identityId: TARGET_ID })).to.throw();
      expect(() => createTransition({ action: 'deleteSettledDocument', until: BigInt(5) })).to.throw();
      expect(() => createTransition({ action: 'deleteSettledDocument', document: DOCUMENT_BYTES })).to.throw();
      expect(() => createTransition({ action: 'deleteSettledDocument', fields: FIELDS })).to.throw();
      expect(() => createTransition({ action: 'deleteSettledDocument', actionId: ACTION_ID })).to.throw();
    });

    it('should create an approval of a team action, which names the action and nothing else', () => {
      const transition = createTransition({ action: 'approveTeamAction' });

      expect(transition.action).to.equal('approveTeamAction');
      expect(transition.actionId?.toString()).to.equal(ACTION_ID);
      expect(transition.identityId).to.equal(undefined);
      expect(transition.documentTypeName).to.equal(undefined);
      expect(transition.documentId).to.equal(undefined);
      expect(transition.document).to.equal(undefined);
      expect(transition.fields).to.equal(undefined);
      expect(transition.until).to.equal(undefined);
      // What the action does and why are the proposal's.
      expect(transition.reason).to.equal(undefined);
    });

    it('should refuse an approval of a team action without its action id, and an action id beside another action', () => {
      expect(() => createTransition({ action: 'approveTeamAction', actionId: null })).to.throw();
      (['ban', 'unban', 'unsuspend', 'warn', 'clearWarnings', 'deleteDocument', 'restoreDocument', 'changeDocumentFields'] as const)
        .forEach((action) => {
          expect(() => createTransition({ action, actionId: ACTION_ID })).to.throw();
        });
    });

    it('should refuse on an approval of a team action what it does not carry', () => {
      expect(() => createTransition({ action: 'approveTeamAction', identityId: TARGET_ID })).to.throw();
      expect(() => createTransition({ action: 'approveTeamAction', documentTypeName: DOCUMENT_TYPE_NAME })).to.throw();
      expect(() => createTransition({ action: 'approveTeamAction', documentId: DOCUMENT_ID })).to.throw();
      expect(() => createTransition({ action: 'approveTeamAction', document: DOCUMENT_BYTES })).to.throw();
      expect(() => createTransition({ action: 'approveTeamAction', fields: FIELDS })).to.throw();
      expect(() => createTransition({ action: 'approveTeamAction', reason: REASON })).to.throw();
      expect(() => createTransition({ action: 'approveTeamAction', until: BigInt(5) })).to.throw();
    });

    it('should create a warning and its clearing, which name an identity', () => {
      const warn = createTransition({ action: 'warn', reason: { code: 1, text: 'first strike' } });

      expect(warn.action).to.equal('warn');
      expect(warn.identityId?.toString()).to.equal(TARGET_ID);
      expect(warn.until).to.equal(undefined);
      expect(warn.reason).to.deep.equal({ code: 1, text: 'first strike' });

      const clear = createTransition({ action: 'clearWarnings' });

      expect(clear.action).to.equal('clearWarnings');
      expect(clear.identityId?.toString()).to.equal(TARGET_ID);
      expect(clear.reason).to.equal(undefined);
      expect(clear.toJSON().action).to.deep.equal({ $type: 'clearWarnings', identityId: TARGET_ID });
    });

    it('should refuse a warning without a reason, and a reason on a clearing', () => {
      expect(() => createTransition({ action: 'warn', reason: null })).to.throw();
      expect(() => createTransition({ action: 'clearWarnings', reason: REASON })).to.throw();
    });

    it('should refuse a document on an action that targets an identity', () => {
      (['ban', 'unban', 'unsuspend', 'warn', 'clearWarnings'] as const).forEach((action) => {
        expect(() => createTransition({ action, documentTypeName: DOCUMENT_TYPE_NAME })).to.throw();
        expect(() => createTransition({ action, documentId: DOCUMENT_ID })).to.throw();
        expect(() => createTransition({ action, document: DOCUMENT_BYTES })).to.throw();
      });
      expect(() => createTransition({
        action: 'suspend',
        until: BigInt(5),
        documentTypeName: DOCUMENT_TYPE_NAME,
        documentId: DOCUMENT_ID,
      })).to.throw();
    });

    it('should refuse an action on an identity without the identity', () => {
      (['ban', 'unban', 'unsuspend', 'warn', 'clearWarnings'] as const).forEach((action) => {
        expect(() => createTransition({ action, identityId: null })).to.throw();
      });
      expect(() => createTransition({ action: 'suspend', until: BigInt(5), identityId: null })).to.throw();
    });

    it('should cite the documents a reason is about, and leave them out when none', () => {
      const documents = [
        { documentTypeName: 'post', documentId: DOCUMENT_ID },
        { documentTypeName: 'reply', documentId: TARGET_ID },
      ];
      const transition = createTransition({ action: 'warn', reason: { text: 'spam', documents } });

      expect(transition.reason).to.deep.equal({ code: null, text: 'spam', documents });
      expect(transition.toJSON().action.reason).to.deep.equal({ code: null, text: 'spam', documents });
      expect(wasm.ContractUserModeration.fromBytes(transition.toBytes()).reason).to.deep.equal({
        code: null,
        text: 'spam',
        documents,
      });
      // A reason about no document carries no `documents`, as before.
      expect(createTransition().reason).to.deep.equal({ code: null, text: 'spam' });
    });

    it('should name the reason document a reason is taken on, and leave it out when none', () => {
      const reason = { text: 'spam', reasonDocumentId: DOCUMENT_ID };
      const transition = createTransition({ action: 'ban', reason });

      expect(transition.reason).to.deep.equal({ code: null, ...reason });
      expect(transition.toJSON().action.reason).to.deep.equal({ code: null, ...reason });
      expect(wasm.ContractUserModeration.fromBytes(transition.toBytes()).reason).to.deep.equal({
        code: null,
        ...reason,
      });
      // A reason naming no reason document carries no `reasonDocumentId`, as before.
      expect(createTransition().reason).to.deep.equal({ code: null, text: 'spam' });
    });

    it('should keep a reason code as written, and an empty text', () => {
      const transition = createTransition({ reason: { code: 65535, text: '' } });

      expect(transition.reason).to.deep.equal({ code: 65535, text: '' });
    });

    it('should refuse a ban or a suspension without a reason', () => {
      expect(() => createTransition({ action: 'ban', reason: null })).to.throw();
      expect(() => createTransition({ action: 'suspend', until: BigInt(5), reason: null })).to.throw();
    });

    it('should refuse a reason on an unban or an unsuspend', () => {
      (['unban', 'unsuspend'] as const).forEach((action) => {
        expect(() => createTransition({ action, reason: REASON })).to.throw();
      });
    });

    it('should refuse a reason code that is not a u16', () => {
      expect(() => createTransition({ reason: { code: 65536, text: 'spam' } })).to.throw();
    });

    it('should create a suspension with its end', () => {
      const transition = createTransition({ action: 'suspend', until: BigInt(1800000000000) });

      expect(transition.action).to.equal('suspend');
      expect(transition.until).to.equal(BigInt(1800000000000));
      expect(transition.reason).to.deep.equal({ code: null, text: 'spam' });
    });

    it('should refuse a suspension without an end', () => {
      expect(() => createTransition({ action: 'suspend' })).to.throw();
    });

    it('should refuse an end on anything but a suspension', () => {
      // Dropped, a caller asking for a timed ban would sign a permanent one.
      (['ban', 'unban', 'unsuspend', 'warn', 'clearWarnings'] as const).forEach((action) => {
        expect(() => createTransition({ action, until: BigInt(1800000000000) })).to.throw();
      });
    });

    it('should refuse an unknown action', () => {
      expect(() => new wasm.ContractUserModeration({
        ownerId: OWNER_ID,
        dataContractId: CONTRACT_ID,
        identityContractNonce: BigInt(1),
        action: 'mute' as never,
        identityId: TARGET_ID,
      })).to.throw();
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('should round trip every action through bytes, base64 and hex', () => {
      for (const transition of [
        createTransition({ action: 'ban' }),
        createTransition({ action: 'unban' }),
        createTransition({ action: 'suspend', until: BigInt(5) }),
        createTransition({ action: 'unsuspend', userFeeIncrease: 3 }),
        createTransition({ action: 'warn', reason: { code: 9, text: 'first strike' } }),
        createTransition({ action: 'clearWarnings' }),
        createTransition({ action: 'deleteDocument', reason: { code: 9, text: 'spam' } }),
        createTransition({ action: 'deleteDocument' }),
        createTransition({ action: 'changeDocumentFields', reason: { text: 'handled' } }),
        createTransition({ action: 'deleteSettledDocument', reason: { text: 'doxxing', reasonDocumentId: TARGET_ID } }),
        createTransition({ action: 'approveTeamAction' }),
      ]) {
        const bytes = transition.toBytes();
        expect(wasm.ContractUserModeration.fromBytes(bytes).toBytes()).to.deep.equal(bytes);
        expect(wasm.ContractUserModeration.fromBase64(transition.toBase64()).toBytes()).to.deep.equal(bytes);
        expect(wasm.ContractUserModeration.fromHex(transition.toHex()).toBytes()).to.deep.equal(bytes);
      }
    });
  });

  describe('setters', () => {
    it('should update the nonce, the ids and the fee', () => {
      const transition = createTransition();

      transition.identityContractNonce = BigInt(8);
      transition.userFeeIncrease = 2;
      transition.ownerId = TARGET_ID;
      transition.dataContractId = OWNER_ID;

      expect(transition.identityContractNonce).to.equal(BigInt(8));
      expect(transition.userFeeIncrease).to.equal(2);
      expect(transition.ownerId.toString()).to.equal(TARGET_ID);
      expect(transition.dataContractId.toString()).to.equal(OWNER_ID);
    });
  });

  describe('toJSON()', () => {
    it('should produce the expected JSON structure', () => {
      const json = createTransition({ action: 'suspend', until: BigInt(1800000000000), userFeeIncrease: 4 }).toJSON();

      expect(json.$formatVersion).to.equal('0');
      expect(json.ownerId).to.equal(OWNER_ID);
      expect(json.dataContractId).to.equal(CONTRACT_ID);
      expect(json.identityContractNonce).to.equal(7);
      expect(json.action).to.deep.equal({
        $type: 'suspend',
        identityId: TARGET_ID,
        until: 1800000000000,
        reason: { code: null, text: 'spam' },
      });
      // The getter gives the reason the shape it has in the JSON
      expect(createTransition({ action: 'suspend', until: BigInt(5) }).reason).to.deep.equal(json.action.reason);
      expect(json.userFeeIncrease).to.equal(4);
      expect(json.signature).to.equal('');
      expect(json.signaturePublicKeyId).to.equal(0);
    });

    it('should tag a field change and name its document and its fields', () => {
      const json = createTransition({ action: 'changeDocumentFields', reason: { code: 4, text: 'handled' } }).toJSON();

      expect(json.action).to.deep.equal({
        $type: 'changeDocumentFields',
        documentTypeName: DOCUMENT_TYPE_NAME,
        documentId: DOCUMENT_ID,
        fields: FIELDS,
        reason: { code: 4, text: 'handled' },
      });
    });

    it('should tag a document deletion and name its document', () => {
      const json = createTransition({ action: 'deleteDocument', reason: { code: 2, text: 'spam' } }).toJSON();

      expect(json.action).to.deep.equal({
        $type: 'deleteDocument',
        documentTypeName: DOCUMENT_TYPE_NAME,
        documentId: DOCUMENT_ID,
        reason: { code: 2, text: 'spam' },
      });
    });

    it('should tag a proposal of the deletion of a settled document and name its document', () => {
      const reason = { code: 2, text: 'doxxing', reasonDocumentId: TARGET_ID };
      const json = createTransition({ action: 'deleteSettledDocument', reason }).toJSON();

      expect(json.action).to.deep.equal({
        $type: 'deleteSettledDocument',
        documentTypeName: DOCUMENT_TYPE_NAME,
        documentId: DOCUMENT_ID,
        reason,
      });
    });

    it('should tag an approval of a team action and name the action alone', () => {
      const json = createTransition({ action: 'approveTeamAction' }).toJSON();

      expect(json.action).to.deep.equal({
        $type: 'approveTeamAction',
        actionId: ACTION_ID,
      });
    });
  });

  describe('fromJSON()', () => {
    it('should restore the transition from JSON', () => {
      const transition = createTransition({ action: 'unban', userFeeIncrease: 4 });

      const restored = wasm.ContractUserModeration.fromJSON(transition.toJSON());

      expect(restored.reason).to.equal(undefined);
      expect(restored.action).to.equal('unban');
      expect(restored.identityId?.toString()).to.equal(TARGET_ID);
      expect(restored.identityContractNonce).to.equal(BigInt(7));
      expect(restored.userFeeIncrease).to.equal(4);
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });

    it('should restore a document deletion from JSON', () => {
      const transition = createTransition({ action: 'deleteDocument', reason: { text: 'spam' } });

      const restored = wasm.ContractUserModeration.fromJSON(transition.toJSON());

      expect(restored.action).to.equal('deleteDocument');
      expect(restored.documentTypeName).to.equal(DOCUMENT_TYPE_NAME);
      expect(restored.documentId?.toString()).to.equal(DOCUMENT_ID);
      expect(restored.identityId).to.equal(undefined);
      expect(restored.reason).to.deep.equal({ code: null, text: 'spam' });
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });

    it('should restore a proposal of the deletion of a settled document from JSON', () => {
      const transition = createTransition({ action: 'deleteSettledDocument', reason: { text: 'doxxing' } });

      const restored = wasm.ContractUserModeration.fromJSON(transition.toJSON());

      expect(restored.action).to.equal('deleteSettledDocument');
      expect(restored.documentTypeName).to.equal(DOCUMENT_TYPE_NAME);
      expect(restored.documentId?.toString()).to.equal(DOCUMENT_ID);
      expect(restored.identityId).to.equal(undefined);
      expect(restored.reason).to.deep.equal({ code: null, text: 'doxxing' });
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });

    it('should restore an approval of a team action from JSON', () => {
      const transition = createTransition({ action: 'approveTeamAction' });

      const restored = wasm.ContractUserModeration.fromJSON(transition.toJSON());

      expect(restored.action).to.equal('approveTeamAction');
      expect(restored.actionId?.toString()).to.equal(ACTION_ID);
      expect(restored.identityId).to.equal(undefined);
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });
  });

  describe('toObject() / fromObject()', () => {
    it('should round trip through a plain object', () => {
      const transition = createTransition({
        action: 'suspend',
        until: BigInt(9),
        reason: { code: 3, text: 'flooding' },
      });

      const obj = transition.toObject();
      expect(obj.$formatVersion).to.equal('0');
      expect(obj.ownerId).to.be.instanceOf(Uint8Array);
      expect(obj.dataContractId.length).to.equal(32);
      expect(obj.identityContractNonce).to.equal(BigInt(7));
      expect(obj.action.$type).to.equal('suspend');
      expect(obj.action.until).to.equal(BigInt(9));

      expect(obj.action.reason).to.deep.equal({ code: 3, text: 'flooding' });

      const restored = wasm.ContractUserModeration.fromObject(obj);
      expect(restored.until).to.equal(BigInt(9));
      expect(restored.reason).to.deep.equal({ code: 3, text: 'flooding' });
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });

    it('should round trip a document deletion through a plain object', () => {
      const transition = createTransition({ action: 'deleteDocument', reason: { code: 3, text: 'spam' } });

      const obj = transition.toObject();
      expect(obj.action.$type).to.equal('deleteDocument');
      expect(obj.action.documentTypeName).to.equal(DOCUMENT_TYPE_NAME);
      expect(obj.action.documentId).to.be.instanceOf(Uint8Array);
      expect(obj.action.identityId).to.equal(undefined);
      expect(obj.action.reason).to.deep.equal({ code: 3, text: 'spam' });

      const restored = wasm.ContractUserModeration.fromObject(obj);
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });

    it('should round trip a proposal of the deletion of a settled document through a plain object', () => {
      const transition = createTransition({ action: 'deleteSettledDocument', reason: { code: 3, text: 'doxxing' } });

      const obj = transition.toObject();
      expect(obj.action.$type).to.equal('deleteSettledDocument');
      expect(obj.action.documentTypeName).to.equal(DOCUMENT_TYPE_NAME);
      expect(obj.action.documentId).to.be.instanceOf(Uint8Array);
      expect(obj.action.identityId).to.equal(undefined);
      expect(obj.action.reason).to.deep.equal({ code: 3, text: 'doxxing' });

      const restored = wasm.ContractUserModeration.fromObject(obj);
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });

    it('should round trip an approval of a team action through a plain object', () => {
      const transition = createTransition({ action: 'approveTeamAction' });

      const obj = transition.toObject();
      expect(obj.action.$type).to.equal('approveTeamAction');
      expect(obj.action.actionId).to.be.instanceOf(Uint8Array);
      expect(obj.action.identityId).to.equal(undefined);
      expect(obj.action.reason).to.equal(undefined);

      const restored = wasm.ContractUserModeration.fromObject(obj);
      expect(restored.actionId?.toString()).to.equal(ACTION_ID);
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });
  });

  describe('toStateTransition() / fromStateTransition()', () => {
    it('should convert to and from a generic state transition', () => {
      const transition = createTransition();
      const generic = transition.toStateTransition();

      expect(generic.actionTypeNumber).to.equal(24);
      expect(generic.identityContractNonce).to.equal(BigInt(7));
      expect(wasm.ContractUserModeration.fromStateTransition(generic).toBytes()).to.deep.equal(transition.toBytes());
    });
  });
});
