import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';


before(async () => {
  await initWasm();
});

describe('TokenPreProgrammedDistribution', () => {
  // Helper to create an Identifier object for testing
  function createIdentifier(base58String: string) {
    return new wasm.Identifier(base58String);
  }

  // Helper to create distributions Map in the expected format
  function createDistributionsMap(timestamp: string, identifierStr: string, amount: bigint) {
    const innerMap = new Map();
    innerMap.set(createIdentifier(identifierStr), amount);

    const outerMap = new Map();
    outerMap.set(timestamp.toString(), innerMap);

    return outerMap;
  }

  describe('serialization / deserialization', () => {
    it('should allow to create from values', () => {
      const distributions = createDistributionsMap(
        '1750140416485',
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );

      const preProgrammedDistribution = new wasm.TokenPreProgrammedDistribution(distributions);

      expect(preProgrammedDistribution).to.be.an.instanceof(wasm.TokenPreProgrammedDistribution);
    });
  });

  describe('getters', () => {
    it('should allow to get distributions', () => {
      const distributions = createDistributionsMap(
        '1750140416485',
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10100),
      );

      const preProgrammedDistribution = new wasm.TokenPreProgrammedDistribution(distributions);

      // The getter returns a Map, check it has the expected structure
      const result = preProgrammedDistribution.distributions;
      expect(result instanceof Map).to.equal(true);
      expect(result.has('1750140416485')).to.equal(true);

      const innerMap = result.get('1750140416485');
      expect(innerMap instanceof Map).to.equal(true);
      expect(innerMap.size).to.equal(1);
    });
  });

  describe('setters', () => {
    it('should allow to set distributions', () => {
      const distributions = createDistributionsMap(
        '1750140416485',
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10100),
      );

      const preProgrammedDistribution = new wasm.TokenPreProgrammedDistribution(distributions);

      const newDistributions = createDistributionsMap(
        '1750140416415',
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(9999999),
      );

      preProgrammedDistribution.distributions = newDistributions;

      const result = preProgrammedDistribution.distributions;
      expect(result instanceof Map).to.equal(true);
      expect(result.has('1750140416415')).to.equal(true);
    });
  });
});
