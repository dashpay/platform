const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../lib/validation/JsonSchemaValidator');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const validateContractFactory = require('../../../lib/contract/validateContractFactory');

const getContractFixture = require('../../../lib/test/fixtures/getContractFixture');

const { expectJsonSchemaError, expectValidationError } = require('../../../lib/test/expect/expectError');

const DuplicateIndexError = require('../../../lib/errors/DuplicateIndexError');
const UniqueIndexMustHaveUserIdPrefixError = require('../../../lib/errors/UniqueIndexMustHaveUserIdPrefixError');
const UndefinedIndexPropertyError = require('../../../lib/errors/UndefinedIndexPropertyError');

describe('validateContractFactory', () => {
  let rawContract;
  let validateContract;

  beforeEach(() => {
    rawContract = getContractFixture().toJSON();

    const ajv = new Ajv();
    const validator = new JsonSchemaValidator(ajv);

    validateContract = validateContractFactory(validator);
  });

  describe('$schema', () => {
    it('should be present', () => {
      delete rawContract.$schema;

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('$schema');
    });

    it('should be a string', () => {
      rawContract.$schema = 1;

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.$schema');
      expect(error.keyword).to.equal('type');
    });

    it('should be a particular url', () => {
      rawContract.$schema = 'wrong';

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('const');
      expect(error.dataPath).to.equal('.$schema');
    });
  });

  describe('name', () => {
    it('should be present', () => {
      delete rawContract.name;

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('name');
    });

    it('should be a string', () => {
      rawContract.name = 1;

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.name');
      expect(error.keyword).to.equal('type');
    });

    it('should be greater or equal to 3', () => {
      rawContract.name = 'a'.repeat(2);

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.name');
      expect(error.keyword).to.equal('minLength');
    });

    it('should be less or equal to 24', () => {
      rawContract.name = 'a'.repeat(25);

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.name');
      expect(error.keyword).to.equal('maxLength');
    });

    it('should be the valid string format', () => {
      const validNames = ['validName', 'valid_name', 'valid-name', 'abc', '123abc', 'abc123',
        'abcdefghigklmnopqrstuvwx', 'ValidName', 'abc_gbf_gdb', 'abc-gbf-gdb'];

      validNames.forEach((name) => {
        rawContract.name = name;

        const result = validateContract(rawContract);

        expectJsonSchemaError(result, 0);
      });
    });

    it('should return an invalid result if a string is in invalid format', () => {
      const invalidNames = ['-invalidname', '_invalidname', 'invalidname-', 'invalidname_', '*(*&^', '$test'];

      invalidNames.forEach((name) => {
        rawContract.name = name;

        const result = validateContract(rawContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.name');
        expect(error.keyword).to.equal('pattern');
      });
    });
  });

  describe('version', () => {
    it('should be present', () => {
      delete rawContract.version;

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('version');
    });

    it('should be a number', () => {
      rawContract.version = 'wrong';

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.version');
      expect(error.keyword).to.equal('type');
    });

    it('should be an integer', () => {
      rawContract.version = 1.2;

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.version');
      expect(error.keyword).to.equal('multipleOf');
    });

    it('should be greater or equal to one', () => {
      rawContract.version = 0;

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.version');
      expect(error.keyword).to.equal('minimum');
    });
  });

  describe('definitions', () => {
    it('may not be present', () => {
      delete rawContract.definitions;

      const result = validateContract(rawContract);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should be an object', () => {
      rawContract.definitions = 1;

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.definitions');
      expect(error.keyword).to.equal('type');
    });

    it('should not be empty', () => {
      rawContract.definitions = {};

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.definitions');
      expect(error.keyword).to.equal('minProperties');
    });

    it('should have no non-alphanumeric properties', () => {
      rawContract.definitions = {
        $subSchema: {},
      };

      const result = validateContract(rawContract);

      expectJsonSchemaError(result, 2);

      const [patternError, propertyNamesError] = result.getErrors();

      expect(patternError.dataPath).to.equal('.definitions');
      expect(patternError.keyword).to.equal('pattern');

      expect(propertyNamesError.dataPath).to.equal('.definitions');
      expect(propertyNamesError.keyword).to.equal('propertyNames');
    });

    it('should have no more than 100 properties', () => {
      rawContract.definitions = {};

      Array(101).fill({}).forEach((item, i) => {
        rawContract.definitions[i] = item;
      });

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.definitions');
      expect(error.keyword).to.equal('maxProperties');
    });

    it('should have valid property names', () => {
      const validNames = ['validName', 'valid_name', 'valid-name', 'abc', '123abc', 'abc123', 'ValidName',
        'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb'];

      validNames.forEach((name) => {
        rawContract.definitions[name] = {};

        const result = validateContract(rawContract);

        expectJsonSchemaError(result, 0);
      });
    });

    it('should return an invalid result if a property has invalid format', () => {
      const invalidNames = ['-invalidname', '_invalidname', 'invalidname-', 'invalidname_', '*(*&^', '$test'];

      invalidNames.forEach((name) => {
        rawContract.definitions[name] = {};

        const result = validateContract(rawContract);

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.definitions');
        expect(error.keyword).to.equal('pattern');
      });
    });
  });

  describe('documents', () => {
    it('should be present', () => {
      delete rawContract.documents;

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('documents');
    });

    it('should be an object', () => {
      rawContract.documents = 1;

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.documents');
      expect(error.keyword).to.equal('type');
    });

    it('should not be empty', () => {
      rawContract.documents = {};

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.documents');
      expect(error.keyword).to.equal('minProperties');
    });

    it('should have valid property names', () => {
      const validNames = ['validName', 'valid_name', 'valid-name', 'abc', '123abc', 'abc123', 'ValidName', 'validName',
        'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb'];

      validNames.forEach((name) => {
        rawContract.documents[name] = rawContract.documents.niceDocument;

        const result = validateContract(rawContract);

        expectJsonSchemaError(result, 0);
      });
    });

    it('should return an invalid result if a property has invalid format', () => {
      const invalidNames = ['-invalidname', '_invalidname', 'invalidname-', 'invalidname_', '*(*&^', '$test'];

      invalidNames.forEach((name) => {
        rawContract.documents[name] = rawContract.documents.niceDocument;

        const result = validateContract(rawContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents');
        expect(error.keyword).to.equal('additionalProperties');
      });
    });

    it('should have no more than 100 properties', () => {
      const niceDocumentDefinition = rawContract.documents.niceDocument;

      rawContract.documents = {};

      Array(101).fill(niceDocumentDefinition).forEach((item, i) => {
        rawContract.documents[i] = item;
      });

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.documents');
      expect(error.keyword).to.equal('maxProperties');
    });

    describe('Document schema', () => {
      it('should not be empty', () => {
        rawContract.documents.niceDocument.properties = {};

        const result = validateContract(rawContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\'].properties');
        expect(error.keyword).to.equal('minProperties');
      });

      it('should have type "object" if defined', () => {
        delete rawContract.documents.niceDocument.properties;

        const result = validateContract(rawContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\']');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('properties');
      });

      it('should have "properties"', () => {
        delete rawContract.documents.niceDocument.properties;

        const result = validateContract(rawContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\']');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('properties');
      });

      it('should have valid property names', () => {
        const validNames = ['validName', 'valid_name', 'valid-name', 'abc', '123abc', 'abc123', 'ValidName', 'validName',
          'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb'];

        validNames.forEach((name) => {
          rawContract.documents.niceDocument.properties[name] = {};

          const result = validateContract(rawContract);

          expectJsonSchemaError(result, 0);
        });
      });

      it('should return an invalid result if a property has invalid format', () => {
        const invalidNames = ['-invalidname', '_invalidname', 'invalidname-', 'invalidname_', '*(*&^', '$test'];

        invalidNames.forEach((name) => {
          rawContract.documents.niceDocument.properties[name] = {};

          const result = validateContract(rawContract);

          expectJsonSchemaError(result, 2);

          const errors = result.getErrors();

          expect(errors[0].dataPath).to.equal('.documents[\'niceDocument\'].properties');
          expect(errors[0].keyword).to.equal('pattern');
          expect(errors[1].dataPath).to.equal('.documents[\'niceDocument\'].properties');
          expect(errors[1].keyword).to.equal('propertyNames');
        });
      });

      it('should have "additionalProperties" defined', () => {
        delete rawContract.documents.niceDocument.additionalProperties;

        const result = validateContract(rawContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\']');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('additionalProperties');
      });

      it('should have "additionalProperties" defined to false', () => {
        rawContract.documents.niceDocument.additionalProperties = true;

        const result = validateContract(rawContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\'].additionalProperties');
        expect(error.keyword).to.equal('const');
      });

      it('should have no more than 100 properties', () => {
        const propertyDefinition = { };

        rawContract.documents.niceDocument.properties = {};

        Array(101).fill(propertyDefinition).forEach((item, i) => {
          rawContract.documents.niceDocument.properties[i] = item;
        });

        const result = validateContract(rawContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\'].properties');
        expect(error.keyword).to.equal('maxProperties');
      });
    });
  });

  describe('indices', () => {
    it('should be an array', () => {
      rawContract.documents.indexedDocument.indices = 'definitely not an array';

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].indices');
      expect(error.keyword).to.equal('type');
    });

    it('should have at least one item', () => {
      rawContract.documents.indexedDocument.indices = [];

      const result = validateContract(rawContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].indices');
      expect(error.keyword).to.equal('minItems');
    });

    describe('index', () => {
      it('should be an object', () => {
        rawContract.documents.indexedDocument.indices = ['something else'];

        const result = validateContract(rawContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].indices[0]');
        expect(error.keyword).to.equal('type');
      });

      it('should have properties definition', () => {
        rawContract.documents.indexedDocument.indices = [{}];

        const result = validateContract(rawContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].indices[0]');
        expect(error.params.missingProperty).to.equal('properties');
        expect(error.keyword).to.equal('required');
      });

      describe('properties definition', () => {
        it('should be an array', () => {
          rawContract.documents.indexedDocument.indices[0]
            .properties = 'something else';

          const result = validateContract(rawContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal(
            '.documents[\'indexedDocument\'].indices[0].properties',
          );
          expect(error.keyword).to.equal('type');
        });

        it('should have at least one property defined', () => {
          rawContract.documents.indexedDocument.indices[0]
            .properties = [];

          const result = validateContract(rawContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal(
            '.documents[\'indexedDocument\'].indices[0].properties',
          );
          expect(error.keyword).to.equal('minItems');
        });

        it('should have no more than 100 property definitions', () => {
          for (let i = 0; i < 100; i++) {
            rawContract.documents.indexedDocument.indices[0]
              .properties.push({
                [`field${i}`]: 'asc',
              });
          }

          const result = validateContract(rawContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal(
            '.documents[\'indexedDocument\'].indices[0].properties',
          );
          expect(error.keyword).to.equal('maxItems');
        });

        describe('property definition', () => {
          it('should be an object', () => {
            rawContract.documents.indexedDocument.indices[0]
              .properties[0] = 'something else';

            const result = validateContract(rawContract);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal(
              '.documents[\'indexedDocument\'].indices[0].properties[0]',
            );
            expect(error.keyword).to.equal('type');
          });

          it('should have at least one property', () => {
            rawContract.documents.indexedDocument.indices[0]
              .properties = [];

            const result = validateContract(rawContract);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal(
              '.documents[\'indexedDocument\'].indices[0].properties',
            );
            expect(error.keyword).to.equal('minItems');
          });

          it('should have no more than one property', () => {
            const property = rawContract.documents.indexedDocument.indices[0]
              .properties[0];

            property.anotherField = 'something';

            const result = validateContract(rawContract);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal(
              '.documents[\'indexedDocument\'].indices[0].properties[0]',
            );
            expect(error.keyword).to.equal('maxProperties');
          });

          it('should have property values only "asc" or "desc"', () => {
            rawContract.documents.indexedDocument.indices[0]
              .properties[0].$userId = 'wrong';

            const result = validateContract(rawContract);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal(
              '.documents[\'indexedDocument\'].indices[0].properties[0][\'$userId\']',
            );
            expect(error.keyword).to.equal('enum');
          });
        });
      });

      it('should have "unique" flag', () => {
        rawContract.documents.indexedDocument.indices[0].unique = undefined;

        const result = validateContract(rawContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].indices[0]');
        expect(error.params.missingProperty).to.equal('unique');
        expect(error.keyword).to.equal('required');
      });

      it('should have "unique" flag equal "true"', () => {
        rawContract.documents.indexedDocument.indices[0].unique = false;

        const result = validateContract(rawContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].indices[0].unique');
        expect(error.keyword).to.equal('const');
      });
    });
  });

  it('should return invalid result if there are additional properties', () => {
    rawContract.additionalProperty = { };

    const result = validateContract(rawContract);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.dataPath).to.equal('');
    expect(error.keyword).to.equal('additionalProperties');
  });

  it('should return invalid result if there are duplicated indices', () => {
    const indexDefinition = Object.assign({},
      rawContract.documents.indexedDocument.indices[0]);

    rawContract.documents.indexedDocument.indices.push(indexDefinition);

    const result = validateContract(rawContract);

    expectValidationError(result, DuplicateIndexError);

    const [error] = result.getErrors();

    expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
    expect(error.getRawContract()).to.deep.equal(rawContract);
    expect(error.getDocumentType()).to.deep.equal('indexedDocument');
  });

  it('should return invalid result if indices don\'t have $userId prefix', () => {
    const indexDefinition = rawContract.documents.indexedDocument.indices[0];

    const firstIndex = indexDefinition.properties.shift();
    indexDefinition.properties.push(firstIndex);

    const result = validateContract(rawContract);

    expectValidationError(result, UniqueIndexMustHaveUserIdPrefixError);

    const [error] = result.getErrors();

    expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
    expect(error.getRawContract()).to.deep.equal(rawContract);
    expect(error.getDocumentType()).to.deep.equal('indexedDocument');
  });

  it('should return invalid result if indices don\'t have $userId prefix as a first field', () => {
    const indexDefinition = rawContract.documents.indexedDocument.indices[0];

    indexDefinition.properties.shift();

    const result = validateContract(rawContract);

    expectValidationError(result, UniqueIndexMustHaveUserIdPrefixError);

    const [error] = result.getErrors();

    expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
  });

  it('should return invalid result if indices has undefined property', () => {
    const indexDefinition = rawContract.documents.indexedDocument.indices[0];

    indexDefinition.properties.push({
      missingProperty: 'asc',
    });

    const result = validateContract(rawContract);

    expectValidationError(result, UndefinedIndexPropertyError);

    const [error] = result.getErrors();

    expect(error.getPropertyName()).to.equal('missingProperty');
    expect(error.getRawContract()).to.deep.equal(rawContract);
    expect(error.getDocumentType()).to.deep.equal('indexedDocument');
    expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
  });

  it('should return valid result if contract is valid', () => {
    const result = validateContract(rawContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
