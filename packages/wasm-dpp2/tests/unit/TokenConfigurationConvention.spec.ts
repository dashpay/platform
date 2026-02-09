import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import tokenLocalization from './mocks/TokenConfiguration/index.ts';

before(async () => {
  await initWasm();
});

describe('TokenConfigurationConvention', () => {
  describe('constructor', () => {
    it('should create instance from object', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: tokenLocalization,
        },
        1,
      );

      expect(convention).to.be.an.instanceof(wasm.TokenConfigurationConvention);
    });
  });

  describe('localizations', () => {
    it('should return localizations as JSON', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: tokenLocalization,
        },
        1,
      );

      expect(convention.localizations.ru.toJSON()).to.deep.equal(tokenLocalization);
    });

    it('should return localizations as wasm instance', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: tokenLocalization,
        },
        1,
      );

      expect(convention.localizations.constructor.name).to.deep.equal('Object');
      expect(convention.localizations.ru.constructor.name).to.deep.equal('TokenConfigurationLocalization');
    });

    it('should set localizations object', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: tokenLocalization,
        },
        1,
      );

      convention.localizations = {
        en: tokenLocalization,
      };

      expect(convention.localizations.constructor.name).to.deep.equal('Object');
      expect(convention.localizations.ru).to.deep.equal(undefined);
      expect(convention.localizations.en.constructor.name).to.deep.equal('TokenConfigurationLocalization');
    });

    it('should set localizations object with wasm instance', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: tokenLocalization,
        },
        1,
      );

      const localization = new wasm.TokenConfigurationLocalization(false, 'singularForm', 'pluralForm');

      convention.localizations = {
        en: localization,
      };

      expect(convention.localizations.constructor.name).to.deep.equal('Object');
      expect(convention.localizations.ru).to.deep.equal(undefined);
      expect(convention.localizations.en.constructor.name).to.deep.equal('TokenConfigurationLocalization');
      expect(convention.localizations.en.toJSON()).to.deep.equal({
        $format_version: '0',
        shouldCapitalize: false,
        singularForm: 'singularForm',
        pluralForm: 'pluralForm',
      });
      expect(localization).to.be.an.instanceof(wasm.TokenConfigurationLocalization);
    });
  });

  describe('decimals', () => {
    it('should return decimals', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: tokenLocalization,
        },
        1,
      );

      expect(convention.decimals).to.deep.equal(1);
    });
  });
});
