import init, * as sdk from '../../dist/sdk.compressed.js';
import contractFixtureV0 from './fixtures/data-contract-v0-crypto-card-game.mjs';
import contractFixtureV1 from './fixtures/data-contract-v1-with-docs-tokens-groups.mjs';

// Platform version constants
const PLATFORM_VERSION_CONTRACT_V0 = 1;
const PLATFORM_VERSION_CONTRACT_V1 = 9; // V1 contracts introduced in Platform v9

// Platform version compatibility ranges
const V0_COMPATIBLE_VERSIONS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]; // V0 works across all versions
const V1_COMPATIBLE_VERSIONS = [9, 10]; // V1 only works from version 9+
const V0_ONLY_VERSIONS = [1, 2, 3, 4, 5, 6, 7, 8]; // Versions that only support V0
const LATEST_KNOWN_VERSION = Math.max(...V0_COMPATIBLE_VERSIONS);

// Helper function for testing contract compatibility across versions
const testContractAcrossVersions = (
  contractFixture,
  contractName,
  compatibleVersions,
  incompatibleVersions = [],
) => {
  compatibleVersions.forEach((version) => {
    it(`should work with platform version ${version}`, () => {
      const contract = sdk.DataContract.fromJSON(contractFixture, version);
      expect(contract).to.be.ok();
      expect(contract.id()).to.equal(contractFixture.id);

      const roundTripped = contract.toJSON();
      expect(roundTripped.id).to.equal(contractFixture.id);

      contract.free();
    });
  });

  incompatibleVersions.forEach((version) => {
    it(`should fail with platform version ${version}`, () => {
      expect(() => {
        sdk.DataContract.fromJSON(contractFixture, version);
      }).to.throw(/unknown version|dpp unknown version/);
    });
  });
};

describe('DataContract', () => {
  before(async () => {
    await init();
  });

  describe('Contract Creation', () => {
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
      expect(roundTripped.documentSchemas.card).to.exist();
      expect(roundTripped.documentSchemas.card.properties.name).to.exist();
      expect(roundTripped.documentSchemas.card.properties.rarity.enum)
        .to.deep.equal(['common', 'uncommon', 'rare', 'legendary']);
      expect(roundTripped.documentSchemas.card.indices).to.have.length(2);

      contract.free();
    });

    it('should create a V1 contract from JSON and expose all properties including tokens and groups', async () => {
      const contract = sdk.DataContract.fromJSON(contractFixtureV1, PLATFORM_VERSION_CONTRACT_V1);

      expect(contract).to.be.ok();
      expect(contract.id()).to.equal(contractFixtureV1.id);

      const roundTripped = contract.toJSON();
      expect(roundTripped).to.be.an('object');
      expect(roundTripped.id).to.equal(contractFixtureV1.id);
      expect(roundTripped.ownerId).to.equal(contractFixtureV1.ownerId);
      expect(roundTripped.version).to.equal(contractFixtureV1.version);
      expect(roundTripped.$format_version).to.equal(contractFixtureV1.$format_version);
      expect(roundTripped.config.sizedIntegerTypes).to.be.true();
      expect(roundTripped.documentSchemas).to.deep.equal(contractFixtureV1.documentSchemas);

      // Verify V1-specific features
      expect(roundTripped.tokens).to.exist();
      expect(roundTripped.tokens['0']).to.exist();
      expect(roundTripped.tokens['0'].baseSupply).to.equal(100);
      expect(roundTripped.tokens['0'].conventions.decimals).to.equal(0);

      expect(roundTripped.groups).to.exist();
      expect(roundTripped.groups['0']).to.exist();
      expect(roundTripped.groups['0'].required_power).to.equal(2);

      expect(roundTripped.keywords).to.deep.equal(contractFixtureV1.keywords);

      contract.free();
    });

    it('should create a contract with only document schemas (no tokens)', () => {
      // V0 fixture already has only documents, no tokens - verify it works
      const contract = sdk.DataContract.fromJSON(contractFixtureV0, PLATFORM_VERSION_CONTRACT_V0);
      const roundTripped = contract.toJSON();

      expect(roundTripped.documentSchemas.card).to.exist();
      expect(roundTripped.tokens).to.equal(undefined);

      contract.free();
    });

    it('should create a contract with only tokens (no documents)', () => {
      // Use V1 fixture but remove documentSchemas
      const contractWithOnlyTokens = {
        ...contractFixtureV1,
        documentSchemas: {},
      };

      const contract = sdk.DataContract.fromJSON(
        contractWithOnlyTokens,
        PLATFORM_VERSION_CONTRACT_V1,
      );
      const roundTripped = contract.toJSON();

      expect(roundTripped.documentSchemas).to.deep.equal({});

      contract.free();
    });
  });

  describe('Version Compatibility', () => {
    it('should fail to create a V1 contract with V0 platform version', async () => {
      expect(() => {
        sdk.DataContract.fromJSON(contractFixtureV1, PLATFORM_VERSION_CONTRACT_V0);
      }).to.throw(/dpp unknown version.*known versions.*\[0\].*received.*1/);
    });
  });

  describe('Validation', () => {
    it('should handle invalid JSON input gracefully', () => {
      expect(() => {
        sdk.DataContract.fromJSON(null, PLATFORM_VERSION_CONTRACT_V0);
      }).to.throw();

      expect(() => {
        sdk.DataContract.fromJSON({}, PLATFORM_VERSION_CONTRACT_V0);
      }).to.throw();

      expect(() => {
        sdk.DataContract.fromJSON({ id: 'invalid' }, PLATFORM_VERSION_CONTRACT_V0);
      }).to.throw();
    });

    it('should reject contracts with invalid property values', () => {
      // Test invalid Base58 ID
      expect(() => {
        sdk.DataContract.fromJSON({
          ...contractFixtureV0,
          id: 'invalid-not-base58!',
        }, PLATFORM_VERSION_CONTRACT_V0);
      }).to.throw();

      // Test negative version number
      expect(() => {
        sdk.DataContract.fromJSON({
          ...contractFixtureV0,
          version: -1,
        }, PLATFORM_VERSION_CONTRACT_V0);
      }).to.throw();

      // Test invalid ownerId
      expect(() => {
        sdk.DataContract.fromJSON({
          ...contractFixtureV0,
          ownerId: 'not-a-valid-id',
        }, PLATFORM_VERSION_CONTRACT_V0);
      }).to.throw();
    });

    it('should require at least one document type or token', () => {
      const contractWithEmptySchemas = {
        $format_version: '0',
        id: contractFixtureV0.id,
        ownerId: contractFixtureV0.ownerId,
        version: 1,
        config: contractFixtureV0.config,
        documentSchemas: {},
      };

      expect(() => {
        sdk.DataContract.fromJSON(contractWithEmptySchemas, PLATFORM_VERSION_CONTRACT_V0);
      }).to.throw(/must have at least one document type or token defined/);
    });
  });

  describe('Data Preservation', () => {
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

  describe('Memory Management', () => {
    it('should handle memory management properly with multiple contracts', async () => {
      const contract1 = sdk.DataContract.fromJSON(contractFixtureV0, PLATFORM_VERSION_CONTRACT_V0);
      const contract2 = sdk.DataContract.fromJSON(contractFixtureV1, PLATFORM_VERSION_CONTRACT_V1);

      expect(contract1.id()).to.equal(contractFixtureV0.id);
      expect(contract2.id()).to.equal(contractFixtureV1.id);

      contract1.free();
      contract2.free();
    });
  });

  describe('Platform Version Compatibility Matrix', () => {
    describe('V0 Contract Compatibility', () => {
      testContractAcrossVersions(contractFixtureV0, 'V0', V0_COMPATIBLE_VERSIONS);
    });

    describe('V1 Contract Compatibility', () => {
      testContractAcrossVersions(contractFixtureV1, 'V1', V1_COMPATIBLE_VERSIONS, V0_ONLY_VERSIONS);
    });

    describe('Edge Cases', () => {
      it('should fail with invalid version numbers', () => {
        const invalidVersions = [
          0, // Zero version
          -1, // Negative version
          LATEST_KNOWN_VERSION + 1, // One beyond latest known
          LATEST_KNOWN_VERSION * 10, // Far beyond reasonable range
        ];

        invalidVersions.forEach((version) => {
          expect(() => {
            sdk.DataContract.fromJSON(contractFixtureV0, version);
          }).to.throw(/unknown version/);
        });
      });

      it('should handle version boundary correctly at V9 transition', () => {
        // V0 contract should work in V9 (backward compatibility)
        const contract = sdk.DataContract.fromJSON(contractFixtureV0, 9);
        expect(contract.id()).to.equal(contractFixtureV0.id);
        contract.free();

        // V1 contract should work in V9 (first supported version)
        const contractV1 = sdk.DataContract.fromJSON(contractFixtureV1, 9);
        expect(contractV1.id()).to.equal(contractFixtureV1.id);
        contractV1.free();

        // V1 contract should fail in V8 (last unsupported version)
        expect(() => {
          sdk.DataContract.fromJSON(contractFixtureV1, 8);
        }).to.throw(/dpp unknown version/);
      });
    });
  });
});
