import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenBurnResult conversion methods', () => {
  const testIdentifier = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  before(async () => {
    await init();
  });

  it('should round-trip through toObject/fromObject', () => {
    const data = {
      ownerId: testIdentifier,
      remainingBalance: 500000n,
      groupPower: 100,
      groupActionStatus: 'ActionNeeded',
    };

    const result = sdk.TokenBurnResult.fromObject(data);

    expect(result.ownerId.toBase58()).to.equal(testIdentifier);
    expect(result.remainingBalance).to.equal(500000n);
    expect(result.groupPower).to.equal(100);
    expect(result.groupActionStatus).to.equal('ActionNeeded');

    const obj = result.toObject();
    const roundtrip = sdk.TokenBurnResult.fromObject(obj);
    expect(roundtrip.groupPower).to.equal(100);
  });

  it('should round-trip through toJSON/fromJSON', () => {
    const data = {
      ownerId: testIdentifier,
      remainingBalance: 500000,
      groupPower: 80,
      groupActionStatus: 'Completed',
    };

    const result = sdk.TokenBurnResult.fromJSON(data);
    const json = result.toJSON();

    expect(json.ownerId).to.equal(testIdentifier);
    expect(json.groupPower).to.equal(80);

    const roundtrip = sdk.TokenBurnResult.fromJSON(json);
    expect(roundtrip.ownerId.toBase58()).to.equal(testIdentifier);
    expect(roundtrip.groupPower).to.equal(80);
  });

  it('should handle null optional fields', () => {
    const data = {
      ownerId: testIdentifier,
      remainingBalance: 100n,
    };

    const result = sdk.TokenBurnResult.fromObject(data);
    expect(result.ownerId.toBase58()).to.equal(testIdentifier);
    expect(result.groupPower).to.be.undefined;
    expect(result.groupActionStatus).to.be.undefined;
    expect(result.document).to.be.undefined;
  });
});
