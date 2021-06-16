const { default: getRE2Class } = require('@dashevo/re2-wasm');

const $RefParser = require('@apidevtools/json-schema-ref-parser');

const createAjv = require('../../../lib/ajv/createAjv');

const JsonSchemaValidator = require('../../../lib/validation/JsonSchemaValidator');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const validateDataContractFactory = require('../../../lib/dataContract/validateDataContractFactory');
const validateDataContractMaxDepthFactory = require('../../../lib/dataContract/stateTransition/validation/validateDataContractMaxDepthFactory');
const enrichDataContractWithBaseSchema = require('../../../lib/dataContract/enrichDataContractWithBaseSchema');
const validateDataContractPatternsFactory = require('../../../lib/dataContract/validateDataContractPatternsFactory');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const { expectJsonSchemaError, expectValidationError } = require('../../../lib/test/expect/expectError');

const DuplicateIndexError = require('../../../lib/errors/DuplicateIndexError');
const UndefinedIndexPropertyError = require('../../../lib/errors/UndefinedIndexPropertyError');
const InvalidIndexPropertyTypeError = require('../../../lib/errors/InvalidIndexPropertyTypeError');
const SystemPropertyIndexAlreadyPresentError = require('../../../lib/errors/SystemPropertyIndexAlreadyPresentError');
const UniqueIndicesLimitReachedError = require('../../../lib/errors/UniqueIndicesLimitReachedError');
const InvalidIndexedPropertyConstraintError = require('../../../lib/errors/InvalidIndexedPropertyConstraintError');
const InvalidCompoundIndexError = require('../../../lib/errors/InvalidCompoundIndexError');
const IncompatibleRe2PatternError = require('../../../lib/document/errors/IncompatibleRe2PatternError');

describe('validateDataContractFactory', function main() {
  this.timeout(15000);

  let dataContract;
  let rawDataContract;
  let validateDataContract;
  let RE2;

  before(async () => {
    RE2 = await getRE2Class();
  });

  beforeEach(async () => {
    dataContract = getDataContractFixture();
    rawDataContract = dataContract.toObject();

    const ajv = createAjv(RE2);
    const jsonSchemaValidator = new JsonSchemaValidator(ajv);

    const validateDataContractMaxDepth = validateDataContractMaxDepthFactory($RefParser);

    const validateDataContractPatterns = validateDataContractPatternsFactory(RE2);

    validateDataContract = validateDataContractFactory(
      jsonSchemaValidator,
      validateDataContractMaxDepth,
      enrichDataContractWithBaseSchema,
      validateDataContractPatterns,
      RE2,
    );
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      rawDataContract = {
        documents: {
          zalupa: {
            type: 'object',
          },
        },
      };
      delete rawDataContract.protocolVersion;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawDataContract.protocolVersion = '1';

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/protocolVersion');
      expect(error.keyword).to.equal('type');
    });

    it('should not be less than 0', async () => {
      rawDataContract.protocolVersion = -1;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minimum');
      expect(error.instancePath).to.equal('/protocolVersion');
    });

    it('should not be greater than current version (0)', async () => {
      rawDataContract.protocolVersion = 1;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maximum');
      expect(error.instancePath).to.equal('/protocolVersion');
    });
  });

  describe('$schema', () => {
    it('should be present', async () => {
      delete rawDataContract.$schema;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('$schema');
    });

    it('should be a string', async () => {
      rawDataContract.$schema = 1;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/$schema');
      expect(error.keyword).to.equal('type');
    });

    it('should be a particular url', async () => {
      rawDataContract.$schema = 'wrong';

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('const');
      expect(error.instancePath).to.equal('/$schema');
    });
  });

  describe('ownerId', () => {
    it('should be present', async () => {
      delete rawDataContract.ownerId;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('ownerId');
    });

    it('should be a byte array', async () => {
      rawDataContract.ownerId = new Array(32).fill('string');

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.instancePath).to.equal('/ownerId/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be no less than 32 bytes', async () => {
      rawDataContract.ownerId = Buffer.alloc(31);

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/ownerId');
      expect(error.keyword).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      rawDataContract.ownerId = Buffer.alloc(33);

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/ownerId');
      expect(error.keyword).to.equal('maxItems');
    });
  });

  describe('$id', () => {
    it('should be present', async () => {
      delete rawDataContract.$id;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('$id');
    });

    it('should be a byte array', async () => {
      rawDataContract.$id = new Array(32).fill('string');

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.instancePath).to.equal('/$id/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be no less than 32 bytes', async () => {
      rawDataContract.$id = Buffer.alloc(31);

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/$id');
      expect(error.keyword).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      rawDataContract.$id = Buffer.alloc(33);

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/$id');
      expect(error.keyword).to.equal('maxItems');
    });
  });

  describe('$defs', () => {
    it('may not be present', async () => {
      delete rawDataContract.$defs;
      delete rawDataContract.documents.prettyDocument;

      const result = await validateDataContract(rawDataContract);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should be an object', async () => {
      rawDataContract.$defs = 1;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/$defs');
      expect(error.keyword).to.equal('type');
    });

    it('should not be empty', async () => {
      rawDataContract.$defs = {};

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/$defs');
      expect(error.keyword).to.equal('minProperties');
    });

    it('should have no non-alphanumeric properties', async () => {
      rawDataContract.$defs = {
        $subSchema: {},
      };

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result, 2);

      const [patternError, propertyNamesError] = result.getErrors();

      expect(patternError.instancePath).to.equal('/$defs');
      expect(patternError.keyword).to.equal('pattern');

      expect(propertyNamesError.instancePath).to.equal('/$defs');
      expect(propertyNamesError.keyword).to.equal('propertyNames');
    });

    it('should have no more than 100 properties', async () => {
      rawDataContract.$defs = {};

      Array(101).fill({ type: 'string' }).forEach((item, i) => {
        rawDataContract.$defs[i] = item;
      });

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/$defs');
      expect(error.keyword).to.equal('maxProperties');
    });

    it('should have valid property names', async () => {
      const validNames = ['validName', 'valid_name', 'valid-name', 'abc', 'ab12c', 'abc123', 'ValidName',
        'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb'];

      await Promise.all(
        validNames.map(async (name) => {
          rawDataContract.$defs[name] = {
            type: 'string',
          };

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result, 0);
        }),
      );
    });

    it('should return an invalid result if a property has invalid format', async () => {
      const invalidNames = ['-invalidname', '_invalidname', 'invalidname-', 'invalidname_', '*(*&^', '$test', '123abci', 'ab'];

      await Promise.all(
        invalidNames.map(async (name) => {
          rawDataContract.$defs[name] = {
            type: 'string',
          };

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result, 2);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal('/$defs');
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

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('documents');
    });

    it('should be an object', async () => {
      rawDataContract.documents = 1;

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/documents');
      expect(error.keyword).to.equal('type');
    });

    it('should not be empty', async () => {
      rawDataContract.documents = {};

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/documents');
      expect(error.keyword).to.equal('minProperties');
    });

    it('should have valid property names (document types)', async () => {
      const validNames = ['validName', 'valid_name', 'valid-name', 'abc', 'a123123bc', 'ab123c', 'ValidName', 'validName',
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
      const invalidNames = ['-invalidname', '_invalidname', 'invalidname-', 'invalidname_', '*(*&^', '$test', '123abc', 'ab'];

      await Promise.all(
        invalidNames.map(async (name) => {
          rawDataContract.documents[name] = rawDataContract.documents.niceDocument;

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result, 2);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal('/documents');
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

      expect(error.instancePath).to.equal('/documents');
      expect(error.keyword).to.equal('maxProperties');
    });

    describe('Document schema', () => {
      it('should not be empty', async () => {
        rawDataContract.documents.niceDocument.properties = {};

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/niceDocument/properties');
        expect(error.keyword).to.equal('minProperties');
      });

      it('should have type "object"', async () => {
        rawDataContract.documents.niceDocument.type = 'string';

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/niceDocument/type');
        expect(error.keyword).to.equal('const');
      });

      it('should have "properties"', async () => {
        delete rawDataContract.documents.niceDocument.properties;

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/niceDocument');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('properties');
      });

      it('should have nested "properties"', async () => {
        rawDataContract.documents.niceDocument.properties.object = {
          type: 'array',
          prefixItems: [
            {
              type: 'object',
              properties: {
                something: {
                  type: 'object',
                },
              },
              additionalProperties: false,
            },
          ],
          items: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result, 3);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/niceDocument/properties/object/prefixItems/0/properties/something');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('properties');
      });

      it('should have valid property names', async () => {
        const validNames = ['validName', 'valid_name', 'valid-name', 'abc', 'a123bc', 'abc123', 'ValidName', 'validName',
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
        const validNames = ['validName', 'valid_name', 'valid-name', 'abc', 'a123bc', 'abc123', 'ValidName', 'validName',
          'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb'];

        rawDataContract.documents.niceDocument.properties.something = {
          type: 'object',
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
        const invalidNames = ['-invalidname', '_invalidname', 'invalidname-', 'invalidname_', '*(*&^', '$test', '123abc', 'ab'];

        await Promise.all(
          invalidNames.map(async (name) => {
            rawDataContract.documents.niceDocument.properties[name] = {};

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result, 3);

            const errors = result.getErrors();

            expect(errors[0].instancePath).to.equal('/documents/niceDocument/properties');
            expect(errors[0].keyword).to.equal('pattern');
            expect(errors[1].instancePath).to.equal('/documents/niceDocument/properties');
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

            expectJsonSchemaError(result, 4);

            const errors = result.getErrors();

            expect(errors[0].instancePath).to.equal(
              '/documents/niceDocument/properties/something/properties',
            );
            expect(errors[0].keyword).to.equal('pattern');
            expect(errors[1].instancePath).to.equal(
              '/documents/niceDocument/properties/something/properties',
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

        expect(error.instancePath).to.equal('/documents/niceDocument');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('additionalProperties');
      });

      it('should have "additionalProperties" defined to false', async () => {
        rawDataContract.documents.niceDocument.additionalProperties = true;

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/niceDocument/additionalProperties');
        expect(error.keyword).to.equal('const');
      });

      it('should have nested "additionalProperties" defined', async () => {
        rawDataContract.documents.niceDocument.properties.object = {
          type: 'array',
          prefixItems: [
            {
              type: 'object',
              properties: {
                something: {
                  type: 'string',
                },
              },
            },
          ],
          items: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/niceDocument/properties/object/prefixItems/0');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('additionalProperties');
      });

      it('should return invalid result if there are additional properties', async () => {
        rawDataContract.additionalProperty = { };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('');
        expect(error.keyword).to.equal('additionalProperties');
      });

      it('should have no more than 100 properties', async () => {
        const propertyDefinition = { };

        rawDataContract.documents.niceDocument.properties = {};

        Array(101).fill(propertyDefinition).forEach((item, i) => {
          rawDataContract.documents.niceDocument.properties[i] = item;
        });

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/niceDocument/properties');
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

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/new/properties/something');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('items');
      });

      it('should have sub schema in items for arrays', async () => {
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

        expectJsonSchemaError(result, 3);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/new/properties/something/items');
        expect(error.keyword).to.equal('type');
        expect(error.params.type).to.equal('object');
      });

      it('should have items if prefixItems is used for arrays', async () => {
        rawDataContract.documents.new = {
          type: 'object',
          properties: {
            something: {
              type: 'array',
              prefixItems: [
                {
                  type: 'string',
                },
                {
                  type: 'number',
                },
              ],
              minItems: 2,
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/new/properties/something');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('items');
      });

      it('should not have items disabled if prefixItems is used for arrays', async () => {
        rawDataContract.documents.new = {
          properties: {
            something: {
              type: 'array',
              prefixItems: [
                {
                  type: 'string',
                },
                {
                  type: 'number',
                },
              ],
              items: true,
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/new/properties/something/items');
        expect(error.keyword).to.equal('const');
        expect(error.params.allowedValue).to.equal(false);
      });

      it('should return invalid result if "default" keyword is used', async () => {
        rawDataContract.documents.indexedDocument.properties.firstName.default = '1';

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/indexedDocument/properties/firstName');
        expect(error.keyword).to.equal('unevaluatedProperties');
      });

      it('should return invalid result if remote `$ref` is used', async () => {
        rawDataContract.documents.indexedDocument = {
          $ref: 'http://remote.com/schema#',
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/indexedDocument/$ref');
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

        expect(error.instancePath).to.equal('/documents/indexedDocument');
        expect(error.keyword).to.equal('unevaluatedProperties');
        expect(error.params.unevaluatedProperty).to.equal('propertyNames');
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

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/indexedDocument/properties/something');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('items');
      });

      it('should have `maxItems` no bigger than 100000 if `uniqueItems` is used', async () => {
        rawDataContract.documents.indexedDocument = {
          type: 'object',
          properties: {
            something: {
              type: 'array',
              uniqueItems: true,
              maxItems: 200000,
              items: {
                type: 'object',
                properties: {
                  property: {
                    type: 'string',
                  },
                },
                additionalProperties: false,
              },
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/indexedDocument/properties/something/maxItems');
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
          msg.startsWith('unknown format "lalala" ignored in schema')
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

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/indexedDocument/properties/something');
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

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/indexedDocument/properties/something/maxLength');
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

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/indexedDocument/properties/something');
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

        expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/indexedDocument/properties/something/maxLength');
        expect(error.keyword).to.equal('maximum');
      });

      it('should not have incompatible patterns', async () => {
        rawDataContract.documents.indexedDocument = {
          type: 'object',
          properties: {
            something: {
              type: 'string',
              maxLength: 100,
              pattern: '^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$',
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectValidationError(result, IncompatibleRe2PatternError);

        const [error] = result.getErrors();

        expect(error.getPattern()).to.equal('^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$');
        expect(error.getPath()).to.equal('/documents/indexedDocument/properties/something');
        expect(error.getOriginalErrorMessage()).to.be.a('string').and.satisfy((msg) => (
          msg.startsWith('Invalid regular expression')
        ));
      });

      describe('byteArray', () => {
        it('should be a boolean', async () => {
          rawDataContract.documents.withByteArrays.properties.byteArrayField.byteArray = 1;

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result, 2);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal('/documents/withByteArrays/properties/byteArrayField/byteArray');
          expect(error.keyword).to.equal('type');
          expect(error.params.type).to.equal('boolean');
        });

        it('should equal to true', async () => {
          rawDataContract.documents.withByteArrays.properties.byteArrayField.byteArray = false;

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result, 2);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal('/documents/withByteArrays/properties/byteArrayField/byteArray');
          expect(error.keyword).to.equal('const');
          expect(error.params.allowedValue).to.equal(true);
        });

        it('should be used with type `array`', async () => {
          rawDataContract.documents.withByteArrays.properties.byteArrayField.type = 'string';

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result, 2);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal('/documents/withByteArrays/properties/byteArrayField/type');
          expect(error.keyword).to.equal('const');
        });

        it('should not be used with `items`', async () => {
          rawDataContract.documents.withByteArrays.properties.byteArrayField.items = {
            type: 'string',
          };

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result, 1);

          const [error] = result.getErrors();

          expect(error.message).to.equal("'byteArray' should not be used with 'items'");
        });
      });

      describe('contentMediaType', () => {
        describe('application/x.dash.dpp.identifier', () => {
          it('should be used with byte array only', async () => {
            delete rawDataContract.documents.withByteArrays.properties.identifierField.byteArray;

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result, 2);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('/documents/withByteArrays/properties/identifierField');
            expect(error.keyword).to.equal('required');
          });

          it('should be used with byte array not shorter than 32 bytes', async () => {
            rawDataContract.documents.withByteArrays.properties.identifierField.minItems = 31;

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result, 2);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('/documents/withByteArrays/properties/identifierField/minItems');
            expect(error.keyword).to.equal('const');
          });

          it('should be used with byte array not longer than 32 bytes', async () => {
            rawDataContract.documents.withByteArrays.properties.identifierField.maxItems = 31;

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result, 2);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal('/documents/withByteArrays/properties/identifierField/maxItems');
            expect(error.keyword).to.equal('const');
          });
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

      expect(error.instancePath).to.equal('/documents/indexedDocument/indices');
      expect(error.keyword).to.equal('type');
    });

    it('should have at least one item', async () => {
      rawDataContract.documents.indexedDocument.indices = [];

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/documents/indexedDocument/indices');
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

        expect(error.instancePath).to.equal('/documents/indexedDocument/indices/0');
        expect(error.keyword).to.equal('type');
      });

      it('should have properties definition', async () => {
        rawDataContract.documents.indexedDocument.indices = [{}];

        const result = await validateDataContract(rawDataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/documents/indexedDocument/indices/0');
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

          expect(error.instancePath).to.equal(
            '/documents/indexedDocument/indices/0/properties',
          );
          expect(error.keyword).to.equal('type');
        });

        it('should have at least one property defined', async () => {
          rawDataContract.documents.indexedDocument.indices[0]
            .properties = [];

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal(
            '/documents/indexedDocument/indices/0/properties',
          );
          expect(error.keyword).to.equal('minItems');
        });

        it('should have no more than 10 property $defs', async () => {
          for (let i = 0; i < 10; i++) {
            rawDataContract.documents.indexedDocument.indices[0]
              .properties.push({ [`field${i}`]: 'asc' });
          }

          const result = await validateDataContract(rawDataContract);

          expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.instancePath).to.equal(
            '/documents/indexedDocument/indices/0/properties',
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

            expect(error.instancePath).to.equal(
              '/documents/indexedDocument/indices/0/properties/0',
            );
            expect(error.keyword).to.equal('type');
          });

          it('should have at least one property', async () => {
            rawDataContract.documents.indexedDocument.indices[0]
              .properties = [];

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal(
              '/documents/indexedDocument/indices/0/properties',
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

            expect(error.instancePath).to.equal(
              '/documents/indexedDocument/indices/0/properties/0',
            );
            expect(error.keyword).to.equal('maxProperties');
          });

          it('should have property values only "asc" or "desc"', async () => {
            rawDataContract.documents.indexedDocument.indices[0]
              .properties[0].$ownerId = 'wrong';

            const result = await validateDataContract(rawDataContract);

            expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.instancePath).to.equal(
              '/documents/indexedDocument/indices/0/properties/0/$ownerId',
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

        expect(error.instancePath).to.equal('/documents/indexedDocument/indices/0/unique');
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

        expect(error.instancePath).to.equal(
          '/documents/indexedDocument/indices',
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

      it('should return invalid result if index property is array with different item definitions', async () => {
        const indexedDocumentDefinition = rawDataContract.documents.indexedDocument;

        indexedDocumentDefinition.properties.arrayProperty = {
          type: 'array',
          prefixItems: [
            {
              type: 'string',
            },
            {
              type: 'number',
            },
          ],
          minItems: 2,
          items: false,
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

  describe('dependentSchemas', () => {
    it('should be an object', async () => {
      rawDataContract.documents.niceDocument = {
        type: 'object',
        properties: {
          abc: {
            type: 'string',
          },
        },
        additionalProperties: false,
        dependentSchemas: 'string',
      };

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('type');
      expect(error.instancePath).to.equal('/documents/niceDocument/dependentSchemas');
      expect(error.message).to.equal('must be object');
    });
  });

  describe('dependentRequired', () => {
    it('should be an object', async () => {
      rawDataContract.documents.niceDocument = {
        type: 'object',
        properties: {
          abc: {
            type: 'string',
          },
        },
        additionalProperties: false,
        dependentRequired: 'string',
      };

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('type');
      expect(error.instancePath).to.equal('/documents/niceDocument/dependentRequired');
      expect(error.message).to.equal('must be object');
    });

    it('should have an array value', async () => {
      rawDataContract.documents.niceDocument = {
        type: 'object',
        properties: {
          abc: {
            type: 'string',
          },
        },
        additionalProperties: false,
        dependentRequired: {
          zxy: {
            type: 'number',
          },
        },
      };

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('type');
      expect(error.instancePath).to.equal('/documents/niceDocument/dependentRequired/zxy');
      expect(error.message).to.equal('must be array');
    });

    it('should have an array of strings', async () => {
      rawDataContract.documents.niceDocument = {
        type: 'object',
        properties: {
          abc: {
            type: 'string',
          },
        },
        additionalProperties: false,
        dependentRequired: {
          zxy: [1, '2'],
        },
      };

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('type');
      expect(error.instancePath).to.equal('/documents/niceDocument/dependentRequired/zxy/0');
      expect(error.message).to.equal('must be string');
    });

    it('should have an array of unique strings', async () => {
      rawDataContract.documents.niceDocument = {
        type: 'object',
        properties: {
          abc: {
            type: 'string',
          },
        },
        additionalProperties: false,
        dependentRequired: {
          zxy: ['1', '2', '2'],
        },
      };

      const result = await validateDataContract(rawDataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('uniqueItems');
      expect(error.instancePath).to.equal('/documents/niceDocument/dependentRequired/zxy');
      expect(error.message).to.equal('must NOT have duplicate items (items ## 2 and 1 are identical)');
    });
  });

  it('should return invalid result with circular $ref pointer', async () => {
    rawDataContract.$defs.object = { $ref: '#/$defs/object' };

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
