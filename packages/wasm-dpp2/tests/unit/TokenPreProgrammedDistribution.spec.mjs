import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('TokenPreProgrammedDistribution', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create from values', () => {
      const preProgrammedDistribution = new wasm.TokenPreProgrammedDistribution(
        {
          1750140416485: {
            PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB: BigInt(10000),
          },
        },
      );

      expect(preProgrammedDistribution.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to get distributions', () => {
      const preProgrammedDistribution = new wasm.TokenPreProgrammedDistribution(
        {
          1750140416485: {
            PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB: BigInt(10100),
          },
        },
      );

      expect(preProgrammedDistribution.distributions).to.deep.equal({
        1750140416485: {
          PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB: BigInt(10100),
        },
      });
    });
  });

  describe('setters', () => {
    it('should allow to set distributions', () => {
      const preProgrammedDistribution = new wasm.TokenPreProgrammedDistribution(
        {
          1750140416485: {
            PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB: BigInt(10100),
          },
        },
      );

      preProgrammedDistribution.distributions = {
        1750140416415: {
          PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB: BigInt(9999999),
        },
      };

      expect(preProgrammedDistribution.distributions).to.deep.equal({
        1750140416415: {
          PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB: BigInt(9999999),
        },
      });
    });
  });
});
