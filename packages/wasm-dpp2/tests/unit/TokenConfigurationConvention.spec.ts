import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { tokenLocalization } from './mocks/TokenConfiguration/index.js';


before(async () => {
  await initWasm();
});

describe('TokenConfigurationConvention', () => {
  describe('serialization / deserialization', () => {
    it('Should allow to create from object', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: tokenLocalization,
        },
        1,
      );

      expect(convention).to.be.an.instanceof(wasm.TokenConfigurationConvention);
    });
  });

  describe('getters', () => {
    it('Should allow to get object of convention in JSON', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: tokenLocalization,
        },
        1,
      );

      expect(convention.localizations.ru.toJSON()).to.deep.equal(tokenLocalization);
    });

    it('Should allow to get object of convention in wasm instance', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: tokenLocalization,
        },
        1,
      );

      expect(convention.localizations.constructor.name).to.deep.equal('Object');
      expect(convention.localizations.ru.constructor.name).to.deep.equal('TokenConfigurationLocalization');
    });

    it('Should allow to get decimals', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: tokenLocalization,
        },
        1,
      );

      expect(convention.decimals).to.deep.equal(1);
    });
  });

  describe('setters', () => {
    it('Should allow to set localizations object ', () => {
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

    it('Should allow to set localizations object with wasm ', () => {
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
});
