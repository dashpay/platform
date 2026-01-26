import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('TokenConfigurationLocalization', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create from values', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      expect(localization).to.be.an.instanceof(wasm.TokenConfigurationLocalization);
    });

    it('should recreate localization from JSON', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');
      const json = localization.toJSON();

      const restored = wasm.TokenConfigurationLocalization.fromJSON(json);

      expect(restored.toJSON()).to.deep.equal(json);
    });

    it('should recreate localization from object', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');
      const object = localization.toJSON();

      const restored = wasm.TokenConfigurationLocalization.fromObject(object);

      expect(restored.toJSON()).to.deep.equal(object);
    });
  });

  describe('getters', () => {
    it('should allow to get shouldCapitalize', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      expect(localization.shouldCapitalize).to.equal(false);
    });

    it('should allow to get pluralForm', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      expect(localization.pluralForm).to.equal('pluralForm');
    });

    it('should allow to get singularForm', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      expect(localization.singularForm).to.equal('singularForm');
    });
  });

  describe('setters', () => {
    it('should allow to set shouldCapitalize', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      localization.shouldCapitalize = true;

      expect(localization.shouldCapitalize).to.equal(true);
    });

    it('should allow to set pluralForm', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      localization.pluralForm = 'pluralForm1212';

      expect(localization.pluralForm).to.equal('pluralForm1212');
    });

    it('should allow to set singularForm', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      localization.singularForm = 'singularForm12121';

      expect(localization.singularForm).to.equal('singularForm12121');
    });
  });
});
