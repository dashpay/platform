import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenDestroyFrozenResult conversion methods', () => {
  before(async () => {
    await init();
  });

  it('should round-trip through toObject/fromObject', () => {
    const data = {
      groupPower: 90,
    };

    const result = sdk.TokenDestroyFrozenResult.fromObject(data);
    expect(result.groupPower).to.equal(90);
    expect(result.document).to.be.undefined;

    const obj = result.toObject();
    const roundtrip = sdk.TokenDestroyFrozenResult.fromObject(obj);
    expect(roundtrip.groupPower).to.equal(90);
  });

  it('should round-trip through toJSON/fromJSON', () => {
    const data = {
      groupPower: 85,
    };

    const result = sdk.TokenDestroyFrozenResult.fromJSON(data);
    expect(result.groupPower).to.equal(85);

    const json = result.toJSON();
    const roundtrip = sdk.TokenDestroyFrozenResult.fromJSON(json);
    expect(roundtrip.groupPower).to.equal(85);
  });

  it('should handle empty data', () => {
    const data = {};

    const result = sdk.TokenDestroyFrozenResult.fromObject(data);
    expect(result.groupPower).to.be.undefined;
    expect(result.document).to.be.undefined;
  });
});
