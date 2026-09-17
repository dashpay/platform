import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

interface KeyLimitsUpdateOptions {
  identityId?: string;
  revision?: bigint;
  nonce?: bigint;
  keyId?: number;
  totalBudget?: bigint;
  expiresAt?: bigint;
  userFeeIncrease?: number;
}

function createTransition(options: KeyLimitsUpdateOptions = {}) {
  return new wasm.IdentityKeyLimitsUpdate({
    identityId: options.identityId ?? '11111111111111111111111111111111',
    revision: options.revision ?? BigInt(3),
    nonce: options.nonce ?? BigInt(7),
    keyId: options.keyId ?? 5,
    totalBudget: options.totalBudget,
    expiresAt: options.expiresAt,
    userFeeIncrease: options.userFeeIncrease,
  });
}

describe('IdentityKeyLimitsUpdate', () => {
  describe('constructor()', () => {
    it('should create a transition that raises the budget', () => {
      const transition = createTransition({ totalBudget: BigInt(2500000000) });

      expect(transition).to.be.an.instanceof(wasm.IdentityKeyLimitsUpdate);
      expect(transition.keyId).to.equal(5);
      expect(transition.totalBudget).to.equal(BigInt(2500000000));
      expect(transition.expiresAt).to.equal(undefined);
      expect(transition.revision).to.equal(BigInt(3));
      expect(transition.nonce).to.equal(BigInt(7));
      expect(transition.userFeeIncrease).to.equal(0);
    });

    it('should create a transition that extends the expiry', () => {
      const transition = createTransition({ expiresAt: BigInt(1800000000000) });

      expect(transition.totalBudget).to.equal(undefined);
      expect(transition.expiresAt).to.equal(BigInt(1800000000000));
    });

    it('should accept an Identifier object', () => {
      const identityId = new wasm.Identifier('11111111111111111111111111111111');
      const transition = new wasm.IdentityKeyLimitsUpdate({
        identityId,
        revision: BigInt(1),
        nonce: BigInt(1),
        keyId: 2,
        totalBudget: BigInt(10),
      });

      expect(transition.identityId.toString()).to.equal('11111111111111111111111111111111');
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('should round trip through bytes, base64 and hex', () => {
      const transition = createTransition({
        totalBudget: BigInt(2500000000),
        expiresAt: BigInt(1800000000000),
        userFeeIncrease: 4,
      });

      const bytes = transition.toBytes();
      expect(wasm.IdentityKeyLimitsUpdate.fromBytes(bytes).toBytes()).to.deep.equal(bytes);
      expect(wasm.IdentityKeyLimitsUpdate.fromBase64(transition.toBase64()).toBytes()).to.deep.equal(bytes);
      expect(wasm.IdentityKeyLimitsUpdate.fromHex(transition.toHex()).toBytes()).to.deep.equal(bytes);
    });
  });

  describe('setters', () => {
    it('should update the limits, the nonce and the revision', () => {
      const transition = createTransition({ totalBudget: BigInt(10) });

      transition.totalBudget = BigInt(20);
      transition.expiresAt = BigInt(30);
      transition.nonce = BigInt(8);
      transition.revision = BigInt(4);
      transition.keyId = 9;
      transition.userFeeIncrease = 2;

      expect(transition.totalBudget).to.equal(BigInt(20));
      expect(transition.expiresAt).to.equal(BigInt(30));
      expect(transition.nonce).to.equal(BigInt(8));
      expect(transition.revision).to.equal(BigInt(4));
      expect(transition.keyId).to.equal(9);
      expect(transition.userFeeIncrease).to.equal(2);
    });
  });

  describe('toStateTransition() / fromStateTransition()', () => {
    it('should convert to a state transition of type 23 and back', () => {
      const transition = createTransition({ totalBudget: BigInt(10) });

      const stateTransition = transition.toStateTransition();
      expect(stateTransition.actionTypeNumber).to.equal(23);
      expect(stateTransition.identityNonce).to.equal(BigInt(7));

      const restored = wasm.IdentityKeyLimitsUpdate.fromStateTransition(stateTransition);
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });
  });
});
