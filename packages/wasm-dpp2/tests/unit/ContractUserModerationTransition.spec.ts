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

interface ModerationOptions {
  action?: 'ban' | 'unban' | 'suspend' | 'unsuspend' | 'warn' | 'clearWarnings' | 'deleteDocument';
  /** `null` leaves the identity out; left undefined, every action but a deleteDocument gets `TARGET_ID` */
  identityId?: string | null;
  /** `null` leaves it out; left undefined, a deleteDocument gets `DOCUMENT_TYPE_NAME` */
  documentTypeName?: string | null;
  /** `null` leaves it out; left undefined, a deleteDocument gets `DOCUMENT_ID` */
  documentId?: string | null;
  until?: bigint;
  /** `null` leaves the reason out; left undefined, a ban, a suspend and a warn get `REASON` */
  reason?: { code?: number; text: string } | null;
  identityContractNonce?: bigint;
  userFeeIncrease?: number;
}

const REASON = { text: 'spam' };

function createTransition(options: ModerationOptions = {}) {
  const action = options.action ?? 'ban';
  const addsAnEntry = action === 'ban' || action === 'suspend' || action === 'warn';
  let { reason } = options;
  if (reason === undefined && addsAnEntry) {
    reason = REASON;
  }
  // A deletion names a document and every other action an identity.
  const deletesADocument = action === 'deleteDocument';
  let { identityId, documentTypeName, documentId } = options;
  if (identityId === undefined && !deletesADocument) {
    identityId = TARGET_ID;
  }
  if (documentTypeName === undefined && deletesADocument) {
    documentTypeName = DOCUMENT_TYPE_NAME;
  }
  if (documentId === undefined && deletesADocument) {
    documentId = DOCUMENT_ID;
  }

  return new wasm.ContractUserModeration({
    ownerId: OWNER_ID,
    dataContractId: CONTRACT_ID,
    identityContractNonce: options.identityContractNonce ?? BigInt(7),
    action,
    identityId: identityId ?? undefined,
    documentTypeName: documentTypeName ?? undefined,
    documentId: documentId ?? undefined,
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

    it('should tag a document deletion and name its document', () => {
      const json = createTransition({ action: 'deleteDocument', reason: { code: 2, text: 'spam' } }).toJSON();

      expect(json.action).to.deep.equal({
        $type: 'deleteDocument',
        documentTypeName: DOCUMENT_TYPE_NAME,
        documentId: DOCUMENT_ID,
        reason: { code: 2, text: 'spam' },
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
