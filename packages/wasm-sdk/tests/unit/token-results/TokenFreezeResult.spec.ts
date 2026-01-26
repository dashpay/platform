import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenFreezeResult conversion methods', () => {
  const testIdentifier = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  before(async () => {
    await init();
  });

  it('should round-trip through toObject/fromObject', () => {
    const data = {
      frozenIdentityId: testIdentifier,
      groupPower: 80,
    };

    const result = sdk.TokenFreezeResult.fromObject(data);

    expect(result.frozenIdentityId.toBase58()).to.equal(testIdentifier);
    expect(result.groupPower).to.equal(80);

    const obj = result.toObject();
    const roundtrip = sdk.TokenFreezeResult.fromObject(obj);
    expect(roundtrip.groupPower).to.equal(80);
  });

  it('should round-trip through toJSON/fromJSON', () => {
    const data = {
      frozenIdentityId: testIdentifier,
      groupPower: 60,
    };

    const result = sdk.TokenFreezeResult.fromJSON(data);

    expect(result.frozenIdentityId.toBase58()).to.equal(testIdentifier);
    expect(result.groupPower).to.equal(60);

    const json = result.toJSON();
    const roundtrip = sdk.TokenFreezeResult.fromJSON(json);
    expect(roundtrip.frozenIdentityId.toBase58()).to.equal(testIdentifier);
    expect(roundtrip.groupPower).to.equal(60);
  });

  it('should handle group action without identity', () => {
    const data = {
      groupPower: 45,
    };

    const result = sdk.TokenFreezeResult.fromObject(data);
    expect(result.frozenIdentityId).to.be.undefined();
    expect(result.groupPower).to.equal(45);
    expect(result.document).to.be.undefined();
  });
});
