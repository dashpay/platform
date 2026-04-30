import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

// These wrappers wrap js_sys::Map under the hood. Without normalisation,
// JSON.stringify silently drops Map entries (Map has no enumerable own
// properties), so the toJSON() tests below would have caught the regression
// fixed in proof_result_shielded.rs.

describe('VerifiedShieldedNullifiers', () => {
  it('fromObject/toObject preserves Map entries', () => {
    const nullifiers = new Map();
    nullifiers.set('deadbeef', true);

    const result = wasm.VerifiedShieldedNullifiers.fromObject({ nullifiers });
    expect(result.nullifiers).to.be.instanceOf(Map);
    expect(result.nullifiers.size).to.equal(1);

    const obj = result.toObject();
    expect(obj.nullifiers).to.be.instanceOf(Map);
  });

  it('toJSON() preserves Map entries through JSON.stringify', () => {
    const nullifiers = new Map();
    nullifiers.set('deadbeef', true);
    nullifiers.set('cafebabe', false);

    const result = wasm.VerifiedShieldedNullifiers.fromObject({ nullifiers });
    const parsed = JSON.parse(JSON.stringify(result.toJSON()));

    expect(parsed.nullifiers).to.have.property('deadbeef', true);
    expect(parsed.nullifiers).to.have.property('cafebabe', false);
  });
});

describe('VerifiedShieldedNullifiersWithAddressInfos', () => {
  it('fromObject/toObject preserves Map entries', () => {
    const nullifiers = new Map();
    nullifiers.set('aa', true);
    const addressInfos = new Map();
    addressInfos.set('bb', null);

    const result = wasm.VerifiedShieldedNullifiersWithAddressInfos.fromObject({
      nullifiers,
      addressInfos,
    });
    expect(result.nullifiers.size).to.equal(1);
    expect(result.addressInfos.size).to.equal(1);
  });

  it('toJSON() preserves Map entries through JSON.stringify', () => {
    const nullifiers = new Map();
    nullifiers.set('aa', true);
    const addressInfos = new Map();
    addressInfos.set('bb', null);

    const result = wasm.VerifiedShieldedNullifiersWithAddressInfos.fromObject({
      nullifiers,
      addressInfos,
    });
    const parsed = JSON.parse(JSON.stringify(result.toJSON()));

    expect(parsed.nullifiers).to.have.property('aa', true);
    expect(parsed.addressInfos).to.have.property('bb');
    expect(parsed.addressInfos.bb).to.equal(null);
  });
});

describe('VerifiedShieldedNullifiersWithWithdrawalDocument', () => {
  it('fromObject/toObject preserves Map entries', () => {
    const nullifiers = new Map();
    nullifiers.set('aa', true);
    const documents = new Map();
    documents.set('bb', null);

    const result = wasm.VerifiedShieldedNullifiersWithWithdrawalDocument.fromObject({
      nullifiers,
      documents,
    });
    expect(result.nullifiers.size).to.equal(1);
    expect(result.documents.size).to.equal(1);
  });

  it('toJSON() preserves Map entries through JSON.stringify', () => {
    const nullifiers = new Map();
    nullifiers.set('aa', true);
    const documents = new Map();
    documents.set('bb', null);

    const result = wasm.VerifiedShieldedNullifiersWithWithdrawalDocument.fromObject({
      nullifiers,
      documents,
    });
    const parsed = JSON.parse(JSON.stringify(result.toJSON()));

    expect(parsed.nullifiers).to.have.property('aa', true);
    expect(parsed.documents).to.have.property('bb');
    expect(parsed.documents.bb).to.equal(null);
  });
});
