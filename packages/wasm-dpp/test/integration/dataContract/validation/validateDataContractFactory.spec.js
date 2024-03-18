const lodashCloneDeep = require('lodash/cloneDeep');
const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const { expectJsonSchemaError, expectValidationError } = require('../../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../../dist');

describe.skip('validateDataContractFactory', () => {
  let dataContract;
  let validateDataContract;

  let DataContractValidator;
  let ValidationResult;
  let JsonSchemaCompilationError;
  let IncompatibleRe2PatternError;
  let DuplicateIndexError;
  let UndefinedIndexPropertyError;
  let UniqueIndicesLimitReachedError;
  let SystemPropertyIndexAlreadyPresentError;
  let InvalidIndexPropertyTypeError;
  let InvalidIndexedPropertyConstraintError;
  let InvalidJsonSchemaRefError;

  before(async () => {
    ({
      DataContractValidator,
      ValidationResult,
      JsonSchemaCompilationError,
      IncompatibleRe2PatternError,
      DuplicateIndexError,
      UndefinedIndexPropertyError,
      UniqueIndicesLimitReachedError,
      SystemPropertyIndexAlreadyPresentError,
      InvalidIndexPropertyTypeError,
      InvalidIndexedPropertyConstraintError,
      InvalidJsonSchemaRefError,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    dataContract = await getDataContractFixture();

    const dataContractValidator = new DataContractValidator();

    validateDataContract = (contract) => dataContractValidator.validate(contract);
  });

  it('should pass validation if created from a fixture', () => {
    const rawDataContract = dataContract.toObject();
    const result = validateDataContract(rawDataContract);
    expect(result.isValid()).to.be.true();
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      const rawDataContract = dataContract.toObject();
      delete rawDataContract.protocolVersion;

      const result = validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      const rawDataContract = dataContract.toObject();

      rawDataContract.protocolVersion = '1';

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/protocolVersion');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be an unsigned integer', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.protocolVersion = -1;

      const result = await validateDataContract(rawDataContract);
      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/protocolVersion');
      expect(error.getKeyword()).to.equal('minimum');
    });
  });

  describe('$schema', () => {
    it('should be present', async () => {
      const rawDataContract = dataContract.toObject();
      delete rawDataContract.$schema;

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('$schema');
    });

    it('should be a string', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.$schema = 1;

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result, 2);

      const [typeError, constError] = result.getErrors();

      expect(typeError.getInstancePath()).to.equal('/$schema');
      expect(typeError.getKeyword()).to.equal('type');

      expect(constError.getInstancePath()).to.equal('/$schema');
      expect(constError.getKeyword()).to.equal('const');
    });

    it('should be a particular url', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.$schema = 'wrong';

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('const');
      expect(error.getInstancePath()).to.equal('/$schema');
    });
  });

  describe('ownerId', () => {
    it('should be present', async () => {
      const rawDataContract = dataContract.toObject();
      delete rawDataContract.ownerId;

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('ownerId');
    });

    it('should be a byte array', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.ownerId = new Array(32).fill('string');

      // We decided to keep protocol error because of failed parse rather than doing hacks
      // too keep it until validator will refuse it
      try {
        await validateDataContract(rawDataContract);
        expect.fail('Should throw an error');
      } catch (e) {
        expect(e).to.have.string('invalid type: string');
      }
    });

    it('should be no less than 32 bytes', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.ownerId = Buffer.alloc(31);

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/ownerId');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.ownerId = Buffer.alloc(33);

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/ownerId');
      expect(error.getKeyword()).to.equal('maxItems');
    });
  });

  describe('$id', () => {
    it('should be present', async () => {
      const rawDataContract = dataContract.toObject();
      delete rawDataContract.$id;

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('$id');
    });

    it('should be a byte array', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.$id = new Array(32).fill('string');

      // We decided to keep protocol error because of failed parse rather than doing hacks
      // too keep it until validator will refuse it
      try {
        await validateDataContract(rawDataContract);
        expect.fail('Should throw an error');
      } catch (e) {
        expect(e).to.have.string('invalid type: string');
      }
    });

    it('should be no less than 32 bytes', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.$id = Buffer.alloc(31);

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/$id');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.$id = Buffer.alloc(33);

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/$id');
      expect(error.getKeyword()).to.equal('maxItems');
    });
  });

  describe('$defs', () => {
    it('may not be present', async () => {
      const rawDataContract = dataContract.toObject();
      delete rawDataContract.$defs;
      delete rawDataContract.documents.prettyDocument;

      const result = await validateDataContract(rawDataContract);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should be an object', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.$defs = 1;

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/$defs');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should not be empty', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.$defs = {};

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/$defs');
      expect(error.getKeyword()).to.equal('minProperties');
    });

    it('should have no non-alphanumeric properties', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.$defs = {
        $subSchema: {},
      };

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [propertyNamesError] = result.getErrors();

      expect(propertyNamesError.getInstancePath()).to.equal('/$defs');
      expect(propertyNamesError.getKeyword()).to.equal('propertyNames');
    });

    it('should have valid properties\' values', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.$defs = {
        yeet: '3a1u9a',
      };

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result, 2);

      const [valueTypeError] = result.getErrors();

      expect(valueTypeError.getInstancePath()).to.equal('/$defs/yeet');
      expect(valueTypeError.getKeyword()).to.equal('type');
    });

    it('should have no more than 100 properties', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.$defs = {};

      Array(101).fill({ type: 'string' }).forEach((item, i) => {
        rawDataContract.$defs[i] = item;
      });

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/$defs');
      expect(error.getKeyword()).to.equal('maxProperties');
    });

    it('should have valid property names', async () => {
      const validNames = ['validName', 'valid_name', 'valid-name', 'abc', 'ab12c', 'abc123', 'ValidName',
        'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb',
        '-validname', '_validname', 'validname-', 'validname_', 'a', 'ab', '1', '123', '123_', '-123', '_123'];

      await Promise.all(
        validNames.map(async (name) => {
          const rawDataContract = dataContract.toObject();

          rawDataContract.$defs = {};
          rawDataContract.$defs[name] = {
            type: 'string',
          };

          rawDataContract.$defs[name] = {
            type: 'string',
          };

          const result = await validateDataContract(rawDataContract);

          await expectJsonSchemaError(result, 0);
        }),
      );
    });

    it('should return an invalid result if a property has invalid format', async () => {
      const invalidNames = ['*(*&^', '$test', '.', '.a'];

      await Promise.all(
        invalidNames.map(async (name) => {
          const rawDataContract = dataContract.toObject();

          rawDataContract.$defs = {};
          rawDataContract.$defs[name] = {
            type: 'string',
          };

          const result = await validateDataContract(rawDataContract);

          await expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.getInstancePath()).to.equal('/$defs');
          expect(error.getKeyword()).to.equal('propertyNames');
        }),
      );
    });
  });

  describe('documents', () => {
    it('should be present', async () => {
      const rawDataContract = dataContract.toObject();
      delete rawDataContract.documents;

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('documents');
    });

    it('should be an object', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.documents = 1;

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/documents');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should not be empty', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.documents = {};

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/documents');
      expect(error.getKeyword()).to.equal('minProperties');
    });

    it('should have valid property names (document types)', async () => {
      const rawDataContract = dataContract.toObject();
      const validNames = ['validName', 'valid_name', 'valid-name', 'abc', 'a123123bc', 'ab123c', 'ValidName', 'validName',
        'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb'];

      await Promise.all(
        validNames.map(async (name) => {
          const clonedDataContract = lodashCloneDeep(rawDataContract);

          clonedDataContract.documents[name] = clonedDataContract.documents.niceDocument;

          const result = await validateDataContract(clonedDataContract);

          await expectJsonSchemaError(result, 0);
        }),
      );
    });

    it('should return an invalid result if a property (document type) has invalid format', async () => {
      const rawDataContract = dataContract.toObject();
      const invalidNames = ['*(*&^', '$test', '.', '.a'];

      await Promise.all(
        invalidNames.map(async (name) => {
          const clonedDataContract = lodashCloneDeep(rawDataContract);

          clonedDataContract.documents[name] = clonedDataContract.documents.niceDocument;

          const result = await validateDataContract(clonedDataContract);

          await expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.getInstancePath()).to.equal('/documents');
          expect(error.getKeyword()).to.equal('propertyNames');
        }),
      );
    });

    it('should have no more than 100 properties', async () => {
      const rawDataContract = dataContract.toObject();
      const niceDocumentDefinition = rawDataContract.documents.niceDocument;

      rawDataContract.documents = {};

      Array(101).fill(niceDocumentDefinition).forEach((item, i) => {
        rawDataContract.documents[i] = item;
      });

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/documents');
      expect(error.getKeyword()).to.equal('maxProperties');
    });

    describe('Document schema', () => {
      it('should not be empty', async () => {
        const rawDataContract = dataContract.toObject();
        rawDataContract.documents.niceDocument.properties = {};

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/niceDocument/properties');
        expect(error.getKeyword()).to.equal('minProperties');
      });

      it('should have type "object"', async () => {
        const rawDataContract = dataContract.toObject();
        rawDataContract.documents.niceDocument.type = 'string';

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/niceDocument/type');
        expect(error.getKeyword()).to.equal('const');
      });

      it('should have "properties"', async () => {
        const rawDataContract = dataContract.toObject();
        delete rawDataContract.documents.niceDocument.properties;

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/niceDocument');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('properties');
      });

      it('should have nested "properties"', async () => {
        const rawDataContract = dataContract.toObject();
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

        await expectJsonSchemaError(result, 8);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/niceDocument/properties/object/prefixItems/0/properties/something');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('properties');
      });

      it('should have valid property names', async () => {
        const rawDataContract = dataContract.toObject();
        const validNames = ['validName', 'valid_name', 'valid-name', 'abc', 'a123bc', 'abc123', 'ValidName', 'validName',
          'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb'];

        await Promise.all(
          validNames.map(async (name) => {
            const clonedDataContract = lodashCloneDeep(rawDataContract);

            clonedDataContract.documents.niceDocument.properties[name] = {
              type: 'string',
            };

            const result = await validateDataContract(clonedDataContract);

            await expectJsonSchemaError(result, 0);
          }),
        );
      });

      it('should have valid nested property names', async () => {
        const rawDataContract = dataContract.toObject();
        const validNames = ['validName', 'valid_name', 'valid-name', 'abc', 'a123bc', 'abc123', 'ValidName', 'validName',
          'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb'];

        rawDataContract.documents.niceDocument.properties.something = {
          type: 'object',
          properties: {},
          additionalProperties: false,
        };

        await Promise.all(
          validNames.map(async (name) => {
            const clonedDataContract = lodashCloneDeep(rawDataContract);

            clonedDataContract.documents.niceDocument.properties.something.properties[name] = {
              type: 'string',
            };

            const result = await validateDataContract(clonedDataContract);

            await expectJsonSchemaError(result, 0);
          }),
        );
      });

      it('should return an invalid result if a property has invalid format', async () => {
        const rawDataContract = dataContract.toObject();
        const invalidNames = ['*(*&^', '$test', '.', '.a'];

        await Promise.all(
          invalidNames.map(async (name) => {
            const clonedDataContract = lodashCloneDeep(rawDataContract);

            clonedDataContract.documents.niceDocument.properties[name] = {};

            const result = await validateDataContract(clonedDataContract);

            await expectJsonSchemaError(result, 2);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal('/documents/niceDocument/properties');
            expect(error.getKeyword()).to.equal('propertyNames');
          }),
        );
      });

      it('should return an invalid result if a nested property has invalid format', async () => {
        const rawDataContract = dataContract.toObject();
        const invalidNames = ['*(*&^', '$test', '.', '.a'];

        rawDataContract.documents.niceDocument.properties.something = {
          properties: {},
          additionalProperties: false,
        };

        await Promise.all(
          invalidNames.map(async (name) => {
            const clonedDataContract = lodashCloneDeep(rawDataContract);

            clonedDataContract.documents.niceDocument.properties.something.properties[name] = {};

            const result = await validateDataContract(clonedDataContract);

            await expectJsonSchemaError(result, 4);

            const [errors] = result.getErrors();

            expect(errors.getInstancePath()).to.equal(
              '/documents/niceDocument/properties/something/properties',
            );
            expect(errors.getKeyword()).to.equal('propertyNames');
          }),
        );
      });

      it('should have "additionalProperties" defined', async () => {
        const rawDataContract = dataContract.toObject();
        delete rawDataContract.documents.niceDocument.additionalProperties;

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/niceDocument');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('additionalProperties');
      });

      it('should have "additionalProperties" defined to false', async () => {
        const rawDataContract = dataContract.toObject();
        rawDataContract.documents.niceDocument.additionalProperties = true;

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/niceDocument/additionalProperties');
        expect(error.getKeyword()).to.equal('const');
      });

      it('should have nested "additionalProperties" defined', async () => {
        const rawDataContract = dataContract.toObject();
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

        await expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/niceDocument/properties/object/prefixItems/0');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('additionalProperties');
      });

      it('should return invalid result if there are additional properties', async () => {
        const rawDataContract = dataContract.toObject();
        rawDataContract.additionalProperty = {};

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('additionalProperties');
      });

      it('should have no more than 100 properties', async () => {
        const rawDataContract = dataContract.toObject();
        const propertyDefinition = {};

        rawDataContract.documents.niceDocument.properties = {};

        Array(101).fill(propertyDefinition).forEach((item, i) => {
          rawDataContract.documents.niceDocument.properties[i] = item;
        });

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result, 406);

        const errors = result.getErrors();

        expect(errors[405].getInstancePath()).to.equal('/documents/niceDocument/properties');
        expect(errors[405].getKeyword()).to.equal('maxProperties');
      });

      it('should have defined items for arrays', async () => {
        const rawDataContract = dataContract.toObject();
        rawDataContract.documents.new = {
          properties: {
            something: {
              type: 'array',
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/new/properties/something');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('items');
      });

      it('should have sub schema in items for arrays', async () => {
        const rawDataContract = dataContract.toObject();
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

        await expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/new/properties/something/items');
        expect(error.getKeyword()).to.equal('type');
        expect(error.getParams().type).to.equal('object');
      });

      it('should have items if prefixItems is used for arrays', async () => {
        const rawDataContract = dataContract.toObject();
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

        await expectJsonSchemaError(result, 4);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/new/properties/something');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('items');
      });

      it('should not have items disabled if prefixItems is used for arrays', async () => {
        const rawDataContract = dataContract.toObject();
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

        await expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/new/properties/something/items');
        expect(error.getKeyword()).to.equal('const');
        expect(error.getParams().allowedValue).to.equal(false);
      });

      it('should return invalid result if "default" keyword is used', async () => {
        const rawDataContract = dataContract.toObject();
        rawDataContract.documents.indexedDocument.properties.firstName.default = '1';

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/indexedDocument/properties/firstName/default');
        expect(error.getKeyword()).to.equal('unevaluatedProperties');
      });

      it('should return invalid result if remote `$ref` is used', async () => {
        const rawDataContract = dataContract.toObject();
        rawDataContract.documents.indexedDocument = {
          $ref: 'http://remote.com/schema#',
        };

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/indexedDocument/$ref');
        expect(error.getKeyword()).to.equal('pattern');
      });

      it('should not have `propertyNames`', async () => {
        const rawDataContract = dataContract.toObject();
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

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/indexedDocument/propertyNames');
        expect(error.getKeyword()).to.equal('unevaluatedProperties');
      });

      it('should have `maxItems` if `uniqueItems` is used', async () => {
        const rawDataContract = dataContract.toObject();
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

        await expectJsonSchemaError(result, 4);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/indexedDocument/properties/something');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('maxItems');
      });

      it('should have `maxItems` no bigger than 100000 if `uniqueItems` is used', async () => {
        const rawDataContract = dataContract.toObject();
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

        await expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/indexedDocument/properties/something/maxItems');
        expect(error.getKeyword()).to.equal('maximum');
      });

      it('should return invalid result if document JSON Schema is not valid', async () => {
        const rawDataContract = dataContract.toObject();
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

        expectValidationError(result, JsonSchemaCompilationError);

        const [error] = result.getErrors();
        // TODO: should be a JsonSchemaCompilationerror instead of JsonSchemaError
        expect(error.getCode()).to.equal(1005);
        expect(error.getKeyword()).to.equal('format');
      });

      it('should have `maxLength` if `pattern` is used', async () => {
        const rawDataContract = dataContract.toObject();
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

        await expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/indexedDocument/properties/something');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('maxLength');
      });

      it('should have `maxLength` no bigger than 50000 if `pattern` is used', async () => {
        const rawDataContract = dataContract.toObject();
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

        await expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/indexedDocument/properties/something/maxLength');
        expect(error.getKeyword()).to.equal('maximum');
      });

      it('should have `maxLength` if `format` is used', async () => {
        const rawDataContract = dataContract.toObject();
        rawDataContract.documents.indexedDocument = {
          type: 'object',
          properties: {
            something: {
              type: 'string',
              format: 'uri',
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/indexedDocument/properties/something');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('maxLength');
      });

      it('should have `maxLength` no bigger than 50000 if `format` is used', async () => {
        const rawDataContract = dataContract.toObject();
        rawDataContract.documents.indexedDocument = {
          type: 'object',
          properties: {
            something: {
              type: 'string',
              format: 'uri',
              maxLength: 60000,
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/indexedDocument/properties/something/maxLength');
        expect(error.getKeyword()).to.equal('maximum');
      });

      it('should not have incompatible patterns', async () => {
        const rawDataContract = dataContract.toObject();
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

        expect(error.getCode()).to.equal(10202);
        expect(error.getPattern()).to.equal('^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$');
        expect(error.getPath()).to.equal('/documents/indexedDocument/properties/something');
      });

      describe('byteArray', () => {
        it('should be a boolean', async () => {
          const rawDataContract = dataContract.toObject();
          rawDataContract.documents.withByteArrays.properties.byteArrayField.byteArray = 1;

          const result = await validateDataContract(rawDataContract);

          await expectJsonSchemaError(result, 4);

          const [error] = result.getErrors();

          expect(error.getInstancePath()).to.equal('/documents/withByteArrays/properties/byteArrayField/byteArray');
          expect(error.getKeyword()).to.equal('type');
          expect(error.getParams().type).to.equal('boolean');
        });

        it('should equal to true', async () => {
          const rawDataContract = dataContract.toObject();
          rawDataContract.documents.withByteArrays.properties.byteArrayField.byteArray = false;

          const result = await validateDataContract(rawDataContract);

          await expectJsonSchemaError(result, 2);

          const [error] = result.getErrors();

          expect(error.getInstancePath()).to.equal('/documents/withByteArrays/properties/byteArrayField/byteArray');
          expect(error.getKeyword()).to.equal('const');
          expect(error.getParams().allowedValue).to.equal(true);
        });

        it('should be used with type `array`', async () => {
          const rawDataContract = dataContract.toObject();
          rawDataContract.documents.withByteArrays.properties.byteArrayField.type = 'string';

          const result = await validateDataContract(rawDataContract);

          await expectJsonSchemaError(result, 2);

          const [error] = result.getErrors();

          expect(error.getInstancePath()).to.equal('/documents/withByteArrays/properties/byteArrayField/type');
          expect(error.getKeyword()).to.equal('const');
        });

        it('should not be used with `items`', async () => {
          const rawDataContract = dataContract.toObject();
          rawDataContract.documents.withByteArrays.properties.byteArrayField.items = {
            type: 'string',
          };

          const result = await validateDataContract(rawDataContract);

          expectValidationError(result, JsonSchemaCompilationError);

          const [error] = result.getErrors();

          expect(error.getCode()).to.equal(1004);
        });
      });

      describe('contentMediaType', () => {
        describe('application/x.dash.dpp.identifier', () => {
          it('should be used with byte array only', async () => {
            const rawDataContract = dataContract.toObject();
            delete rawDataContract.documents.withByteArrays.properties.identifierField.byteArray;

            const result = await validateDataContract(rawDataContract);

            await expectJsonSchemaError(result, 4);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal('/documents/withByteArrays/properties/identifierField');
            expect(error.getKeyword()).to.equal('required');
          });

          it('should be used with byte array not shorter than 32 bytes', async () => {
            const rawDataContract = dataContract.toObject();
            rawDataContract.documents.withByteArrays.properties.identifierField.minItems = 31;

            const result = await validateDataContract(rawDataContract);

            await expectJsonSchemaError(result, 2);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal('/documents/withByteArrays/properties/identifierField/minItems');
            expect(error.getKeyword()).to.equal('const');
          });

          it('should be used with byte array not longer than 32 bytes', async () => {
            const rawDataContract = dataContract.toObject();
            rawDataContract.documents.withByteArrays.properties.identifierField.maxItems = 31;

            const result = await validateDataContract(rawDataContract);

            await expectJsonSchemaError(result, 2);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal('/documents/withByteArrays/properties/identifierField/maxItems');
            expect(error.getKeyword()).to.equal('const');
          });
        });
      });
    });
  });

  describe('indices', () => {
    it('should be an array', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.documents.indexedDocument.indices = 'definitely not an array';

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/documents/indexedDocument/indices');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should have at least one item', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.documents.indexedDocument.indices = [];

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/documents/indexedDocument/indices');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should return invalid result if there are duplicated indices', async () => {
      const rawDataContract = dataContract.toObject();
      const indexDefinition = {
        ...rawDataContract.documents.indexedDocument.indices[0],
        name: 'otherIndexName',
      };

      rawDataContract.documents.indexedDocument.indices.push(indexDefinition);

      const result = await validateDataContract(rawDataContract);

      expectValidationError(result, DuplicateIndexError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1008);
      expect(error.getIndexName()).to.equal('otherIndexName');
      expect(error.getDocumentType()).to.deep.equal('indexedDocument');
    });

    it('should return invalid result if there are duplicated index names', async () => {
      const rawDataContract = dataContract.toObject();
      const indexDefinition = {
        ...rawDataContract.documents.indexedDocument.indices[0],
      };

      rawDataContract.documents.indexedDocument.indices.push(indexDefinition);

      const result = await validateDataContract(rawDataContract);

      expect(result.isValid()).to.be.false();

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1048);
      expect(error.getDocumentType()).to.deep.equal('indexedDocument');
      expect(error.getDuplicateIndexName()).to.deep.equal('index1');
    });

    describe('index', () => {
      it('should be an object', async () => {
        const rawDataContract = dataContract.toObject();
        rawDataContract.documents.indexedDocument.indices = ['something else'];

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/indexedDocument/indices/0');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should have properties definition', async () => {
        const rawDataContract = dataContract.toObject();
        rawDataContract.documents.indexedDocument.indices = [{}];

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result, 2);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/indexedDocument/indices/0');
        expect(error.getParams().missingProperty).to.equal('properties');
        expect(error.getKeyword()).to.equal('required');
      });

      describe('properties definition', () => {
        it('should be an array', async () => {
          const rawDataContract = dataContract.toObject();
          rawDataContract.documents.indexedDocument.indices[0]
            .properties = 'something else';

          const result = await validateDataContract(rawDataContract);

          await expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.getInstancePath()).to.equal(
            '/documents/indexedDocument/indices/0/properties',
          );
          expect(error.getKeyword()).to.equal('type');
        });

        it('should have at least one property defined', async () => {
          const rawDataContract = dataContract.toObject();
          rawDataContract.documents.indexedDocument.indices[0]
            .properties = [];

          const result = await validateDataContract(rawDataContract);

          await expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.getInstancePath()).to.equal(
            '/documents/indexedDocument/indices/0/properties',
          );
          expect(error.getKeyword()).to.equal('minItems');
        });

        it('should have no more than 10 property $defs', async () => {
          const rawDataContract = dataContract.toObject();
          for (let i = 0; i < 10; i++) {
            rawDataContract.documents.indexedDocument.indices[0]
              .properties.push({ [`field${i}`]: 'asc' });
          }

          const result = await validateDataContract(rawDataContract);

          await expectJsonSchemaError(result);

          const [error] = result.getErrors();

          expect(error.getInstancePath()).to.equal(
            '/documents/indexedDocument/indices/0/properties',
          );
          expect(error.getKeyword()).to.equal('maxItems');
        });

        describe('property definition', () => {
          it('should be an object', async () => {
            const rawDataContract = dataContract.toObject();
            rawDataContract.documents.indexedDocument.indices[0]
              .properties[0] = 'something else';

            const result = await validateDataContract(rawDataContract);

            await expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal(
              '/documents/indexedDocument/indices/0/properties/0',
            );
            expect(error.getKeyword()).to.equal('type');
          });

          it('should have at least one property', async () => {
            const rawDataContract = dataContract.toObject();
            rawDataContract.documents.indexedDocument.indices[0]
              .properties = [];

            const result = await validateDataContract(rawDataContract);

            await expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal(
              '/documents/indexedDocument/indices/0/properties',
            );
            expect(error.getKeyword()).to.equal('minItems');
          });

          it('should have no more than one property', async () => {
            const rawDataContract = dataContract.toObject();
            const property = rawDataContract.documents.indexedDocument.indices[0]
              .properties[0];

            property.anotherField = 'something';

            const result = await validateDataContract(rawDataContract);

            await expectJsonSchemaError(result, 2);

            const error = result.getErrors()[1];

            expect(error.getInstancePath()).to.equal(
              '/documents/indexedDocument/indices/0/properties/0',
            );
            expect(error.getKeyword()).to.equal('maxProperties');
          });

          it('should have property values only "asc" or "desc"', async () => {
            const rawDataContract = dataContract.toObject();
            rawDataContract.documents.indexedDocument.indices[0]
              .properties[0].$ownerId = 'wrong';

            const result = await validateDataContract(rawDataContract);

            await expectJsonSchemaError(result);

            const [error] = result.getErrors();

            expect(error.getInstancePath()).to.equal(
              '/documents/indexedDocument/indices/0/properties/0/$ownerId',
            );
            expect(error.getKeyword()).to.equal('enum');
          });
        });
      });

      describe('property names', () => {
        it('should have valid property names (indices)', async () => {
          const rawDataContract = dataContract.toObject();
          const validNames = ['validName', 'valid_name', 'valid-name', 'abc', 'a123123bc', 'ab123c', 'ValidName', 'validName',
            'abcdefghigklmnopqrstuvwxyz01234567890abcdefghigklmnopqrstuvwxyz', 'abc_gbf_gdb', 'abc-gbf-gdb'];

          await Promise.all(
            validNames.map(async (name) => {
              const clonedDataContract = lodashCloneDeep(rawDataContract);

              clonedDataContract.documents.indexedDocument.properties[name] = { type: 'string', maxLength: 63 };
              clonedDataContract.documents.indexedDocument.indices[0].properties.push({ [name]: 'asc' });
              clonedDataContract.documents.indexedDocument.required.push(name);

              const result = await validateDataContract(clonedDataContract);

              await expectJsonSchemaError(result, 0);
            }),
          );
        });

        it('should return an invalid result if a property (indices) has invalid format', async () => {
          const rawDataContract = dataContract.toObject();
          const invalidNames = ['a.', '.a'];

          rawDataContract.documents.indexedDocument = {
            type: 'object',
            properties: {
              a: {
                type: 'object',
                properties: {
                  property: {
                    type: 'string',
                    maxLength: 63,
                  },
                },
                additionalProperties: false,
              },
            },
            indices: [
              {
                name: 'index1',
                properties: [],
                unique: true,
              },
            ],
            additionalProperties: false,
          };

          await Promise.all(
            invalidNames.map(async (name) => {
              const clonedDataContract = lodashCloneDeep(rawDataContract);

              clonedDataContract.documents.indexedDocument.indices[0].properties.push({ [name]: 'asc' });

              const result = await validateDataContract(clonedDataContract);

              expectValidationError(result, UndefinedIndexPropertyError, 1);
            }),
          );
        });
      });

      it('should have "unique" flag to be of a boolean type', async () => {
        const rawDataContract = dataContract.toObject();
        rawDataContract.documents.indexedDocument.indices[0].unique = 12;

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/documents/indexedDocument/indices/0/unique');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should have no more than 10 indices', async () => {
        const rawDataContract = dataContract.toObject();
        for (let i = 0; i < 10; i++) {
          const propertyName = `field${i}`;

          rawDataContract.documents.indexedDocument.properties[propertyName] = { type: 'string' };

          rawDataContract.documents.indexedDocument.indices.push({
            properties: [{ [propertyName]: 'asc' }],
          });
        }

        const result = await validateDataContract(rawDataContract);

        await expectJsonSchemaError(result, 11);

        const error = result.getErrors()[10];

        expect(error.getInstancePath()).to.equal(
          '/documents/indexedDocument/indices',
        );
        expect(error.getKeyword()).to.equal('maxItems');
      });

      it('should have no more than 3 unique indices', async () => {
        const rawDataContract = dataContract.toObject();
        for (let i = 0; i < 4; i++) {
          const propertyName = `field${i}`;

          rawDataContract.documents.indexedDocument.properties[propertyName] = {
            type: 'string',
            maxLength: 63,
          };

          rawDataContract.documents.indexedDocument.indices.push({
            name: `index_${i}`,
            properties: [{ [propertyName]: 'asc' }],
            unique: true,
          });
        }

        const result = await validateDataContract(rawDataContract);

        expectValidationError(result, UniqueIndicesLimitReachedError);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(1017);
        expect(error.getDocumentType()).to.equal('indexedDocument');
      });

      it('should return invalid result if $id is specified as an indexed property', async () => {
        const rawDataContract = dataContract.toObject();
        const indexDefinition = {
          name: 'index_1',
          properties: [
            { $id: 'asc' },
            { firstName: 'asc' },
          ],
        };

        const indeciesDefinition = rawDataContract.documents.indexedDocument.indices;

        indeciesDefinition.push(indexDefinition);

        const result = await validateDataContract(rawDataContract);

        expectValidationError(result, SystemPropertyIndexAlreadyPresentError);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(1015);
        expect(error.getPropertyName()).to.equal('$id');
        expect(error.getDocumentType()).to.deep.equal('indexedDocument');
        expect(error.getIndexName()).to.deep.equal('index_1');
      });

      it('should return invalid result if indices has undefined property', async () => {
        const rawDataContract = dataContract.toObject();
        const indexDefinition = rawDataContract.documents.indexedDocument.indices[0];

        indexDefinition.properties.push({
          missingProperty: 'asc',
        });

        const result = await validateDataContract(rawDataContract);

        expectValidationError(result, UndefinedIndexPropertyError);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(1016);
        expect(error.getPropertyName()).to.equal('missingProperty');
        expect(error.getDocumentType()).to.deep.equal('indexedDocument');
        expect(error.getIndexName()).to.deep.equal('index1');
      });

      it('should return invalid result if index property is object', async () => {
        const rawDataContract = dataContract.toObject();
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

        expect(error.getCode()).to.equal(1013);
        expect(error.getPropertyName()).to.equal('objectProperty');
        expect(error.getPropertyType()).to.equal('object');
        expect(error.getDocumentType()).to.deep.equal('indexedDocument');
        expect(error.getIndexName()).to.deep.equal('index1');
      });

      it('should return invalid result if index property is an array', async () => {
        const rawDataContract = dataContract.toObject();
        rawDataContract.documents.indexedArray = {
          type: 'object',
          indices: [
            {
              name: 'index1',
              properties: [
                { mentions: 'asc' },
              ],
            },
          ],
          properties: {
            mentions: {
              type: 'array',
              prefixItems: [
                {
                  type: 'string',
                  maxLength: 100,
                },
              ],
              minItems: 1,
              maxItems: 5,
              items: false,
            },
          },
          additionalProperties: false,
        };

        const result = await validateDataContract(rawDataContract);

        expectValidationError(result, InvalidIndexPropertyTypeError);

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(1013);
        expect(error.getPropertyName()).to.equal('mentions');
        expect(error.getPropertyType()).to.equal('array');
        expect(error.getDocumentType()).to.deep.equal('indexedArray');
        expect(error.getIndexName()).to.equal('index1');
      });

      it('should return invalid result if index property is array of objects', async () => {
        const rawDataContract = dataContract.toObject();
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

        expect(error.getCode()).to.equal(1013);
        expect(error.getPropertyName()).to.equal('arrayProperty');
        expect(error.getPropertyType()).to.equal('array');
        expect(error.getDocumentType()).to.deep.equal('indexedDocument');
        expect(error.getIndexName()).to.equal('index1');
      });

      // TODO: support indexed arrays
      it.skip(
        'should return invalid result if index property is an array of different types',
        async () => {
          const rawDataContract = dataContract.toObject();

          const indexedDocumentDefinition = rawDataContract.documents.indexedArray;

          const indexDefinition = indexedDocumentDefinition.indices[0];

          rawDataContract.documents.indexedArray.properties.mentions.prefixItems = [
            {
              type: 'string',
            },
            {
              type: 'number',
            },
          ];

          rawDataContract.documents.indexedArray.properties.mentions.minItems = 2;

          const result = await validateDataContract(rawDataContract);
          expectValidationError(result, InvalidIndexPropertyTypeError);

          const error = result.getFirstError();

          expect(error.getCode()).to.equal(1013);
          expect(error.getPropertyName()).to.equal('mentions');
          expect(error.getPropertyType()).to.equal('array');
          expect(error.getDocumentType()).to.deep.equal('indexedArray');
          expect(error.getIndexDefinition().toObject()).to.deep.equal(indexDefinition);
        },
      );

      // TODO: support indexed arrays
      it.skip('should return invalid result if index property contained prefixItems array of arrays', async () => {
        const rawDataContract = dataContract.toObject();

        const indexedDocumentDefinition = rawDataContract.documents.indexedArray;

        const indexDefinition = indexedDocumentDefinition.indices[0];

        rawDataContract.documents.indexedArray.properties.mentions.prefixItems = [
          {
            type: 'array',
            items: {
              type: 'string',
            },
          },
        ];

        const result = await validateDataContract(rawDataContract);
        expectValidationError(result, InvalidIndexPropertyTypeError);

        const error = result.getFirstError();

        expect(error.getCode()).to.equal(1013);
        expect(error.getPropertyName()).to.equal('mentions');
        expect(error.getPropertyType()).to.equal('array');
        expect(error.getDocumentType()).to.deep.equal('indexedArray');
        expect(error.getIndexDefinition().toObject()).to.deep.equal(indexDefinition);
      });

      // TODO: support indexed arrays
      it.skip('should return invalid result if index property contained prefixItems array of objects', async () => {
        const rawDataContract = dataContract.toObject();

        const indexedDocumentDefinition = rawDataContract.documents.indexedArray;

        const indexDefinition = indexedDocumentDefinition.indices[0];

        rawDataContract.documents.indexedArray.properties.mentions.prefixItems = [
          {
            type: 'object',
            properties: {
              something: {
                type: 'string',
              },
            },
            additionalProperties: false,
          },
        ];

        const result = await validateDataContract(rawDataContract);
        expectValidationError(result, InvalidIndexPropertyTypeError);

        const error = result.getFirstError();

        expect(error.getCode()).to.equal(1013);
        expect(error.getPropertyName()).to.equal('mentions');
        expect(error.getPropertyType()).to.equal('array');
        expect(error.getDocumentType()).to.deep.equal('indexedArray');
        expect(error.getIndexDefinition().toObject()).to.deep.equal(indexDefinition);
      });

      it('should return invalid result if index property is array of arrays', async () => {
        const rawDataContract = dataContract.toObject();
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

        expect(error.getCode()).to.equal(1013);
        expect(error.getPropertyName()).to.equal('arrayProperty');
        expect(error.getPropertyType()).to.equal('array');
        expect(error.getDocumentType()).to.deep.equal('indexedDocument');
        expect(error.getIndexName()).to.equal('index1');
      });

      it('should return invalid result if index property is array with different item definitions', async () => {
        const rawDataContract = dataContract.toObject();
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

        expect(error.getCode()).to.equal(1013);
        expect(error.getPropertyName()).to.equal('arrayProperty');
        expect(error.getPropertyType()).to.equal('array');
        expect(error.getDocumentType()).to.deep.equal('indexedDocument');
        expect(error.getIndexName()).to.equal('index1');
      });
    });
  });

  describe('signatureSecurityLevelRequirement', () => {
    it('should be a number', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.documents.indexedDocument.signatureSecurityLevelRequirement = 'definitely not a number';

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result, 2);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/documents/indexedDocument/signatureSecurityLevelRequirement');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be one of the available values', async () => {
      const rawDataContract = dataContract.toObject();
      rawDataContract.documents.indexedDocument.signatureSecurityLevelRequirement = 199;

      const result = await validateDataContract(rawDataContract);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/documents/indexedDocument/signatureSecurityLevelRequirement');
      expect(error.getKeyword()).to.equal('enum');
    });
  });

  describe('dependentSchemas', () => {
    it('should be an object', async () => {
      const rawDataContract = dataContract.toObject();
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

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('type');
      expect(error.getInstancePath()).to.equal('/documents/niceDocument/dependentSchemas');
    });
  });

  describe('dependentRequired', () => {
    it('should be an object', async () => {
      const rawDataContract = dataContract.toObject();
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

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('type');
      expect(error.getInstancePath()).to.equal('/documents/niceDocument/dependentRequired');
    });

    it('should have an array value', async () => {
      const rawDataContract = dataContract.toObject();
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

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('type');
      expect(error.getInstancePath()).to.equal('/documents/niceDocument/dependentRequired/zxy');
    });

    it('should have an array of strings', async () => {
      const rawDataContract = dataContract.toObject();
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

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('type');
      expect(error.getInstancePath()).to.equal('/documents/niceDocument/dependentRequired/zxy/0');
    });

    it('should have an array of unique strings', async () => {
      const rawDataContract = dataContract.toObject();
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

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('uniqueItems');
      expect(error.getInstancePath()).to.equal('/documents/niceDocument/dependentRequired/zxy');
    });
  });

  it('should return invalid result with circular $ref pointer', async () => {
    const rawDataContract = dataContract.toObject();
    rawDataContract.$defs = {};
    rawDataContract.$defs.object = { $ref: '#/$defs/object' };

    const result = await validateDataContract(rawDataContract);

    expectValidationError(result, InvalidJsonSchemaRefError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1014);
  });

  it('should return invalid result if indexed string property missing maxLength constraint', async () => {
    const rawDataContract = dataContract.toObject();
    delete rawDataContract.documents.indexedDocument.properties.firstName.maxLength;

    const result = await validateDataContract(rawDataContract);

    expectValidationError(result, InvalidIndexedPropertyConstraintError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1012);
    expect(error.getPropertyName()).to.equal('firstName');
    expect(error.getConstraintName()).to.equal('maxLength');
    expect(error.getReason()).to.equal('should be less or equal than 63');
  });

  it('should return invalid result if indexed string property have to big maxLength', async () => {
    const rawDataContract = dataContract.toObject();
    rawDataContract.documents.indexedDocument.properties.firstName.maxLength = 2048;

    const result = await validateDataContract(rawDataContract);

    expectValidationError(result, InvalidIndexedPropertyConstraintError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1012);
    expect(error.getPropertyName()).to.equal('firstName');
    expect(error.getConstraintName()).to.equal('maxLength');
    expect(error.getReason()).to.equal('should be less or equal than 63');
  });

  // TODO: support indexed arrays
  it.skip(
    'should return invalid result if indexed array property missing maxItems constraint',
    async () => {
      const rawDataContract = dataContract.toObject();
      delete rawDataContract.documents.indexedArray.properties.mentions.maxItems;

      const result = await validateDataContract(rawDataContract);

      expectValidationError(result, InvalidIndexedPropertyConstraintError);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1012);
      expect(error.getPropertyName()).to.equal('mentions');
      expect(error.getConstraintName()).to.equal('maxItems');
      expect(error.getReason()).to.equal('should be less or equal 63');
    },
  );

  // TODO support indexed arrays
  it.skip('should return invalid result if indexed array property have to big maxItems', async () => {
    const rawDataContract = dataContract.toObject();
    rawDataContract.documents.indexedArray.properties.mentions.maxItems = 2048;

    const result = await validateDataContract(rawDataContract);

    expectValidationError(result, InvalidIndexedPropertyConstraintError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1012);
    expect(error.getPropertyName()).to.equal('mentions');
    expect(error.getConstraintName()).to.equal('maxItems');
    expect(error.getReason()).to.equal('should be less or equal 63');
  });

  // TODO support indexed arrays
  it.skip('should return invalid result if indexed array property have string item without maxItems constraint', async () => {
    const rawDataContract = dataContract.toObject();
    delete rawDataContract.documents.indexedArray.properties.mentions.maxItems;

    const result = await validateDataContract(rawDataContract);

    expectValidationError(result, InvalidIndexedPropertyConstraintError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1012);
    expect(error.getPropertyName()).to.equal('mentions');
    expect(error.getConstraintName()).to.equal('maxItems');
    expect(error.getReason()).to.equal('should be less or equal 63');
  });

  // TODO support indexed arrays
  it.skip('should return invalid result if indexed array property have string item with maxItems bigger than 1024', async () => {
    const rawDataContract = dataContract.toObject();
    rawDataContract.documents.indexedArray.properties.mentions.maxItems = 2048;

    const result = await validateDataContract(rawDataContract);

    expectValidationError(result, InvalidIndexedPropertyConstraintError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1012);
    expect(error.getPropertyName()).to.equal('mentions');
    expect(error.getConstraintName()).to.equal('maxItems');
    expect(error.getReason()).to.equal('should be less or equal 63');
  });

  it('should return invalid result if indexed byte array property missing maxItems constraint', async () => {
    const rawDataContract = dataContract.toObject();
    delete rawDataContract.documents.withByteArrays.properties.byteArrayField.maxItems;

    const result = await validateDataContract(rawDataContract);

    expectValidationError(result, InvalidIndexedPropertyConstraintError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1012);
    expect(error.getPropertyName()).to.equal('byteArrayField');
    expect(error.getConstraintName()).to.equal('maxItems');
    expect(error.getReason()).to.equal('should be less or equal 255');
  });

  it('should return invalid result if indexed byte array property have to big maxItems', async () => {
    const rawDataContract = dataContract.toObject();
    rawDataContract.documents.withByteArrays.properties.byteArrayField.maxItems = 8192;

    const result = await validateDataContract(rawDataContract);

    expectValidationError(result, InvalidIndexedPropertyConstraintError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1012);
    expect(error.getPropertyName()).to.equal('byteArrayField');
    expect(error.getConstraintName()).to.equal('maxItems');
    expect(error.getReason()).to.equal('should be less or equal 255');
  });

  it('should return valid result if Data Contract is valid', async () => {
    const rawDataContract = dataContract.toObject();
    const result = await validateDataContract(rawDataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
