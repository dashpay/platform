const createAjv = require('../../../lib/ajv/createAjv');

const JsonSchemaValidator = require('../../../lib/validation/JsonSchemaValidator');
const ValidationResult = require('../../../lib/validation/ValidationResult');

const DataContract = require('../../../lib/dataContract/DataContract');

const validateDocumentFactory = require('../../../lib/document/validateDocumentFactory');
const enrichDataContractWithBaseSchema = require('../../../lib/dataContract/enrichDataContractWithBaseSchema');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');

const MissingDocumentTypeError = require('../../../lib/errors/MissingDocumentTypeError');
const InvalidDocumentTypeError = require('../../../lib/errors/InvalidDocumentTypeError');
const MismatchDocumentContractIdAndDataContractError = require('../../../lib/errors/MismatchDocumentContractIdAndDataContractError');

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('../../../lib/test/expect/expectError');

const generateRandomIdentifier = require('../../../lib/test/utils/generateRandomIdentifier');

describe('validateDocumentFactory', () => {
  let dataContract;
  let rawDocuments;
  let rawDocument;
  let documents;
  let validateDocument;
  let validator;

  beforeEach(function beforeEach() {
    const ajv = createAjv();

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

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('$protocolVersion');
      });

      it('should be an integer', () => {
        rawDocument.$protocolVersion = '1';

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$protocolVersion');
        expect(error.keyword).to.equal('type');
      });

      it('should not be less than 0', () => {
        rawDocument.$protocolVersion = -1;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$protocolVersion');
        expect(error.keyword).to.equal('minimum');
      });

      it('should not be greater than current Document protocol version (0)', () => {
        rawDocument.$protocolVersion = 1;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$protocolVersion');
        expect(error.keyword).to.equal('maximum');
      });
    });

    describe('$id', () => {
      it('should be present', () => {
        delete rawDocument.$id;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('$id');
      });

      it('should be a byte array', () => {
        rawDocument.$id = {};

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$id');
        expect(error.keyword).to.equal('byteArray');
      });

      it('should be no less than 32 bytes', () => {
        rawDocument.$id = Buffer.alloc(31);

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$id');
        expect(error.keyword).to.equal('minBytesLength');
      });

      it('should be no longer than 32 bytes', () => {
        rawDocument.$id = Buffer.alloc(33);

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$id');
        expect(error.keyword).to.equal('maxBytesLength');
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

        expect(error.getRawDocument()).to.equal(rawDocument);
      });

      it('should be defined in Data Contract', () => {
        rawDocument.$type = 'undefinedDocument';

        const result = validateDocument(rawDocument, dataContract);

        expectValidationError(
          result,
          InvalidDocumentTypeError,
        );

        const [error] = result.getErrors();

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

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('$revision');
      });

      it('should be a number', () => {
        rawDocument.$revision = 'string';

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$revision');
        expect(error.keyword).to.equal('type');
      });

      it('should be an integer', () => {
        rawDocument.$revision = 1.1;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$revision');
        expect(error.keyword).to.equal('type');
      });

      it('should be greater or equal to one', () => {
        rawDocument.$revision = -1;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$revision');
        expect(error.keyword).to.equal('minimum');
      });
    });

    describe('$dataContractId', () => {
      it('should be present', () => {
        delete rawDocument.$dataContractId;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('$dataContractId');
      });

      it('should be a byte array', () => {
        rawDocument.$dataContractId = {};

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$dataContractId');
        expect(error.keyword).to.equal('byteArray');
      });

      it('should be no less than 32 bytes', () => {
        rawDocument.$dataContractId = Buffer.alloc(31);

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$dataContractId');
        expect(error.keyword).to.equal('minBytesLength');
      });

      it('should be no longer than 32 bytes', () => {
        rawDocument.$dataContractId = Buffer.alloc(33);

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$dataContractId');
        expect(error.keyword).to.equal('maxBytesLength');
      });
    });

    describe('$ownerId', () => {
      it('should be present', () => {
        delete rawDocument.$ownerId;

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('$ownerId');
      });

      it('should be a byte array', () => {
        rawDocument.$ownerId = {};

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$ownerId');
        expect(error.keyword).to.equal('byteArray');
      });

      it('should be no less than 32 bytes', () => {
        rawDocument.$ownerId = Buffer.alloc(31);

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$ownerId');
        expect(error.keyword).to.equal('minBytesLength');
      });

      it('should be no longer than 32 bytes', () => {
        rawDocument.$ownerId = Buffer.alloc(33);

        const result = validateDocument(rawDocument, dataContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$ownerId');
        expect(error.keyword).to.equal('maxBytesLength');
      });
    });
  });

  describe('Data Contract schema', () => {
    it('should return an error if the first document is not valid against Data Contract', () => {
      rawDocument.name = 1;

      const result = validateDocument(rawDocument, dataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.name');
      expect(error.keyword).to.equal('type');
    });

    it('should return an error if the second document is not valid against Data Contract', () => {
      // eslint-disable-next-line prefer-destructuring
      rawDocument = rawDocuments[1];
      rawDocument.undefined = 1;

      const result = validateDocument(rawDocument, dataContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('additionalProperties');
    });
  });

  it('should return invalid result if a document contractId is not equal to Data Contract ID', () => {
    rawDocument.$dataContractId = generateRandomIdentifier();

    const result = validateDocument(rawDocument, dataContract);

    expectValidationError(result, MismatchDocumentContractIdAndDataContractError);

    const [error] = result.getErrors();

    expect(error.getDataContract()).to.equal(dataContract);
    expect(error.getRawDocument()).to.equal(rawDocument);
  });

  it('return invalid result if binary field exceeds `maxBytesLength`', () => {
    // eslint-disable-next-line prefer-destructuring
    rawDocument = getDocumentsFixture(dataContract)[8].toObject();

    rawDocument.byteArrayField = Buffer.alloc(32);

    const result = validateDocument(rawDocument, dataContract);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.dataPath).to.equal('.byteArrayField');
    expect(error.keyword).to.equal('maxBytesLength');
  });

  it('should return valid result is a document is valid', () => {
    const result = validateDocument(rawDocument, dataContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
