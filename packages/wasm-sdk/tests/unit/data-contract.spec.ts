import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import contractFixtureV0 from './fixtures/data-contract-v0-crypto-card-game.ts';
import contractFixtureV1 from './fixtures/data-contract-v1-with-docs-tokens-groups.ts';

// Platform version constants
const PLATFORM_VERSION_CONTRACT_V0 = 1;
const PLATFORM_VERSION_CONTRACT_V1 = 9; // V1 contracts introduced in Platform v9

// Platform version compatibility ranges
const V0_COMPATIBLE_VERSIONS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]; // V0 supported versions
const V1_COMPATIBLE_VERSIONS = [9, 10]; // V1 starts at platform version 9
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
      const contract = sdk.DataContract.fromJSON(contractFixture, true, version);
      expect(contract).to.be.ok();
      expect(contract.id.toBase58()).to.equal(contractFixture.id);

      const roundTripped = contract.toJSON(version);
      expect(roundTripped.id).to.equal(contractFixture.id);

      contract.free();
    });
  });

  incompatibleVersions.forEach((version) => {
    it(`should fail with platform version ${version}`, () => {
      expect(() => {
        sdk.DataContract.fromJSON(contractFixture, true, version);
      }).to.throw(/unknown version|dpp unknown version/);
    });
  });
};

describe('DataContract', () => {
  before(async () => {
    await init();
  });

  describe('fromJSON()', () => {
    describe('V0 contract creation', () => {
      it('should create a V0 contract and expose id', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV0,
          true,
          PLATFORM_VERSION_CONTRACT_V0,
        );

        expect(contract).to.be.ok();
        expect(contract.id.toBase58()).to.equal(contractFixtureV0.id);
        contract.free();
      });

      it('should create a V0 contract with correct ownerId', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV0,
          true,
          PLATFORM_VERSION_CONTRACT_V0,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V0);
        expect(roundTripped.ownerId).to.equal(contractFixtureV0.ownerId);
        contract.free();
      });

      it('should create a V0 contract with correct version', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV0,
          true,
          PLATFORM_VERSION_CONTRACT_V0,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V0);
        expect(roundTripped.version).to.equal(contractFixtureV0.version);
        contract.free();
      });

      it('should create a V0 contract with correct format version', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV0,
          true,
          PLATFORM_VERSION_CONTRACT_V0,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V0);
        expect(roundTripped.$format_version).to.equal(contractFixtureV0.$format_version);
        contract.free();
      });

      it('should create a V0 contract with correct config', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV0,
          true,
          PLATFORM_VERSION_CONTRACT_V0,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V0);
        expect(roundTripped.config).to.deep.equal(contractFixtureV0.config);
        contract.free();
      });

      it('should create a V0 contract with correct document schemas', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV0,
          true,
          PLATFORM_VERSION_CONTRACT_V0,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V0);
        expect(roundTripped.documentSchemas).to.deep.equal(contractFixtureV0.documentSchemas);
        contract.free();
      });

      it('should create a V0 contract with card document schema', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV0,
          true,
          PLATFORM_VERSION_CONTRACT_V0,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V0);
        expect(roundTripped.documentSchemas.card).to.exist();
        expect(roundTripped.documentSchemas.card.properties.name).to.exist();
        contract.free();
      });

      it('should create a V0 contract with correct rarity enum', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV0,
          true,
          PLATFORM_VERSION_CONTRACT_V0,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V0);
        expect(roundTripped.documentSchemas.card.properties.rarity.enum)
          .to.deep.equal(['common', 'uncommon', 'rare', 'legendary']);
        contract.free();
      });

      it('should create a V0 contract with correct indices', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV0,
          true,
          PLATFORM_VERSION_CONTRACT_V0,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V0);
        expect(roundTripped.documentSchemas.card.indices).to.have.length(2);
        contract.free();
      });
    });

    describe('V1 contract creation', () => {
      it('should create a V1 contract and expose id', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );

        expect(contract).to.be.ok();
        expect(contract.id.toBase58()).to.equal(contractFixtureV1.id);
        contract.free();
      });

      it('should create a V1 contract with correct ownerId', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);
        expect(roundTripped.ownerId).to.equal(contractFixtureV1.ownerId);
        contract.free();
      });

      it('should create a V1 contract with correct version', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);
        expect(roundTripped.version).to.equal(contractFixtureV1.version);
        contract.free();
      });

      it('should create a V1 contract with correct format version', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);
        expect(roundTripped.$format_version).to.equal(contractFixtureV1.$format_version);
        contract.free();
      });

      it('should create a V1 contract with sizedIntegerTypes enabled', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);
        expect(roundTripped.config.sizedIntegerTypes).to.be.true();
        contract.free();
      });

      it('should create a V1 contract with correct document schemas', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);
        expect(roundTripped.documentSchemas).to.deep.equal(contractFixtureV1.documentSchemas);
        contract.free();
      });

      it('should create a V1 contract with tokens', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);
        expect(roundTripped.tokens).to.exist();
        expect(roundTripped.tokens['0']).to.exist();
        contract.free();
      });

      it('should create a V1 contract with correct token baseSupply', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);
        expect(roundTripped.tokens['0'].baseSupply).to.equal(100);
        contract.free();
      });

      it('should create a V1 contract with correct token decimals', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);
        expect(roundTripped.tokens['0'].conventions.decimals).to.equal(0);
        contract.free();
      });

      it('should create a V1 contract with groups', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);
        expect(roundTripped.groups).to.exist();
        expect(roundTripped.groups['0']).to.exist();
        contract.free();
      });

      it('should create a V1 contract with correct group required_power', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);
        expect(roundTripped.groups['0'].required_power).to.equal(2);
        contract.free();
      });

      it('should create a V1 contract with correct keywords', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);
        expect(roundTripped.keywords).to.deep.equal(contractFixtureV1.keywords);
        contract.free();
      });
    });

    describe('document-only contracts', () => {
      it('should create a contract with only document schemas (no tokens)', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV0,
          true,
          PLATFORM_VERSION_CONTRACT_V0,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V0);

        expect(roundTripped.documentSchemas.card).to.exist();
        expect(roundTripped.tokens).to.equal(undefined);

        contract.free();
      });
    });

    describe('token-only contracts', () => {
      it('should create a contract with only tokens (no documents)', () => {
        const contractWithOnlyTokens = {
          ...contractFixtureV1,
          documentSchemas: {},
        };

        const contract = sdk.DataContract.fromJSON(
          contractWithOnlyTokens,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);

        expect(roundTripped.documentSchemas).to.deep.equal({});

        contract.free();
      });
    });

    describe('version compatibility', () => {
      it('should fail to create a V1 contract with V0 platform version', () => {
        expect(() => {
          sdk.DataContract.fromJSON(
            contractFixtureV1,
            true,
            PLATFORM_VERSION_CONTRACT_V0,
          );
        }).to.throw(/dpp unknown version.*known versions.*\[0\].*received.*1/);
      });
    });

    describe('input validation', () => {
      it('should throw for null input', () => {
        expect(() => {
          sdk.DataContract.fromJSON(null, true, PLATFORM_VERSION_CONTRACT_V0);
        }).to.throw();
      });

      it('should throw for empty object input', () => {
        expect(() => {
          sdk.DataContract.fromJSON({}, true, PLATFORM_VERSION_CONTRACT_V0);
        }).to.throw();
      });

      it('should throw for object with only invalid id', () => {
        expect(() => {
          sdk.DataContract.fromJSON({ id: 'invalid' }, true, PLATFORM_VERSION_CONTRACT_V0);
        }).to.throw();
      });
    });
  });

  describe('fromValue()', () => {
    describe('input validation', () => {
      it('should reject invalid Base58 ID', () => {
        expect(() => {
          sdk.DataContract.fromValue({
            ...contractFixtureV0,
            id: 'invalid-not-base58!',
          }, true, PLATFORM_VERSION_CONTRACT_V0);
        }).to.throw();
      });

      it('should reject negative version number', () => {
        expect(() => {
          sdk.DataContract.fromValue({
            ...contractFixtureV0,
            version: -1,
          }, true, PLATFORM_VERSION_CONTRACT_V0);
        }).to.throw();
      });

      it('should reject invalid ownerId', () => {
        expect(() => {
          sdk.DataContract.fromValue({
            ...contractFixtureV0,
            ownerId: 'not-a-valid-id',
          }, true, PLATFORM_VERSION_CONTRACT_V0);
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
          sdk.DataContract.fromJSON(
            contractWithEmptySchemas,
            true,
            PLATFORM_VERSION_CONTRACT_V0,
          );
        }).to.throw(/must have at least one document type or token defined/);
      });
    });
  });

  describe('toJSON()', () => {
    describe('V0 contract round-trip', () => {
      it('should preserve all data through JSON round-trip', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV0,
          true,
          PLATFORM_VERSION_CONTRACT_V0,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V0);

        const contract2 = sdk.DataContract.fromJSON(
          roundTripped,
          true,
          PLATFORM_VERSION_CONTRACT_V0,
        );
        const roundTripped2 = contract2.toJSON(PLATFORM_VERSION_CONTRACT_V0);

        expect(roundTripped2).to.deep.equal(roundTripped);

        contract.free();
        contract2.free();
      });
    });

    describe('V1 contract round-trip', () => {
      it('should preserve all data through JSON round-trip', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);

        const contract2 = sdk.DataContract.fromJSON(
          roundTripped,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const roundTripped2 = contract2.toJSON(PLATFORM_VERSION_CONTRACT_V1);

        expect(roundTripped2).to.deep.equal(roundTripped);

        contract.free();
        contract2.free();
      });

      it('should preserve all data through JSON round-trip (object output)', () => {
        const contract = sdk.DataContract.fromJSON(
          contractFixtureV1,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );
        const jsonRepresentation = contract.toJSON(PLATFORM_VERSION_CONTRACT_V1);

        const contract2 = sdk.DataContract.fromJSON(
          jsonRepresentation,
          true,
          PLATFORM_VERSION_CONTRACT_V1,
        );

        expect(contract2.toJSON(PLATFORM_VERSION_CONTRACT_V1)).to.deep.equal(jsonRepresentation);

        contract.free();
        contract2.free();
      });
    });
  });

  describe('free()', () => {
    it('should handle memory management properly with multiple contracts', () => {
      const contract1 = sdk.DataContract.fromJSON(
        contractFixtureV0,
        true,
        PLATFORM_VERSION_CONTRACT_V0,
      );
      const contract2 = sdk.DataContract.fromJSON(
        contractFixtureV1,
        true,
        PLATFORM_VERSION_CONTRACT_V1,
      );

      expect(contract1.id.toBase58()).to.equal(contractFixtureV0.id);
      expect(contract2.id.toBase58()).to.equal(contractFixtureV1.id);

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
      it('should fail with zero version', () => {
        expect(() => {
          sdk.DataContract.fromJSON(contractFixtureV0, true, 0);
        }).to.throw(/unknown version|unknown platform version value/);
      });

      it('should fail with negative version', () => {
        expect(() => {
          sdk.DataContract.fromJSON(contractFixtureV0, true, -1);
        }).to.throw(/unknown version|unknown platform version value/);
      });

      it('should fail with version beyond latest known', () => {
        expect(() => {
          sdk.DataContract.fromJSON(contractFixtureV0, true, LATEST_KNOWN_VERSION + 1);
        }).to.throw(/unknown version|unknown platform version value/);
      });

      it('should fail with version far beyond reasonable range', () => {
        expect(() => {
          sdk.DataContract.fromJSON(contractFixtureV0, true, LATEST_KNOWN_VERSION * 10);
        }).to.throw(/unknown version|unknown platform version value/);
      });

      it('should support V0 contract at V9 transition (backward compatibility)', () => {
        const contract = sdk.DataContract.fromJSON(contractFixtureV0, true, 9);
        expect(contract.id.toBase58()).to.equal(contractFixtureV0.id);
        contract.free();
      });

      it('should support V1 contract at V9 (first supported version)', () => {
        const contractV1 = sdk.DataContract.fromJSON(contractFixtureV1, true, 9);
        expect(contractV1.id.toBase58()).to.equal(contractFixtureV1.id);
        contractV1.free();
      });

      it('should fail V1 contract at V8 (last unsupported version)', () => {
        expect(() => {
          sdk.DataContract.fromJSON(contractFixtureV1, true, 8);
        }).to.throw(/dpp unknown version/);
      });
    });
  });
});
