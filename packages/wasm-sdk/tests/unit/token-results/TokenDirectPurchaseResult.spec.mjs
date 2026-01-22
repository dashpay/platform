import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenDirectPurchaseResult conversion methods', () => {
  const testIdentifier = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  before(async () => {
    await init();
  });

  it('should round-trip through toObject/fromObject', () => {
    const data = {
      buyerId: testIdentifier,
      newBalance: 2000000n,
      groupPower: 55,
    };

    const result = sdk.TokenDirectPurchaseResult.fromObject(data);

    expect(result.buyerId.toBase58()).to.equal(testIdentifier);
    expect(result.newBalance).to.equal(2000000n);
    expect(result.groupPower).to.equal(55);

    const obj = result.toObject();
    const roundtrip = sdk.TokenDirectPurchaseResult.fromObject(obj);
    expect(roundtrip.groupPower).to.equal(55);
  });

  it('should round-trip through toJSON/fromJSON', () => {
    const data = {
      buyerId: testIdentifier,
      newBalance: 2000000,
      groupPower: 40,
    };

    const result = sdk.TokenDirectPurchaseResult.fromJSON(data);

    expect(result.buyerId.toBase58()).to.equal(testIdentifier);
    expect(result.newBalance).to.equal(2000000n);
    expect(result.groupPower).to.equal(40);

    const json = result.toJSON();
    const roundtrip = sdk.TokenDirectPurchaseResult.fromJSON(json);
    expect(roundtrip.buyerId.toBase58()).to.equal(testIdentifier);
    expect(roundtrip.groupPower).to.equal(40);
  });

  it('should handle group action without balance', () => {
    const data = {
      groupPower: 30,
    };

    const result = sdk.TokenDirectPurchaseResult.fromObject(data);
    expect(result.buyerId).to.be.undefined;
    expect(result.newBalance).to.be.undefined;
    expect(result.groupPower).to.equal(30);
    expect(result.document).to.be.undefined;
  });
});
