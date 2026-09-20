import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

const OWNER_ID = '11111111111111111111111111111111';
const CONTRACT_ID = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';
const TARGET_ID = '2QjL594djCH2NyDsn45vd6yQjEDHupMKo7CEGVTHtQxU';

interface ModerationOptions {
  action?: 'ban' | 'unban' | 'suspend' | 'unsuspend';
  until?: bigint;
  identityContractNonce?: bigint;
  userFeeIncrease?: number;
}

function createTransition(options: ModerationOptions = {}) {
  return new wasm.ContractUserModeration({
    ownerId: OWNER_ID,
    dataContractId: CONTRACT_ID,
    identityContractNonce: options.identityContractNonce ?? BigInt(7),
    action: options.action ?? 'ban',
    identityId: TARGET_ID,
    until: options.until,
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
      expect(transition.identityId.toString()).to.equal(TARGET_ID);
      expect(transition.identityContractNonce).to.equal(BigInt(7));
      expect(transition.until).to.equal(undefined);
      expect(transition.userFeeIncrease).to.equal(0);
    });

    it('should create a suspension with its end', () => {
      const transition = createTransition({ action: 'suspend', until: BigInt(1800000000000) });

      expect(transition.action).to.equal('suspend');
      expect(transition.until).to.equal(BigInt(1800000000000));
    });

    it('should refuse a suspension without an end', () => {
      expect(() => createTransition({ action: 'suspend' })).to.throw();
    });

    it('should refuse an end on anything but a suspension', () => {
      // Dropped, a caller asking for a timed ban would sign a permanent one.
      (['ban', 'unban', 'unsuspend'] as const).forEach((action) => {
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
      expect(json.action).to.deep.equal({ $type: 'suspend', identityId: TARGET_ID, until: 1800000000000 });
      expect(json.userFeeIncrease).to.equal(4);
      expect(json.signature).to.equal('');
      expect(json.signaturePublicKeyId).to.equal(0);
    });
  });

  describe('fromJSON()', () => {
    it('should restore the transition from JSON', () => {
      const transition = createTransition({ action: 'unban', userFeeIncrease: 4 });

      const restored = wasm.ContractUserModeration.fromJSON(transition.toJSON());

      expect(restored.action).to.equal('unban');
      expect(restored.identityId.toString()).to.equal(TARGET_ID);
      expect(restored.identityContractNonce).to.equal(BigInt(7));
      expect(restored.userFeeIncrease).to.equal(4);
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });
  });

  describe('toObject() / fromObject()', () => {
    it('should round trip through a plain object', () => {
      const transition = createTransition({ action: 'suspend', until: BigInt(9) });

      const obj = transition.toObject();
      expect(obj.$formatVersion).to.equal('0');
      expect(obj.ownerId).to.be.instanceOf(Uint8Array);
      expect(obj.dataContractId.length).to.equal(32);
      expect(obj.identityContractNonce).to.equal(BigInt(7));
      expect(obj.action.$type).to.equal('suspend');
      expect(obj.action.until).to.equal(BigInt(9));

      const restored = wasm.ContractUserModeration.fromObject(obj);
      expect(restored.until).to.equal(BigInt(9));
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
