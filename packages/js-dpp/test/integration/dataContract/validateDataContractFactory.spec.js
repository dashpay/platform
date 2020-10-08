const $RefParser = require('@apidevtools/json-schema-ref-parser');

const createAjv = require('../../../lib/ajv/createAjv');

const JsonSchemaValidator = require('../../../lib/validation/JsonSchemaValidator');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const validateDataContractFactory = require('../../../lib/dataContract/validateDataContractFactory');
const validateDataContractMaxDepthFactory = require('../../../lib/dataContract/stateTransition/validation/validateDataContractMaxDepthFactory');
const enrichDataContractWithBaseSchema = require('../../../lib/dataContract/enrichDataContractWithBaseSchema');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const { expectJsonSchemaError, expectValidationError } = require('../../../lib/test/expect/expectError');

const DuplicateIndexError = require('../../../lib/errors/DuplicateIndexError');
const UndefinedIndexPropertyError = require('../../../lib/errors/UndefinedIndexPropertyError');
const InvalidIndexPropertyTypeError = require('../../../lib/errors/InvalidIndexPropertyTypeError');
const SystemPropertyIndexAlreadyPresentError = require('../../../lib/errors/SystemPropertyIndexAlreadyPresentError');
const UniqueIndicesLimitReachedError = require('../../../lib/errors/UniqueIndicesLimitReachedError');
const InvalidIndexedPropertyConstraintError = require('../../../lib/errors/InvalidIndexedPropertyConstraintError');
const InvalidCompoundIndexError = require('../../../lib/errors/InvalidCompoundIndexError');

describe('validateDataContractFactory', () => {
  let dataContract;
  let rawDataContract;
  let validateDataContract;

  beforeEach(() => {
    dataContract = getDataContractFixture();
    rawDataContract = dataContract.toObject();

    const jsonSchemaValidator = new JsonSchemaValidator(createAjv());

    const validateDataContractMaxDepth = validateDataContractMaxDepthFactory($RefParser);

    validateDataContract = validateDataContractFactory(
      jsonSchemaValidator,
      validateDataContractMaxDepth,
      enrichDataContractWithBaseSchema,
    );
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawDataContract.protocolVersion;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawDataContract.protocolVersion = '1';

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.protocolVersion');
      expect(error.keyword).to.equal('type');
    });

    it('should not be less than 0', async () => {
      rawDataContract.protocolVersion = -1;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minimum');
      expect(error.dataPath).to.equal('.protocolVersion');
    });

    it('should not be greater than current version (0)', async () => {
      rawDataContract.protocolVersion = 1;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maximum');
      expect(error.dataPath).to.equal('.protocolVersion');
    });
  });

  describe('$schema', () => {
    it('should be present', async () => {
      delete rawDataContract.$schema;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('$schema');
    });

    it('should be a string', async () => {
      rawDataContract.$schema = 1;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.$schema');
      expect(error.keyword).to.equal('type');
    });

    it('should be a particular url', async () => {
      rawDataContract.$schema = 'wrong';

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('const');
      expect(error.dataPath).to.equal('.$schema');
    });
  });

  describe('ownerId', () => {
    it('should be present', async () => {
      delete rawDataContract.ownerId;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('ownerId');
    });

    it('should be a byte array', async () => {
      rawDataContract.ownerId = {};

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.ownerId');
      expect(error.keyword).to.equal('byteArray');
    });

    it('should be no less than 32 bytes', async () => {
      rawDataContract.ownerId = Buffer.alloc(31);

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.ownerId');
      expect(error.keyword).to.equal('minBytesLength');
    });

    it('should be no longer than 32 bytes', async () => {
      rawDataContract.ownerId = Buffer.alloc(33);

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.ownerId');
      expect(error.keyword).to.equal('maxBytesLength');
    });
  });

  describe('$id', () => {
    it('should be present', async () => {
      delete rawDataContract.$id;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('$id');
    });

    it('should be a byte array', async () => {
      rawDataContract.$id = {};

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.$id');
      expect(error.keyword).to.equal('byteArray');
    });

    it('should be no less than 32 bytes', async () => {
      rawDataContract.$id = Buffer.alloc(31);

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.$id');
      expect(error.keyword).to.equal('minBytesLength');
    });

    it('should be no longer than 32 bytes', async () => {
      rawDataContract.$id = Buffer.alloc(33);

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.$id');
      expect(error.keyword).to.equal('maxBytesLength');
    });
  });

  describe('definitions', () => {
    it('may not be present', async () => {
      delete rawDataContract.definitions;
      delete rawDataContract.documents.prettyDocument;

      const result = await validateDataContract(rawDataContract);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should be an object', async () => {
      rawDataContract.definitions = 1;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.definitions');
      expect(error.keyword).to.equal('type');
    });

    it('should not be empty', async () => {
      rawDataContract.definitions = {};

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.definitions');
      expect(error.keyword).to.equal('minProperties');
    });

    it('should have no non-alphanumeric properties', async () => {
      rawDataContract.definitions = {
        $subSchema: {},
      };

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result, 2);

      const [patternError, propertyNamesError] = result.getErrors();

      expect(patternError.dataPath).to.equal('.definitions');
      expect(patternError.keyword).to.equal('pattern');

      expect(propertyNamesError.dataPath).to.equal('.definitions');
      expect(propertyNamesError.keyword).to.equal('propertyNames');
    });

    it('should have no more than 100 properties', async () => {
      rawDataContract.definitions = {};

      Array(101).fill({ type: 'string' }).forEach((item, i) => {
        rawDataContract.definitions[i] = item;
      });

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.definitions');
      expect(error.keyword).to.equal('maxProperties');
    });

    it('should have valid property names', async () => {
      const validNames = ['validName', 'valid_name', 'valid-name', 'abc', '123abc', 'abc123', 'ValidName',
        'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb'];

      await Promise.all(
        validNames.map(async (name) => {
          rawDataContract.definitions[name] = {
            type: 'string',
          };

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result, 0);
        }),
      );
    });

    it('should return an invalid result if a property has invalid format', async () => {
      const invalidNames = ['-invalidname', '_invalidname', 'invalidname-', 'invalidname_', '*(*&^', '$test'];

      await Promise.all(
        invalidNames.map(async (name) => {
          rawDataContract.definitions[name] = {
            type: 'string',
          };

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result, 2);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.definitions');
          expect(error.keyword).to.equal('pattern');
        }),
      );
    });
  });

  describe('documents', () => {
    it('should be present', async () => {
      delete rawDataContract.documents;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('documents');
    });

    it('should be an object', async () => {
      rawDataContract.documents = 1;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.documents');
      expect(error.keyword).to.equal('type');
    });

    it('should not be empty', async () => {
      rawDataContract.documents = {};

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.documents');
      expect(error.keyword).to.equal('minProperties');
    });

    it('should have valid property names (document types)', async () => {
      const validNames = ['validName', 'valid_name', 'valid-name', 'abc', '123abc', 'abc123', 'ValidName', 'validName',
        'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb'];

      await Promise.all(
        validNames.map(async (name) => {
          rawDataContract.documents[name] = rawDataContract.documents.niceDocument;

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result, 0);
        }),
      );
    });

    it('should return an invalid result if a property (document type) has invalid format', async () => {
      const invalidNames = ['-invalidname', '_invalidname', 'invalidname-', 'invalidname_', '*(*&^', '$test'];

      await Promise.all(
        invalidNames.map(async (name) => {
          rawDataContract.documents[name] = rawDataContract.documents.niceDocument;

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result, 2);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.documents');
          expect(error.keyword).to.equal('pattern');
        }),
      );
    });

    it('should have no more than 100 properties', async () => {
      const niceDocumentDefinition = rawDataContract.documents.niceDocument;

      rawDataContract.documents = {};

      Array(101).fill(niceDocumentDefinition).forEach((item, i) => {
        rawDataContract.documents[i] = item;
      });

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.documents');
      expect(error.keyword).to.equal('maxProperties');
    });

    describe('Document schema', () => {
      it('should not be empty', async () => {
        rawDataContract.documents.niceDocument.properties = {};

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\'].properties');
        expect(error.keyword).to.equal('minProperties');
      });

      it('should have type "object"', async () => {
        rawDataContract.documents.niceDocument.type = 'string';

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\'].type');
        expect(error.keyword).to.equal('const');
      });

      it('should have "properties"', async () => {
        delete rawDataContract.documents.niceDocument.properties;

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\']');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('.properties');
      });

      it('should have nested "properties"', async () => {
        rawDataContract.documents.niceDocument.properties.object = {
          type: 'array',
          items: [
            {
              properties: {
                something: {
                  additionalProperties: false,
                },
              },
              additionalProperties: false,
            },
          ],
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result, 3);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\'].properties[\'object\'].items[0].properties[\'something\']');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('.properties');
      });

      it('should have valid property names', async () => {
        const validNames = ['validName', 'valid_name', 'valid-name', 'abc', '123abc', 'abc123', 'ValidName', 'validName',
          'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb'];

        await Promise.all(
          validNames.map(async (name) => {
            rawDataContract.documents.niceDocument.properties[name] = {
              type: 'string',
            };

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result, 0);
          }),
        );
      });

      it('should have valid nested property names', async () => {
        const validNames = ['validName', 'valid_name', 'valid-name', 'abc', '123abc', 'abc123', 'ValidName', 'validName',
          'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb'];

        rawDataContract.documents.niceDocument.properties.something = {
          properties: {},
          additionalProperties: false,
        };

        await Promise.all(
          validNames.map(async (name) => {
            rawDataContract.documents.niceDocument.properties.something.properties[name] = {
              type: 'string',
            };

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result, 0);
          }),
        );
      });

      it('should return an invalid result if a property has invalid format', async () => {
        const invalidNames = ['-invalidname', '_invalidname', 'invalidname-', 'invalidname_', '*(*&^', '$test'];

        await Promise.all(
          invalidNames.map(async (name) => {
            rawDataContract.documents.niceDocument.properties[name] = {};

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result, 2);

            const errors = result.getErrors();

            expect(errors[0].dataPath).to.equal('.documents[\'niceDocument\'].properties');
            expect(errors[0].keyword).to.equal('pattern');
            expect(errors[1].dataPath).to.equal('.documents[\'niceDocument\'].properties');
            expect(errors[1].keyword).to.equal('propertyNames');
          }),
        );
      });

      it('should return an invalid result if a nested property has invalid format', async () => {
        const invalidNames = ['-invalidname', '_invalidname', 'invalidname-', 'invalidname_', '*(*&^', '$test'];

        rawDataContract.documents.niceDocument.properties.something = {
          properties: {},
          additionalProperties: false,
        };

        await Promise.all(
          invalidNames.map(async (name) => {
            rawDataContract.documents.niceDocument.properties.something.properties[name] = {};

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result, 2);

            const errors = result.getErrors();

            expect(errors[0].dataPath).to.equal(
              '.documents[\'niceDocument\'].properties[\'something\'].properties',
            );
            expect(errors[0].keyword).to.equal('pattern');
            expect(errors[1].dataPath).to.equal(
              '.documents[\'niceDocument\'].properties[\'something\'].properties',
            );
            expect(errors[1].keyword).to.equal('propertyNames');
          }),
        );
      });

      it('should have "additionalProperties" defined', async () => {
        delete rawDataContract.documents.niceDocument.additionalProperties;

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\']');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('.additionalProperties');
      });

      it('should have "additionalProperties" defined to false', async () => {
        rawDataContract.documents.niceDocument.additionalProperties = true;

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\'].additionalProperties');
        expect(error.keyword).to.equal('const');
      });

      it('should have nested "additionalProperties" defined', async () => {
        rawDataContract.documents.niceDocument.properties.object = {
          type: 'array',
          items: [
            {
              properties: {
                something: {
                  type: 'string',
                },
              },
            },
          ],
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result, 3);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\'].properties[\'object\'].items[0]');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('.additionalProperties');
      });

      it('should return invalid result if there are additional properties', async () => {
        rawDataContract.additionalProperty = { };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('additionalProperties');
      });

      it('should have no more than 100 properties', async () => {
        const propertyDefinition = { };

        rawDataContract.documents.niceDocument.properties = {};

        Array(101).fill(propertyDefinition).forEach((item, i) => {
          rawDataContract.documents.niceDocument.properties[i] = item;
        });

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'niceDocument\'].properties');
        expect(error.keyword).to.equal('maxProperties');
      });

      it('should have defined items for arrays', async () => {
        rawDataContract.documents.new = {
          properties: {
            something: {
              type: 'array',
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'new\'].properties[\'something\']');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('.items');
      });

      it('should not have additionalItems for arrays if items is subschema', async () => {
        rawDataContract.documents.new = {
          properties: {
            something: {
              type: 'array',
              items: {
                type: 'string',
              },
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result, 0);
      });

      it('should have additionalItems for arrays', async () => {
        rawDataContract.documents.new = {
          properties: {
            something: {
              type: 'array',
              items: [
                {
                  type: 'string',
                },
                {
                  type: 'number',
                },
              ],
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'new\'].properties[\'something\']');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('.additionalItems');
      });

      it('should have additionalItems disabled for arrays', async () => {
        rawDataContract.documents.new = {
          properties: {
            something: {
              type: 'array',
              items: [
                {
                  type: 'string',
                },
                {
                  type: 'number',
                },
              ],
              additionalItems: false,
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result, 0);
      });

      it('should not have additionalItems enabled for arrays', async () => {
        rawDataContract.documents.new = {
          properties: {
            something: {
              type: 'array',
              items: [
                {
                  type: 'string',
                },
                {
                  type: 'number',
                },
              ],
              additionalItems: true,
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result, 3);

        const [shouldBeAnObjectError, shouldEqualConstant] = result.getErrors();

        expect(shouldBeAnObjectError.dataPath).to.equal(
          '.documents[\'new\'].properties[\'something\'].additionalItems',
        );
        expect(shouldBeAnObjectError.keyword).to.equal('type');

        expect(shouldEqualConstant.dataPath).to.equal(
          '.documents[\'new\'].properties[\'something\'].additionalItems',
        );
        expect(shouldEqualConstant.keyword).to.equal('const');
      });

      it('should return invalid result if "default" keyword is used', async () => {
        rawDataContract.documents.indexedDocument.properties.firstName.default = '1';

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].properties[\'firstName\']');
        expect(error.keyword).to.equal('additionalProperties');
      });

      it('should return invalid result if remote `$ref` is used', async () => {
        rawDataContract.documents.indexedDocument = {
          $ref: 'http://remote.com/schema#',
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].$ref');
        expect(error.keyword).to.equal('pattern');
      });

      it('should not have `propertyNames`', async () => {
        rawDataContract.documents.indexedDocument = {
          type: 'object',
          properties: {
            something: {
              type: 'string',
            },
          },
          propertyNames: {
            pattern: 'abc',
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\']');
        expect(error.keyword).to.equal('additionalProperties');
        expect(error.params.additionalProperty).to.equal('propertyNames');
      });

      it('should have `maxItems` if `uniqueItems` is used', async () => {
        rawDataContract.documents.indexedDocument = {
          type: 'object',
          properties: {
            something: {
              type: 'array',
              uniqueItems: true,
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].properties[\'something\']');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('maxItems');
      });

      it('should have `maxItems` no bigger than 100000 if `uniqueItems` is used', async () => {
        rawDataContract.documents.indexedDocument = {
          type: 'object',
          properties: {
            something: {
              type: 'array',
              uniqueItems: true,
              maxItems: 200000,
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].properties[\'something\'].maxItems');
        expect(error.keyword).to.equal('maximum');
      });

      it('should return invalid result if document JSON Schema is not valid', async () => {
        rawDataContract.documents.indexedDocument = {
          type: 'object',
          properties: {
            something: {
              type: 'string',
              format: 'lalala',
              maxLength: 100,
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.message).to.be.a('string').and.satisfy((msg) => (
          msg.startsWith('unknown format "lalala" is used')
        ));
      });

      it('should have `maxLength` if `pattern` is used', async () => {
        rawDataContract.documents.indexedDocument = {
          type: 'object',
          properties: {
            something: {
              type: 'string',
              pattern: 'a',
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].properties[\'something\']');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('maxLength');
      });

      it('should have `maxLength` no bigger than 50000 if `pattern` is used', async () => {
        rawDataContract.documents.indexedDocument = {
          type: 'object',
          properties: {
            something: {
              type: 'string',
              pattern: 'a',
              maxLength: 60000,
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].properties[\'something\'].maxLength');
        expect(error.keyword).to.equal('maximum');
      });

      it('should have `maxLength` if `format` is used', async () => {
        rawDataContract.documents.indexedDocument = {
          type: 'object',
          properties: {
            something: {
              type: 'string',
              format: 'url',
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].properties[\'something\']');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('maxLength');
      });

      it('should have `maxLength` no bigger than 50000 if `format` is used', async () => {
        rawDataContract.documents.indexedDocument = {
          type: 'object',
          properties: {
            something: {
              type: 'string',
              format: 'url',
              maxLength: 60000,
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].properties[\'something\'].maxLength');
        expect(error.keyword).to.equal('maximum');
      });

      describe('byteArray', () => {
        it('should be a boolean', async () => {
          rawDataContract.documents.withByteArrays.properties.byteArrayField.byteArray = 1;

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.documents[\'withByteArrays\'].properties[\'byteArrayField\'].byteArray');
          expect(error.keyword).to.equal('type');
          expect(error.params.type).to.equal('boolean');
        });

        it('should equal to true', async () => {
          rawDataContract.documents.withByteArrays.properties.byteArrayField.byteArray = false;

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.documents[\'withByteArrays\'].properties[\'byteArrayField\'].byteArray');
          expect(error.keyword).to.equal('const');
          expect(error.params.allowedValue).to.equal(true);
        });

        it('should be used with type `object`', async () => {
          rawDataContract.documents.withByteArrays.properties.byteArrayField.type = 'string';

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.documents[\'withByteArrays\'].properties[\'byteArrayField\'].type');
          expect(error.keyword).to.equal('const');
        });
      });

      describe('minBytesLength', () => {
        it('should be a integer', async () => {
          rawDataContract.documents.withByteArrays.properties.byteArrayField.minBytesLength = 'something';

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.documents[\'withByteArrays\'].properties[\'byteArrayField\'].minBytesLength');
          expect(error.keyword).to.equal('type');
          expect(error.params.type).to.equal('integer');
        });

        it('should be not less than 0', async () => {
          rawDataContract.documents.withByteArrays.properties.byteArrayField.minBytesLength = -1;

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.documents[\'withByteArrays\'].properties[\'byteArrayField\'].minBytesLength');
          expect(error.keyword).to.equal('minimum');
        });

        it('should be used with `byteArray`', async () => {
          rawDataContract.documents.withByteArrays.properties.byteArrayField = {
            minBytesLength: 1,
          };

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.documents[\'withByteArrays\'].properties[\'byteArrayField\']');
          expect(error.keyword).to.equal('dependencies');
        });
      });

      describe('maxBytesLength', () => {
        it('should be a integer', async () => {
          rawDataContract.documents.withByteArrays.properties.byteArrayField.minBytesLength = 'something';

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.documents[\'withByteArrays\'].properties[\'byteArrayField\'].minBytesLength');
          expect(error.keyword).to.equal('type');
          expect(error.params.type).to.equal('integer');
        });

        it('should be not less than 0', async () => {
          rawDataContract.documents.withByteArrays.properties.byteArrayField.maxBytesLength = -1;

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.documents[\'withByteArrays\'].properties[\'byteArrayField\'].maxBytesLength');
          expect(error.keyword).to.equal('minimum');
        });

        it('should be used with `byteArray`', async () => {
          rawDataContract.documents.withByteArrays.properties.byteArrayField = {
            maxBytesLength: 1,
          };

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal('.documents[\'withByteArrays\'].properties[\'byteArrayField\']');
          expect(error.keyword).to.equal('dependencies');
        });
      });
    });
  });

  describe('indices', () => {
    it('should be an array', async () => {
      rawDataContract.documents.indexedDocument.indices = 'definitely not an array';

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].indices');
      expect(error.keyword).to.equal('type');
    });

    it('should have at least one item', async () => {
      rawDataContract.documents.indexedDocument.indices = [];

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].indices');
      expect(error.keyword).to.equal('minItems');
    });

    it('should return invalid result if there are duplicated indices', async () => {
      const indexDefinition = { ...rawDataContract.documents.indexedDocument.indices[0] };

      rawDataContract.documents.indexedDocument.indices.push(indexDefinition);

      const result = await validateDataContract(rawDataContract);

      expectValidationError(result, DuplicateIndexError);

      const [error] = result.getErrors();

      expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
      expect(error.getRawDataContract()).to.deep.equal(rawDataContract);
      expect(error.getDocumentType()).to.deep.equal('indexedDocument');
    });

    describe('index', () => {
      it('should be an object', async () => {
        rawDataContract.documents.indexedDocument.indices = ['something else'];

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].indices[0]');
        expect(error.keyword).to.equal('type');
      });

      it('should have properties definition', async () => {
        rawDataContract.documents.indexedDocument.indices = [{}];

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].indices[0]');
        expect(error.params.missingProperty).to.equal('properties');
        expect(error.keyword).to.equal('required');
      });

      describe('properties definition', () => {
        it('should be an array', async () => {
          rawDataContract.documents.indexedDocument.indices[0]
            .properties = 'something else';

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal(
            '.documents[\'indexedDocument\'].indices[0].properties',
          );
          expect(error.keyword).to.equal('type');
        });

        it('should have at least one property defined', async () => {
          rawDataContract.documents.indexedDocument.indices[0]
            .properties = [];

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal(
            '.documents[\'indexedDocument\'].indices[0].properties',
          );
          expect(error.keyword).to.equal('minItems');
        });

        it('should have no more than 10 property definitions', async () => {
          for (let i = 0; i < 10; i++) {
            rawDataContract.documents.indexedDocument.indices[0]
              .properties.push({ [`field${i}`]: 'asc' });
          }

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.dataPath).to.equal(
            '.documents[\'indexedDocument\'].indices[0].properties',
          );
          expect(error.keyword).to.equal('maxItems');
        });

        describe('property definition', () => {
          it('should be an object', async () => {
            rawDataContract.documents.indexedDocument.indices[0]
              .properties[0] = 'something else';

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal(
              '.documents[\'indexedDocument\'].indices[0].properties[0]',
            );
            expect(error.keyword).to.equal('type');
          });

          it('should have at least one property', async () => {
            rawDataContract.documents.indexedDocument.indices[0]
              .properties = [];

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal(
              '.documents[\'indexedDocument\'].indices[0].properties',
            );
            expect(error.keyword).to.equal('minItems');
          });

          it('should have no more than one property', async () => {
            const property = rawDataContract.documents.indexedDocument.indices[0]
              .properties[0];

            property.anotherField = 'something';

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal(
              '.documents[\'indexedDocument\'].indices[0].properties[0]',
            );
            expect(error.keyword).to.equal('maxProperties');
          });

          it('should have property values only "asc" or "desc"', async () => {
            rawDataContract.documents.indexedDocument.indices[0]
              .properties[0].$ownerId = 'wrong';

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.dataPath).to.equal(
              '.documents[\'indexedDocument\'].indices[0].properties[0][\'$ownerId\']',
            );
            expect(error.keyword).to.equal('enum');
          });
        });
      });

      it('should have "unique" flag to be of a boolean type', async () => {
        rawDataContract.documents.indexedDocument.indices[0].unique = 12;

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.documents[\'indexedDocument\'].indices[0].unique');
        expect(error.keyword).to.equal('type');
      });

      it('should have no more than 10 indices', async () => {
        for (let i = 0; i < 10; i++) {
          const propertyName = `field${i}`;

          rawDataContract.documents.indexedDocument.properties[propertyName] = { type: 'string' };

          rawDataContract.documents.indexedDocument.indices.push({
            properties: [{ [propertyName]: 'asc' }],
          });
        }

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal(
          '.documents[\'indexedDocument\'].indices',
        );
        expect(error.keyword).to.equal('maxItems');
      });

      it('should have no more than 3 unique indices', async () => {
        for (let i = 0; i < 4; i++) {
          const propertyName = `field${i}`;

          rawDataContract.documents.indexedDocument.properties[propertyName] = {
            type: 'string',
            maxLength: 256,
          };

          rawDataContract.documents.indexedDocument.indices.push({
            properties: [{ [propertyName]: 'asc' }],
            unique: true,
          });
        }

        const result = await validateDataContract(rawDataContract);

        expectValidationError(result, UniqueIndicesLimitReachedError);

        const [error] = result.getErrors();

        expect(error.getRawDataContract()).to.equal(rawDataContract);
        expect(error.getDocumentType()).to.equal('indexedDocument');
      });

      it('should return invalid result if index is prebuilt', async () => {
        const indexDefinition = {
          properties: [
            { $id: 'asc' },
          ],
        };

        const indeciesDefinition = rawDataContract.documents.indexedDocument.indices;

        indeciesDefinition.push(indexDefinition);

        const result = await validateDataContract(rawDataContract);

        expectValidationError(result, SystemPropertyIndexAlreadyPresentError);

        const [error] = result.getErrors();

        expect(error.getPropertyName()).to.equal('$id');
        expect(error.getRawDataContract()).to.deep.equal(rawDataContract);
        expect(error.getDocumentType()).to.deep.equal('indexedDocument');
        expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
      });

      it('should return invalid result if indices has undefined property', async () => {
        const indexDefinition = rawDataContract.documents.indexedDocument.indices[0];

        indexDefinition.properties.push({
          missingProperty: 'asc',
        });

        const result = await validateDataContract(rawDataContract);

        expectValidationError(result, UndefinedIndexPropertyError);

        const [error] = result.getErrors();

        expect(error.getPropertyName()).to.equal('missingProperty');
        expect(error.getRawDataContract()).to.deep.equal(rawDataContract);
        expect(error.getDocumentType()).to.deep.equal('indexedDocument');
        expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
      });

      it('should return invalid result if index property is object', async () => {
        const indexedDocumentDefinition = rawDataContract.documents.indexedDocument;

        indexedDocumentDefinition.properties.objectProperty = {
          type: 'object',
          properties: {
            something: {
              type: 'string',
            },
          },
          additionalProperties: false,
        };

        indexedDocumentDefinition.required.push('objectProperty');

        const indexDefinition = indexedDocumentDefinition.indices[0];

        indexDefinition.properties.push({
          objectProperty: 'asc',
        });

        const result = await validateDataContract(rawDataContract);

        expectValidationError(result, InvalidIndexPropertyTypeError);

        const [error] = result.getErrors();

        expect(error.getPropertyName()).to.equal('objectProperty');
        expect(error.getPropertyType()).to.equal('object');
        expect(error.getRawDataContract()).to.deep.equal(rawDataContract);
        expect(error.getDocumentType()).to.deep.equal('indexedDocument');
        expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
      });

      it('should return invalid result if index property is array of objects', async () => {
        const indexedDocumentDefinition = rawDataContract.documents.indexedDocument;

        indexedDocumentDefinition.properties.arrayProperty = {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              something: {
                type: 'string',
              },
            },
            additionalProperties: false,
          },
        };

        indexedDocumentDefinition.required.push('arrayProperty');

        const indexDefinition = indexedDocumentDefinition.indices[0];

        indexDefinition.properties.push({
          arrayProperty: 'asc',
        });

        const result = await validateDataContract(rawDataContract);

        expectValidationError(result, InvalidIndexPropertyTypeError);

        const [error] = result.getErrors();

        expect(error.getPropertyName()).to.equal('arrayProperty');
        expect(error.getPropertyType()).to.equal('array');
        expect(error.getRawDataContract()).to.deep.equal(rawDataContract);
        expect(error.getDocumentType()).to.deep.equal('indexedDocument');
        expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
      });

      it('should return invalid result if index property is array of arrays', async () => {
        const indexedDocumentDefinition = rawDataContract.documents.indexedDocument;

        indexedDocumentDefinition.properties.arrayProperty = {
          type: 'array',
          items: {
            type: 'array',
            items: {
              type: 'string',
            },
          },
        };

        indexedDocumentDefinition.required.push('arrayProperty');

        const indexDefinition = indexedDocumentDefinition.indices[0];

        indexDefinition.properties.push({
          arrayProperty: 'asc',
        });

        const result = await validateDataContract(rawDataContract);

        expectValidationError(result, InvalidIndexPropertyTypeError);

        const [error] = result.getErrors();

        expect(error.getPropertyName()).to.equal('arrayProperty');
        expect(error.getPropertyType()).to.equal('array');
        expect(error.getRawDataContract()).to.deep.equal(rawDataContract);
        expect(error.getDocumentType()).to.deep.equal('indexedDocument');
        expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
      });

      it('should return invalid result if index property is array with many item definitions', async () => {
        const indexedDocumentDefinition = rawDataContract.documents.indexedDocument;

        indexedDocumentDefinition.properties.arrayProperty = {
          type: 'array',
          items: [{
            type: 'string',
          }, {
            type: 'number',
          }],
          additionalItems: false,
        };

        indexedDocumentDefinition.required.push('arrayProperty');

        const indexDefinition = indexedDocumentDefinition.indices[0];

        indexDefinition.properties.push({
          arrayProperty: 'asc',
        });

        const result = await validateDataContract(rawDataContract);

        expectValidationError(result, InvalidIndexPropertyTypeError);

        const [error] = result.getErrors();

        expect(error.getPropertyName()).to.equal('arrayProperty');
        expect(error.getPropertyType()).to.equal('array');
        expect(error.getRawDataContract()).to.deep.equal(rawDataContract);
        expect(error.getDocumentType()).to.deep.equal('indexedDocument');
        expect(error.getIndexDefinition()).to.deep.equal(indexDefinition);
      });

      it('should return invalid result if unique compound index contains both required and optional properties', async () => {
        rawDataContract.documents.optionalUniqueIndexedDocument.required.splice(-1);

        const result = await validateDataContract(rawDataContract);

        expectValidationError(result, InvalidCompoundIndexError);

        const [error] = result.getErrors();

        expect(error.getIndexDefinition()).to.deep.equal(
          rawDataContract.documents.optionalUniqueIndexedDocument.indices[1],
        );
        expect(error.getDocumentType()).to.equal('optionalUniqueIndexedDocument');
      });
    });
  });

  it('should return invalid result with circular $ref pointer', async () => {
    rawDataContract.definitions.object = { $ref: '#/definitions/object' };

    const result = await validateDataContract(rawDataContract);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.message).to.be.a('string').and.satisfy((msg) => (
      msg.startsWith('Circular $ref pointer')
    ));
  });

  it('should return invalid result if indexed property missing maxLength constraint', async () => {
    delete rawDataContract.documents.indexedDocument.properties.firstName.maxLength;

    const result = await validateDataContract(rawDataContract);

    expectValidationError(result, InvalidIndexedPropertyConstraintError);

    const [error] = result.getErrors();

    expect(error.getPropertyName()).to.equal('firstName');
    expect(error.getConstraintName()).to.equal('maxLength');
    expect(error.getReason()).to.equal('should be set');
  });

  it('should return invalid result if indexed property have to big maxLength', async () => {
    rawDataContract.documents.indexedDocument.properties.firstName.maxLength = 2048;

    const result = await validateDataContract(rawDataContract);

    expectValidationError(result, InvalidIndexedPropertyConstraintError);

    const [error] = result.getErrors();

    expect(error.getPropertyName()).to.equal('firstName');
    expect(error.getConstraintName()).to.equal('maxLength');
    expect(error.getReason()).to.equal('should be less or equal 1024');
  });

  it('should return valid result if Data Contract is valid', async () => {
    const result = await validateDataContract(rawDataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
