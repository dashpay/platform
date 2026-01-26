import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenMintResult conversion methods', () => {
  const testIdentifier = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  before(async () => {
    await init();
  });

  it('should round-trip through toObject/fromObject', () => {
    const data = {
      recipientId: testIdentifier,
      newBalance: 1000000n,
      groupPower: 75,
      groupActionStatus: 'Completed',
    };

    const result = sdk.TokenMintResult.fromObject(data);

    expect(result.recipientId.toBase58()).to.equal(testIdentifier);
    expect(result.newBalance).to.equal(1000000n);
    expect(result.groupPower).to.equal(75);
    expect(result.groupActionStatus).to.equal('Completed');
    expect(result.document).to.be.undefined;

    const obj = result.toObject();
    expect(obj.groupPower).to.equal(75);
    expect(obj.groupActionStatus).to.equal('Completed');

    const roundtrip = sdk.TokenMintResult.fromObject(obj);
    expect(roundtrip.groupPower).to.equal(75);
    expect(roundtrip.groupActionStatus).to.equal('Completed');
  });

  it('should round-trip through toJSON/fromJSON', () => {
    // Note: fromJSON expects numeric values for u64 fields, not strings
    const data = {
      recipientId: testIdentifier,
      newBalance: 1000000,
      groupPower: 50,
      groupActionStatus: 'Pending',
    };

    const result = sdk.TokenMintResult.fromJSON(data);
    const json = result.toJSON();

    expect(json.recipientId).to.equal(testIdentifier);
    expect(json.groupPower).to.equal(50);
    expect(json.groupActionStatus).to.equal('Pending');

    const roundtrip = sdk.TokenMintResult.fromJSON(json);
    expect(roundtrip.recipientId.toBase58()).to.equal(testIdentifier);
    expect(roundtrip.groupPower).to.equal(50);
  });

  it('should handle optional fields being undefined', () => {
    const data = {
      groupPower: 25,
    };

    const result = sdk.TokenMintResult.fromObject(data);
    expect(result.recipientId).to.be.undefined;
    expect(result.newBalance).to.be.undefined;
    expect(result.groupPower).to.equal(25);
    expect(result.groupActionStatus).to.be.undefined;
    expect(result.document).to.be.undefined;
  });
});
