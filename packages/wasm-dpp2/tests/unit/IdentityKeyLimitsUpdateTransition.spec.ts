import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

interface KeyLimitsUpdateOptions {
  identityId?: string;
  nonce?: bigint;
  keyId?: number;
  totalBudget?: bigint;
  expiresAt?: bigint;
  userFeeIncrease?: number;
}

function createTransition(options: KeyLimitsUpdateOptions = {}) {
  return new wasm.IdentityKeyLimitsUpdate({
    identityId: options.identityId ?? '11111111111111111111111111111111',
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
    it('should update the limits and the nonce', () => {
      const transition = createTransition({ totalBudget: BigInt(10) });

      transition.totalBudget = BigInt(20);
      transition.expiresAt = BigInt(30);
      transition.nonce = BigInt(8);
      transition.keyId = 9;
      transition.userFeeIncrease = 2;

      expect(transition.totalBudget).to.equal(BigInt(20));
      expect(transition.expiresAt).to.equal(BigInt(30));
      expect(transition.nonce).to.equal(BigInt(8));
      expect(transition.keyId).to.equal(9);
      expect(transition.userFeeIncrease).to.equal(2);
    });
  });

  describe('toJSON()', () => {
    it('should produce the expected JSON structure', () => {
      const transition = createTransition({
        totalBudget: BigInt(2500000000),
        expiresAt: BigInt(1800000000000),
        userFeeIncrease: 4,
      });

      const json = transition.toJSON();

      expect(json.$formatVersion).to.equal('0');
      expect(json.identityId).to.equal('11111111111111111111111111111111');
      expect(json.nonce).to.equal(7);
      expect(json.keyId).to.equal(5);
      expect(json.totalBudget).to.equal(2500000000);
      expect(json.expiresAt).to.equal(1800000000000);
      expect(json.userFeeIncrease).to.equal(4);
      expect(json.signature).to.equal('');
      expect(json.signaturePublicKeyId).to.equal(0);
    });

    it('should leave out a limit that is not raised', () => {
      const json = createTransition({ totalBudget: BigInt(10) }).toJSON();

      expect(json.totalBudget).to.equal(10);
      expect(json).to.not.have.property('expiresAt');
    });
  });

  describe('fromJSON()', () => {
    it('should restore the transition from JSON', () => {
      const transition = createTransition({
        totalBudget: BigInt(2500000000),
        expiresAt: BigInt(1800000000000),
        userFeeIncrease: 4,
      });

      const restored = wasm.IdentityKeyLimitsUpdate.fromJSON(transition.toJSON());

      expect(restored.identityId.toString()).to.equal('11111111111111111111111111111111');
      expect(restored.nonce).to.equal(BigInt(7));
      expect(restored.keyId).to.equal(5);
      expect(restored.totalBudget).to.equal(BigInt(2500000000));
      expect(restored.expiresAt).to.equal(BigInt(1800000000000));
      expect(restored.userFeeIncrease).to.equal(4);
      expect(restored.signaturePublicKeyId).to.equal(0);
      expect(restored.signature).to.deep.equal(Uint8Array.from([]));
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });

    it('should restore a transition with one limit left out', () => {
      const transition = createTransition({ expiresAt: BigInt(30) });

      const restored = wasm.IdentityKeyLimitsUpdate.fromJSON(transition.toJSON());

      expect(restored.totalBudget).to.equal(undefined);
      expect(restored.expiresAt).to.equal(BigInt(30));
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });
  });

  describe('toObject()', () => {
    it('should produce the expected object structure', () => {
      const transition = createTransition({
        totalBudget: BigInt(2500000000),
        expiresAt: BigInt(1800000000000),
        userFeeIncrease: 4,
      });

      const obj = transition.toObject();

      expect(obj.$formatVersion).to.equal('0');
      expect(obj.identityId).to.be.instanceOf(Uint8Array);
      expect(obj.identityId.length).to.equal(32);
      expect(obj.nonce).to.equal(BigInt(7));
      expect(obj.keyId).to.equal(5);
      expect(obj.totalBudget).to.equal(BigInt(2500000000));
      expect(obj.expiresAt).to.equal(BigInt(1800000000000));
      expect(obj.userFeeIncrease).to.equal(4);
      expect(obj.signature).to.be.instanceOf(Uint8Array);
      expect(obj.signature.length).to.equal(0);
      expect(obj.signaturePublicKeyId).to.equal(0);
    });
  });

  describe('fromObject()', () => {
    it('should restore the transition from an object', () => {
      const transition = createTransition({
        totalBudget: BigInt(2500000000),
        expiresAt: BigInt(1800000000000),
        userFeeIncrease: 4,
      });

      const restored = wasm.IdentityKeyLimitsUpdate.fromObject(transition.toObject());

      expect(restored.identityId.toString()).to.equal('11111111111111111111111111111111');
      expect(restored.keyId).to.equal(5);
      expect(restored.totalBudget).to.equal(BigInt(2500000000));
      expect(restored.expiresAt).to.equal(BigInt(1800000000000));
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });

    it('should restore a transition with one limit left out', () => {
      const transition = createTransition({ totalBudget: BigInt(10) });

      const restored = wasm.IdentityKeyLimitsUpdate.fromObject(transition.toObject());

      expect(restored.totalBudget).to.equal(BigInt(10));
      expect(restored.expiresAt).to.equal(undefined);
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
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
