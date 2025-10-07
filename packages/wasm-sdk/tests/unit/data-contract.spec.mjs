import init, * as sdk from '../../dist/sdk.compressed.js';
import contractFixtureV0 from './fixtures/data-contract-v0-crypto-card-game.mjs';
import contractFixtureV1 from './fixtures/data-contract-v1-with-docs-tokens-groups.mjs';

const PLATFORM_VERSION_CONTRACT_V0 = 1;
const PLATFORM_VERSION_CONTRACT_V1 = 9; // V1 contracts introduced in Platform v9

describe('DataContract', () => {
  before(async () => {
    await init();
  });

  it('should create a V0 contract from JSON and expose all properties', async () => {
    const contract = sdk.DataContract.fromJSON(contractFixtureV0, PLATFORM_VERSION_CONTRACT_V0);

    expect(contract).to.be.ok();
    expect(contract.id()).to.equal(contractFixtureV0.id);

    const roundTripped = contract.toJSON();
    expect(roundTripped).to.be.an('object');
    expect(roundTripped.id).to.equal(contractFixtureV0.id);
    expect(roundTripped.ownerId).to.equal(contractFixtureV0.ownerId);
    expect(roundTripped.version).to.equal(contractFixtureV0.version);
    expect(roundTripped.$format_version).to.equal(contractFixtureV0.$format_version);
    expect(roundTripped.config).to.deep.equal(contractFixtureV0.config);
    expect(roundTripped.documentSchemas).to.deep.equal(contractFixtureV0.documentSchemas);

    // Verify document schema structure
    expect(roundTripped.documentSchemas.card).to.exist;
    expect(roundTripped.documentSchemas.card.properties.name).to.exist;
    expect(roundTripped.documentSchemas.card.properties.rarity.enum).to.deep.equal(['common', 'uncommon', 'rare', 'legendary']);
    expect(roundTripped.documentSchemas.card.indices).to.have.length(2);

    contract.free();
  });

  // TODO: Re-enable following once PR 2794 or similar fixes underlying issue
  it.skip('should create a V1 contract from JSON and expose all properties including tokens and groups', async () => {
    const contract = sdk.DataContract.fromJSON(contractFixtureV1, PLATFORM_VERSION_CONTRACT_V1);

    expect(contract).to.be.ok();
    expect(contract.id()).to.equal(contractFixtureV1.id);

    const roundTripped = contract.toJSON();
    expect(roundTripped).to.be.an('object');
    expect(roundTripped.id).to.equal(contractFixtureV1.id);
    expect(roundTripped.ownerId).to.equal(contractFixtureV1.ownerId);
    expect(roundTripped.version).to.equal(contractFixtureV1.version);
    expect(roundTripped.$format_version).to.equal(contractFixtureV1.$format_version);
    expect(roundTripped.config.sizedIntegerTypes).to.be.true;
    expect(roundTripped.documentSchemas).to.deep.equal(contractFixtureV1.documentSchemas);

    // Verify V1-specific features
    expect(roundTripped.tokens).to.exist;
    expect(roundTripped.tokens['0']).to.exist;
    expect(roundTripped.tokens['0'].baseSupply).to.equal(100);
    expect(roundTripped.tokens['0'].conventions.decimals).to.equal(0);

    expect(roundTripped.groups).to.exist;
    expect(roundTripped.groups['0']).to.exist;
    expect(roundTripped.groups['0'].required_power).to.equal(2);

    expect(roundTripped.keywords).to.deep.equal(contractFixtureV1.keywords);

    contract.free();
  });

  it('should fail to create a V1 contract with V0 platform version', async () => {
    expect(() => {
      sdk.DataContract.fromJSON(contractFixtureV1, PLATFORM_VERSION_CONTRACT_V0);
    }).to.throw(/dpp unknown version.*known versions.*\[0\].*received.*1/);
  });

  it('should handle memory management properly with multiple contracts', async () => {
    const contract1 = sdk.DataContract.fromJSON(contractFixtureV0, PLATFORM_VERSION_CONTRACT_V0);
    const contract2 = sdk.DataContract.fromJSON(contractFixtureV1, PLATFORM_VERSION_CONTRACT_V1);

    expect(contract1.id()).to.equal(contractFixtureV0.id);
    expect(contract2.id()).to.equal(contractFixtureV1.id);

    contract1.free();
    contract2.free();
  });

  it('should preserve all data through JSON round-trip for V0 contract', async () => {
    const contract = sdk.DataContract.fromJSON(contractFixtureV0, PLATFORM_VERSION_CONTRACT_V0);
    const roundTripped = contract.toJSON();

    // Create a new contract from the round-tripped JSON
    const contract2 = sdk.DataContract.fromJSON(roundTripped, PLATFORM_VERSION_CONTRACT_V0);
    const roundTripped2 = contract2.toJSON();

    expect(roundTripped2).to.deep.equal(roundTripped);

    contract.free();
    contract2.free();
  });

  it('should preserve all data through JSON round-trip for V1 contract', async () => {
    const contract = sdk.DataContract.fromJSON(contractFixtureV1, PLATFORM_VERSION_CONTRACT_V1);
    const roundTripped = contract.toJSON();

    // Create a new contract from the round-tripped JSON
    const contract2 = sdk.DataContract.fromJSON(roundTripped, PLATFORM_VERSION_CONTRACT_V1);
    const roundTripped2 = contract2.toJSON();

    expect(roundTripped2).to.deep.equal(roundTripped);

    contract.free();
    contract2.free();
  });  
});
