import init, * as sdk from '../../dist/sdk.compressed.js';
import contractFixture from './fixtures/data-contract-crypto-card-game.mjs';

const PLATFORM_VERSION = 1;

describe('DataContract', () => {
  before(async () => {
    await init();
  });

  it('should create a contract from JSON and expose identifiers', async () => {
    const contract = sdk.DataContract.fromJSON(contractFixture, PLATFORM_VERSION);

    expect(contract).to.be.ok();
    expect(contract.id()).to.equal(contractFixture.id);

    const roundTripped = contract.toJSON();
    expect(roundTripped).to.be.an('object');
    expect(roundTripped.id).to.equal(contractFixture.id);

    contract.free();
  });
});
