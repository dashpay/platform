import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
// The canonical corpus rs-dpp generates and pins; every client mirror of
// the contract configuration replays it. See
// packages/rs-dpp/src/data_contract/config/vectors/mod.rs.
import vectors from '../../../rs-dpp/src/data_contract/config/vectors/contract_config_vectors.json' with { type: 'json' };

interface VectorCase {
  name: string;
  platformVersion: number;
  contract: Record<string, unknown>;
  canonical: Record<string, unknown>;
  expect: Record<string, unknown>;
}

const cases = vectors.cases as VectorCase[];

/**
 * `expect` is written in JSON form, where an absent key requirement is
 * `null`. The `config` getter returns object form, where it is `undefined`.
 */
function toObjectForm(config: Record<string, unknown>): Record<string, unknown> {
  return Object.fromEntries(
    Object.entries(config).map(([key, value]) => [key, value === null ? undefined : value]),
  );
}

before(async () => {
  await initWasm();
});

describe('DataContract configuration vectors', () => {
  it('should carry every case the mirror suite expects', () => {
    expect(cases.map((c) => c.name)).to.deep.equal([
      'v1_defaults',
      'v1_all_set',
      'v0_defaults_in_v1_envelope',
      'v0_key_requirements',
      'v0_envelope',
      'v1_unknown_field_is_ignored',
    ]);
  });

  cases.forEach((vector) => {
    describe(vector.name, () => {
      it('should expose the expected configuration through the config getter', () => {
        const platformVersion = new wasm.PlatformVersion(vector.platformVersion);
        const dataContract = wasm.DataContract.fromJSON(vector.contract, true, platformVersion);

        expect(dataContract.config).to.deep.equal(toObjectForm(vector.expect));
      });

      it('should render the canonical config block through toJSON', () => {
        const platformVersion = new wasm.PlatformVersion(vector.platformVersion);
        const dataContract = wasm.DataContract.fromJSON(vector.contract, true, platformVersion);

        const json = dataContract.toJSON(platformVersion);

        expect(json.$formatVersion).to.equal(vector.canonical.$formatVersion);
        expect(json.config).to.deep.equal(vector.canonical.config);
      });

      if (vector.expect.$formatVersion === '1') {
        it('should round-trip the expected configuration through setConfig', () => {
          const platformVersion = new wasm.PlatformVersion(vector.platformVersion);
          const dataContract = wasm.DataContract.fromJSON(vector.contract, true, platformVersion);

          // The getter's output feeds setConfig back untouched; no cast.
          const config = toObjectForm(vector.expect);
          dataContract.setConfig(config, platformVersion);

          expect(dataContract.config).to.deep.equal(config);
        });
      }
    });
  });

  it('should accept the bare flags without a format tag and default sizedIntegerTypes', () => {
    const vector = cases.find((c) => c.name === 'v1_all_set') as VectorCase;
    const platformVersion = new wasm.PlatformVersion(vector.platformVersion);
    const dataContract = wasm.DataContract.fromJSON(vector.contract, true, platformVersion);

    const flags = Object.fromEntries(
      Object.entries(vector.expect).filter(([key]) => key !== '$formatVersion' && key !== 'sizedIntegerTypes'),
    );
    dataContract.setConfig(toObjectForm(flags), platformVersion);

    expect(dataContract.config).to.deep.equal({
      ...toObjectForm(flags),
      $formatVersion: '1',
      sizedIntegerTypes: true,
    });
  });

  it('should ignore unknown configuration keys and drop them when re-serializing', () => {
    const vector = cases.find((c) => c.name === 'v1_unknown_field_is_ignored');
    expect(vector).to.exist();
    const { contract, canonical, platformVersion: version } = vector as VectorCase;
    const contractConfig = contract.config as Record<string, unknown>;
    const canonicalConfig = canonical.config as Record<string, unknown>;
    const unknownKeys = Object.keys(contractConfig).filter((key) => !(key in canonicalConfig));
    expect(unknownKeys).to.not.be.empty();

    const platformVersion = new wasm.PlatformVersion(version);
    const dataContract = wasm.DataContract.fromJSON(contract, true, platformVersion);

    const config = dataContract.config as Record<string, unknown>;
    unknownKeys.forEach((key) => {
      expect(config).to.not.have.property(key);
    });
    expect(dataContract.toJSON(platformVersion).config).to.deep.equal(canonicalConfig);
  });
});
