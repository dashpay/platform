import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenEmergencyActionResult conversion methods', () => {
  before(async () => {
    await init();
  });

  it('should round-trip through toObject/fromObject', () => {
    const data = {
      groupPower: 55,
    };

    const result = sdk.TokenEmergencyActionResult.fromObject(data);
    expect(result.groupPower).to.equal(55);
    expect(result.document).to.be.undefined;

    const obj = result.toObject();
    const roundtrip = sdk.TokenEmergencyActionResult.fromObject(obj);
    expect(roundtrip.groupPower).to.equal(55);
  });

  it('should round-trip through toJSON/fromJSON', () => {
    const data = {
      groupPower: 45,
    };

    const result = sdk.TokenEmergencyActionResult.fromJSON(data);
    expect(result.groupPower).to.equal(45);

    const json = result.toJSON();
    const roundtrip = sdk.TokenEmergencyActionResult.fromJSON(json);
    expect(roundtrip.groupPower).to.equal(45);
  });

  it('should handle empty data', () => {
    const data = {};

    const result = sdk.TokenEmergencyActionResult.fromObject(data);
    expect(result.groupPower).to.be.undefined;
    expect(result.document).to.be.undefined;
  });
});
