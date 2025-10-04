import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('TokenConfigurationLocalization', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create from values', () => {
      const localization = new wasm.TokenConfigurationLocalizationWASM(false, 'singularForm', 'pluralForm');

      expect(localization.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to get shouldCapitalize', () => {
      const localization = new wasm.TokenConfigurationLocalizationWASM(false, 'singularForm', 'pluralForm');

      expect(localization.shouldCapitalize).to.equal(false);
    });

    it('should allow to get pluralForm', () => {
      const localization = new wasm.TokenConfigurationLocalizationWASM(false, 'singularForm', 'pluralForm');

      expect(localization.pluralForm).to.equal('pluralForm');
    });

    it('should allow to get singularForm', () => {
      const localization = new wasm.TokenConfigurationLocalizationWASM(false, 'singularForm', 'pluralForm');

      expect(localization.singularForm).to.equal('singularForm');
    });
  });

  describe('setters', () => {
    it('should allow to set shouldCapitalize', () => {
      const localization = new wasm.TokenConfigurationLocalizationWASM(false, 'singularForm', 'pluralForm');

      localization.shouldCapitalize = true;

      expect(localization.shouldCapitalize).to.equal(true);
    });

    it('should allow to set pluralForm', () => {
      const localization = new wasm.TokenConfigurationLocalizationWASM(false, 'singularForm', 'pluralForm');

      localization.pluralForm = 'pluralForm1212';

      expect(localization.pluralForm).to.equal('pluralForm1212');
    });

    it('should allow to set singularForm', () => {
      const localization = new wasm.TokenConfigurationLocalizationWASM(false, 'singularForm', 'pluralForm');

      localization.singularForm = 'singularForm12121';

      expect(localization.singularForm).to.equal('singularForm12121');
    });
  });
});
