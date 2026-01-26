import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenTransferResult conversion methods', () => {
  before(async () => {
    await init();
  });

  it('should round-trip through toObject/fromObject', () => {
    const data = {
      senderBalance: 900000n,
      recipientBalance: 100000n,
      groupPower: 25,
    };

    const result = sdk.TokenTransferResult.fromObject(data);

    expect(result.senderBalance).to.equal(900000n);
    expect(result.recipientBalance).to.equal(100000n);
    expect(result.groupPower).to.equal(25);

    const obj = result.toObject();
    const roundtrip = sdk.TokenTransferResult.fromObject(obj);
    expect(roundtrip.groupPower).to.equal(25);
  });

  it('should round-trip through toJSON/fromJSON', () => {
    const data = {
      senderBalance: 900000,
      recipientBalance: 100000,
      groupPower: 25,
    };

    const result = sdk.TokenTransferResult.fromJSON(data);

    expect(result.senderBalance).to.equal(900000n);
    expect(result.recipientBalance).to.equal(100000n);
    expect(result.groupPower).to.equal(25);

    const json = result.toJSON();
    const roundtrip = sdk.TokenTransferResult.fromJSON(json);
    expect(roundtrip.groupPower).to.equal(25);
  });

  it('should handle group action without balances', () => {
    const data = {
      groupPower: 50,
    };

    const result = sdk.TokenTransferResult.fromObject(data);
    expect(result.senderBalance).to.be.undefined;
    expect(result.recipientBalance).to.be.undefined;
    expect(result.groupPower).to.equal(50);
    expect(result.document).to.be.undefined;
  });
});
