const { getRE2Class } = require('@dashevo/wasm-re2');

const createAjv = require('@dashevo/dpp/lib/ajv/createAjv');

const JsonSchemaValidatorJs = require('@dashevo/dpp/lib/validation/JsonSchemaValidator');
const ValidationResultJs = require('@dashevo/dpp/lib/validation/ValidationResult');

const DataContractJs = require('@dashevo/dpp/lib/dataContract/DataContract');

const validateDocumentFactoryJs = require('@dashevo/dpp/lib/document/validation/validateDocumentFactory');
const enrichDataContractWithBaseSchemaJs = require('@dashevo/dpp/lib/dataContract/enrichDataContractWithBaseSchema');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const MissingDocumentTypeErrorJs = require('@dashevo/dpp/lib/errors/consensus/basic/document/MissingDocumentTypeError');
const InvalidDocumentTypeErrorJs = require('@dashevo/dpp/lib/errors/consensus/basic/document/InvalidDocumentTypeError');

const { default: loadWasmDpp } = require('../../../dist');


let JsonSchemaValidator;
let ValidationResult;
let DataContract;
let validateDocumentFactory;

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('@dashevo/dpp/lib/test/expect/expectError');
const SomeConsensusError = require('@dashevo/dpp/lib/test/mocks/SomeConsensusError');

describe('validateDocumentFactory', () => {
  let dataContractJs;
  let rawDocuments;
  let rawDocument;
  let documents;
  let validateDocumentJs;
  let validateDocument;
  let validatorJs;
  let validator;
  let validateProtocolVersionMock;

  beforeEach(async () => {
    ({
      ValidationResult, DataContract, JsonSchemaValidator,
      // Identifier, ProtocolVersionValidator, DocumentValidator, DocumentFactory,
      // DataContract, Document,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    const RE2 = await getRE2Class();
    const ajv = createAjv(RE2);

    validatorJs = new JsonSchemaValidatorJs(ajv);

    this.sinonSandbox.spy(validatorJs, 'validate');

    dataContractJs = getDataContractFixture();
    const dataContract = DataContract.fromBuffer(dataContractJs.toBuffer());

    validator = new JsonSchemaValidator(dataContract);
    validateProtocolVersionMock = this.sinonSandbox.stub().returns(new ValidationResultJs());

    validateDocumentJs = validateDocumentFactoryJs(
      validatorJs,
      enrichDataContractWithBaseSchemaJs,
      validateProtocolVersionMock,
    );

    documents = getDocumentsFixture(dataContractJs);
    rawDocuments = documents.map((o) => o.toObject());
    [rawDocument] = rawDocuments;
  });

  describe('Base schema', () => {
    describe('$protocolVersion', () => {
      it('should be present', () => {
        delete rawDocument.$protocolVersion;

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$protocolVersion');
      });

      it('should be an integer', () => {
        rawDocument.$protocolVersion = '1';

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$protocolVersion');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should be valid', async () => {
        rawDocument.$protocolVersion = -1;

        const protocolVersionError = new SomeConsensusError('test');
        const protocolVersionResult = new ValidationResultJs([
          protocolVersionError,
        ]);

        validateProtocolVersionMock.returns(protocolVersionResult);

        const result = await validateDocumentJs(rawDocument, dataContractJs);

        expectValidationError(result, SomeConsensusError);

        const [error] = result.getErrors();

        expect(error).to.equal(protocolVersionError);

        expect(validateProtocolVersionMock).to.be.calledOnceWith(
          rawDocument.$protocolVersion,
        );
      });
    });

    describe('$id', () => {
      it('should be present', () => {
        delete rawDocument.$id;

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$id');
      });

      it('should be a byte array', () => {
        rawDocument.$id = new Array(32).fill('string');

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result, 2);

        const [error, byteArrayError] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$id/0');
        expect(error.getKeyword()).to.equal('type');

        expect(byteArrayError.getKeyword()).to.equal('byteArray');
      });

      it('should be no less than 32 bytes', () => {
        rawDocument.$id = Buffer.alloc(31);

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$id');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 32 bytes', () => {
        rawDocument.$id = Buffer.alloc(33);

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$id');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });

    describe('$type', () => {
      let DataContractMock;

      afterEach(() => {
        if (DataContractMock) {
          DataContractMock.restore();
        }
      });

      it('should be present', () => {
        delete rawDocument.$type;

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectValidationError(
          result,
          MissingDocumentTypeErrorJs,
        );

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(1028);
      });

      it('should be defined in Data Contract', () => {
        rawDocument.$type = 'undefinedDocument';

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectValidationError(
          result,
          InvalidDocumentTypeErrorJs,
        );

        const [error] = result.getErrors();

        expect(error.getCode()).to.equal(1024);
        expect(error.getType()).to.equal('undefinedDocument');
      });

      it('should throw an error if getDocumentSchemaRef throws error', function it() {
        const someError = new Error();

        DataContractMock = this.sinonSandbox.stub(DataContractJs.prototype, 'getDocumentSchemaRef').throws(someError);

        let error;
        try {
          validateDocumentJs(rawDocument, dataContractJs);
        } catch (e) {
          error = e;
        }

        expect(error).to.equal(someError);

        expect(dataContractJs.getDocumentSchemaRef).to.have.been.calledOnce();
      });
    });

    describe('$revision', () => {
      it('should be present', () => {
        delete rawDocument.$revision;

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$revision');
      });

      it('should be a number', () => {
        rawDocument.$revision = 'string';

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$revision');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should be an integer', () => {
        rawDocument.$revision = 1.1;

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$revision');
        expect(error.getKeyword()).to.equal('type');
      });

      it('should be greater or equal to one', () => {
        rawDocument.$revision = -1;

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$revision');
        expect(error.getKeyword()).to.equal('minimum');
      });
    });

    describe('$dataContractId', () => {
      it('should be present', () => {
        delete rawDocument.$dataContractId;

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$dataContractId');
      });

      it('should be a byte array', () => {
        rawDocument.$dataContractId = new Array(32).fill('string');

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result, 2);

        const [error, byteArrayError] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$dataContractId/0');
        expect(error.getKeyword()).to.equal('type');

        expect(byteArrayError.getKeyword()).to.equal('byteArray');
      });

      it('should be no less than 32 bytes', () => {
        rawDocument.$dataContractId = Buffer.alloc(31);

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$dataContractId');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 32 bytes', () => {
        rawDocument.$dataContractId = Buffer.alloc(33);

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/$dataContractId');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });

    describe('$ownerId', () => {
      it('should be present', () => {
        delete rawDocument.$ownerId;

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('');
        expect(error.getKeyword()).to.equal('required');
        expect(error.getParams().missingProperty).to.equal('$ownerId');
      });

      it('should be a byte array', () => {
        rawDocument.$ownerId = new Array(32).fill('string');

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result, 2);

        const [error, byteArrayError] = result.getErrors();

        expect(error.instancePath).to.equal('/$ownerId/0');
        expect(error.getKeyword()).to.equal('type');

        expect(byteArrayError.getKeyword()).to.equal('byteArray');
      });

      it('should be no less than 32 bytes', () => {
        rawDocument.$ownerId = Buffer.alloc(31);

        const result = validateDocumentJs(rawDocument, dataContractJs);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/$ownerId');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 32 bytes', () => {
        rawDocument.$ownerId = Buffer.alloc(33);

        const result = validateDocumentJs(rawDocument, dataContractJs);

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

      const result = validateDocumentJs(rawDocument, dataContractJs);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/name');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should return an error if the second document is not valid against Data Contract', () => {
      // eslint-disable-next-line prefer-destructuring
      rawDocument = rawDocuments[1];
      rawDocument.undefined = 1;

      const result = validateDocumentJs(rawDocument, dataContractJs);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.getKeyword()).to.equal('additionalProperties');
    });
  });

  it('return invalid result if a byte array exceeds `maxItems`', () => {
    // eslint-disable-next-line prefer-destructuring
    rawDocument = getDocumentsFixture(dataContractJs)[8].toObject();

    rawDocument.byteArrayField = Buffer.alloc(32);

    const result = validateDocumentJs(rawDocument, dataContractJs);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.instancePath).to.equal('/byteArrayField');
    expect(error.getKeyword()).to.equal('maxItems');
  });

  it('should return valid result is a document is valid', () => {
    const result = validateDocumentJs(rawDocument, dataContractJs);

    expect(result).to.be.an.instanceOf(ValidationResultJs);
    expect(result.isValid()).to.be.true();
  });
});
