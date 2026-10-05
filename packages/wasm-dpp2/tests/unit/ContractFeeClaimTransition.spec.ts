import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

const OWNER_ID = '11111111111111111111111111111111';
const CONTRACT_ID = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

interface FeeClaimOptions {
  pot?: 'owner' | 'moderators';
  identityContractNonce?: bigint;
  userFeeIncrease?: number;
}

function createTransition(options: FeeClaimOptions = {}) {
  return new wasm.ContractFeeClaim({
    ownerId: OWNER_ID,
    dataContractId: CONTRACT_ID,
    identityContractNonce: options.identityContractNonce ?? BigInt(7),
    pot: options.pot ?? 'owner',
    userFeeIncrease: options.userFeeIncrease,
  });
}

describe('ContractFeeClaim', () => {
  describe('constructor()', () => {
    it('should create a claim of the owner pot', () => {
      const transition = createTransition();

      expect(transition).to.be.an.instanceof(wasm.ContractFeeClaim);
      expect(transition.pot).to.equal('owner');
      expect(transition.ownerId.toString()).to.equal(OWNER_ID);
      expect(transition.dataContractId.toString()).to.equal(CONTRACT_ID);
      expect(transition.identityContractNonce).to.equal(BigInt(7));
      expect(transition.userFeeIncrease).to.equal(0);
    });

    it('should create a claim of the moderators pot', () => {
      const transition = createTransition({ pot: 'moderators', userFeeIncrease: 3 });

      expect(transition.pot).to.equal('moderators');
      expect(transition.userFeeIncrease).to.equal(3);
    });

    it('should refuse an unknown pot', () => {
      expect(() => createTransition({ pot: 'treasury' as never })).to.throw();
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('should round trip every pot through bytes, base64 and hex', () => {
      for (const transition of [
        createTransition({ pot: 'owner' }),
        createTransition({ pot: 'moderators', userFeeIncrease: 3 }),
      ]) {
        const bytes = transition.toBytes();
        expect(wasm.ContractFeeClaim.fromBytes(bytes).toBytes()).to.deep.equal(bytes);
        expect(wasm.ContractFeeClaim.fromBase64(transition.toBase64()).toBytes()).to.deep.equal(bytes);
        expect(wasm.ContractFeeClaim.fromHex(transition.toHex()).toBytes()).to.deep.equal(bytes);
      }
    });
  });

  describe('setters', () => {
    it('should update the nonce, the ids, the pot and the fee', () => {
      const transition = createTransition();

      transition.identityContractNonce = BigInt(8);
      transition.userFeeIncrease = 2;
      transition.pot = 'moderators';
      transition.ownerId = CONTRACT_ID;
      transition.dataContractId = OWNER_ID;

      expect(transition.identityContractNonce).to.equal(BigInt(8));
      expect(transition.userFeeIncrease).to.equal(2);
      expect(transition.pot).to.equal('moderators');
      expect(transition.ownerId.toString()).to.equal(CONTRACT_ID);
      expect(transition.dataContractId.toString()).to.equal(OWNER_ID);
    });

    it('should refuse an unknown pot', () => {
      const transition = createTransition();

      expect(() => {
        transition.pot = 'treasury' as never;
      }).to.throw();
    });
  });

  describe('toJSON()', () => {
    it('should produce the expected JSON structure', () => {
      const json = createTransition({ pot: 'moderators', userFeeIncrease: 4 }).toJSON();

      expect(json.$formatVersion).to.equal('0');
      expect(json.ownerId).to.equal(OWNER_ID);
      expect(json.dataContractId).to.equal(CONTRACT_ID);
      expect(json.identityContractNonce).to.equal(7);
      expect(json.pot).to.equal('moderators');
      expect(json.userFeeIncrease).to.equal(4);
      expect(json.signature).to.equal('');
      expect(json.signaturePublicKeyId).to.equal(0);
    });
  });

  describe('fromJSON()', () => {
    it('should restore the transition from JSON', () => {
      const transition = createTransition({ pot: 'moderators', userFeeIncrease: 4 });

      const restored = wasm.ContractFeeClaim.fromJSON(transition.toJSON());

      expect(restored.pot).to.equal('moderators');
      expect(restored.dataContractId.toString()).to.equal(CONTRACT_ID);
      expect(restored.identityContractNonce).to.equal(BigInt(7));
      expect(restored.userFeeIncrease).to.equal(4);
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });
  });

  describe('toObject() / fromObject()', () => {
    it('should round trip through a plain object', () => {
      const transition = createTransition({ pot: 'moderators' });

      const obj = transition.toObject();
      expect(obj.$formatVersion).to.equal('0');
      expect(obj.ownerId).to.be.instanceOf(Uint8Array);
      expect(obj.dataContractId.length).to.equal(32);
      expect(obj.identityContractNonce).to.equal(BigInt(7));
      expect(obj.pot).to.equal('moderators');

      const restored = wasm.ContractFeeClaim.fromObject(obj);
      expect(restored.pot).to.equal('moderators');
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });
  });

  describe('toStateTransition() / fromStateTransition()', () => {
    it('should convert to and from a generic state transition', () => {
      const transition = createTransition();
      const generic = transition.toStateTransition();

      expect(generic.actionTypeNumber).to.equal(25);
      expect(generic.identityContractNonce).to.equal(BigInt(7));
      expect(wasm.ContractFeeClaim.fromStateTransition(generic).toBytes()).to.deep.equal(transition.toBytes());
    });
  });
});
