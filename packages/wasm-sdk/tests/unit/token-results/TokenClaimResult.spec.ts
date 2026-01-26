import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenClaimResult conversion methods', () => {
  before(async () => {
    await init();
  });

  it('should round-trip through toObject/fromObject', () => {
    const data = {
      groupPower: 33,
    };

    const result = sdk.TokenClaimResult.fromObject(data);
    expect(result.groupPower).to.equal(33);
    expect(result.document).to.be.undefined();

    const obj = result.toObject();
    const roundtrip = sdk.TokenClaimResult.fromObject(obj);
    expect(roundtrip.groupPower).to.equal(33);
  });

  it('should round-trip through toJSON/fromJSON', () => {
    const data = {
      groupPower: 42,
    };

    const result = sdk.TokenClaimResult.fromJSON(data);
    expect(result.groupPower).to.equal(42);

    const json = result.toJSON();
    const roundtrip = sdk.TokenClaimResult.fromJSON(json);
    expect(roundtrip.groupPower).to.equal(42);
  });

  it('should handle empty data', () => {
    const data = {};

    const result = sdk.TokenClaimResult.fromObject(data);
    expect(result.groupPower).to.be.undefined();
    expect(result.document).to.be.undefined();
  });
});
