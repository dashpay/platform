import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('TokenConfigurationLocalization', () => {
  describe('constructor', () => {
    it('should create instance from values', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      expect(localization).to.be.an.instanceof(wasm.TokenConfigurationLocalization);
    });
  });

  describe('toJSON()', () => {
    it('should convert to JSON', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');
      const json = localization.toJSON();

      expect(json).to.deep.equal({
        $format_version: '0',
        shouldCapitalize: false,
        singularForm: 'singularForm',
        pluralForm: 'pluralForm',
      });
    });
  });

  describe('fromJSON()', () => {
    it('should create instance from JSON', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');
      const json = localization.toJSON();

      const restored = wasm.TokenConfigurationLocalization.fromJSON(json);

      expect(restored.toJSON()).to.deep.equal(json);
    });
  });

  describe('fromObject()', () => {
    it('should create instance from object', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');
      const object = localization.toJSON();

      const restored = wasm.TokenConfigurationLocalization.fromObject(object);

      expect(restored.toJSON()).to.deep.equal(object);
    });
  });

  describe('shouldCapitalize', () => {
    it('should return shouldCapitalize', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      expect(localization.shouldCapitalize).to.equal(false);
    });

    it('should set shouldCapitalize', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      localization.shouldCapitalize = true;

      expect(localization.shouldCapitalize).to.equal(true);
    });
  });

  describe('pluralForm', () => {
    it('should return pluralForm', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      expect(localization.pluralForm).to.equal('pluralForm');
    });

    it('should set pluralForm', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      localization.pluralForm = 'pluralForm1212';

      expect(localization.pluralForm).to.equal('pluralForm1212');
    });
  });

  describe('singularForm', () => {
    it('should return singularForm', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      expect(localization.singularForm).to.equal('singularForm');
    });

    it('should set singularForm', () => {
      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      localization.singularForm = 'singularForm12121';

      expect(localization.singularForm).to.equal('singularForm12121');
    });
  });
});
