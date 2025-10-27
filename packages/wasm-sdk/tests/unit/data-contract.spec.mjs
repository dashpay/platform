import init, * as sdk from '../../dist/sdk.compressed.js';
import contractFixtureV0 from './fixtures/data-contract-v0-crypto-card-game.mjs';
import contractFixtureV1 from './fixtures/data-contract-v1-with-docs-tokens-groups.mjs';

// Platform version configuration
const PLATFORM_VERSIONS = {
  MIN: 1,
  MAX: 10, // Update as new platform versions are released
};

// Contract format version configuration
// To add a new format version, add an entry here with:
// - The Platform version when introduced
// - A fixture for the new format
// Example: V2: { introduced: 12, fixture: contractFixtureV2 }
const CONTRACT_FORMAT_VERSIONS = {
  V0: { introduced: 1, fixture: contractFixtureV0 },
  V1: { introduced: 9, fixture: contractFixtureV1 },
};

// Validate configuration
if (PLATFORM_VERSIONS.MIN >= PLATFORM_VERSIONS.MAX) {
  throw new Error(`Invalid PLATFORM_VERSIONS: MIN (${PLATFORM_VERSIONS.MIN}) must be less than MAX (${PLATFORM_VERSIONS.MAX})`);
}

Object.entries(CONTRACT_FORMAT_VERSIONS).forEach(([key, config]) => {
  if (config.introduced < PLATFORM_VERSIONS.MIN || config.introduced > PLATFORM_VERSIONS.MAX) {
    throw new Error(`Invalid ${key}.introduced (${config.introduced}): must be between ${PLATFORM_VERSIONS.MIN} and ${PLATFORM_VERSIONS.MAX}`);
  }
});

// Auto-generate compatibility data for all formats
const FORMATS = Object.entries(CONTRACT_FORMAT_VERSIONS).reduce((acc, [formatKey, config]) => {
  const compatibleVersions = Array.from(
    { length: PLATFORM_VERSIONS.MAX - config.introduced + 1 },
    (_, i) => i + config.introduced
  );

  const incompatibleVersions = Array.from(
    { length: Math.max(0, config.introduced - PLATFORM_VERSIONS.MIN) },
    (_, i) => i + PLATFORM_VERSIONS.MIN
  );

  acc[formatKey] = {
    ...config,
    compatibleVersions,
    incompatibleVersions,
    platformVersion: config.introduced,
  };

  return acc;
}, {});

const LATEST_KNOWN_VERSION = PLATFORM_VERSIONS.MAX;

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
      const contract = sdk.DataContract.fromJSON(contractFixtureV0, FORMATS.V0.platformVersion);

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

    // TODO: enable test once an SDK fix to support this is merged
    it.skip('should create a V1 contract from JSON and expose all properties including tokens and groups', async () => {
      const contract = sdk.DataContract.fromJSON(contractFixtureV1, FORMATS.V1.platformVersion);

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
      const contract = sdk.DataContract.fromJSON(contractFixtureV0, FORMATS.V0.platformVersion);
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
        FORMATS.V1.platformVersion,
      );
      const roundTripped = contract.toJSON();

      expect(roundTripped.documentSchemas).to.deep.equal({});

      contract.free();
    });
  });

  describe('Version Compatibility', () => {
    it('should fail to create a V1 contract with V0 platform version', async () => {
      expect(() => {
        sdk.DataContract.fromJSON(contractFixtureV1, FORMATS.V0.platformVersion);
      }).to.throw(/dpp unknown version.*known versions.*\[0\].*received.*1/);
    });
  });

  describe('Validation', () => {
    it('should handle invalid JSON input gracefully', () => {
      expect(() => {
        sdk.DataContract.fromJSON(null, FORMATS.V0.platformVersion);
      }).to.throw();

      expect(() => {
        sdk.DataContract.fromJSON({}, FORMATS.V0.platformVersion);
      }).to.throw();

      expect(() => {
        sdk.DataContract.fromJSON({ id: 'invalid' }, FORMATS.V0.platformVersion);
      }).to.throw();
    });

    it('should reject contracts with invalid property values', () => {
      // Test invalid Base58 ID
      expect(() => {
        sdk.DataContract.fromJSON({
          ...contractFixtureV0,
          id: 'invalid-not-base58!',
        }, FORMATS.V0.platformVersion);
      }).to.throw();

      // Test negative version number
      expect(() => {
        sdk.DataContract.fromJSON({
          ...contractFixtureV0,
          version: -1,
        }, FORMATS.V0.platformVersion);
      }).to.throw();

      // Test invalid ownerId
      expect(() => {
        sdk.DataContract.fromJSON({
          ...contractFixtureV0,
          ownerId: 'not-a-valid-id',
        }, FORMATS.V0.platformVersion);
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
        sdk.DataContract.fromJSON(contractWithEmptySchemas, FORMATS.V0.platformVersion);
      }).to.throw(/must have at least one document type or token defined/);
    });
  });

  describe('Data Preservation', () => {
    it('should preserve all data through JSON round-trip for V0 contract', async () => {
      const contract = sdk.DataContract.fromJSON(contractFixtureV0, FORMATS.V0.platformVersion);
      const roundTripped = contract.toJSON();

      // Create a new contract from the round-tripped JSON
      const contract2 = sdk.DataContract.fromJSON(roundTripped, FORMATS.V0.platformVersion);
      const roundTripped2 = contract2.toJSON();

      expect(roundTripped2).to.deep.equal(roundTripped);

      contract.free();
      contract2.free();
    });

    it('should preserve all data through JSON round-trip for V1 contract', async () => {
      const contract = sdk.DataContract.fromJSON(contractFixtureV1, FORMATS.V1.platformVersion);
      const roundTripped = contract.toJSON();

      // Create a new contract from the round-tripped JSON
      const contract2 = sdk.DataContract.fromJSON(roundTripped, FORMATS.V1.platformVersion);
      const roundTripped2 = contract2.toJSON();

      expect(roundTripped2).to.deep.equal(roundTripped);

      contract.free();
      contract2.free();
    });
  });

  describe('Memory Management', () => {
    it('should handle memory management properly with multiple contracts', async () => {
      const contract1 = sdk.DataContract.fromJSON(contractFixtureV0, FORMATS.V0.platformVersion);
      const contract2 = sdk.DataContract.fromJSON(contractFixtureV1, FORMATS.V1.platformVersion);

      expect(contract1.id()).to.equal(contractFixtureV0.id);
      expect(contract2.id()).to.equal(contractFixtureV1.id);

      contract1.free();
      contract2.free();
    });
  });

  describe('Platform Version Compatibility Matrix', () => {
    // Dynamically test all defined contract formats
    Object.entries(FORMATS).forEach(([formatKey, formatData]) => {
      describe(`${formatKey} Contract Compatibility`, () => {
        testContractAcrossVersions(
          formatData.fixture,
          formatKey,
          formatData.compatibleVersions,
          formatData.incompatibleVersions
        );
      });
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
