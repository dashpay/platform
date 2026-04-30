import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import {
  fakeOrchardAction,
  ZERO_ANCHOR,
  ZERO_BINDING_SIG,
  ZERO_PROOF,
} from './helpers/shielded.ts';

before(async () => {
  await initWasm();
});

describe('UnshieldTransition', () => {
  const addrBytes = new Uint8Array([
    0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
  ]);

  function createTransition() {
    const outputAddr = wasm.PlatformAddress.fromBytes(addrBytes);

    return new wasm.UnshieldTransition({
      outputAddress: outputAddr,
      actions: [fakeOrchardAction()],
      unshieldingAmount: BigInt(50_000),
      anchor: ZERO_ANCHOR,
      proof: ZERO_PROOF,
      bindingSignature: ZERO_BINDING_SIG,
    });
  }

  describe('constructor()', () => {
    it('should construct with outputAddress + Orchard fields', () => {
      const t = createTransition();
      expect(t).to.be.an.instanceof(wasm.UnshieldTransition);
    });

    it('should reject missing outputAddress', () => {
      expect(() => new wasm.UnshieldTransition({
          actions: [fakeOrchardAction()],
          unshieldingAmount: BigInt(0),
          anchor: ZERO_ANCHOR,
          proof: ZERO_PROOF,
          bindingSignature: ZERO_BINDING_SIG,
        })).to.throw();
    });
  });

  describe('getters', () => {
    it('returns typed PlatformAddress for outputAddress', () => {
      const t = createTransition();
      const addr = t.outputAddress;
      expect(addr).to.be.an.instanceof(wasm.PlatformAddress);
      expect(addr.toBytes()).to.deep.equal(addrBytes);
    });

    it('returns typed Orchard actions', () => {
      const t = createTransition();
      expect(t.actions[0]).to.be.an.instanceof(wasm.SerializedOrchardAction);
    });

    it('returns the unshielding amount', () => {
      const t = createTransition();
      expect(t.unshieldingAmount).to.equal(BigInt(50_000));
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('round-trips via bytes', () => {
      const t = createTransition();
      const bytes = t.toBytes();
      const restored = wasm.UnshieldTransition.fromBytes(bytes);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toObject() / toJSON()', () => {
    it('toObject() emits outputAddress as a 21-byte Uint8Array', () => {
      const t = createTransition();
      const obj = t.toObject();
      expect(obj.outputAddress).to.be.instanceOf(Uint8Array);
      expect(obj.outputAddress.length).to.equal(21);
    });

    it('toJSON() emits outputAddress as a 42-char hex string', () => {
      const t = createTransition();
      const json = t.toJSON();
      expect(json.outputAddress).to.be.a('string').with.lengthOf(42); // 21 bytes hex
    });
  });

  describe('toStateTransition()', () => {
    it('should convert to StateTransition wrapper', () => {
      const t = createTransition();
      expect(t.toStateTransition()).to.exist();
    });
  });
});
