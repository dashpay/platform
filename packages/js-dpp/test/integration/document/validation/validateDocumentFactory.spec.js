const { default: getRE2Class } = require('@dashevo/re2-wasm');

const createAjv = require('../../../../lib/ajv/createAjv');

const JsonSchemaValidator = require('../../../../lib/validation/JsonSchemaValidator');
const ValidationResult = require('../../../../lib/validation/ValidationResult');

const DataContract = require('../../../../lib/dataContract/DataContract');

const validateDocumentFactory = require('../../../../lib/document/validation/validateDocumentFactory');
const enrichDataContractWithBaseSchema = require('../../../../lib/dataContract/enrichDataContractWithBaseSchema');

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');

const MissingDocumentTypeError = require('../../../../lib/errors/consensus/basic/document/MissingDocumentTypeError');
const InvalidDocumentTypeError = require('../../../../lib/errors/consensus/basic/document/InvalidDocumentTypeError');

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('../../../../lib/test/expect/expectError');

describe('validateDocumentFactory', () => {
  let dataContract;
  let rawDocuments;
  let rawDocument;
  let documents;
  let validateDocument;
  let validator;

  beforeEach(async function beforeEach() {
    const RE2 = await getRE2Class();
    const ajv = createAjv(RE2);

    validator = new JsonSchemaValidator(ajv);

    this.sinonSandbox.spy(validator, 'validate');

    dataContract = getDataContractFixture();

    validateDocument = validateDocumentFactory(
      validator,
      enrichDataContractWithBaseSchema,
    );

    documents = getDocumentsFixture(dataContract);
    rawDocuments = documents.map((o) => o.toObject());
    [rawDocument] = rawDocuments;
  });

  describe('Base schema', () => {
    describe('$protocolVersion', () => {
      it('should be present', () => {
        delete rawDocument.$protocolVersion;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$protocolVersion');
      });

      it('should be an integer', () => {
        rawDocument.$protocolVersion = '1';

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$protocolVersion');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should not be less than 0', () => {
        rawDocument.$protocolVersion = -1;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$protocolVersion');
        expect(error.getKeyword()).to.equal('minimum');
      });

      it('should not be greater than current Document protocol version (0)', () => {
        rawDocument.$protocolVersion = 1;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$protocolVersion');
        expect(error.getKeyword()).to.equal('maximum');
      });
    });

    describe('$id', () => {
      it('should be present', () => {
        delete rawDocument.$id;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$id');
      });

      it('should be a byte array', () => {
        rawDocument.$id = new Array(32).fill('string');

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result, 2);

        const [error, byteArrayError] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$id/0');
        expect(error.getKeyword()).to.equal('type');

        expect(byteArrayError.getKeyword()).to.equal('byteArray');
      });

      it('should be no less than 32 bytes', () => {
        rawDocument.$id = Buffer.alloc(31);

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$id');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 32 bytes', () => {
        rawDocument.$id = Buffer.alloc(33);

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$id');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });

    describe('$type', () => {
      it('should be present', () => {
        delete rawDocument.$type;

        const result = validateDocument(rawDocument, dataContract);

        expectValidationError(
          result,
          MissingDocumentTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(1028);
      });

      it('should be defined in Data Contract', () => {
        rawDocument.$type = 'undefinedDocument';

        const result = validateDocument(rawDocument, dataContract);

        expectValidationError(
          result,
          InvalidDocumentTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(1024);
        expect(error.getType()).to.equal('undefinedDocument');
      });

      it('should throw an error if getDocumentSchemaRef throws error', function it() {
        const someError = new Error();

        this.sinonSandbox.stub(DataContract.prototype, 'getDocumentSchemaRef').throws(someError);

        let error;
        try {
          validateDocument(rawDocument, dataContract);
        } catch (e) {
          error = e;
        }

        expect(error).to.equal(someError);

        expect(dataContract.getDocumentSchemaRef).to.have.been.calledOnce();
      });
    });

    describe('$revision', () => {
      it('should be present', () => {
        delete rawDocument.$revision;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$revision');
      });

      it('should be a number', () => {
        rawDocument.$revision = 'string';

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$revision');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should be an integer', () => {
        rawDocument.$revision = 1.1;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$revision');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should be greater or equal to one', () => {
        rawDocument.$revision = -1;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$revision');
        expect(error.getKeyword()).to.equal('minimum');
      });
    });

    describe('$dataContractId', () => {
      it('should be present', () => {
        delete rawDocument.$dataContractId;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$dataContractId');
      });

      it('should be a byte array', () => {
        rawDocument.$dataContractId = new Array(32).fill('string');

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result, 2);

        const [error, byteArrayError] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$dataContractId/0');
        expect(error.getKeyword()).to.equal('type');

        expect(byteArrayError.getKeyword()).to.equal('byteArray');
      });

      it('should be no less than 32 bytes', () => {
        rawDocument.$dataContractId = Buffer.alloc(31);

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$dataContractId');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 32 bytes', () => {
        rawDocument.$dataContractId = Buffer.alloc(33);

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$dataContractId');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });

    describe('$ownerId', () => {
      it('should be present', () => {
        delete rawDocument.$ownerId;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$ownerId');
      });

      it('should be a byte array', () => {
        rawDocument.$ownerId = new Array(32).fill('string');

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result, 2);

        const [error, byteArrayError] = result.getErrors();

        expect(error.instancePath).to.equal('/$ownerId/0');
        expect(error.getKeyword()).to.equal('type');

        expect(byteArrayError.getKeyword()).to.equal('byteArray');
      });

      it('should be no less than 32 bytes', () => {
        rawDocument.$ownerId = Buffer.alloc(31);

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/$ownerId');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 32 bytes', () => {
        rawDocument.$ownerId = Buffer.alloc(33);

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/$ownerId');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });
  });

  describe('Data Contract schema', () => {
    it('should return an error if the first document is not valid against Data Contract', () => {
      rawDocument.name = 1;

      const result = validateDocument(rawDocument, dataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/name');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should return an error if the second document is not valid against Data Contract', () => {
      // eslint-disable-next-line prefer-destructuring
      rawDocument = rawDocuments[1];
      rawDocument.undefined = 1;

      const result = validateDocument(rawDocument, dataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.getKeyword()).to.equal('additionalProperties');
    });
  });

  it('return invalid result if a byte array exceeds `maxItems`', () => {
    // eslint-disable-next-line prefer-destructuring
    rawDocument = getDocumentsFixture(dataContract)[8].toObject();

    rawDocument.byteArrayField = Buffer.alloc(32);

    const result = validateDocument(rawDocument, dataContract);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.instancePath).to.equal('/byteArrayField');
    expect(error.getKeyword()).to.equal('maxItems');
  });

  it('should return valid result is a document is valid', () => {
    const result = validateDocument(rawDocument, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
