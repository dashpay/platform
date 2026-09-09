import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('VerifiedAssetLockConsumedWithAddressInfos', () => {
  function fromObject(overrides = {}) {
    return wasm.VerifiedAssetLockConsumedWithAddressInfos.fromObject({
      status: 'consumed',
      initialCreditValue: BigInt(1000),
      remainingCreditValue: BigInt(400),
      addressInfos: {},
      ...overrides,
    });
  }

  describe('fromObject()', () => {
    it('accepts BigInt, string and number credit values', () => {
      expect(fromObject().initialCreditValue).to.equal(BigInt(1000));
      expect(fromObject({ initialCreditValue: '1000' }).initialCreditValue)
        .to.equal(BigInt(1000));
      expect(fromObject({ initialCreditValue: 1000 }).initialCreditValue)
        .to.equal(BigInt(1000));
    });

    it('treats absent / null credit values as absent', () => {
      const t = fromObject({ initialCreditValue: undefined, remainingCreditValue: null });
      expect(t.initialCreditValue).to.be.undefined();
      expect(t.remainingCreditValue).to.be.undefined();
    });

    // Present-but-garbage must error rather than silently becoming absent —
    // otherwise a malformed proof-result object is indistinguishable from
    // "no surplus" and bad input is hidden at the WASM boundary.
    it('rejects present-but-malformed credit values', () => {
      expect(() => fromObject({ initialCreditValue: 'not_a_number' })).to.throw();
      expect(() => fromObject({ remainingCreditValue: -5 })).to.throw();
      expect(() => fromObject({ initialCreditValue: 1.5 })).to.throw();
    });

    it('round-trips via toJSON / JSON.stringify', () => {
      const t = fromObject();
      const restored = wasm.VerifiedAssetLockConsumedWithAddressInfos.fromJSON(
        JSON.parse(JSON.stringify(t.toJSON())),
      );
      expect(restored.initialCreditValue).to.equal(BigInt(1000));
      expect(restored.remainingCreditValue).to.equal(BigInt(400));
    });
  });
});
