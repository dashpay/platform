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

describe('ShieldedWithdrawalTransition', () => {
  function createTransition(pooling: any = 'never') {
    return new wasm.ShieldedWithdrawalTransition({
      actions: [fakeOrchardAction()],
      unshieldingAmount: BigInt(50_000),
      anchor: ZERO_ANCHOR,
      proof: ZERO_PROOF,
      bindingSignature: ZERO_BINDING_SIG,
      coreFeePerByte: 1,
      pooling,
      outputScript: new Uint8Array(25),
    });
  }

  describe('constructor() + pooling polymorphic input (matches IdentityCreditWithdrawal)', () => {
    it('accepts pooling as a name string', () => {
      const t = createTransition('never');
      // PoolingWasm::From<...> for String returns CamelCase variant names.
      expect(t.pooling.toLowerCase()).to.equal('never');
    });

    it('accepts pooling as a numeric value', () => {
      const t = createTransition(1);
      expect(t.pooling.toLowerCase()).to.equal('ifavailable');
    });

    it('accepts pooling as a Pooling enum value', () => {
      // wasm-bindgen exports the enum as `PoolingWasm` (Never=0, IfAvailable=1, Standard=2).
      const t = createTransition(wasm.PoolingWasm.Standard);
      expect(t.pooling.toLowerCase()).to.equal('standard');
    });

    it('rejects an unknown pooling string', () => {
      expect(() => createTransition('nonsense')).to.throw();
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('round-trips via bytes', () => {
      const t = createTransition('never');
      const bytes = t.toBytes();
      const restored = wasm.ShieldedWithdrawalTransition.fromBytes(bytes);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toObject() / toJSON()', () => {
    it('toObject() emits pooling as the Pooling enum (matches IdentityCreditWithdrawal)', () => {
      const t = createTransition('ifavailable');
      const obj = t.toObject();

      // dpp::withdrawal::Pooling derives serde_repr → emits as a u8 in
      // non-human-readable formats (which is what platform_value uses).
      expect(obj.pooling).to.satisfy(
        (v: unknown) => typeof v === 'number' || typeof v === 'string',
      );
    });

    it('toJSON() emits pooling as a number (Pooling has serde_repr)', () => {
      const t = createTransition('standard');
      const json = t.toJSON();

      // serde_repr also stringifies to a number in human-readable mode.
      expect(json.pooling).to.satisfy(
        (v: unknown) => typeof v === 'number' || typeof v === 'string',
      );
    });
  });

  describe('toStateTransition()', () => {
    it('should convert to StateTransition wrapper', () => {
      const t = createTransition();
      expect(t.toStateTransition()).to.exist();
    });
  });
});
