import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import {
  fakeOrchardAction,
  ZERO_ANCHOR,
  ZERO_BINDING_SIG,
  ZERO_PROOF,
} from './helpers/shielded.ts';
import { instantLockBytes, transactionBytes } from './mocks/Locks/index.js';

before(async () => {
  await initWasm();
});

describe('ShieldFromAssetLockTransition', () => {
  function createAssetLockProof() {
    return wasm.AssetLockProof.createInstantAssetLockProof(
      instantLockBytes,
      transactionBytes,
      0,
    );
  }

  function createTransition() {
    return new wasm.ShieldFromAssetLockTransition({
      assetLockProof: createAssetLockProof(),
      actions: [fakeOrchardAction()],
      valueBalance: BigInt(0),
      anchor: ZERO_ANCHOR,
      proof: ZERO_PROOF,
      bindingSignature: ZERO_BINDING_SIG,
      signature: new Uint8Array(0),
    });
  }

  describe('constructor()', () => {
    it('should construct with assetLockProof + Orchard fields', () => {
      const t = createTransition();
      expect(t).to.be.an.instanceof(wasm.ShieldFromAssetLockTransition);
    });

    it('should reject missing assetLockProof', () => {
      expect(() => new wasm.ShieldFromAssetLockTransition({
          actions: [fakeOrchardAction()],
          valueBalance: BigInt(0),
          anchor: ZERO_ANCHOR,
          proof: ZERO_PROOF,
          bindingSignature: ZERO_BINDING_SIG,
          signature: new Uint8Array(0),
        })).to.throw();
    });
  });

  describe('getters', () => {
    it('returns AssetLockProof and typed actions', () => {
      const t = createTransition();
      expect(t.assetLockProof).to.be.an.instanceof(wasm.AssetLockProof);
      expect(t.assetLockProof.lockType).to.equal('instant');
      expect(t.actions).to.be.an('array').with.lengthOf(1);
      expect(t.actions[0]).to.be.an.instanceof(wasm.SerializedOrchardAction);
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('round-trips via bytes', () => {
      const t = createTransition();
      const bytes = t.toBytes();
      const restored = wasm.ShieldFromAssetLockTransition.fromBytes(bytes);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toObject() / toJSON()', () => {
    it('toObject() emits AssetLockProof in internally-tagged Object form', () => {
      const t = createTransition();
      const obj = t.toObject();
      // Internally-tagged: { $type: "instant" | "chain", ...flattened inner fields }
      expect(obj.assetLockProof).to.be.an('object');
      expect(obj.assetLockProof.$type).to.be.oneOf(['instant', 'chain']);
      expect(obj.actions).to.be.an('array').with.lengthOf(1);
      expect(obj.anchor).to.be.instanceOf(Uint8Array).with.lengthOf(32);
      expect(obj.bindingSignature).to.be.instanceOf(Uint8Array).with.lengthOf(64);
    });

    it('toJSON() emits AssetLockProof + byte fields with the JSON shape', () => {
      const t = createTransition();
      const json = t.toJSON();
      expect(json.assetLockProof).to.be.an('object');
      expect(json.assetLockProof.$type).to.be.oneOf(['instant', 'chain']);
      expect(json.actions).to.be.an('array').with.lengthOf(1);
      expect(json.anchor).to.be.a('string').with.lengthOf(44); // 32 bytes base64
      expect(json.bindingSignature).to.be.a('string').with.lengthOf(88); // 64 bytes base64
    });
  });

  describe('toStateTransition()', () => {
    it('should convert to StateTransition wrapper', () => {
      const t = createTransition();
      expect(t.toStateTransition()).to.exist();
    });
  });
});
