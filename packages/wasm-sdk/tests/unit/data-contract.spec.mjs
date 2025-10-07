import init, * as sdk from '../../dist/sdk.compressed.js';
import contractFixtureV0 from './fixtures/data-contract-v0-crypto-card-game.mjs';
import contractFixtureV1 from './fixtures/data-contract-v1-with-docs-tokens-groups.mjs';

const PLATFORM_VERSION_CONTRACT_V0 = 1;
const PLATFORM_VERSION_CONTRACT_V1 = 9; // V1 contracts introduced in Platform v9

describe('DataContract', () => {
  before(async () => {
    await init();
  });

  it('should create a V0 contract from JSON and expose identifiers', async () => {
    const contract = sdk.DataContract.fromJSON(contractFixtureV0, PLATFORM_VERSION_CONTRACT_V0);

    expect(contract).to.be.ok();
    expect(contract.id()).to.equal(contractFixtureV0.id);

    const roundTripped = contract.toJSON();
    expect(roundTripped).to.be.an('object');
    expect(roundTripped.id).to.equal(contractFixtureV0.id);

    contract.free();
  });

  it('should create a V1 contract from JSON and expose identifiers', async () => {
    const contract = sdk.DataContract.fromJSON(contractFixtureV1, PLATFORM_VERSION_CONTRACT_V1);

    expect(contract).to.be.ok();
    expect(contract.id()).to.equal(contractFixtureV1.id);

    const roundTripped = contract.toJSON();
    expect(roundTripped).to.be.an('object');
    expect(roundTripped.id).to.equal(contractFixtureV1.id);

    contract.free();
  });
});
